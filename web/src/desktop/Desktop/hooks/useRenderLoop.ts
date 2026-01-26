/**
 * Render Loop Hook
 *
 * PERFORMANCE OPTIMIZATION: Direct DOM updates bypass React reconciliation.
 *
 * All animation logic lives in Rust. This hook:
 * 1. Runs a single RAF loop that calls Rust's tick_frame()
 * 2. Updates background renderer with viewport/workspace info
 * 3. Updates window positions DIRECTLY via DOM (not React state)
 * 4. Only triggers React re-render when window LIST changes (add/remove)
 *
 * This eliminates React reconciliation overhead during animations, achieving
 * smooth 60fps even with many windows.
 */

import { useRef, useEffect, useState, useCallback } from 'react';
import type { DesktopController } from '../../hooks/useSupervisor';
import { syncStoresFromFrame, resetSyncState } from '../../sync';
import type { WindowInfo, WorkspaceInfo, FrameData } from '@/stores/types';
import type { DesktopBackgroundType } from '../types';
import { windowListChanged } from '../types';

interface UseRenderLoopProps {
  desktop: DesktopController;
  backgroundRef: React.MutableRefObject<DesktopBackgroundType | null>;
  onBackgroundReady: () => void;
  workspaceInfoRef: React.MutableRefObject<WorkspaceInfo | null>;
  canvasRef: React.RefObject<HTMLCanvasElement | null>;
}

interface UseRenderLoopResult {
  windows: WindowInfo[];
  setWindowRef: (id: number, el: HTMLDivElement | null) => void;
}

/** Update background renderer with frame data */
function updateBackgroundRenderer(
  bg: DesktopBackgroundType,
  frame: FrameData,
  backgroundSwitchedRef: React.MutableRefObject<boolean>
): void {
  const { viewport, workspaceInfo, workspaceDimensions } = frame;

  bg.set_viewport(viewport.zoom, viewport.center.x, viewport.center.y);

  bg.set_workspace_info(
    workspaceInfo.count,
    workspaceInfo.active,
    JSON.stringify(workspaceInfo.backgrounds)
  );

  bg.set_workspace_dimensions(
    workspaceDimensions.width,
    workspaceDimensions.height,
    workspaceDimensions.gap
  );

  // Sync background renderer with active desktop's background from Rust state
  const targetBackground = workspaceInfo.backgrounds[workspaceInfo.actualActive] || 'grain';
  const currentRendererBg = bg.get_current_background();

  if (frame.transitioning) {
    // During transition: Switch background when visual workspace changes
    if (targetBackground !== currentRendererBg && !backgroundSwitchedRef.current) {
      if (workspaceInfo.active === workspaceInfo.actualActive) {
        bg.set_background(targetBackground);
        backgroundSwitchedRef.current = true;
      }
    }
  } else {
    // Transition complete: Ensure background is correct and reset switch flag
    if (targetBackground !== currentRendererBg) {
      bg.set_background(targetBackground);
    }
    backgroundSwitchedRef.current = false;
  }

  // CROSSFADE MODEL: Show void layer when in void mode or transitioning
  bg.set_transitioning(frame.showVoid);

  bg.render();
}

/** Update window DOM elements directly (bypass React) */
function updateWindowDom(
  win: WindowInfo,
  el: HTMLDivElement,
  fadingOutWindowsRef: React.MutableRefObject<Set<number>>,
  prevOpacityRef: React.MutableRefObject<Map<number, number>>
): void {
  if (win.state === 'minimized') return;

  const wasFadingOut = fadingOutWindowsRef.current.has(win.id);
  const targetOpacity = win.opacity;

  // Cancel fade-out if window reappeared with opacity > 0
  if (wasFadingOut && targetOpacity > 0) {
    fadingOutWindowsRef.current.delete(win.id);
  }

  // Make sure it's visible (in case it was hidden)
  el.style.visibility = 'visible';

  // Clear any CSS animations/transitions - opacity is driven directly by Rust
  if (el.style.animation !== 'none' && el.style.animation !== '') {
    el.style.animation = 'none';
  }
  if (el.style.transition !== 'none') {
    el.style.transition = 'none';
  }

  // Update previous opacity tracking
  prevOpacityRef.current.set(win.id, targetOpacity);

  // GPU-accelerated transform instead of left/top
  el.style.transform = `translate3d(${win.screenRect.x}px, ${win.screenRect.y}px, 0)`;
  el.style.width = `${win.screenRect.width}px`;
  el.style.height = `${win.screenRect.height}px`;
  el.style.zIndex = String(win.zOrder + 10);

  // CRITICAL: Override position to absolute if Panel set it to relative
  if (el.style.position !== 'absolute') {
    el.style.position = 'absolute';
    el.style.left = '0';
    el.style.top = '0';
  }

  // Always set opacity directly - Rust controls the smooth transitions
  el.style.opacity = String(win.opacity);
}

