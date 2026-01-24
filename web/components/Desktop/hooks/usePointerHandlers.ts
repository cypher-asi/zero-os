/**
 * Pointer Handlers Hook
 *
 * Manages pointer events for the desktop component.
 */

import { useCallback, useEffect } from 'react';
import type { DesktopController } from '../../../desktop/hooks/useSupervisor';
import type { SelectionBox, BackgroundMenuState } from '../types';

interface UsePointerHandlersProps {
  desktop: DesktopController;
  initialized: boolean;
  containerRef: React.RefObject<HTMLDivElement | null>;
  backgroundMenu: BackgroundMenuState;
  setBackgroundMenu: React.Dispatch<React.SetStateAction<BackgroundMenuState>>;
  selectionBox: SelectionBox | null;
  setSelectionBox: React.Dispatch<React.SetStateAction<SelectionBox | null>>;
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
}: UsePointerHandlersProps): UsePointerHandlersResult {
  // Global pointer move/up handlers to catch drag events
  useEffect(() => {
    if (!initialized) return;

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
  }, [desktop, initialized]);

  // Use capture phase for panning so it intercepts before windows
  useEffect(() => {
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
  }, [desktop, containerRef]);

  // Forward pointer events to Rust (bubble phase for normal interactions)
  const handlePointerDown = useCallback(
    (e: React.PointerEvent) => {
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
    [desktop, backgroundMenu, setBackgroundMenu, setSelectionBox, containerRef]
  );

  const handlePointerMove = useCallback(
    (e: React.PointerEvent) => {
      desktop.pointer_move(e.clientX, e.clientY);

      if (selectionBox) {
        setSelectionBox((prev) =>
          prev ? { ...prev, currentX: e.clientX, currentY: e.clientY } : null
        );
      }
    },
    [desktop, selectionBox, setSelectionBox]
  );

  const handlePointerUp = useCallback(() => {
    desktop.pointer_up();
    setSelectionBox(null);
  }, [desktop, setSelectionBox]);

  const handlePointerLeave = useCallback(() => {
    desktop.pointer_up();
    setSelectionBox(null);
  }, [desktop, setSelectionBox]);

  const handleWheel = useCallback(
    (e: React.WheelEvent) => {
      if (e.ctrlKey) {
        desktop.wheel(e.deltaX, e.deltaY, e.clientX, e.clientY, e.ctrlKey);
      }
    },
    [desktop]
  );

  // Handle right-click for background menu
  const handleContextMenu = useCallback(
    (e: React.MouseEvent) => {
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
    [setBackgroundMenu, containerRef]
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
