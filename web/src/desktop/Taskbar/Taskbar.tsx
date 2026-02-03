import { useState, useEffect, useCallback, useRef } from 'react';
import { Button } from '@cypher-asi/zui';
import { useWindowActions } from '../hooks/useWindows';
import { useDesktopActions } from '../hooks/useDesktops';
import { useWindowStore, selectWindows, useDesktopStore, selectDesktops } from '@/stores';
import { BeginMenu } from './BeginMenu/BeginMenu';
import { IdentityPanel } from './IdentityPanel';
import { ProcessPanel } from './ProcessPanel';
import { DateTime } from './DateTime';
import {
  TerminalSquare,
  AppWindow,
  Circle,
  Plus,
  KeyRound,
  CreditCard,
  Settings,
  Bell,
  Cpu,
} from 'lucide-react';
import styles from './Taskbar.module.css';

// Get the appropriate icon for a window based on its title
function getWindowIcon(title: string) {
  const lowerTitle = title.toLowerCase();
  if (
    lowerTitle.includes('terminal') ||
    lowerTitle.includes('shell') ||
    lowerTitle.includes('bash')
  ) {
    return <TerminalSquare size={16} />;
  }
  if (lowerTitle.includes('settings')) {
    return <Settings size={16} />;
  }
  // Default icon for other apps
  return <AppWindow size={16} />;
}

interface TaskbarProps {
  /** When true, taskbar interactions are disabled (pre-auth lock) */
  isLocked?: boolean;
}

