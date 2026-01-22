import { useCallback } from 'react';
import { Panel, Button, Text, Label } from '@cypher-asi/zui';
import styles from './AppPermissions.module.css';

// =============================================================================
// Types
// =============================================================================

/**
 * Types of kernel objects that can be accessed via capabilities
 */
export type ObjectType =
  | 'Endpoint'
  | 'Console'
  | 'Storage'
  | 'Network'
  | 'Process'
  | 'Memory';

/**
 * Permission bits for capabilities
 */
export interface Permissions {
  read: boolean;
  write: boolean;
  grant: boolean;
}

/**
 * Information about a granted capability
 */
export interface CapabilityInfo {
  /** Capability slot in the process's CSpace */
  slot: number;
  /** Object type */
  objectType: ObjectType;
  /** Permissions */
  permissions: Permissions;
}

/**
 * App manifest information
 */
export interface AppManifest {
  /** Unique app identifier (e.g., "com.example.myapp") */
  id: string;
  /** Display name */
  name: string;
  /** Version string */
  version: string;
  /** Whether this is a factory (trusted) app */
  isFactory?: boolean;
}

// =============================================================================
// Component Props
// =============================================================================

export interface AppPermissionsProps {
  /** App manifest */
  app: AppManifest;
  /** Currently granted capabilities */
  grantedCaps: CapabilityInfo[];
  /** Called when user wants to revoke a permission */
  onRevoke: (objectType: ObjectType) => void;
  /** Called when user wants to revoke all permissions */
  onRevokeAll?: () => void;
  /** Loading state */
  isLoading?: boolean;
}

// =============================================================================
// Helper Functions
// =============================================================================

function formatObjectType(type: ObjectType): string {
  switch (type) {
    case 'Endpoint':
      return 'IPC Endpoint';
    case 'Console':
      return 'Console Access';
    case 'Storage':
      return 'Storage';
    case 'Network':
      return 'Network';
    case 'Process':
      return 'Process Management';
    case 'Memory':
      return 'Memory Access';
    default:
      return type;
  }
}

function formatPermissions(perms: Permissions): string {
  const parts: string[] = [];
  if (perms.read) parts.push('R');
  if (perms.write) parts.push('W');
  if (perms.grant) parts.push('G');
  return parts.join('/') || 'none';
}

function getObjectTypeIcon(type: ObjectType): string {
  switch (type) {
    case 'Endpoint':
      return '↔';
    case 'Console':
      return '⌨';
    case 'Storage':
      return '💾';
    case 'Network':
      return '🌐';
    case 'Process':
      return '⚙';
    case 'Memory':
      return '🧠';
    default:
      return '?';
  }
}

// =============================================================================
// AppPermissions Component
// =============================================================================

/**
 * Settings component for viewing and managing app permissions.
 *
 * Displays the list of granted capabilities for an app and allows
 * the user to revoke individual permissions or all permissions at once.
 */
export function AppPermissions({
  app,
  grantedCaps,
  onRevoke,
  onRevokeAll,
  isLoading = false,
}: AppPermissionsProps) {
  const handleRevoke = useCallback(
    (objectType: ObjectType) => {
      onRevoke(objectType);
    },
    [onRevoke]
  );

  if (isLoading) {
    return (
      <Panel variant="glass" className={styles.container}>
        <div className={styles.loading}>
          <div className={styles.spinner} />
          <Text as="span" size="xs" variant="muted">
            Loading permissions...
          </Text>
        </div>
      </Panel>
    );
  }

  return (
    <Panel variant="glass" className={styles.container}>
      {/* Header */}
      <div className={styles.header}>
        <div className={styles.appIcon}>🔐</div>
        <div className={styles.appInfo}>
          <Text as="div" size="sm" className={styles.appName}>
            {app.name}
          </Text>
          <Text as="div" size="xs" className={styles.appId}>
            {app.id}
          </Text>
        </div>
        {app.isFactory && (
          <Label size="xs" variant="success">
            Factory App
          </Label>
        )}
      </div>

      {/* Permissions List */}
      {grantedCaps.length === 0 ? (
        <div className={styles.emptyState}>
          <div className={styles.emptyIcon}>🛡️</div>
          <Text as="p" size="sm" variant="muted">
            No permissions granted
          </Text>
          <Text as="p" size="xs" variant="muted">
            This app has not been granted any capabilities.
          </Text>
        </div>
      ) : (
        <div className={styles.permissionsList}>
          {grantedCaps.map((cap) => (
            <Panel
              key={`${cap.objectType}-${cap.slot}`}
              variant="glass"
              className={styles.permissionCard}
            >
              <div className={styles.permissionIcon}>
                {getObjectTypeIcon(cap.objectType)}
              </div>
              <div className={styles.permissionInfo}>
                <div className={styles.permissionHeader}>
                  <span className={styles.permissionType}>
                    {formatObjectType(cap.objectType)}
                  </span>
                  <span className={styles.permissionPerms}>
                    {formatPermissions(cap.permissions)}
                  </span>
                </div>
                <Text as="span" size="xs" className={styles.permissionSlot}>
                  slot {cap.slot}
                </Text>
              </div>
              <Button
                variant="danger"
                size="sm"
                onClick={() => handleRevoke(cap.objectType)}
                className={styles.revokeButton}
              >
                Revoke
              </Button>
            </Panel>
          ))}
        </div>
      )}

      {/* Footer with stats and danger zone */}
      {grantedCaps.length > 0 && (
        <>
          <div className={styles.footer}>
            <Text as="span" size="xs" className={styles.stats}>
              {grantedCaps.length} permission{grantedCaps.length !== 1 ? 's' : ''} granted
            </Text>
          </div>

          {onRevokeAll && grantedCaps.length > 1 && (
            <div className={styles.dangerZone}>
              <Text as="div" size="sm" className={styles.dangerTitle}>
                Danger Zone
              </Text>
              <Text as="p" size="xs" className={styles.dangerText}>
                Revoking all permissions may cause the app to stop working correctly.
              </Text>
              <Button variant="danger" size="sm" onClick={onRevokeAll}>
                Revoke All Permissions
              </Button>
            </div>
          )}
        </>
      )}
    </Panel>
  );
}

