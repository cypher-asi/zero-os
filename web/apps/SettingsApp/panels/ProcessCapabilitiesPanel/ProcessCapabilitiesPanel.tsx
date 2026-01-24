import { useCallback, useMemo } from 'react';
import {
  GroupCollapsible,
  Menu,
  type MenuItem,
  Label,
  Button,
  Card,
  CardItem,
} from '@cypher-asi/zui';
import {
  ArrowLeftRight,
  Keyboard,
  HardDrive,
  Globe,
  Cog,
  Cpu,
  ShieldOff,
  AlertTriangle,
} from 'lucide-react';
import { useSupervisor } from '../../../../desktop/hooks/useSupervisor';
import type { CapabilityInfo, ObjectType, AppManifest } from '../../../../types/permissions';
import styles from './ProcessCapabilitiesPanel.module.css';

// =============================================================================
// Types
// =============================================================================

interface AppWithPermissions {
  manifest: AppManifest;
  pid: number;
  grantedCaps: CapabilityInfo[];
}

interface ProcessCapabilitiesPanelProps {
  app: AppWithPermissions;
  supervisor: ReturnType<typeof useSupervisor>;
  onRefresh: () => void;
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

function formatPermissions(perms: { read: boolean; write: boolean; grant: boolean }): string {
  const parts: string[] = [];
  if (perms.read) parts.push('R');
  if (perms.write) parts.push('W');
  if (perms.grant) parts.push('G');
  return parts.join('/') || 'none';
}

function getObjectTypeIcon(type: ObjectType): React.ReactNode {
  const size = 14;
  switch (type) {
    case 'Endpoint':
      return <ArrowLeftRight size={size} />;
    case 'Console':
      return <Keyboard size={size} />;
    case 'Storage':
      return <HardDrive size={size} />;
    case 'Network':
      return <Globe size={size} />;
    case 'Process':
      return <Cog size={size} />;
    case 'Memory':
      return <Cpu size={size} />;
    default:
      return null;
  }
}

// =============================================================================
// ProcessCapabilitiesPanel Component
// =============================================================================

/**
 * Panel for viewing and managing capabilities of a specific process.
 * Uses the standard Settings panel structure (GroupCollapsible + Menu).
 */
export function ProcessCapabilitiesPanel({
  app,
  supervisor,
  onRefresh,
}: ProcessCapabilitiesPanelProps) {
  // Handle revoking a single capability
  const handleRevoke = useCallback(
    (objectType: ObjectType) => {
      if (!supervisor) return;

      const cap = app.grantedCaps.find((c) => c.objectType === objectType);
      if (!cap) return;

      const success = supervisor.revoke_capability(BigInt(app.pid), cap.slot);
      if (success) {
        console.log(
          `[ProcessCapabilitiesPanel] Revoked ${objectType} from PID ${app.pid} slot ${cap.slot}`
        );
      } else {
        console.error(
          `[ProcessCapabilitiesPanel] Failed to revoke ${objectType} from PID ${app.pid}`
        );
      }

      onRefresh();
    },
    [app, supervisor, onRefresh]
  );

  // Handle revoking all capabilities
  const handleRevokeAll = useCallback(() => {
    if (!supervisor) return;

    const pidBigInt = BigInt(app.pid);
    let successCount = 0;
    for (const cap of app.grantedCaps) {
      if (supervisor.revoke_capability(pidBigInt, cap.slot)) {
        successCount++;
      }
    }
    console.log(
      `[ProcessCapabilitiesPanel] Revoked ${successCount}/${app.grantedCaps.length} caps from PID ${app.pid}`
    );

    onRefresh();
  }, [app, supervisor, onRefresh]);

  // Build menu items for capabilities
  const capabilityItems: MenuItem[] = useMemo(() => {
    return app.grantedCaps.map((cap) => ({
      id: `${cap.objectType}-${cap.slot}`,
      label: formatObjectType(cap.objectType),
      icon: getObjectTypeIcon(cap.objectType),
      status: (
        <div className={styles.menuStatus}>
          <Label size="xs" variant="default" style={{ fontFamily: 'monospace' }}>
            {formatPermissions(cap.permissions)}
          </Label>
          <Button
            variant="danger"
            size="xs"
            onClick={(e) => {
              e.stopPropagation();
              handleRevoke(cap.objectType);
            }}
          >
            Revoke
          </Button>
        </div>
      ),
    }));
  }, [app.grantedCaps, handleRevoke]);

  // Empty state
  if (app.grantedCaps.length === 0) {
    return (
      <div className={styles.panelContainer}>
        <GroupCollapsible
          title="Granted Capabilities"
          count={0}
          defaultOpen
          className={styles.collapsibleSection}
        >
          <div className={styles.emptyState}>
            <ShieldOff size={32} strokeWidth={1} />
            <span>No capabilities granted</span>
          </div>
        </GroupCollapsible>
      </div>
    );
  }

  return (
    <div className={styles.panelContainer}>
      {/* Granted Capabilities Section */}
      <GroupCollapsible
        title="Granted Capabilities"
        count={app.grantedCaps.length}
        defaultOpen
        className={styles.collapsibleSection}
      >
        <div className={styles.menuContent}>
          <Menu items={capabilityItems} background="none" border="none" />
        </div>
      </GroupCollapsible>

      {/* Danger Zone - only show if multiple capabilities */}
      {app.grantedCaps.length > 1 && (
        <div className={styles.dangerZoneSection}>
          <Card className={styles.dangerCard}>
            <CardItem
              icon={<AlertTriangle size={16} />}
              title="Revoke All Capabilities"
              description="This may cause the app to stop working correctly"
              className={styles.dangerCardItem}
            >
              <Button variant="danger" size="sm" onClick={handleRevokeAll}>
                Revoke All
              </Button>
            </CardItem>
          </Card>
        </div>
      )}
    </div>
  );
}
