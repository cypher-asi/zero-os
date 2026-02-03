import { useState, useMemo, useCallback } from 'react';
import { Panel, Explorer, type ExplorerNode, useTheme, type AccentColor, Avatar, Text } from '@cypher-asi/zui';
import { ChevronDown } from 'lucide-react';
import { useWindowActions } from '@desktop/hooks/useWindows';
import { buildContactExplorerData, getContactById, getStatusColors, type ContactStatus } from './types';
import styles from './ZeroChatApp.module.css';

// Accent color hex values (must match ZUI theme)
const ACCENT_HEX: Record<AccentColor, string> = {
  cyan: '#01f4cb',
  blue: '#3b82f6',
  purple: '#8b5cf6',
  green: '#22c55e',
  orange: '#f97316',
  rose: '#f43f5e',
};

/**
 * ZeroChat App - ICQ/MSN Messenger-style chat application
 *
 * Uses ZUI components: Panel, Explorer
 * Layout: Full-bleed file-explorer style with contact categories
 *
 * Clicking a contact opens a new ConversationWindow for that contact.
 */
export function ZeroChatApp() {
  const [selected, setSelected] = useState<string[]>([]);
  const { launchApp } = useWindowActions();
  const { accent } = useTheme();

  const statusColors = useMemo(() => getStatusColors(ACCENT_HEX[accent]), [accent]);

  // Build explorer data from mock contacts
  const explorerData = useMemo(() => buildContactExplorerData(), []);

  // Handle contact selection - open conversation window
  const handleSelect = useCallback(
    (selectedIds: string[]) => {
      setSelected(selectedIds);
      const id = selectedIds[0];
      if (!id) return;

      // Only handle contact nodes (not category nodes)
      if (id.startsWith('contact-')) {
        const contactId = id.replace('contact-', '');
        const contact = getContactById(contactId);
        if (contact) {
          // Launch conversation window with contact ID in app ID
          launchApp(`zerochat-conversation-${contactId}`);
        }
      }
    },
    [launchApp]
  );

  // Custom node renderer with status indicator on far right
  const renderNode = useCallback(
    (node: ExplorerNode) => {
      const status = node.metadata?.status as ContactStatus | undefined;
      if (status) {
        // Contact node - show name with status dot on right
        return (
          <div className={styles.contactNode}>
            <span className={styles.contactName}>{node.label}</span>
            <span
              className={styles.statusDot}
              style={{ backgroundColor: statusColors[status] }}
            />
          </div>
        );
      }
      // Category node - default label
      return <span>{node.label}</span>;
    },
    [statusColors]
  );

  return (
    <Panel border="none" background="none" className={styles.container}>
      {/* Profile header */}
      <div className={styles.profileHeader}>
        <div className={styles.avatarBorder}>
          <Avatar name="You" icon size="md" />
        </div>
        <div className={styles.profileInfo}>
          <div className={styles.profileName}>
            <Text size="sm" weight="semibold">You</Text>
            <span className={styles.statusLabel} style={{ color: statusColors.online }}>
              (Online)
            </span>
            <ChevronDown size={14} className={styles.statusChevron} />
          </div>
          <Text size="xs" variant="muted" className={styles.statusMessage}>
            &lt;Type a personal message...&gt;
          </Text>
        </div>
      </div>

      {/* Contacts Explorer */}
      <div className={styles.explorerContainer}>
        <Explorer
          data={explorerData}
          onSelect={handleSelect}
          defaultExpandedIds={['friends', 'work', 'family']}
          defaultSelectedIds={selected}
          searchable
          searchPlaceholder="Search contacts..."
          enableDragDrop={false}
          enableMultiSelect={false}
          expandOnSelect
          compact
          renderNode={renderNode}
        />
      </div>
    </Panel>
  );
}
