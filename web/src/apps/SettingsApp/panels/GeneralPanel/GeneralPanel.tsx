import { Menu, GroupCollapsible, type MenuItem } from '@cypher-asi/zui';
import { Clock, Globe } from 'lucide-react';
import styles from './GeneralPanel.module.css';

interface GeneralPanelProps {
  timeFormat24h: boolean;
  timezone: string;
  onTimeFormatChange: (value: boolean) => void;
  onTimezoneChange: (value: string) => void;
}

/**
 * General Settings Panel
 * - Date/Time format preferences
 * - Timezone selection
 */
export function GeneralPanel({
  timeFormat24h,
  timezone,
  onTimeFormatChange,
  onTimezoneChange,
}: GeneralPanelProps) {
  const timeFormatItems: MenuItem[] = [
    { id: '12h', label: '12-hour', icon: <Clock size={14} /> },
    { id: '24h', label: '24-hour', icon: <Clock size={14} /> },
  ];

  const timezoneItems: MenuItem[] = [
    { id: 'UTC', label: 'UTC', icon: <Globe size={14} /> },
    { id: 'America/New_York', label: 'Eastern (US)', icon: <Globe size={14} /> },
    { id: 'America/Chicago', label: 'Central (US)', icon: <Globe size={14} /> },
    { id: 'America/Denver', label: 'Mountain (US)', icon: <Globe size={14} /> },
    { id: 'America/Los_Angeles', label: 'Pacific (US)', icon: <Globe size={14} /> },
    { id: 'Europe/London', label: 'London', icon: <Globe size={14} /> },
    { id: 'Europe/Paris', label: 'Paris', icon: <Globe size={14} /> },
    { id: 'Europe/Berlin', label: 'Berlin', icon: <Globe size={14} /> },
    { id: 'Asia/Tokyo', label: 'Tokyo', icon: <Globe size={14} /> },
    { id: 'Asia/Shanghai', label: 'Shanghai', icon: <Globe size={14} /> },
    { id: 'Asia/Singapore', label: 'Singapore', icon: <Globe size={14} /> },
    { id: 'Australia/Sydney', label: 'Sydney', icon: <Globe size={14} /> },
  ];

  const handleTimeFormatSelect = (id: string) => {
    onTimeFormatChange(id === '24h');
  };

  return (
    <div className={styles.panelContainer}>
      <GroupCollapsible
        title="Time Format"
        count={2}
        defaultOpen
        className={styles.collapsibleSection}
      >
        <div className={styles.menuContent}>
          <Menu
            items={timeFormatItems}
            value={timeFormat24h ? '24h' : '12h'}
            onChange={handleTimeFormatSelect}
            background="none"
            border="none"
          />
        </div>
      </GroupCollapsible>

      <GroupCollapsible
        title="Timezone"
        count={timezoneItems.length}
        defaultOpen
        className={styles.collapsibleSection}
      >
        <div className={styles.menuContent}>
          <Menu
            items={timezoneItems}
            value={timezone}
            onChange={onTimezoneChange}
            background="none"
            border="none"
          />
        </div>
      </GroupCollapsible>
    </div>
  );
}
