import type { ReactNode } from 'react';
import type { WindowInfo } from '../../hooks/useWindows';
import { useWindowActions } from '../../hooks/useWindows';
import styles from './WindowContent.module.css';

// Frame style constants (must match Rust FRAME_STYLE)
const FRAME_STYLE = {
  titleBarHeight: 32,
  borderRadius: 8,
  resizeHandleSize: 8,
  cornerHandleSize: 12, // Larger corners for easier diagonal targeting
};

interface WindowContentProps {
  window: WindowInfo;
  children: ReactNode;
}

export function WindowContent({ window: win, children }: WindowContentProps) {
  const { focusWindow, minimizeWindow, maximizeWindow, closeWindow } = useWindowActions();

  // Position the entire window frame (title bar + content)
  const style: React.CSSProperties = {
    left: win.screenRect.x,
    top: win.screenRect.y,
    width: win.screenRect.width,
    height: win.screenRect.height,
    zIndex: win.zOrder + 1, // +1 so windows are above desktop background
    borderRadius: FRAME_STYLE.borderRadius,
  };

  const handleWindowClick = () => {
    if (!win.focused) {
      focusWindow(win.id);
    }
  };

  const handleMinimize = (e: React.MouseEvent) => {
    e.stopPropagation();
    minimizeWindow(win.id);
  };

  const handleMaximize = (e: React.MouseEvent) => {
    e.stopPropagation();
    maximizeWindow(win.id);
  };

  const handleClose = (e: React.MouseEvent) => {
    e.stopPropagation();
    closeWindow(win.id);
  };

  const handleSize = FRAME_STYLE.resizeHandleSize;
  const cornerSize = FRAME_STYLE.cornerHandleSize;

  return (
    <div
      className={`${styles.window} ${win.focused ? styles.focused : ''}`}
      style={style}
      data-window-id={win.id}
      onPointerDown={handleWindowClick}
    >
      {/* Resize handles - positioned at edges and corners */}
      {/* These don't stop propagation - events bubble to Desktop where Rust handles resize */}
      <div className={`${styles.resizeHandle} ${styles.resizeN}`} style={{ height: handleSize }} />
      <div className={`${styles.resizeHandle} ${styles.resizeS}`} style={{ height: handleSize }} />
      <div className={`${styles.resizeHandle} ${styles.resizeE}`} style={{ width: handleSize }} />
      <div className={`${styles.resizeHandle} ${styles.resizeW}`} style={{ width: handleSize }} />
      {/* Corners use larger handles for easier diagonal targeting */}
      <div className={`${styles.resizeHandle} ${styles.resizeNE}`} style={{ width: cornerSize, height: cornerSize }} />
      <div className={`${styles.resizeHandle} ${styles.resizeNW}`} style={{ width: cornerSize, height: cornerSize }} />
      <div className={`${styles.resizeHandle} ${styles.resizeSE}`} style={{ width: cornerSize, height: cornerSize }} />
      <div className={`${styles.resizeHandle} ${styles.resizeSW}`} style={{ width: cornerSize, height: cornerSize }} />

      {/* Title bar */}
      <div className={styles.titleBar} style={{ height: FRAME_STYLE.titleBarHeight }}>
        <span className={styles.title}>{win.title}</span>
        <div className={styles.buttons}>
          <button 
            className={`${styles.btn} ${styles.minimize}`} 
            aria-label="Minimize"
            onClick={handleMinimize}
          >
            −
          </button>
          <button 
            className={`${styles.btn} ${styles.maximize}`} 
            aria-label="Maximize"
            onClick={handleMaximize}
          >
            □
          </button>
          <button 
            className={`${styles.btn} ${styles.close}`} 
            aria-label="Close"
            onClick={handleClose}
          >
            ×
          </button>
        </div>
      </div>
      
      {/* Content area - focus window but stop propagation to allow input focus */}
      <div 
        className={styles.content} 
        onPointerDown={(e) => {
          if (!win.focused) {
            focusWindow(win.id);
          }
          e.stopPropagation();
        }}
      >
        {children}
      </div>
    </div>
  );
}
