import {
  Button,
  Card,
  CardItem,
  Label,
} from '@cypher-asi/zui';
import {
  Copy,
  Check,
  AlertTriangle,
} from 'lucide-react';
import styles from './NeuralKeyPanel.module.css';

interface NeuralShard {
  index: number;
  hex: string;
}

interface ShardDisplayProps {
  shards: NeuralShard[];
  isCopied: (key: string) => boolean;
  onCopyAll: () => void;
  onCopyShard: (hex: string, key: string) => void;
}

/**
 * Truncate shard hex for display (first 10...last 10 chars)
 */
function truncateShardHex(hex: string): string {
  if (hex.length <= 20) return hex;
  return `${hex.slice(0, 10)}...${hex.slice(-10)}`;
}

/**
 * Shard Display Component
 * 
 * Displays Neural Key shards for backup with copy functionality.
 * Used in the backup wizard step.
 */
export function ShardDisplay({
  shards,
  isCopied,
  onCopyAll,
  onCopyShard,
}: ShardDisplayProps) {
  return (
    <div className={styles.wizardStepContent}>
      <Card className={styles.warningCard}>
        <CardItem
          icon={<AlertTriangle size={16} />}
          title="Save these shards now!"
          description="These will only be shown once. Store each shard in a separate secure location. To recover your identity on a new device, you'll need 1 shard plus your password."
          className={styles.warningCardItem}
        />
      </Card>

      <div className={styles.copyAllContainer}>
        <Button
          variant={isCopied('all') ? 'primary' : 'secondary'}
          size="sm"
          onClick={onCopyAll}
        >
          {isCopied('all') ? (
            <>
              <Check size={14} />
              Copied All Shards
            </>
          ) : (
            <>
              <Copy size={14} />
              Copy All Shards
            </>
          )}
        </Button>
      </div>

      <div className={styles.shardsContainer}>
        {shards.map((shard, index) => (
          <div key={shard.index} className={styles.shardItem}>
            <div className={styles.shardInfo}>
              <Label size="xs" variant="default">
                Shard {shard.index}
              </Label>
              <code className={styles.shardCodeInline}>{truncateShardHex(shard.hex)}</code>
            </div>
            <Button
              variant={isCopied(`shard-${index}`) ? 'primary' : 'ghost'}
              size="xs"
              onClick={() => onCopyShard(shard.hex, `shard-${index}`)}
            >
              {isCopied(`shard-${index}`) ? (
                <>
                  <Check size={12} />
                  Copied
                </>
              ) : (
                <>
                  <Copy size={12} />
                  Copy
                </>
              )}
            </Button>
          </div>
        ))}
      </div>
    </div>
  );
}
