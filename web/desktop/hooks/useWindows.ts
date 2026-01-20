import { useState, useEffect, useCallback } from 'react';
import { useDesktopController, useSupervisor } from './useSupervisor';

// =============================================================================
// Window Types
// =============================================================================

// Window info with screen-space rect (for React positioning)
// Note: This is returned by Rust's tick_frame() in the unified render loop.
// Components that need screen rects should receive them as props from Desktop.tsx,
// NOT by polling independently (which causes animation jank).
export interface WindowInfo {
  id: number;
  title: string;
  appId: string;
  state: 'normal' | 'minimized' | 'maximized' | 'fullscreen';
  focused: boolean;
  zOrder: number;
  opacity: number;
  contentInteractive: boolean;
  screenRect: {
    x: number;
    y: number;
    width: number;
    height: number;
  };
}

// Basic window data (for taskbar, window lists - not animation-critical)
export interface WindowData {
  id: number;
  title: string;
  appId: string;
  position: { x: number; y: number };
  size: { width: number; height: number };
  state: 'normal' | 'minimized' | 'maximized' | 'fullscreen';
  zOrder: number;
  focused: boolean;
}

// =============================================================================
// DEPRECATED: useWindowScreenRects
// =============================================================================
// This hook has been removed. Window screen rects are now provided by the
// unified render loop in Desktop.tsx via Rust's tick_frame() method.
// This ensures windows and background are always in sync during animations.
// =============================================================================

// Hook to get all windows data
export function useWindows(): WindowData[] {
  const desktop = useDesktopController();
  const [windows, setWindows] = useState<WindowData[]>([]);

  useEffect(() => {
    if (!desktop) return;

    const update = () => {
      try {
        const json = desktop.get_windows_json();
        const parsed = JSON.parse(json) as WindowData[];
        setWindows(parsed);
      } catch (e) {
        console.error('Failed to parse windows:', e);
      }
    };

    // Update periodically
    update();
    const interval = setInterval(update, 100);
    return () => clearInterval(interval);
  }, [desktop]);

  return windows;
}

// Hook to get focused window ID
export function useFocusedWindow(): number | null {
  const desktop = useDesktopController();
  const [focusedId, setFocusedId] = useState<number | null>(null);

  useEffect(() => {
    if (!desktop) return;

    const update = () => {
      const id = desktop.get_focused_window();
      setFocusedId(id !== undefined ? Number(id) : null);
    };

    update();
    const interval = setInterval(update, 100);
    return () => clearInterval(interval);
  }, [desktop]);

  return focusedId;
}

// Hook for window actions
export function useWindowActions() {
  const desktop = useDesktopController();
  const supervisor = useSupervisor();

  const createWindow = useCallback(
    (title: string, x: number, y: number, w: number, h: number, appId: string, contentInteractive: boolean = false) => {
      if (!desktop) return null;
      return Number(desktop.create_window(title, x, y, w, h, appId, contentInteractive));
    },
    [desktop]
  );

  const closeWindow = useCallback(
    (id: number) => {
      if (!desktop) return;
      
      // Get the process ID associated with this window (if any)
      const processId = desktop.get_window_process_id(BigInt(id));
      
      // Close the window
      desktop.close_window(BigInt(id));
      
      // Kill the associated process if it exists
      if (processId !== undefined && supervisor) {
        console.log(`[useWindows] Killing process ${processId} for window ${id}`);
        supervisor.kill_process(Number(processId));
      }
    },
    [desktop, supervisor]
  );

  const focusWindow = useCallback(
    (id: number) => {
      desktop?.focus_window(BigInt(id));
    },
    [desktop]
  );

  const panToWindow = useCallback(
    (id: number) => {
      desktop?.pan_to_window(BigInt(id));
    },
    [desktop]
  );

  const minimizeWindow = useCallback(
    (id: number) => {
      desktop?.minimize_window(BigInt(id));
    },
    [desktop]
  );

  const maximizeWindow = useCallback(
    (id: number) => {
      desktop?.maximize_window(BigInt(id));
    },
    [desktop]
  );

  const restoreWindow = useCallback(
    (id: number) => {
      desktop?.restore_window(BigInt(id));
    },
    [desktop]
  );

  const launchApp = useCallback(
    (appId: string) => {
      if (!desktop) return null;
      return Number(desktop.launch_app(appId));
    },
    [desktop]
  );

  return {
    createWindow,
    closeWindow,
    focusWindow,
    panToWindow,
    minimizeWindow,
    maximizeWindow,
    restoreWindow,
    launchApp,
  };
}
