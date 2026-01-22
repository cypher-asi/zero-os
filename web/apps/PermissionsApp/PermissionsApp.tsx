import { useState, useCallback, useEffect, useMemo } from 'react';
import { Panel, Text, Label } from '@cypher-asi/zui';
import { AppPermissions, type CapabilityInfo, type ObjectType } from '../../components/AppPermissions';
import type { AppManifest } from '../../components/PermissionDialog';
import { useSupervisor } from '../../desktop/hooks/useSupervisor';
import styles from './PermissionsApp.module.css';

// =============================================================================
// Types
// =============================================================================

interface ProcessWithCapabilities {
  pid: number;
  name: string;
  state: string;
  capabilities: {
    slot: number;
    objectType: ObjectType;
    permissions: { read: boolean; write: boolean; grant: boolean };
  }[];
}

interface AppWithPermissions {
  manifest: AppManifest;
  pid: number;
  grantedCaps: CapabilityInfo[];
}

// =============================================================================
// Helper Functions
// =============================================================================

function getAppIcon(name: string): string {
  const lowerName = name.toLowerCase();
  if (lowerName.includes('terminal')) return '⌨';
  if (lowerName.includes('clock')) return '🕐';
  if (lowerName.includes('calculator')) return '🔢';
  if (lowerName.includes('init')) return '⚡';
  if (lowerName.includes('idle')) return '💤';
  return '📱';
}

function getObjectTypeIcon(type: ObjectType): string {
  switch (type) {
    case 'Endpoint': return '↔';
    case 'Console': return '⌨';
    case 'Storage': return '💾';
    case 'Network': return '🌐';
    case 'Process': return '⚙';
    case 'Memory': return '🧠';
    default: return '?';
  }
}

function formatObjectType(type: ObjectType): string {
  switch (type) {
    case 'Endpoint': return 'IPC Endpoint';
    case 'Console': return 'Console Access';
    case 'Storage': return 'Storage';
    case 'Network': return 'Network';
    case 'Process': return 'Process Management';
    case 'Memory': return 'Memory Access';
    default: return type;
  }
}

function formatPermissions(perms: { read: boolean; write: boolean; grant: boolean }): string {
  const parts: string[] = [];
  if (perms.read) parts.push('R');
  if (perms.write) parts.push('W');
  if (perms.grant) parts.push('G');
  return parts.join('/') || 'none';
}

/**
 * Convert process data from supervisor to app manifest format
 */
function processToApp(proc: ProcessWithCapabilities): AppWithPermissions {
  // Determine if this is a factory (system) app
  const isFactory = proc.name === 'init' || proc.name === 'terminal' || 
                    proc.name === 'clock' || proc.name === 'calculator';
  
  return {
    manifest: {
      id: `com.orbital.${proc.name}`,
      name: proc.name.charAt(0).toUpperCase() + proc.name.slice(1),
      version: '1.0.0',
      description: `${proc.name} process`,
      capabilities: proc.capabilities.map(cap => ({
        objectType: cap.objectType,
        permissions: cap.permissions,
        reason: `${cap.objectType} access`,
        required: true,
      })),
      isFactory,
    },
    pid: proc.pid,
    grantedCaps: proc.capabilities.map(cap => ({
      slot: cap.slot,
      objectType: cap.objectType,
      permissions: cap.permissions,
    })),
  };
}

// =============================================================================
// PermissionsApp Component
// =============================================================================

