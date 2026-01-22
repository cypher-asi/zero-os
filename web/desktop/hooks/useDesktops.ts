import { useState, useEffect, useCallback, useRef } from 'react';
import { useDesktopController } from './useSupervisor';
import type { LayerOpacities, ViewMode } from '../types';

// Desktop info from Rust
export interface DesktopInfo {
  id: number;
  name: string;
  active: boolean;
  windowCount: number;
}

const DESKTOP_STORAGE_KEY = 'zero-desktop-settings';

// Hook to get all desktops
export function useDesktops(): DesktopInfo[] {
  const desktop = useDesktopController();
  const [desktops, setDesktops] = useState<DesktopInfo[]>([]);

  useEffect(() => {
    if (!desktop) return;

    const update = () => {
      try {
        const json = desktop.get_desktops_json();
        const parsed = JSON.parse(json) as DesktopInfo[];
        setDesktops(parsed);
      } catch (e) {
        console.error('Failed to parse desktops:', e);
      }
    };

    update();
    const interval = setInterval(update, 200);
    return () => clearInterval(interval);
  }, [desktop]);

  return desktops;
}

// Hook to get active desktop index
export function useActiveDesktop(): number {
  const desktop = useDesktopController();
  const [active, setActive] = useState(0);

  useEffect(() => {
    if (!desktop) return;

    const update = () => {
      setActive(desktop.get_active_desktop());
    };

    update();
    const interval = setInterval(update, 200);
    return () => clearInterval(interval);
  }, [desktop]);

  return active;
}

// Hook for desktop actions
export function useDesktopActions() {
  const desktop = useDesktopController();

  const createDesktop = useCallback(
    (name: string) => {
      if (!desktop) return null;
      return desktop.create_desktop(name);
    },
    [desktop]
  );

  const switchDesktop = useCallback(
    (index: number) => {
      desktop?.switch_desktop(index);
    },
    [desktop]
  );

  return {
    createDesktop,
    switchDesktop,
  };
}

// Hook to get the current view mode
export function useViewMode(): ViewMode {
  const desktop = useDesktopController();
  const [viewMode, setViewMode] = useState<ViewMode>('desktop');

  useEffect(() => {
    if (!desktop) return;

    const update = () => {
      try {
        const mode = desktop.get_view_mode() as string;
        // Map legacy 'workspace' to 'desktop'
        if (mode === 'workspace') {
          setViewMode('desktop');
        } else {
          setViewMode(mode as ViewMode);
        }
      } catch (e) {
        // DesktopController may not have this method yet
      }
    };

    update();
    const interval = setInterval(update, 100); // More frequent for responsive UI
    return () => clearInterval(interval);
  }, [desktop]);

  return viewMode;
}

// Hook to check if in void mode
export function useIsInVoid(): boolean {
  const desktop = useDesktopController();
  const [isInVoid, setIsInVoid] = useState(false);

  useEffect(() => {
    if (!desktop) return;

    const update = () => {
      try {
        setIsInVoid(desktop.is_in_void());
      } catch (e) {
        // DesktopController may not have this method yet
      }
    };

    update();
    const interval = setInterval(update, 100);
    return () => clearInterval(interval);
  }, [desktop]);

  return isInVoid;
}

// Hook for void actions
export function useVoidActions() {
  const desktop = useDesktopController();

  const enterVoid = useCallback(() => {
    desktop?.enter_void();
  }, [desktop]);

  const exitVoid = useCallback(
    (desktopIndex: number) => {
      desktop?.exit_void(desktopIndex);
    },
    [desktop]
  );

  return { enterVoid, exitVoid };
}

// Hook to get layer opacities during crossfade transitions
// Returns { desktop: number, void: number } where values are 0.0-1.0
export function useLayerOpacities(): LayerOpacities {
  const desktop = useDesktopController();
  const [opacities, setOpacities] = useState<LayerOpacities>({ desktop: 1.0, void: 0.0 });

  useEffect(() => {
    if (!desktop) return;

    const update = () => {
      try {
        const mode = desktop.get_view_mode() as string;
        const transitioning = desktop.is_animating_viewport?.() ?? false;

        if (transitioning) {
          // During transition, both layers visible with 50/50 opacity
          setOpacities({ desktop: 0.5, void: 0.5 });
        } else if (mode === 'workspace' || mode === 'desktop') {
          setOpacities({ desktop: 1.0, void: 0.0 });
        } else if (mode === 'void') {
          setOpacities({ desktop: 0.0, void: 1.0 });
        }
      } catch (e) {
        // Default to desktop visible
        setOpacities({ desktop: 1.0, void: 0.0 });
      }
    };

    update();
    const interval = setInterval(update, 50); // Fast updates for smooth transitions
    return () => clearInterval(interval);
  }, [desktop]);

  return opacities;
}

// =============================================================================
// Backward Compatibility Aliases (deprecated)
// =============================================================================

/** @deprecated Use DesktopInfo instead */
export type WorkspaceInfo = DesktopInfo;

/** @deprecated Use useDesktops instead */
export const useWorkspaces = useDesktops;

/** @deprecated Use useActiveDesktop instead */
export const useActiveWorkspace = useActiveDesktop;

/** @deprecated Use useDesktopActions instead */
export function useWorkspaceActions() {
  const actions = useDesktopActions();
  return {
    createWorkspace: actions.createDesktop,
    switchWorkspace: actions.switchDesktop,
  };
}
