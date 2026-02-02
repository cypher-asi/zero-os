import { Shield, User, ArrowUpCircle, Check, Lock, Loader } from 'lucide-react';
import { useTierStatus } from '../../../hooks/useTierStatus';
import { useZeroIdAuth } from '../../../hooks/useZeroIdAuth';
import styles from './TierStatusPanel.module.css';

interface TierStatusPanelProps {
  /** Callback when upgrade button is clicked */
  onUpgradeClick?: () => void;
}

/**
 * TierStatusPanel - Displays the current identity tier status
 *
 * Shows whether the identity is managed or self-sovereign,
 * the number of linked auth methods, and upgrade eligibility.
 */
export function TierStatusPanel({ onUpgradeClick }: TierStatusPanelProps) {
  const { tierStatus, isLoading, error, refresh, isManaged, isSelfSovereign } = useTierStatus();
  const { remoteAuthState } = useZeroIdAuth();

  // Not connected to ZID
  if (!remoteAuthState) {
    return (
      <div className={styles.panel}>
        <div className={styles.content}>
          <div className={styles.notConnected}>
            <div className={styles.notConnectedIcon}>
              <Lock size={24} />
            </div>
            <p className={styles.notConnectedText}>
              Connect to ZERO ID to view your identity tier status.
            </p>
          </div>
        </div>
      </div>
    );
  }

  // Loading state
  if (isLoading && !tierStatus) {
    return (
      <div className={styles.panel}>
        <div className={styles.content}>
          <div className={styles.loading}>
            <Loader size={24} className={styles.spinner} />
          </div>
        </div>
      </div>
    );
  }

  // Error state
  if (error && !tierStatus) {
    return (
      <div className={styles.panel}>
        <div className={styles.content}>
          <div className={styles.error}>
            {error}
            <button onClick={refresh} style={{ marginLeft: '8px', cursor: 'pointer' }}>
              Retry
            </button>
          </div>
        </div>
      </div>
    );
  }

  // No tier status yet (shouldn't happen if connected)
  if (!tierStatus) {
    return (
      <div className={styles.panel}>
        <div className={styles.content}>
          <div className={styles.notConnected}>
            <div className={styles.notConnectedIcon}>
              <User size={24} />
            </div>
            <p className={styles.notConnectedText}>
              Unable to load tier status. Please try again later.
            </p>
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className={styles.panel}>
      <div className={styles.content}>
        {/* Tier Card */}
        <div className={styles.tierCard}>
          <div className={styles.tierHeader}>
            <div
              className={`${styles.tierIcon} ${
                isSelfSovereign ? styles.tierIconSelfSovereign : styles.tierIconManaged
              }`}
            >
              {isSelfSovereign ? <Shield size={20} /> : <User size={20} />}
            </div>
            <div className={styles.tierInfo}>
              <h3 className={styles.tierTitle}>
                {isSelfSovereign ? 'Self-Sovereign' : 'Managed'} Identity
              </h3>
              <p className={styles.tierDescription}>
                {isSelfSovereign
                  ? 'You have full control over your keys'
                  : 'Your identity is protected by ZERO ID'}
              </p>
            </div>
          </div>

          {/* Self-sovereign badge */}
          {isSelfSovereign && (
            <div className={styles.tierBadge}>
              <Shield size={14} />
              Full Key Custody
            </div>
          )}
        </div>

        {/* Stats */}
        <div className={styles.stats}>
          <div className={styles.stat}>
            <div className={styles.statValue}>{tierStatus.authMethodsCount}</div>
            <div className={styles.statLabel}>Auth Methods</div>
          </div>
          <div className={styles.stat}>
            <div className={styles.statValue}>
              {isSelfSovereign ? 'Full' : 'Custodial'}
            </div>
            <div className={styles.statLabel}>Key Control</div>
          </div>
        </div>

        {/* Upgrade Section (only for managed identities) */}
        {isManaged && (
          <div
            className={`${styles.upgradeSection} ${
              !tierStatus.canUpgrade ? styles.upgradeSectionDisabled : ''
            }`}
          >
            <div className={styles.upgradeHeader}>
              <ArrowUpCircle size={16} />
              <h4 className={styles.upgradeTitle}>Upgrade to Self-Sovereign</h4>
            </div>
            <p className={styles.upgradeDescription}>
              Take full control of your identity by generating a Neural Key. Your keys will be
              stored only on your devices.
            </p>

            {/* Requirements list */}
            {tierStatus.upgradeRequirements.length > 0 && (
              <ul className={styles.requirementsList}>
                {tierStatus.upgradeRequirements.map((req, i) => (
                  <li key={i} className={styles.requirementItem}>
                    {req}
                  </li>
                ))}
              </ul>
            )}

            {/* Upgrade eligibility check */}
            {tierStatus.canUpgrade && tierStatus.upgradeRequirements.length === 0 && (
              <ul className={styles.requirementsList}>
                <li className={`${styles.requirementItem} ${styles.requirementMet}`}>
                  <Check size={12} style={{ marginRight: '4px' }} />
                  All requirements met
                </li>
              </ul>
            )}

            <button
              className={styles.upgradeButton}
              onClick={onUpgradeClick}
              disabled={!tierStatus.canUpgrade}
            >
              <Shield size={16} />
              {tierStatus.canUpgrade ? 'Start Upgrade' : 'Requirements Not Met'}
            </button>
          </div>
        )}
      </div>
    </div>
  );
}
