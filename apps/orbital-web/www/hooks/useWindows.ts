import { useState, useEffect, useCallback } from 'react';
import { useSupervisor } from './useSupervisor';

// Window info from Rust
export interface WindowInfo {
  id: number;
  title: string;
  appId: string;
  state: 'normal' | 'minimized' | 'maximized' | 'fullscreen';
  focused: boolean;
  zOrder: number;
  screenRect: {
    x: number;
    y: number;
    width: number;
    height: number;
  };
}

// Full window data from Rust
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

// Hook to get window screen rects (for positioning React overlays)
export function useWindowScreenRects(): WindowInfo[] {
  const supervisor = useSupervisor();
  const [windows, setWindows] = useState<WindowInfo[]>([]);

  useEffect(() => {
    if (!supervisor) return;

    const update = () => {
      try {
        const json = supervisor.get_window_screen_rects_json();
        const parsed = JSON.parse(json) as WindowInfo[];
        setWindows(parsed);
      } catch (e) {
        console.error('Failed to parse window rects:', e);
      }
    };

    // Update every animation frame
    let animationId: number;
    const animate = () => {
      update();
      animationId = requestAnimationFrame(animate);
    };
    animate();

    return () => cancelAnimationFrame(animationId);
  }, [supervisor]);

  return windows;
}

// Hook to get all windows data
export function useWindows(): WindowData[] {
  const supervisor = useSupervisor();
  const [windows, setWindows] = useState<WindowData[]>([]);

  useEffect(() => {
    if (!supervisor) return;

    const update = () => {
      try {
        const json = supervisor.get_windows_json();
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
  }, [supervisor]);

  return windows;
}

// Hook to get focused window ID
export function useFocusedWindow(): number | null {
  const supervisor = useSupervisor();
  const [focusedId, setFocusedId] = useState<number | null>(null);

  useEffect(() => {
    if (!supervisor) return;

    const update = () => {
      const id = supervisor.get_focused_window();
      setFocusedId(id !== undefined ? Number(id) : null);
    };

    update();
    const interval = setInterval(update, 100);
    return () => clearInterval(interval);
  }, [supervisor]);

  return focusedId;
}

// Hook for window actions
export function useWindowActions() {
  const supervisor = useSupervisor();

  const createWindow = useCallback(
    (title: string, x: number, y: number, w: number, h: number, appId: string) => {
      if (!supervisor) return null;
      return Number(supervisor.create_window(title, x, y, w, h, appId));
    },
    [supervisor]
  );

  const closeWindow = useCallback(
    (id: number) => {
      supervisor?.close_window(BigInt(id));
    },
    [supervisor]
  );

  const focusWindow = useCallback(
    (id: number) => {
      supervisor?.focus_window(BigInt(id));
    },
    [supervisor]
  );

  const minimizeWindow = useCallback(
    (id: number) => {
      supervisor?.minimize_window(BigInt(id));
    },
    [supervisor]
  );

  const maximizeWindow = useCallback(
    (id: number) => {
      supervisor?.maximize_window(BigInt(id));
    },
    [supervisor]
  );

  const restoreWindow = useCallback(
    (id: number) => {
      supervisor?.restore_window(BigInt(id));
    },
    [supervisor]
  );

  const launchApp = useCallback(
    (appId: string) => {
      if (!supervisor) return null;
      return Number(supervisor.launch_app(appId));
    },
    [supervisor]
  );

  return {
    createWindow,
    closeWindow,
    focusWindow,
    minimizeWindow,
    maximizeWindow,
    restoreWindow,
    launchApp,
  };
}