/** Hide windows that are filtered out during transitions */
function hideFilteredWindows(
  currentWindowIds: Set<number>,
  windowRefsMap: React.MutableRefObject<Map<number, HTMLDivElement>>,
  fadingOutWindowsRef: React.MutableRefObject<Set<number>>,
  pendingWindowsRef: React.MutableRefObject<WindowInfo[] | null>,
  windowIdsRef: React.MutableRefObject<Set<number>>,
  setWindows: React.Dispatch<React.SetStateAction<WindowInfo[]>>
): void {
  for (const [id, el] of windowRefsMap.current.entries()) {
    if (!currentWindowIds.has(id)) {
      // Window was filtered out - hide immediately (Rust already faded it)
      if (!fadingOutWindowsRef.current.has(id)) {
        fadingOutWindowsRef.current.add(id);
        el.style.visibility = 'hidden';

        // Capture id in closure for the timeout
        const windowId = id;

        // Small delay to ensure Rust transition completed, then clean up
        setTimeout(() => {
          fadingOutWindowsRef.current.delete(windowId);

          // If all fade-outs complete and we have pending windows, apply them
          if (fadingOutWindowsRef.current.size === 0 && pendingWindowsRef.current) {
            windowIdsRef.current = new Set(pendingWindowsRef.current.map((w) => w.id));
            setWindows(pendingWindowsRef.current);
            pendingWindowsRef.current = null;
          }
        }, 50);
      }
    }
  }
}

