/**
 * AppBar Component
 *
 * A floating vertical app bar positioned on the left side of the desktop,
 * providing quick access to frequently used applications.
 */

import { Button, Panel } from '@cypher-asi/zui';
import { useWindowActions } from '../hooks/useWindows';
import {
  Brain,
  MessageSquare,
  Rss,
  Coins,
  ListTodo,
  StickyNote,
  Folder,
} from 'lucide-react';
import type { LucideIcon } from 'lucide-react';
import styles from './AppBar.module.css';

interface AppBarItem {
  id: string;
  label: string;
  icon: LucideIcon;
}

const APPBAR_ITEMS: AppBarItem[] = [
  { id: 'agents', label: 'Agents', icon: Brain },
  { id: 'chat', label: 'Chat', icon: MessageSquare },
  { id: 'feed', label: 'Feed', icon: Rss },
  { id: 'tokens', label: 'Tokens', icon: Coins },
  { id: 'tasks', label: 'Tasks', icon: ListTodo },
  { id: 'notes', label: 'Notes', icon: StickyNote },
  { id: 'files', label: 'Files', icon: Folder },
];

interface AppBarProps {
  /** When true, app bar interactions are disabled (pre-auth lock) */
  isLocked?: boolean;
}

export function AppBar({ isLocked = false }: AppBarProps) {
  const { launchOrFocusApp } = useWindowActions();

  const handleAppClick = (appId: string) => {
    if (isLocked) return;
    launchOrFocusApp(appId);
  };

  return (
    <div
      className={styles.appBarWrapper}
      style={isLocked ? { pointerEvents: 'none' } : undefined}
    >
      <Panel variant="glass" border="future" className={styles.appBar}>
        {APPBAR_ITEMS.map((item) => {
          const IconComponent = item.icon;
          return (
            <Button
              key={item.id}
              variant="transparent"
              rounded="none"
              iconOnly
              className={styles.appButton}
              onClick={() => handleAppClick(item.id)}
              title={item.label}
              aria-label={`Open ${item.label}`}
              disabled={isLocked}
            >
              <IconComponent size={28} />
            </Button>
          );
        })}
      </Panel>
    </div>
  );
}
