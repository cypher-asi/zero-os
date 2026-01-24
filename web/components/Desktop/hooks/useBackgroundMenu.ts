/**
 * Background Menu Hook
 *
 * Manages background menu state and handlers.
 */

import { useState, useCallback, useEffect } from 'react';
import type { BackgroundMenuState, BackgroundInfo, WorkspaceInfo, DesktopBackgroundType } from '../types';

interface UseBackgroundMenuProps {
  backgroundRef: React.MutableRefObject<DesktopBackgroundType | null>;
  workspaceInfoRef: React.MutableRefObject<WorkspaceInfo | null>;
}

interface UseBackgroundMenuResult {
  backgroundMenu: BackgroundMenuState;
  setBackgroundMenu: React.Dispatch<React.SetStateAction<BackgroundMenuState>>;
  backgrounds: BackgroundInfo[];
  setBackgrounds: React.Dispatch<React.SetStateAction<BackgroundInfo[]>>;
  getActiveBackground: () => string;
  setBackground: (id: string) => void;
  handleBackgroundSelect: (id: string) => void;
  closeBackgroundMenu: () => void;
  handleBackgroundReady: () => void;
}

export function useBackgroundMenu({
  backgroundRef,
  workspaceInfoRef,
}: UseBackgroundMenuProps): UseBackgroundMenuResult {
  const [backgroundMenu, setBackgroundMenu] = useState<BackgroundMenuState>({
    x: 0,
    y: 0,
    visible: false,
  });
  const [backgrounds, setBackgrounds] = useState<BackgroundInfo[]>([]);

  // Get active background from workspace info
  const getActiveBackground = useCallback((): string => {
    if (workspaceInfoRef.current) {
      return workspaceInfoRef.current.backgrounds[workspaceInfoRef.current.actualActive] || 'grain';
    }
    return 'grain';
  }, [workspaceInfoRef]);

  // Set background via the background renderer
  const setBackground = useCallback(
    (id: string): void => {
      if (backgroundRef.current?.is_initialized()) {
        backgroundRef.current.set_background(id);
      }
    },
    [backgroundRef]
  );

  const closeBackgroundMenu = useCallback((): void => {
    setBackgroundMenu((prev) => ({ ...prev, visible: false }));
  }, []);

  const handleBackgroundSelect = useCallback(
    (id: string): void => {
      setBackground(id);
      closeBackgroundMenu();
    },
    [setBackground, closeBackgroundMenu]
  );

  // Callback when background renderer is ready
  const handleBackgroundReady = useCallback((): void => {
    if (backgroundRef.current) {
      try {
        const availableJson = backgroundRef.current.get_available_backgrounds();
        const available = JSON.parse(availableJson) as BackgroundInfo[];
        setBackgrounds(available);
      } catch (e) {
        console.error('[desktop] Failed to initialize backgrounds:', e);
      }
    }
  }, [backgroundRef]);

  // Close background menu when clicking outside
  useEffect(() => {
    if (!backgroundMenu.visible) return;

    const handleClickOutside = (): void => {
      closeBackgroundMenu();
    };

    // Small delay to prevent immediate close on right-click
    const timeoutId = setTimeout(() => {
      document.addEventListener('click', handleClickOutside);
    }, 10);

    return () => {
      clearTimeout(timeoutId);
      document.removeEventListener('click', handleClickOutside);
    };
  }, [backgroundMenu.visible, closeBackgroundMenu]);

  return {
    backgroundMenu,
    setBackgroundMenu,
    backgrounds,
    setBackgrounds,
    getActiveBackground,
    setBackground,
    handleBackgroundSelect,
    closeBackgroundMenu,
    handleBackgroundReady,
  };
}