export function useRenderLoop({
  desktop,
  backgroundRef,
  onBackgroundReady,
  workspaceInfoRef,
  canvasRef,
}: UseRenderLoopProps): UseRenderLoopResult {
  const animationFrameRef = useRef<number | null>(null);

  // Window list state - only updated when windows are added/removed
  const [windows, setWindows] = useState<WindowInfo[]>([]);

  // Track window IDs to detect list changes
  const windowIdsRef = useRef<Set<number>>(new Set());

  // Map of window ID -> DOM element ref for direct position updates
  const windowRefsMap = useRef<Map<number, HTMLDivElement>>(new Map());

  // Store latest window data for each window (used by refs)
  const windowDataRef = useRef<Map<number, WindowInfo>>(new Map());

  // Track windows that are currently fading out
  const fadingOutWindowsRef = useRef<Set<number>>(new Set());

  // Track previous opacity for each window
  const prevOpacityRef = useRef<Map<number, number>>(new Map());

  // Store pending windows when we need to delay React update for fade-out
  const pendingWindowsRef = useRef<WindowInfo[] | null>(null);

  // Track pending background change during transitions
  const backgroundSwitchedRef = useRef<boolean>(false);

  // Initialize WebGPU background renderer and run unified render loop
  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;

    // Set canvas size to match display size
    const updateCanvasSize = (): void => {
      const rect = canvas.getBoundingClientRect();
      const dpr = window.devicePixelRatio || 1;
      canvas.width = Math.floor(rect.width * dpr);
      canvas.height = Math.floor(rect.height * dpr);

      // Resize renderer if initialized
      if (backgroundRef.current?.is_initialized()) {
        backgroundRef.current.resize(canvas.width, canvas.height);
      }
    };

    // Initialize the background renderer
    const initBackground = async (): Promise<void> => {
      try {
        updateCanvasSize();
        console.log('[Desktop] Starting background initialization...');

        // Dynamically import the DesktopBackground from the WASM module
        const { DesktopBackground } = await import('../../../../pkg/zos_supervisor_web.js');
        const background = new DesktopBackground() as DesktopBackgroundType;

        // Add timeout for WebGPU init - don't block forever
        const initPromise = background.init(canvas);
        const timeoutPromise = new Promise<void>((_, reject) =>
          setTimeout(() => reject(new Error('WebGPU init timeout')), 5000)
        );

        try {
          await Promise.race([initPromise, timeoutPromise]);
          backgroundRef.current = background;
          console.log('[Desktop] Background initialized successfully');
          onBackgroundReady();
        } catch (initError) {
          console.warn(
            '[Desktop] Background init failed/timeout, continuing without GPU background:',
            initError
          );
        }

        // Unified render loop
        let lastTime = 0;
        let lastAnimating = false;

        const render = (time: number): void => {
          animationFrameRef.current = requestAnimationFrame(render);

          // Adaptive framerate: 60fps when animating, 15fps when idle
          const fps = lastAnimating ? 60 : 15;
          const interval = 1000 / fps;

          // Track whether we have a working background renderer
          const hasBackground = backgroundRef.current?.is_initialized() ?? false;

          if (time - lastTime >= interval) {
            lastTime = time - ((time - lastTime) % interval);

            try {
              // SINGLE call to Rust - tick + get all data atomically
              const frameJson = desktop.tick_frame();
              const frame: FrameData = JSON.parse(frameJson);

              // Sync Zustand stores from frame data
              syncStoresFromFrame(frame);

              // Debug: Log first frame and any frame with windows
              if (lastTime === 0 || (frame.windows.length > 0 && windowIdsRef.current.size === 0)) {
                console.log('[Desktop] Frame data:', {
                  windowCount: frame.windows.length,
                  viewMode: frame.viewMode,
                  windows: frame.windows.map((w) => ({ id: w.id, appId: w.appId })),
                });
              }

              // Update adaptive framerate based on animation state
              lastAnimating = frame.animating;

              // Update background renderer with frame data (only if available)
              if (hasBackground && backgroundRef.current) {
                updateBackgroundRenderer(backgroundRef.current, frame, backgroundSwitchedRef);
              }

              // Store workspace info for parent component access
              workspaceInfoRef.current = frame.workspaceInfo;

              // Build set of current window IDs for quick lookup
              const currentWindowIds = new Set(frame.windows.map((w) => w.id));

              // Hide windows that are filtered out
              hideFilteredWindows(
                currentWindowIds,
                windowRefsMap,
                fadingOutWindowsRef,
                pendingWindowsRef,
                windowIdsRef,
                setWindows
              );

              // Update stored window data
              for (const win of frame.windows) {
                windowDataRef.current.set(win.id, win);
              }

              // Direct DOM updates for existing windows
              for (const win of frame.windows) {
                const el = windowRefsMap.current.get(win.id);
                if (el && win.state !== 'minimized') {
                  updateWindowDom(win, el, fadingOutWindowsRef, prevOpacityRef);
                }
              }

              // Only update React state when window LIST changes (add/remove)
              if (windowListChanged(frame.windows, windowIdsRef.current)) {
                console.log(
                  '[Desktop] Window list changed:',
                  frame.windows.map((w) => ({ id: w.id, appId: w.appId, state: w.state }))
                );
                if (fadingOutWindowsRef.current.size > 0) {
                  // Delay React update until fade-outs complete
                  pendingWindowsRef.current = frame.windows;
                } else {
                  windowIdsRef.current = new Set(frame.windows.map((w) => w.id));
                  setWindows(frame.windows);
                }
              }
            } catch (e) {
              console.error('[desktop] Render error:', e);
            }
          }
        };

        animationFrameRef.current = requestAnimationFrame(render);
      } catch (e) {
        console.warn('[desktop] WebGPU not available, falling back to CSS background:', e);
      }
    };

    initBackground();

    // Handle resize
    const handleResize = (): void => {
      updateCanvasSize();
    };
    window.addEventListener('resize', handleResize);

    return () => {
      window.removeEventListener('resize', handleResize);
      if (animationFrameRef.current !== null) {
        cancelAnimationFrame(animationFrameRef.current);
      }
      resetSyncState();
    };
  }, [desktop, backgroundRef, onBackgroundReady, workspaceInfoRef, canvasRef]);

  // Callback to register window DOM refs
  const setWindowRef = useCallback((id: number, el: HTMLDivElement | null) => {
    if (el) {
      windowRefsMap.current.set(id, el);
    } else {
      windowRefsMap.current.delete(id);
      windowDataRef.current.delete(id);
      prevOpacityRef.current.delete(id);
    }
  }, []);

  return { windows, setWindowRef };
}
