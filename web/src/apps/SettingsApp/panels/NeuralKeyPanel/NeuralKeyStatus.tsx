import {
  GroupCollapsible,
  ButtonCopy,
  Card,
  CardItem,
  Label,
} from '@cypher-asi/zui';
import {
  Binary,
  Key,
  Calendar,
  AlertTriangle,
} from 'lucide-react';
import { SetupStatusSection } from './SetupStatusSection';
import styles from './NeuralKeyPanel.module.css';

interface PublicIdentifiers {
  identitySigningPubKey: string;
  machineSigningPubKey: string;
  machineEncryptionPubKey: string;
}

interface NeuralKeyStatusProps {
  publicIdentifiers: PublicIdentifiers | null;
  createdAt: number | null;
  // Setup status props
  hasMachineKey: boolean;
  isEnrolled: boolean;
  sessionStatus: 'none' | 'expired' | 'active';
  enrollmentRetrying: boolean;
  sessionRetrying: boolean;
  enrollmentError: string | null;
  sessionError: string | null;
  isAuthenticating: boolean;
  getTimeRemaining: () => string;
  onRetryEnrollment: () => void;
  onRetrySession: () => void;
}

/**
 * Format date for display
 */
function formatDate(timestamp: number): string {
  return new Date(timestamp).toLocaleDateString('en-US', {
    year: 'numeric',
    month: 'short',
    day: 'numeric',
    hour: '2-digit',
    minute: '2-digit',
  });
}

/**
 * Format public key for display (truncate)
 */
function formatPubKey(key: string): string {
  if (key.length <= 20) return key;
  return `${key.slice(0, 10)}...${key.slice(-8)}`;
}

/**
 * Neural Key Status Component
 * 
 * Displays the active state of a Neural Key including:
 * - Key status indicator
 * - Identity key fingerprint
 * - Creation date
 * - Setup status section
 * - Recovery warning
 */
export function NeuralKeyStatus({
  publicIdentifiers,
  createdAt,
  hasMachineKey,
  isEnrolled,
  sessionStatus,
  enrollmentRetrying,
  sessionRetrying,
  enrollmentError,
  sessionError,
  isAuthenticating,
  getTimeRemaining,
  onRetryEnrollment,
  onRetrySession,
}: NeuralKeyStatusProps) {
  return (
    <div className={styles.panelContainer}>
      <GroupCollapsible title="Neural Key" defaultOpen className={styles.collapsibleSection}>
        <div className={styles.identitySection}>
          {publicIdentifiers && (
            <div className={styles.keyDetailsRow}>
              <div className={styles.statusHeroColumn}>
                <div className={styles.statusIconActive}>
                  <Binary size={32} />
                </div>
                <Label size="sm" variant="success">
                  Neural Key Active
                </Label>
              </div>

              <div className={styles.keyDetailsColumn}>
                <div className={styles.keyDetailItem}>
                  <div className={styles.keyDetailLabel}>
                    <Key size={14} />
                    <span>Identity Key</span>
                  </div>
                  <div className={styles.neuralprintRow}>
                    <code className={styles.keyDetailValue}>
                      {formatPubKey(publicIdentifiers.identitySigningPubKey)}
                    </code>
                    <ButtonCopy text={publicIdentifiers.identitySigningPubKey} />
                  </div>
                </div>

                <div className={styles.keyDetailItem}>
                  <div className={styles.keyDetailLabel}>
                    <Calendar size={14} />
                    <span>Created</span>
                  </div>
                  <span className={styles.keyDetailValue}>
                    {createdAt ? formatDate(createdAt) : 'Unknown'}
                  </span>
                </div>
              </div>
            </div>
          )}

          <SetupStatusSection
            hasMachineKey={hasMachineKey}
            isEnrolled={isEnrolled}
            sessionStatus={sessionStatus}
            enrollmentRetrying={enrollmentRetrying}
            sessionRetrying={sessionRetrying}
            enrollmentError={enrollmentError}
            sessionError={sessionError}
            isAuthenticating={isAuthenticating}
            getTimeRemaining={getTimeRemaining}
            onRetryEnrollment={onRetryEnrollment}
            onRetrySession={onRetrySession}
          />

          <Card className={styles.infoCard}>
            <CardItem
              icon={<AlertTriangle size={16} />}
              title="Lost your shards or password?"
              description="If you forget your password and lose all 3 backup shards, you won't be able to recover your identity on a new device."
            />
          </Card>
        </div>
      </GroupCollapsible>
    </div>
  );
}
