import { useEffect, useRef, useState, useCallback } from 'react';
import { PanelDrill, type PanelDrillItem } from '@cypher-asi/zui';
import { PanelDrillProvider } from './context';
import { IdentityPanelContent } from './IdentityPanelContent';
import { AuthPanel, type AuthView } from './modals';
import styles from './IdentityPanel.module.css';

interface IdentityPanelProps {
  onClose: () => void;
  containerRef?: React.RefObject<HTMLDivElement>;
}

export function IdentityPanel({ onClose, containerRef }: IdentityPanelProps) {
  const panelRef = useRef<HTMLDivElement>(null);

  // Unified auth panel state - null means closed, otherwise shows the specified view
  const [authPanelView, setAuthPanelView] = useState<AuthView | null>(null);

  // Use refs to avoid recreating content on every render while keeping callbacks fresh
  const onCloseRef = useRef(onClose);
  onCloseRef.current = onClose;

  // Push a new panel onto the stack
  const pushPanel = useCallback((item: PanelDrillItem) => {
    setStack((prev) => [...prev, item]);
  }, []);

  const pushPanelRef = useRef(pushPanel);
  pushPanelRef.current = pushPanel;

  // Initialize stack with root item content immediately to avoid layout shift
  const [stack, setStack] = useState<PanelDrillItem[]>(() => [
    {
      id: 'identity',
      label: 'Identity',
      content: (
        <IdentityPanelContent
          onClose={() => onCloseRef.current()}
          onShowLoginModal={() => setAuthPanelView('login')}
          onShowRegisterWizard={() => setAuthPanelView('register')}
          onPushPanel={(item) => pushPanelRef.current(item)}
        />
      ),
    },
  ]);

  // Navigate back one level in the stack
  const navigateBack = useCallback(() => {
    setStack((prev) => {
      if (prev.length <= 1) return prev;
      return prev.slice(0, -1);
    });
  }, []);

  // Handle breadcrumb navigation
  const handleNavigate = useCallback((_id: string, index: number) => {
    setStack((prev) => prev.slice(0, index + 1));
  }, []);

  // Close auth panel
  const handleCloseAuthPanel = useCallback(() => {
    setAuthPanelView(null);
  }, []);

  // Click outside handler
  useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      const target = event.target as Node;
      // Ignore clicks inside the panel
      if (panelRef.current && panelRef.current.contains(target)) {
        return;
      }
      // Ignore clicks on the container (includes the toggle button)
      if (containerRef?.current && containerRef.current.contains(target)) {
        return;
      }
      onClose();
    };

    document.addEventListener('mousedown', handleClickOutside);
    return () => {
      document.removeEventListener('mousedown', handleClickOutside);
    };
  }, [onClose, containerRef]);

  return (
    <div ref={panelRef} className={styles.panelWrapper}>
      <PanelDrillProvider onNavigateBack={navigateBack} onPushPanel={pushPanel}>
        <PanelDrill
          stack={stack}
          onNavigate={handleNavigate}
          showBreadcrumb={true}
          className={styles.panel}
          variant="glass"
          border="future"
        />
      </PanelDrillProvider>

      {/* Unified Auth Panel - Single overlay for login/register views */}
      {authPanelView && (
        <AuthPanel
          initialView={authPanelView}
          onClose={handleCloseAuthPanel}
          onSelfSovereignSelected={() => {
            // TODO: Navigate to Neural Key panel
            setAuthPanelView(null);
          }}
        />
      )}
    </div>
  );
}
