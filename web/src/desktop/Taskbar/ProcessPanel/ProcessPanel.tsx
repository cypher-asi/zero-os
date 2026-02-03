import { useEffect, useRef } from 'react';
import { Panel } from '@cypher-asi/zui';
import { ProcessPanelContent } from './ProcessPanelContent';
import styles from './ProcessPanel.module.css';

interface ProcessPanelProps {
  onClose: () => void;
  containerRef?: React.RefObject<HTMLDivElement>;
}

export function ProcessPanel({ onClose, containerRef }: ProcessPanelProps) {
  const panelRef = useRef<HTMLDivElement>(null);

  // Click outside handler
  useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      const target = event.target as Node;
      // Ignore clicks inside the panel
      if (panelRef.current && panelRef.current.contains(target)) {
        return;
      }
      // Ignore clicks on the container (includes the toggle button)
      if (containerRef?.current && containerRef.current.contains(target)) {
        return;
      }
      onClose();
    };

    document.addEventListener('mousedown', handleClickOutside);
    return () => {
      document.removeEventListener('mousedown', handleClickOutside);
    };
  }, [onClose, containerRef]);

  return (
    <div ref={panelRef} className={styles.panelWrapper}>
      <Panel variant="glass" border="future" className={styles.panel}>
        <ProcessPanelContent onClose={onClose} />
      </Panel>
    </div>
  );
}
