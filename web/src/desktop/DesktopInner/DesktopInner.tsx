/**
 * Desktop Inner Component
 *
 * Renders canvas and windows using frame data from Rust.
 */

import { useRef } from 'react';
import type { DesktopController } from '../hooks/useSupervisor';
import type { WorkspaceInfo } from '@/stores/types';
import { WindowContent } from '../WindowContent';
import { Taskbar } from '../Taskbar';
import { AppRouter } from '@apps/AppRouter/AppRouter';
import { useRenderLoop } from '../Desktop/hooks/useRenderLoop';
import type { DesktopBackgroundType } from '../Desktop/types';
import styles from '../Desktop/Desktop.module.css';

interface DesktopInnerProps {
  desktop: DesktopController;
  backgroundRef: React.MutableRefObject<DesktopBackgroundType | null>;
  onBackgroundReady: () => void;
  workspaceInfoRef: React.MutableRefObject<WorkspaceInfo | null>;
}

export function DesktopInner({
  desktop,
  backgroundRef,
  onBackgroundReady,
  workspaceInfoRef,
}: DesktopInnerProps): JSX.Element {
  const canvasRef = useRef<HTMLCanvasElement>(null);

  const { windows, setWindowRef } = useRenderLoop({
    desktop,
    backgroundRef,
    onBackgroundReady,
    workspaceInfoRef,
    canvasRef,
  });

  return (
    <>
      {/* WebGPU canvas for background with procedural shaders */}
      <canvas id="desktop-canvas" ref={canvasRef} className={styles.canvas} />

      {/* React overlays for window content - positions updated via direct DOM */}
      {windows
        .filter((w) => w.state !== 'minimized')
        .map((w) => (
          <WindowContent key={w.id} ref={(el) => setWindowRef(w.id, el)} window={w}>
            <AppRouter appId={w.appId} windowId={w.id} processId={w.processId} />
          </WindowContent>
        ))}

      <Taskbar />
    </>
  );
}
