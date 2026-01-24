import { useState, useCallback } from 'react';
import { Button, Card, CardItem, Input, Text } from '@cypher-asi/zui';
import { Cpu, Check, X, AlertTriangle, Loader, Shield } from 'lucide-react';
import { useMachineKeys, type KeyScheme } from '../../../../desktop/hooks/useMachineKeys';
import { usePanelDrill } from '../../context';
import styles from './GenerateMachineKeyPanel.module.css';

/**
 * Generate Machine Key Panel
 *
 * Drill-down panel for creating a new machine key.
 * Shows a form with machine name input, key scheme selector, and Generate/Cancel buttons.
 */
export function GenerateMachineKeyPanel() {
  const { createMachineKey } = useMachineKeys();
  const { navigateBack } = usePanelDrill();
  const [machineName, setMachineName] = useState('');
  const [keyScheme, setKeyScheme] = useState<KeyScheme>('classical');
  const [isGenerating, setIsGenerating] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleGenerate = useCallback(async () => {
    if (!machineName.trim()) return;

    setIsGenerating(true);
    setError(null);
    try {
      await createMachineKey(machineName.trim(), undefined, keyScheme);
      // Navigate back after successful creation
      navigateBack();
    } catch (err) {
      console.error('Failed to create machine key:', err);
      setError(err instanceof Error ? err.message : 'Failed to generate machine key');
    } finally {
      setIsGenerating(false);
    }
  }, [machineName, keyScheme, createMachineKey, navigateBack]);

  const handleCancel = useCallback(() => {
    // Navigate back to Machine Keys panel
    navigateBack();
  }, [navigateBack]);

  return (
    <div className={styles.panelContainer}>
      <div className={styles.identitySection}>
        <div className={styles.heroIcon}>
          <Cpu size={48} strokeWidth={1} />
        </div>
        <Text size="md" className={styles.heroTitle}>
          Generate Machine Key
        </Text>
        <Text size="sm" className={styles.heroDescription}>
          Give this machine a recognizable name and choose a key scheme for cryptographic
          operations.
        </Text>

        <div className={styles.addForm}>
          <Input
            value={machineName}
            onChange={(e) => setMachineName(e.target.value)}
            placeholder="Machine name (e.g., Work Laptop)"
            autoFocus
            onKeyDown={(e) => {
              if (e.key === 'Enter' && machineName.trim() && !isGenerating) {
                handleGenerate();
              }
            }}
          />

          <div className={styles.schemeSection}>
            <Text size="xs" className={styles.schemeLabel}>
              Key Scheme
            </Text>
            <select
              value={keyScheme}
              onChange={(e) => setKeyScheme(e.target.value as KeyScheme)}
              className={styles.schemeSelect}
            >
              <option value="classical">Classical (Ed25519 + X25519)</option>
              <option value="pq_hybrid">Post-Quantum Hybrid (+ ML-DSA-65 + ML-KEM-768)</option>
            </select>
          </div>

          {keyScheme === 'pq_hybrid' && (
            <Card className={styles.infoCard}>
              <CardItem
                icon={<Shield size={14} />}
                title="Post-Quantum Protection"
                description="PQ keys are larger (~3KB vs 64 bytes) but provide long-term protection against quantum computers. Recommended for high-security applications."
                className={styles.infoCardItem}
              />
            </Card>
          )}

          {error && (
            <Card className={styles.dangerCard}>
              <CardItem
                icon={<AlertTriangle size={14} />}
                title="Error"
                description={error}
                className={styles.dangerCardItem}
              />
            </Card>
          )}

          <div className={styles.addFormButtons}>
            <Button variant="ghost" size="md" onClick={handleCancel} disabled={isGenerating}>
              <X size={14} />
              Cancel
            </Button>
            <Button
              variant="primary"
              size="md"
              onClick={handleGenerate}
              disabled={isGenerating || !machineName.trim()}
            >
              {isGenerating ? (
                <>
                  <Loader size={14} className={styles.spinner} />
                  Generating...
                </>
              ) : (
                <>
                  <Check size={14} />
                  Generate
                </>
              )}
            </Button>
          </div>
        </div>
      </div>
    </div>
  );
}
