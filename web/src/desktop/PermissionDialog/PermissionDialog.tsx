import { useState, useCallback, useMemo, useEffect, useRef } from 'react';
import { Panel, Button, Text, Label } from '@cypher-asi/zui';
import type {
  ObjectType,
  Permissions,
  CapabilityRequest,
  AppManifest,
} from '@/types/permissions';
import styles from './PermissionDialog.module.css';

// Re-export types for consumers
export type { ObjectType, Permissions, CapabilityRequest, AppManifest };

/**
 * App manifest with required fields for permission dialog
 * The dialog requires description and capabilities to be present
 */
export interface PermissionDialogApp extends AppManifest {
  description: string;
  capabilities: CapabilityRequest[];
}

// =============================================================================
// Component Props
// =============================================================================

export interface PermissionDialogProps {
  /** App requesting capabilities */
  app: PermissionDialogApp;
  /** Called when user approves (with list of approved capabilities) */
  onApprove: (approved: CapabilityRequest[]) => void;
  /** Called when user denies all capabilities */
  onDeny: () => void;
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
// PermissionDialog Component
// =============================================================================

/**
 * Capability request dialog for third-party apps.
 *
 * Shows requested capabilities from the app manifest and allows users to
 * selectively approve or deny capabilities. Required capabilities are
 * automatically selected and cannot be deselected.
 */
export function PermissionDialog({ app, onApprove, onDeny }: PermissionDialogProps) {
  const dialogRef = useRef<HTMLDivElement>(null);

  // Initialize selected state: required capabilities are always selected
  const initialSelected = useMemo(() => {
    const set = new Set<number>();
    app.capabilities.forEach((cap, i) => {
      if (cap.required) {
        set.add(i);
      }
    });
    return set;
  }, [app.capabilities]);

  const [selected, setSelected] = useState<Set<number>>(initialSelected);

  // Focus trap and keyboard handling for modal dialog
  useEffect(() => {
    const dialog = dialogRef.current;
    if (!dialog) return;

    // Focus the dialog when it opens
    dialog.focus();

    // Get all focusable elements within the dialog
    const getFocusableElements = (): HTMLElement[] => {
      return Array.from(
        dialog.querySelectorAll<HTMLElement>(
          'button:not([disabled]), [href], input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])'
        )
      );
    };

    const handleKeyDown = (e: KeyboardEvent) => {
      // Close on Escape
      if (e.key === 'Escape') {
        e.preventDefault();
        onDeny();
        return;
      }

      // Focus trap on Tab
      if (e.key === 'Tab') {
        const focusable = getFocusableElements();
        if (focusable.length === 0) return;

        const firstElement = focusable[0];
        const lastElement = focusable[focusable.length - 1];

        if (e.shiftKey) {
          // Shift+Tab: if on first element, go to last
          if (document.activeElement === firstElement) {
            e.preventDefault();
            lastElement.focus();
          }
        } else {
          // Tab: if on last element, go to first
          if (document.activeElement === lastElement) {
            e.preventDefault();
            firstElement.focus();
          }
        }
      }
    };

    dialog.addEventListener('keydown', handleKeyDown);
    return () => dialog.removeEventListener('keydown', handleKeyDown);
  }, [onDeny]);

  // Toggle a capability selection
  const toggleCapability = useCallback(
    (index: number) => {
      const cap = app.capabilities[index];
      // Can't toggle required capabilities
      if (cap.required) return;

      setSelected((prev) => {
        const next = new Set(prev);
        if (next.has(index)) {
          next.delete(index);
        } else {
          next.add(index);
        }
        return next;
      });
    },
    [app.capabilities]
  );

  // Handle approve - pass only selected capabilities
  const handleApprove = useCallback(() => {
    const approved = app.capabilities.filter((_, i) => selected.has(i));
    onApprove(approved);
  }, [app.capabilities, selected, onApprove]);

  // Check if any required capability is missing (shouldn't happen with proper init)
  const hasRequiredCapabilities = app.capabilities
    .filter((c) => c.required)
    .every((_, i) => selected.has(i));

  // Check if this is a potentially dangerous capability request
  const hasSensitivePermissions = app.capabilities.some(
    (cap) =>
      cap.objectType === 'Network' ||
      cap.objectType === 'Process' ||
      (cap.objectType === 'Storage' && cap.permissions.write)
  );

  return (
    <div className={styles.overlay} onClick={onDeny} role="presentation" aria-hidden="true">
      <Panel
        ref={dialogRef}
        variant="glass"
        className={styles.dialog}
        onClick={(e: React.MouseEvent) => e.stopPropagation()}
        role="dialog"
        aria-modal="true"
        aria-labelledby="permission-dialog-title"
        aria-describedby="permission-dialog-description"
        tabIndex={-1}
      >
        {/* Header */}
        <div className={styles.header}>
          <div className={styles.title}>
            <div className={styles.icon} aria-hidden="true">
              🔐
            </div>
            <Text as="span" size="sm" className={styles.appName} id="permission-dialog-title">
              {app.name} Permission Request
            </Text>
            <Label size="xs" variant="default">
              v{app.version}
            </Label>
          </div>
          <Text as="div" size="sm" className={styles.subtitle} id="permission-dialog-description">
            This app is requesting the following capabilities:
          </Text>
        </div>

        {/* Content */}
        <div className={styles.content}>
          {/* Warning for sensitive capabilities */}
          {hasSensitivePermissions && (
            <div className={styles.warning}>
              <span className={styles.warningIcon}>⚠</span>
              <Text as="span" size="xs" className={styles.warningText}>
                This app requests sensitive capabilities. Only approve if you trust the source.
              </Text>
            </div>
          )}

          {app.capabilities.length === 0 ? (
            <div className={styles.emptyState}>
              <Text as="p" size="sm" variant="muted">
                This app doesn't require any special capabilities.
              </Text>
            </div>
          ) : (
            <div className={styles.permissionList}>
              {app.capabilities.map((cap, index) => (
                <div
                  key={index}
                  className={`${styles.permissionItem} ${
                    cap.required ? styles.permissionItemDisabled : ''
                  }`}
                >
                  <input
                    type="checkbox"
                    className={styles.checkbox}
                    checked={selected.has(index)}
                    disabled={cap.required}
                    onChange={() => toggleCapability(index)}
                    aria-label={`${cap.required ? 'Required: ' : ''}${formatObjectType(cap.objectType)} - ${formatPermissions(cap.permissions)} access`}
                  />
                  <div className={styles.permissionInfo}>
                    <div className={styles.permissionHeader}>
                      <span className={styles.permissionType}>
                        {getObjectTypeIcon(cap.objectType)} {formatObjectType(cap.objectType)}
                      </span>
                      <span className={styles.permissionPerms}>
                        {formatPermissions(cap.permissions)}
                      </span>
                      {cap.required && <span className={styles.requiredBadge}>Required</span>}
                    </div>
                    <Text as="p" size="xs" variant="muted" className={styles.permissionReason}>
                      {cap.reason}
                    </Text>
                  </div>
                </div>
              ))}
            </div>
          )}
        </div>

        {/* Footer */}
        <div className={styles.footer}>
          <Button variant="ghost" size="md" onClick={onDeny}>
            Deny All
          </Button>
          <Button
            variant="primary"
            size="md"
            onClick={handleApprove}
            disabled={!hasRequiredCapabilities}
          >
            {selected.size === 0 ? 'Allow' : `Allow Selected (${selected.size})`}
          </Button>
        </div>
      </Panel>
    </div>
  );
}
