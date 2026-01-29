import { createContext, useContext, type ReactNode } from 'react';
import type { PanelDrillItem } from '@cypher-asi/zui';

/**
 * PanelDrill navigation context for IdentityPanel
 *
 * Provides navigation functions for drill-down panels, replacing window events
 * with proper React data flow.
 */
interface PanelDrillContextValue {
  /** Navigate back one level in the panel stack */
  navigateBack: () => void;
  /** Push a new panel onto the stack */
  pushPanel: (item: PanelDrillItem) => void;
}

const PanelDrillContext = createContext<PanelDrillContextValue | null>(null);

interface PanelDrillProviderProps {
  children: ReactNode;
  onNavigateBack: () => void;
  onPushPanel: (item: PanelDrillItem) => void;
}

/**
 * Provider for PanelDrill navigation context
 *
 * Wrap your PanelDrill content with this provider to enable navigation
 * from child panels without using window events.
 */
export function PanelDrillProvider({
  children,
  onNavigateBack,
  onPushPanel,
}: PanelDrillProviderProps): ReactNode {
  const value: PanelDrillContextValue = {
    navigateBack: onNavigateBack,
    pushPanel: onPushPanel,
  };

  return <PanelDrillContext.Provider value={value}>{children}</PanelDrillContext.Provider>;
}

/**
 * Hook to access PanelDrill navigation functions
 *
 * @returns Navigation functions for the current PanelDrill context
 * @throws Error if used outside of PanelDrillProvider
 *
 * @example
 * ```tsx
 * function MyPanel() {
 *   const { navigateBack, pushPanel } = usePanelDrill();
 *
 *   const handleCancel = () => navigateBack();
 *   const handleDrillDown = () => pushPanel({ id: 'details', label: 'Details', content: <Details /> });
 *
 *   return <Button onClick={handleCancel}>Cancel</Button>;
 * }
 * ```
 */
export function usePanelDrill(): PanelDrillContextValue {
  const context = useContext(PanelDrillContext);
  if (!context) {
    throw new Error('usePanelDrill must be used within a PanelDrillProvider');
  }
  return context;
}

/**
 * Hook to optionally access PanelDrill navigation functions
 *
 * Returns null if not within a PanelDrillProvider, allowing components
 * to work both inside and outside of a drill context.
 *
 * @returns Navigation functions or null if outside PanelDrillProvider
 */
export function usePanelDrillOptional(): PanelDrillContextValue | null {
  return useContext(PanelDrillContext);
}
