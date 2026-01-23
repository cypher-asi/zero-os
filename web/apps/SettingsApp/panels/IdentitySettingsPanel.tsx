import { Menu, GroupCollapsible, Label, type MenuItem } from '@cypher-asi/zui';
import { Brain, Cpu, Users, ChevronRight, CheckCircle, XCircle } from 'lucide-react';
import styles from './panels.module.css';

interface IdentitySettingsPanelProps {
  hasNeuralKey: boolean;
  machineKeyCount: number;
  linkedAccountCount: number;
}

/**
 * Identity Settings Panel
 * - Neural Key status and management
 * - Machine Keys overview
 * - Linked Accounts overview
 */
export function IdentitySettingsPanel({
  hasNeuralKey,
  machineKeyCount,
  linkedAccountCount,
}: IdentitySettingsPanelProps) {
  const handleSelect = (id: string) => {
    console.log('[settings] Identity panel selected:', id);
    // TODO: Navigate to detail panels
  };

  const neuralKeyItems: MenuItem[] = [
    {
      id: 'neural-key',
      label: 'Neural Key Status',
      icon: <Brain size={14} />,
      suffix: hasNeuralKey ? (
        <div className={styles.statusBadge}>
          <CheckCircle size={12} className={styles.successIcon} />
          <Label size="xs" variant="success">Active</Label>
        </div>
      ) : (
        <div className={styles.statusBadge}>
          <XCircle size={12} className={styles.warningIcon} />
          <Label size="xs" variant="warning">Not Set</Label>
        </div>
      ),
      endIcon: <ChevronRight size={14} />,
    },
  ];

  const machineKeyItems: MenuItem[] = [
    {
      id: 'machine-keys',
      label: 'Registered Devices',
      icon: <Cpu size={14} />,
      endIcon: <ChevronRight size={14} />,
    },
  ];

  const linkedAccountItems: MenuItem[] = [
    {
      id: 'linked-accounts',
      label: 'Connected Services',
      icon: <Users size={14} />,
      endIcon: <ChevronRight size={14} />,
    },
  ];

  return (
    <div className={styles.panelContainer}>
      <GroupCollapsible
        title="Neural Key"
        count={hasNeuralKey ? 1 : 0}
        defaultOpen
        className={styles.collapsibleSection}
      >
        <div className={styles.menuContent}>
          <Menu items={neuralKeyItems} onChange={handleSelect} background="none" border="none" />
        </div>
      </GroupCollapsible>

      <GroupCollapsible
        title="Machine Keys"
        count={machineKeyCount}
        defaultOpen
        className={styles.collapsibleSection}
      >
        <div className={styles.menuContent}>
          <Menu items={machineKeyItems} onChange={handleSelect} background="none" border="none" />
        </div>
      </GroupCollapsible>

      <GroupCollapsible
        title="Linked Accounts"
        count={linkedAccountCount}
        defaultOpen
        className={styles.collapsibleSection}
      >
        <div className={styles.menuContent}>
          <Menu items={linkedAccountItems} onChange={handleSelect} background="none" border="none" />
        </div>
      </GroupCollapsible>
    </div>
  );
}
