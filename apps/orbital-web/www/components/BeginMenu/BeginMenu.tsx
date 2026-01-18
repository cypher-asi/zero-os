import { useEffect, useRef } from 'react';
import { useWindowActions } from '../../hooks/useWindows';
import { useSupervisor } from '../../hooks/useSupervisor';
import styles from './BeginMenu.module.css';

interface BeginMenuProps {
  onClose: () => void;
}

const AVAILABLE_APPS = [
  { id: 'terminal', icon: '▸', name: 'Terminal', description: 'Command line & system monitor' },
  { id: 'settings', icon: '⚙', name: 'Settings', description: 'System configuration' },
  { id: 'files', icon: '📁', name: 'Files', description: 'File explorer' },
];

export function BeginMenu({ onClose }: BeginMenuProps) {
  const { launchApp } = useWindowActions();
  const supervisor = useSupervisor();
  const menuRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      if (menuRef.current && !menuRef.current.contains(event.target as Node)) {
        onClose();
      }
    };

    // Use mousedown to catch the click before it bubbles
    document.addEventListener('mousedown', handleClickOutside);
    return () => {
      document.removeEventListener('mousedown', handleClickOutside);
    };
  }, [onClose]);

  const handleLaunchApp = (appId: string) => {
    launchApp(appId);
    onClose();
  };

  const handleShutdown = () => {
    // In browser context, we just close the menu
    // Could add confirmation dialog
    onClose();
    if (supervisor) {
      supervisor.send_input('shutdown');
    }
  };

  return (
    <div ref={menuRef} className={styles.menu}>
      <div className={styles.header}>
        <span className={styles.logo}>◆</span>
        <span className={styles.title}>Orbital OS</span>
      </div>

      <div className={styles.apps}>
        {AVAILABLE_APPS.map((app) => (
          <button
            key={app.id}
            className={styles.appItem}
            onClick={() => handleLaunchApp(app.id)}
          >
            <span className={styles.appIcon}>{app.icon}</span>
            <div className={styles.appText}>
              <span className={styles.appName}>{app.name}</span>
              <span className={styles.appDesc}>{app.description}</span>
            </div>
          </button>
        ))}
      </div>

      <div className={styles.footer}>
        <button className={styles.shutdownBtn} onClick={handleShutdown}>
          <span className={styles.shutdownIcon}>⏻</span>
          <span>Shutdown</span>
        </button>
      </div>
    </div>
  );
}