export function PermissionsApp() {
  const supervisor = useSupervisor();
  const [processes, setProcesses] = useState<ProcessWithCapabilities[]>([]);
  const [selectedApp, setSelectedApp] = useState<AppWithPermissions | null>(null);
  const [refreshKey, setRefreshKey] = useState(0);

  // Fetch real process data from supervisor
  useEffect(() => {
    if (!supervisor) {
      console.log('[PermissionsApp] No supervisor available');
      return;
    }

    const fetchProcesses = () => {
      try {
        // Check if the method exists
        if (typeof supervisor.get_processes_with_capabilities_json !== 'function') {
          console.error('[PermissionsApp] get_processes_with_capabilities_json is not a function - WASM may need rebuild');
          // Fall back to get_process_list_json if available
          if (typeof supervisor.get_process_list_json === 'function') {
            const json = supervisor.get_process_list_json();
            console.log('[PermissionsApp] Fallback process list:', json);
            const data = JSON.parse(json);
            const converted: ProcessWithCapabilities[] = data.map((p: any) => ({
              pid: p.pid,
              name: p.name,
              state: p.state,
              capabilities: [],
            }));
            const filtered = converted.filter(p => p.name !== 'idle' && p.state !== 'Zombie');
            setProcesses(filtered);
          }
          return;
        }

        const json = supervisor.get_processes_with_capabilities_json();
        console.log('[PermissionsApp] Processes with caps:', json);
        const data: ProcessWithCapabilities[] = JSON.parse(json);
        // Filter out system processes we don't want to show (like idle)
        const filtered = data.filter(p => p.name !== 'idle' && p.state !== 'Zombie');
        setProcesses(filtered);
      } catch (e) {
        console.error('[PermissionsApp] Failed to fetch processes:', e);
      }
    };

    // Initial fetch
    fetchProcesses();

    // Poll for updates every 2 seconds
    const interval = setInterval(fetchProcesses, 2000);
    return () => clearInterval(interval);
  }, [supervisor, refreshKey]);

  // Convert processes to app format
  const apps = useMemo(() => {
    return processes.map(processToApp);
  }, [processes]);

  // Update selectedApp when processes change
  useEffect(() => {
    if (selectedApp) {
      const updated = apps.find(a => a.pid === selectedApp.pid);
      if (updated) {
        setSelectedApp(updated);
      } else {
        setSelectedApp(null);
      }
    }
  }, [apps, selectedApp?.pid]);

  // Calculate stats
  const totalApps = apps.length;
  const totalPermissions = apps.reduce((sum, app) => sum + app.grantedCaps.length, 0);

  // Handle revoking a permission
  const handleRevoke = useCallback((objectType: ObjectType) => {
    if (!selectedApp || !supervisor) return;

    // Find the capability slot for this object type
    const cap = selectedApp.grantedCaps.find(c => c.objectType === objectType);
    if (!cap) return;

    // Send revoke command to supervisor
    // The terminal command format is: revoke <pid> <slot>
    supervisor.send_input(`revoke ${selectedApp.pid} ${cap.slot}`);

    // Trigger refresh
    setRefreshKey(k => k + 1);
  }, [selectedApp, supervisor]);

  // Handle revoking all permissions
  const handleRevokeAll = useCallback(() => {
    if (!selectedApp || !supervisor) return;

    // Revoke each capability
    for (const cap of selectedApp.grantedCaps) {
      supervisor.send_input(`revoke ${selectedApp.pid} ${cap.slot}`);
    }

    // Trigger refresh
    setRefreshKey(k => k + 1);
  }, [selectedApp, supervisor]);

  return (
    <div className={styles.container}>
      {/* Header */}
      <div className={styles.header}>
        <div className={styles.title}>
          <div className={styles.titleIcon}>🛡️</div>
          <Text as="span" size="lg" weight="semibold">
            Permissions
          </Text>
        </div>
        <div className={styles.stats}>
          <div className={styles.stat}>
            <span className={styles.statValue}>{totalApps}</span>
            <span className={styles.statLabel}>apps</span>
          </div>
          <div className={styles.stat}>
            <span className={styles.statValue}>{totalPermissions}</span>
            <span className={styles.statLabel}>grants</span>
          </div>
        </div>
      </div>

      {/* Content */}
      <div className={styles.content}>
        {apps.length === 0 ? (
          <div className={styles.emptyState}>
            <div className={styles.emptyIcon}>🔒</div>
            <div className={styles.emptyTitle}>No Apps</div>
            <div className={styles.emptyText}>
              No applications are currently running.
            </div>
          </div>
        ) : (
          <div className={styles.appList}>
            {apps.map((app) => (
              <Panel
                key={app.manifest.id}
                variant="glass"
                className={styles.appCard}
                onClick={() => setSelectedApp(app)}
              >
                <div className={styles.appCardHeader}>
                  <div className={styles.appIcon}>{getAppIcon(app.manifest.name)}</div>
                  <div className={styles.appInfo}>
                    <div className={styles.appName}>
                      {app.manifest.name}
                      {app.manifest.isFactory && (
                        <Label size="xs" variant="success">System</Label>
                      )}
                      <Label size="xs" variant="default">PID {app.pid}</Label>
                    </div>
                    <div className={styles.appId}>{app.manifest.id}</div>
                  </div>
                </div>
                <div className={styles.permissionTags}>
                  {app.grantedCaps.length === 0 ? (
                    <span className={styles.noPermissions}>No capabilities</span>
                  ) : (
                    app.grantedCaps.map((cap) => (
                      <div key={`${cap.objectType}-${cap.slot}`} className={styles.permissionTag}>
                        <span className={styles.permissionTagIcon}>
                          {getObjectTypeIcon(cap.objectType)}
                        </span>
                        <span>{formatObjectType(cap.objectType)}</span>
                        <span>({formatPermissions(cap.permissions)})</span>
                      </div>
                    ))
                  )}
                </div>
              </Panel>
            ))}
          </div>
        )}
      </div>

      {/* Detail Panel */}
      {selectedApp && (
        <div className={styles.detailPanel}>
          <div className={styles.detailHeader}>
            <button
              className={styles.backButton}
              onClick={() => setSelectedApp(null)}
            >
              ←
            </button>
            <div className={styles.appIcon}>{getAppIcon(selectedApp.manifest.name)}</div>
            <div className={styles.appInfo}>
              <div className={styles.appName}>
                {selectedApp.manifest.name}
                {selectedApp.manifest.isFactory && (
                  <Label size="xs" variant="success">System</Label>
                )}
              </div>
              <div className={styles.appId}>{selectedApp.manifest.id}</div>
            </div>
          </div>
          <div className={styles.detailContent}>
            <AppPermissions
              app={selectedApp.manifest}
              grantedCaps={selectedApp.grantedCaps}
              onRevoke={handleRevoke}
              onRevokeAll={selectedApp.grantedCaps.length > 1 ? handleRevokeAll : undefined}
            />
          </div>
        </div>
      )}
    </div>
  );
}
