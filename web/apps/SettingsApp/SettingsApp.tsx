import { useState, useCallback, useMemo, useRef, useEffect } from 'react';
import { Panel, Navigator, type NavigatorItem, PanelDrill, type PanelDrillItem } from '@cypher-asi/zui';
import { Clock, User, Shield, Palette } from 'lucide-react';
import { GeneralPanel } from './panels/GeneralPanel';
import { IdentitySettingsPanel } from './panels/IdentitySettingsPanel';
import { PermissionsPanel } from './panels/PermissionsPanel';
import { ThemePanel } from './panels/ThemePanel';
import styles from './SettingsApp.module.css';

// Custom event type for settings navigation
declare global {
  interface WindowEventMap {
    'settings:navigate': CustomEvent<{ section: SettingsArea }>;
  }
}

// Pending navigation state - used when Settings is opened from another component
// This handles the race condition where the event is dispatched before Settings mounts
let pendingNavigation: SettingsArea | null = null;

export function setPendingSettingsNavigation(section: SettingsArea) {
  pendingNavigation = section;
}

// Settings areas
type SettingsArea = 'general' | 'identity' | 'permissions' | 'theme';

interface SettingsState {
  activeArea: SettingsArea;
  // General
  timeFormat24h: boolean;
  timezone: string;
  // Identity summary
  hasNeuralKey: boolean;
  machineKeyCount: number;
  linkedAccountCount: number;
}

const DEFAULT_STATE: SettingsState = {
  activeArea: 'general',
  timeFormat24h: false,
  timezone: 'UTC',
  hasNeuralKey: false,
  machineKeyCount: 0,
  linkedAccountCount: 0,
};

// Area labels
const AREA_LABELS: Record<SettingsArea, string> = {
  general: 'General',
  identity: 'Identity',
  permissions: 'Permissions',
  theme: 'Theme',
};

/**
 * Settings App - System settings management
 *
 * Uses ZUI components: Panel, Navigator, PanelDrill
 * Layout: Split-pane with left navigation and right PanelDrill content
 */
