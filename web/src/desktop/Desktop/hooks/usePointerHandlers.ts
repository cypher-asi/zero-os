/**
 * Pointer Handlers Hook
 *
 * Manages pointer events for the desktop component.
 */

import { useCallback, useEffect } from 'react';
import type { DesktopController } from '../../hooks/useSupervisor';
import type { SelectionBox, BackgroundMenuState } from '../types';

interface UsePointerHandlersProps {
  desktop: DesktopController;
  initialized: boolean;
  containerRef: React.RefObject<HTMLDivElement | null>;
  backgroundMenu: BackgroundMenuState;
  setBackgroundMenu: React.Dispatch<React.SetStateAction<BackgroundMenuState>>;
  selectionBox: SelectionBox | null;
  setSelectionBox: React.Dispatch<React.SetStateAction<SelectionBox | null>>;
  /** When true, all pointer interactions are disabled (pre-auth lock) */
  isLocked?: boolean;
}

interface UsePointerHandlersResult {
  handlePointerDown: (e: React.PointerEvent) => void;
  handlePointerMove: (e: React.PointerEvent) => void;
  handlePointerUp: () => void;
  handlePointerLeave: () => void;
  handleWheel: (e: React.WheelEvent) => void;
  handleContextMenu: (e: React.MouseEvent) => void;
}

export function usePointerHandlers({
  desktop,
  initialized,
  containerRef,
  backgroundMenu,
  setBackgroundMenu,
  selectionBox,
  setSelectionBox,
  isLocked = false,
}: UsePointerHandlersProps): UsePointerHandlersResult {
  // Global pointer move/up handlers to catch drag events
  // Don't register when locked
  useEffect(() => {
    if (!initialized || isLocked) return;

    const handleGlobalPointerMove = (e: PointerEvent): void => {
      desktop.pointer_move(e.clientX, e.clientY);
    };

    const handleGlobalPointerUp = (): void => {
      desktop.pointer_up();
    };

    window.addEventListener('pointermove', handleGlobalPointerMove);
    window.addEventListener('pointerup', handleGlobalPointerUp);
    return () => {
      window.removeEventListener('pointermove', handleGlobalPointerMove);
      window.removeEventListener('pointerup', handleGlobalPointerUp);
    };
  }, [desktop, initialized, isLocked]);

  // Use capture phase for panning so it intercepts before windows
  // Don't register when locked
  useEffect(() => {
    if (isLocked) return;

    const container = containerRef.current;
    if (!container) return;

    const handleCapturePointerDown = (e: PointerEvent): void => {
      const isPanGesture = e.button === 1 || (e.button === 0 && (e.ctrlKey || e.shiftKey));
      if (isPanGesture) {
        const result = JSON.parse(
          desktop.pointer_down(e.clientX, e.clientY, e.button, e.ctrlKey, e.shiftKey)
        );
        if (result.type === 'handled') {
          e.preventDefault();
          e.stopPropagation();
        }
      }
    };

    container.addEventListener('pointerdown', handleCapturePointerDown, { capture: true });
    return () =>
      container.removeEventListener('pointerdown', handleCapturePointerDown, { capture: true });
  }, [desktop, containerRef, isLocked]);

  // Forward pointer events to Rust (bubble phase for normal interactions)
  // All handlers are no-ops when locked
  const handlePointerDown = useCallback(
    (e: React.PointerEvent) => {
      if (isLocked) return;

      // Close background menu only if clicking directly on the desktop or canvas
      const target = e.target as HTMLElement;
      const isDesktopClick = target === containerRef.current || target.tagName === 'CANVAS';

      if (backgroundMenu.visible && isDesktopClick) {
        setBackgroundMenu({ ...backgroundMenu, visible: false });
        return; // Don't process further, just close the menu
      }

      const result = JSON.parse(
        desktop.pointer_down(e.clientX, e.clientY, e.button, e.ctrlKey, e.shiftKey)
      );
      if (result.type === 'handled') {
        e.preventDefault();
      }

      // Start selection box on left-click directly on desktop background
      if (
        e.button === 0 &&
        !e.ctrlKey &&
        !e.shiftKey &&
        result.type !== 'handled' &&
        e.target === containerRef.current
      ) {
        setSelectionBox({
          startX: e.clientX,
          startY: e.clientY,
          currentX: e.clientX,
          currentY: e.clientY,
        });
      }
    },
    [desktop, backgroundMenu, setBackgroundMenu, setSelectionBox, containerRef, isLocked]
  );

  const handlePointerMove = useCallback(
    (e: React.PointerEvent) => {
      if (isLocked) return;

      desktop.pointer_move(e.clientX, e.clientY);

      if (selectionBox) {
        setSelectionBox((prev) =>
          prev ? { ...prev, currentX: e.clientX, currentY: e.clientY } : null
        );
      }
    },
    [desktop, selectionBox, setSelectionBox, isLocked]
  );

  const handlePointerUp = useCallback(() => {
    if (isLocked) return;

    desktop.pointer_up();
    setSelectionBox(null);
  }, [desktop, setSelectionBox, isLocked]);

  const handlePointerLeave = useCallback(() => {
    if (isLocked) return;

    desktop.pointer_up();
    setSelectionBox(null);
  }, [desktop, setSelectionBox, isLocked]);

  const handleWheel = useCallback(
    (e: React.WheelEvent) => {
      if (isLocked) return;

      if (e.ctrlKey) {
        desktop.wheel(e.deltaX, e.deltaY, e.clientX, e.clientY, e.ctrlKey);
      }
    },
    [desktop, isLocked]
  );

  // Handle right-click for background menu
  const handleContextMenu = useCallback(
    (e: React.MouseEvent) => {
      if (isLocked) return;

      // Only show background menu when right-clicking on the desktop background itself
      if (e.target === containerRef.current || (e.target as HTMLElement).tagName === 'CANVAS') {
        e.preventDefault();
        setBackgroundMenu({
          x: e.clientX,
          y: e.clientY,
          visible: true,
        });
      }
    },
    [setBackgroundMenu, containerRef, isLocked]
  );

  return {
    handlePointerDown,
    handlePointerMove,
    handlePointerUp,
    handlePointerLeave,
    handleWheel,
    handleContextMenu,
  };
}
