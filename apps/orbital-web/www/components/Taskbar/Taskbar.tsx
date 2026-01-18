import { useState } from 'react';
import { useWindows, useWindowActions } from '../../hooks/useWindows';
import { useWorkspaces, useWorkspaceActions } from '../../hooks/useWorkspaces';
import { BeginMenu } from '../BeginMenu/BeginMenu';
import styles from './Taskbar.module.css';

export function Taskbar() {
  const [beginMenuOpen, setBeginMenuOpen] = useState(false);
  const windows = useWindows();
  const workspaces = useWorkspaces();
  const { focusWindow, restoreWindow } = useWindowActions();
  const { createWorkspace, switchWorkspace } = useWorkspaceActions();

  const handleWindowClick = (e: React.MouseEvent, windowId: number, state: string) => {
    e.stopPropagation(); // Prevent event from bubbling to Desktop
    if (state === 'minimized') {
      restoreWindow(windowId);
    }
    focusWindow(windowId);
  };

  const handleAddWorkspace = () => {
    const count = workspaces.length;
    createWorkspace(`Workspace ${count + 1}`);
  };

  return (
    <div className={styles.taskbar}>
      {/* Begin Button - Left */}
      <div className={styles.beginSection}>
        <button
          className={`${styles.beginButton} ${beginMenuOpen ? styles.active : ''}`}
          onClick={() => setBeginMenuOpen(!beginMenuOpen)}
        >
          <span className={styles.beginIcon}>◆</span>
          <span className={styles.beginText}>Begin</span>
        </button>

        {beginMenuOpen && <BeginMenu onClose={() => setBeginMenuOpen(false)} />}
      </div>

      {/* Active Windows - Center */}
      <div className={styles.windowsSection}>
        {windows.map((win) => (
          <button
            key={win.id}
            className={`${styles.windowItem} ${win.focused ? styles.focused : ''} ${win.state === 'minimized' ? styles.minimized : ''}`}
            onClick={(e) => handleWindowClick(e, win.id, win.state)}
            title={win.title}
          >
            <span className={styles.windowIcon}>□</span>
            <span className={styles.windowTitle}>{win.title}</span>
          </button>
        ))}
      </div>

      {/* Workspace Indicators - Right */}
      <div className={styles.workspacesSection}>
        {workspaces.map((ws, i) => (
          <button
            key={ws.id}
            className={`${styles.workspaceBtn} ${ws.active ? styles.active : ''}`}
            onClick={() => switchWorkspace(i)}
            title={ws.name}
          >
            {i + 1}
          </button>
        ))}
        <button
          className={styles.workspaceAdd}
          onClick={handleAddWorkspace}
          title="Add workspace"
        >
          +
        </button>
      </div>
    </div>
  );
}
