import { useRef, useEffect, useState, useCallback } from 'react';
import { SupervisorProvider, Supervisor } from '../../hooks/useSupervisor';
import { useWindowScreenRects, WindowInfo } from '../../hooks/useWindows';
import { WindowContent } from '../WindowContent/WindowContent';
import { Taskbar } from '../Taskbar/Taskbar';
import { AppRouter } from '../../apps/AppRouter/AppRouter';
import styles from './Desktop.module.css';

interface DesktopProps {
  supervisor: Supervisor;
}

// Inner component that uses the Supervisor context
function DesktopInner() {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const windows = useWindowScreenRects();

  return (
    <>
      {/* WebGPU canvas for rendering window frames (placeholder for now) */}
      <canvas
        id="desktop-canvas"
        ref={canvasRef}
        className={styles.canvas}
      />

      {/* React overlays for window content - positioned by Rust */}
      {windows
        .filter((w) => w.state !== 'minimized')
        .map((w) => (
          <WindowContent key={w.id} window={w}>
            <AppRouter appId={w.appId} windowId={w.id} />
          </WindowContent>
        ))}

      <Taskbar />
    </>
  );
}

export function Desktop({ supervisor }: DesktopProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const [initialized, setInitialized] = useState(false);

  // Initialize desktop engine
  useEffect(() => {
    if (initialized) return;

    const container = containerRef.current;
    if (!container) return;

    const rect = container.getBoundingClientRect();
    supervisor.init_desktop(rect.width, rect.height);
    setInitialized(true);
  }, [supervisor, initialized]);

  // Handle resize
  useEffect(() => {
    if (!initialized) return;

    const handleResize = () => {
      const container = containerRef.current;
      if (!container) return;

      const rect = container.getBoundingClientRect();
      supervisor.resize_desktop(rect.width, rect.height);
    };

    window.addEventListener('resize', handleResize);
    return () => window.removeEventListener('resize', handleResize);
  }, [supervisor, initialized]);

  // Prevent browser zoom on Ctrl+scroll (needs non-passive listener)
  useEffect(() => {
    const container = containerRef.current;
    if (!container) return;

    const handleNativeWheel = (e: WheelEvent) => {
      if (e.ctrlKey) {
        e.preventDefault();
      }
    };

    container.addEventListener('wheel', handleNativeWheel, { passive: false });
    return () => container.removeEventListener('wheel', handleNativeWheel);
  }, []);

  // Use capture phase for panning so it intercepts before windows
  useEffect(() => {
    const container = containerRef.current;
    if (!container) return;

    const handleCapturePointerDown = (e: PointerEvent) => {
      // Middle mouse button OR Ctrl/Shift + primary button = pan (intercept before windows)
      const isPanGesture = e.button === 1 || (e.button === 0 && (e.ctrlKey || e.shiftKey));
      if (isPanGesture) {
        const result = JSON.parse(
          supervisor.desktop_pointer_down(e.clientX, e.clientY, e.button, e.ctrlKey, e.shiftKey)
        );
        if (result.type === 'handled') {
          e.preventDefault();
          e.stopPropagation();
        }
      }
    };

    // Capture phase runs before bubble phase, so we get the event first
    container.addEventListener('pointerdown', handleCapturePointerDown, { capture: true });
    return () => container.removeEventListener('pointerdown', handleCapturePointerDown, { capture: true });
  }, [supervisor]);

  // Forward pointer events to Rust (bubble phase for normal interactions)
  const handlePointerDown = useCallback(
    (e: React.PointerEvent) => {
      const result = JSON.parse(
        supervisor.desktop_pointer_down(e.clientX, e.clientY, e.button, e.ctrlKey, e.shiftKey)
      );
      if (result.type === 'handled') {
        e.preventDefault();
      }
    },
    [supervisor]
  );

  const handlePointerMove = useCallback(
    (e: React.PointerEvent) => {
      supervisor.desktop_pointer_move(e.clientX, e.clientY);
    },
    [supervisor]
  );

  const handlePointerUp = useCallback(() => {
    supervisor.desktop_pointer_up();
  }, [supervisor]);

  // Release drag state when pointer leaves the desktop (e.g., goes off-screen)
  const handlePointerLeave = useCallback(() => {
    supervisor.desktop_pointer_up();
  }, [supervisor]);

  const handleWheel = useCallback(
    (e: React.WheelEvent) => {
      supervisor.desktop_wheel(
        e.deltaX,
        e.deltaY,
        e.clientX,
        e.clientY,
        e.ctrlKey
      );
    },
    [supervisor]
  );

  return (
    <SupervisorProvider value={supervisor}>
      <div
        ref={containerRef}
        className={styles.desktop}
        onPointerDown={handlePointerDown}
        onPointerMove={handlePointerMove}
        onPointerUp={handlePointerUp}
        onPointerLeave={handlePointerLeave}
        onWheel={handleWheel}
      >
        {initialized && <DesktopInner />}
      </div>
    </SupervisorProvider>
  );
}