export function SettingsApp() {
  const [state, setState] = useState<SettingsState>(DEFAULT_STATE);
  
  // Use ref for pushPanel to avoid circular dependency in content creation
  const pushPanelRef = useRef<(item: PanelDrillItem) => void>(() => {});

  // Helper to create content for a given area
  // Called once when switching sections, not on every render
  const createContentForArea = useCallback((area: SettingsArea): React.ReactNode => {
    switch (area) {
      case 'general':
        return (
          <GeneralPanel
            timeFormat24h={state.timeFormat24h}
            timezone={state.timezone}
            onTimeFormatChange={(value) =>
              setState((prev) => ({ ...prev, timeFormat24h: value }))
            }
            onTimezoneChange={(value) =>
              setState((prev) => ({ ...prev, timezone: value }))
            }
          />
        );
      case 'identity':
        return (
          <IdentitySettingsPanel
            hasNeuralKey={state.hasNeuralKey}
            machineKeyCount={state.machineKeyCount}
            linkedAccountCount={state.linkedAccountCount}
          />
        );
      case 'permissions':
        // Use ref to avoid recreating when pushPanel changes
        return <PermissionsPanel onDrillDown={(item) => pushPanelRef.current(item)} />;
      case 'theme':
        return <ThemePanel />;
    }
  }, [state.timeFormat24h, state.timezone, state.hasNeuralKey, state.machineKeyCount, state.linkedAccountCount]);

  // Initialize stack with root item - content created once on mount
  const [stack, setStack] = useState<PanelDrillItem[]>(() => [{
    id: 'general',
    label: AREA_LABELS.general,
    content: (
      <GeneralPanel
        timeFormat24h={DEFAULT_STATE.timeFormat24h}
        timezone={DEFAULT_STATE.timezone}
        onTimeFormatChange={(value) =>
          setState((prev) => ({ ...prev, timeFormat24h: value }))
        }
        onTimezoneChange={(value) =>
          setState((prev) => ({ ...prev, timezone: value }))
        }
      />
    ),
  }]);

  // Navigation items
  const navItems: NavigatorItem[] = useMemo(
    () => [
      {
        id: 'general',
        label: 'General',
        icon: <Clock size={14} />,
      },
      {
        id: 'identity',
        label: 'Identity',
        icon: <User size={14} />,
      },
      {
        id: 'permissions',
        label: 'Permissions',
        icon: <Shield size={14} />,
      },
      {
        id: 'theme',
        label: 'Theme',
        icon: <Palette size={14} />,
      },
    ],
    []
  );

  // Push a new panel onto the stack
  const pushPanel = useCallback((item: PanelDrillItem) => {
    setStack(prev => [...prev, item]);
  }, []);

  // Keep ref in sync with pushPanel
  useEffect(() => {
    pushPanelRef.current = pushPanel;
  }, [pushPanel]);

  // Handle area selection from left menu - reset stack to new section
  const handleAreaSelect = useCallback((id: string) => {
    const area = id as SettingsArea;
    setState((prev) => ({ ...prev, activeArea: area }));
    // Reset stack to just the new section's root item
    setStack([{
      id: area,
      label: AREA_LABELS[area],
      content: createContentForArea(area),
    }]);
  }, [createContentForArea]);

  // Handle breadcrumb navigation - truncate stack to clicked index
  const handleNavigate = useCallback((_id: string, index: number) => {
    setStack(prev => prev.slice(0, index + 1));
  }, []);

  // Update root panel content when relevant state changes
  // This ensures GeneralPanel sees updated values after user interaction
  useEffect(() => {
    setStack(prev => {
      if (prev.length === 0) return prev;
      const rootArea = prev[0].id as SettingsArea;
      // Only update if we're at root level (no drill-down) to preserve drill state
      if (prev.length === 1) {
        return [{
          ...prev[0],
          content: createContentForArea(rootArea),
        }];
      }
      return prev;
    });
  }, [state.timeFormat24h, state.timezone, state.hasNeuralKey, state.machineKeyCount, state.linkedAccountCount, createContentForArea]);

  // Listen for external navigation events (e.g., from Identity Panel)
  useEffect(() => {
    const handleNavigateEvent = (event: CustomEvent<{ section: SettingsArea }>) => {
      const { section } = event.detail;
      console.log('[SettingsApp] Received navigation event:', section);
      
      // Navigate to the requested section
      if (section && AREA_LABELS[section]) {
        handleAreaSelect(section);
      }
    };

    window.addEventListener('settings:navigate', handleNavigateEvent);
    return () => {
      window.removeEventListener('settings:navigate', handleNavigateEvent);
    };
  }, [handleAreaSelect]);

  // Check for pending navigation on mount (handles race condition when Settings is opened)
  // Note: We intentionally use a ref to capture handleAreaSelect to avoid re-running
  // this effect when handleAreaSelect changes - we only want to check pending navigation once
  const handleAreaSelectRef = useRef(handleAreaSelect);
  handleAreaSelectRef.current = handleAreaSelect;
  
  useEffect(() => {
    if (pendingNavigation) {
      console.log('[SettingsApp] Found pending navigation:', pendingNavigation);
      handleAreaSelectRef.current(pendingNavigation);
      pendingNavigation = null;
    }
  }, []); // Only run on mount

  return (
    <Panel border="none" className={styles.container}>
      {/* Left Navigation Sidebar */}
      <div className={styles.sidebar}>
        <Navigator
          items={navItems}
          value={state.activeArea}
          onChange={handleAreaSelect}
          background="none"
          border="none"
        />
      </div>

      {/* Right Content Area with PanelDrill */}
      <div className={styles.content}>
        <PanelDrill
          stack={stack}
          onNavigate={handleNavigate}
          showBreadcrumb={true}
          variant="default"
          className={styles.panelDrill}
        />
      </div>
    </Panel>
  );
}