export function Taskbar({ isLocked = false }: TaskbarProps) {
  const [beginMenuOpen, setBeginMenuOpen] = useState(false);
  const [identityPanelOpen, setIdentityPanelOpen] = useState(false);
  const [processPanelOpen, setProcessPanelOpen] = useState(false);
  const beginSectionRef = useRef<HTMLDivElement>(null);
  const neuralKeyWrapperRef = useRef<HTMLDivElement>(null);
  const processPanelWrapperRef = useRef<HTMLDivElement>(null);

  // Use Zustand stores directly for better performance
  const windows = useWindowStore(selectWindows);
  const desktops = useDesktopStore(selectDesktops);

  const { focusWindow, panToWindow, restoreWindow } = useWindowActions();
  const { createDesktop, switchDesktop } = useDesktopActions();

  // Toggle begin menu with 'z' key when not in an input field
  const toggleBeginMenu = useCallback(() => {
    setBeginMenuOpen((prev) => !prev);
  }, []);

  useEffect(() => {
    // Don't register 'z' key listener when locked
    if (isLocked) return;

    const handleKeyDown = (e: KeyboardEvent) => {
      // Only respond to 'z' key without modifiers
      if (e.key !== 'z' || e.ctrlKey || e.shiftKey || e.altKey || e.metaKey) {
        return;
      }

      // Ignore if focus is in an input field
      const target = e.target as HTMLElement;
      const tagName = target.tagName.toLowerCase();
      if (tagName === 'input' || tagName === 'textarea' || target.isContentEditable) {
        return;
      }

      e.preventDefault();
      toggleBeginMenu();
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [toggleBeginMenu, isLocked]);

  const handleWindowClick = (
    e: React.MouseEvent,
    windowId: number,
    state: string,
    focused: boolean
  ) => {
    e.stopPropagation(); // Prevent event from bubbling to Desktop
    if (isLocked) return; // No-op when locked

    if (state === 'minimized') {
      restoreWindow(windowId);
      // Always pan to minimized windows when restoring
      panToWindow(windowId);
    } else if (!focused) {
      // Only pan to unfocused windows - clicking an already-focused window
      // should not move the viewport (preserves user's current view)
      focusWindow(windowId);
      panToWindow(windowId);
    }
    // If already focused and not minimized, do nothing - user already sees this window
  };

  const handleAddDesktop = () => {
    if (isLocked) return; // No-op when locked
    const count = desktops.length;
    createDesktop(`Desktop ${count + 1}`);
  };

  return (
    <div className={styles.taskbar} style={isLocked ? { pointerEvents: 'none' } : undefined}>
      {/* Begin Button - Left */}
      <div ref={beginSectionRef} className={styles.beginSection}>
        <Button
          variant={beginMenuOpen ? 'glass' : 'transparent'}
          rounded="none"
          iconOnly
          className={`${styles.beginBtn} ${beginMenuOpen ? styles.beginBtnActive : ''}`}
          onClick={() => !isLocked && setBeginMenuOpen(!beginMenuOpen)}
          title="Begin Menu (Press Z)"
          aria-label="Begin Menu (Press Z)"
          aria-expanded={beginMenuOpen}
          aria-haspopup="menu"
          selected={beginMenuOpen}
          selectedBgColor="transparent"
          disabled={isLocked}
        >
          <span className={styles.beginIcon}>
            <Circle size={16} className={styles.beginCircle} />
            <span className={styles.beginSlash}>/</span>
          </span>
        </Button>

        {!isLocked && beginMenuOpen && (
          <BeginMenu onClose={() => setBeginMenuOpen(false)} containerRef={beginSectionRef} />
        )}
      </div>

      {/* Active Windows - Center */}
      <div className={styles.windowsSection}>
        {/* Sort by id to maintain stable order regardless of focus/z-order changes */}
        {[...windows].sort((a, b) => a.id - b.id).map((win) => (
          <Button
            key={win.id}
            variant={win.focused ? 'glass' : 'transparent'}
            rounded="none"
            textCase="uppercase"
            icon={getWindowIcon(win.title)}
            className={`${styles.windowItem} ${win.state === 'minimized' ? styles.minimized : ''}`}
            onClick={(e) => handleWindowClick(e, win.id, win.state, win.focused)}
            title={win.title}
            selected={win.focused}
            selectedBgColor="transparent"
            disabled={isLocked}
          >
            <span className={styles.windowTitle}>{win.title}</span>
          </Button>
        ))}
      </div>

      {/* Desktop Indicators - Right */}
      <div className={styles.workspacesSection}>
        {desktops.map((d, i) => (
          <Button
            key={d.id}
            variant={d.active ? 'glass' : 'transparent'}
            rounded="none"
            iconOnly
            className={styles.workspaceBtn}
            onClick={() => !isLocked && switchDesktop(i)}
            title={d.name}
            aria-label={`Switch to ${d.name}`}
            aria-pressed={d.active}
            selected={d.active}
            selectedBgColor="transparent"
            disabled={isLocked}
          >
            {i + 1}
          </Button>
        ))}
        <Button
          variant="transparent"
          rounded="none"
          iconOnly
          className={styles.workspaceAdd}
          onClick={handleAddDesktop}
          title="Add desktop"
          aria-label="Add new desktop"
          selected={false}
          selectedBgColor="transparent"
          disabled={isLocked}
        >
          <Plus size={16} />
        </Button>
        <Button
          variant="transparent"
          rounded="none"
          iconOnly
          className={styles.walletBtn}
          onClick={() => !isLocked && console.log('[taskbar] Wallet clicked')}
          title="Wallet"
          aria-label="Open Wallet"
          selected={false}
          selectedBgColor="transparent"
          disabled={isLocked}
        >
          <CreditCard size={16} />
        </Button>
        <Button
          variant="transparent"
          rounded="none"
          iconOnly
          className={styles.notificationBtn}
          onClick={() => !isLocked && console.log('[taskbar] Notifications clicked')}
          title="Notifications"
          aria-label="Open Notifications"
          selected={false}
          selectedBgColor="transparent"
          disabled={isLocked}
        >
          <Bell size={16} />
        </Button>
        <div ref={processPanelWrapperRef} className={styles.processPanelWrapper}>
          <Button
            variant={processPanelOpen ? 'glass' : 'transparent'}
            rounded="none"
            iconOnly
            className={`${styles.processBtn} ${processPanelOpen ? styles.processBtnActive : ''}`}
            onClick={() => !isLocked && setProcessPanelOpen(!processPanelOpen)}
            title="Processes"
            aria-label="Open Processes"
            aria-expanded={processPanelOpen}
            aria-haspopup="true"
            selected={processPanelOpen}
            selectedBgColor="transparent"
            disabled={isLocked}
          >
            <Cpu size={16} />
          </Button>

          {!isLocked && processPanelOpen && (
            <ProcessPanel
              onClose={() => setProcessPanelOpen(false)}
              containerRef={processPanelWrapperRef}
            />
          )}
        </div>
        <DateTime />
        <div ref={neuralKeyWrapperRef} className={styles.neuralKeyWrapper}>
          <Button
            variant={identityPanelOpen ? 'glass' : 'transparent'}
            rounded="none"
            iconOnly
            className={`${styles.neuralKey} ${identityPanelOpen ? styles.neuralKeyActive : ''}`}
            onClick={() => !isLocked && setIdentityPanelOpen(!identityPanelOpen)}
            title="Neural Link - Identity & Security"
            aria-label="Neural Link - Identity & Security"
            aria-expanded={identityPanelOpen}
            aria-haspopup="true"
            selected={identityPanelOpen}
            selectedBgColor="transparent"
            disabled={isLocked}
          >
            <KeyRound size={16} />
          </Button>

          {!isLocked && identityPanelOpen && (
            <IdentityPanel
              onClose={() => setIdentityPanelOpen(false)}
              containerRef={neuralKeyWrapperRef}
            />
          )}
        </div>
      </div>
    </div>
  );
}
