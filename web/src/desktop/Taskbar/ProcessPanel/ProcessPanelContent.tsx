import { useEffect, useState, useCallback, useRef } from 'react';
import { Button, Label, Text, Menu, type MenuItem } from '@cypher-asi/zui';
import { X } from 'lucide-react';
import { useSupervisor } from '@desktop/hooks/useSupervisor';
import { withSupervisorGuard } from '@desktop/main';
import styles from './ProcessPanel.module.css';

interface ProcessInfo {
  pid: number;
  name: string;
  state: string;
  memory: number;
  worker_id: number;
}

const PROCESS_MENU_ITEMS: MenuItem[] = [
  { id: 'kill', label: 'Kill Process', icon: <X size={12} /> },
];

interface AxiomStats {
  commits: number;
  events: number;
  persisted: number;
  pending: number;
  storage_ready: boolean;
}

// Color palette for processes
const COLORS = [
  '#01f4cb',
  '#60a5fa',
  '#f472b6',
  '#facc15',
  '#a78bfa',
  '#fb923c',
  '#01f4cb',
  '#f87171',
  '#818cf8',
  '#01f4cb',
  '#fbbf24',
  '#f97316',
];

function formatBytes(bytes: number): string {
  if (bytes >= 1024 * 1024) {
    const val = bytes / (1024 * 1024);
    return (val % 1 === 0 ? val.toFixed(0) : val.toFixed(1)) + ' MB';
  }
  if (bytes >= 1024) {
    const val = bytes / 1024;
    return (val % 1 === 0 ? val.toFixed(0) : val.toFixed(1)) + ' KB';
  }
  return bytes + ' B';
}

interface ProcessPanelContentProps {
  onClose: () => void;
}

export function ProcessPanelContent({ onClose: _onClose }: ProcessPanelContentProps) {
  const supervisor = useSupervisor();
  const [processes, setProcesses] = useState<ProcessInfo[]>([]);
  const [axiomStats, setAxiomStats] = useState<AxiomStats | null>(null);
  const [openMenuPid, setOpenMenuPid] = useState<number | null>(null);
  const menuWrapperRef = useRef<HTMLDivElement>(null);

  // Close menu when clicking outside
  useEffect(() => {
    if (openMenuPid === null) return;

    const handleClickOutside = (event: MouseEvent) => {
      if (menuWrapperRef.current && !menuWrapperRef.current.contains(event.target as Node)) {
        setOpenMenuPid(null);
      }
    };

    document.addEventListener('mousedown', handleClickOutside);
    return () => document.removeEventListener('mousedown', handleClickOutside);
  }, [openMenuPid]);

  // Update dashboard data
  useEffect(() => {
    if (!supervisor) return;

    const update = () => {
      // Use supervisor guard to prevent "recursive use of an object" errors
      withSupervisorGuard(() => {
        try {
          const procs = JSON.parse(supervisor.get_process_list_json());
          setProcesses(procs);

          const stats = JSON.parse(supervisor.get_axiom_stats_json());
          setAxiomStats(stats);
        } catch {
          // Ignore parse errors or guard returning undefined
        }
      });
    };

    update();
    const interval = setInterval(update, 500);
    return () => clearInterval(interval);
  }, [supervisor]);

  const spawnProcess = useCallback(
    (type: string) => {
      if (supervisor) {
        supervisor.send_input(`spawn ${type}`);
      }
    },
    [supervisor]
  );

  const killProcess = useCallback(
    (pid: number) => {
      if (supervisor) {
        supervisor.send_input(`kill ${pid}`);
      }
    },
    [supervisor]
  );

  const totalMemory = processes.reduce((s, p) => s + p.memory, 0);

  return (
    <div className={styles.contentWrapper}>
      {/* Processes Section */}
      <div>
        <div className={styles.sectionHeader}>
          <span className={styles.sectionTitle}>Processes</span>
          <span className={styles.sectionStats}>{processes.length}</span>
        </div>
        <div className={styles.processList}>
          {processes.map((p, i) => (
            <div key={p.pid} className={styles.processItem}>
              <Text as="span" size="xs" variant="muted" className={styles.processPid}>
                {p.pid}
              </Text>
              <span className={styles.processName} style={{ color: COLORS[i % COLORS.length] }}>
                {p.name}
              </span>
              <Text as="span" size="xs" variant="muted" className={styles.processMem}>
                {formatBytes(p.memory)}
              </Text>
              <Label
                variant={
                  p.state === 'Running'
                    ? 'success'
                    : p.state === 'Blocked'
                      ? 'warning'
                      : 'danger'
                }
                size="xs"
                className={styles.processState}
              >
                {p.state}
              </Label>
              <div className={styles.processMoreWrapper}>
                <Button
                  variant="ghost"
                  size="sm"
                  iconOnly
                  className={styles.moreButton}
                  onMouseDown={(e) => e.stopPropagation()}
                  onClick={() => setOpenMenuPid(openMenuPid === p.pid ? null : p.pid)}
                >
                  ⋮
                </Button>
                {openMenuPid === p.pid && (
                  <div
                    ref={menuWrapperRef}
                    className={styles.processMenuWrapper}
                    onMouseDown={(e) => e.stopPropagation()}
                  >
                    <Menu
                      items={PROCESS_MENU_ITEMS}
                      onChange={(id) => {
                        if (id === 'kill') {
                          killProcess(p.pid);
                        }
                        setOpenMenuPid(null);
                      }}
                      variant="solid"
                      border="default"
                      rounded="md"
                      width={140}
                    />
                  </div>
                )}
              </div>
            </div>
          ))}
        </div>
      </div>

      {/* Memory Section */}
      <div className={styles.memorySection}>
        <div className={styles.sectionHeader}>
          <span className={styles.sectionTitle}>Memory</span>
          <span className={styles.sectionStats}>{formatBytes(totalMemory)}</span>
        </div>
        <div className={styles.memBar}>
          {processes.map((p, i) => {
            const total = totalMemory || 1;
            const pct = (p.memory / total) * 100;
            return (
              <div
                key={p.pid}
                className={styles.memSegment}
                style={{ width: `${pct}%`, background: COLORS[i % COLORS.length] }}
                title={`${p.name}: ${formatBytes(p.memory)}`}
              />
            );
          })}
        </div>
        <div className={styles.memLegend}>
          {processes.slice(0, 4).map((p, i) => (
            <div key={p.pid} className={styles.memLegendItem}>
              <span
                className={styles.memLegendColor}
                style={{ background: COLORS[i % COLORS.length] }}
              />
              <span>{p.name}</span>
            </div>
          ))}
        </div>
      </div>

      {/* Axiom Section */}
      {axiomStats && (
        <div className={styles.axiomSection}>
          <div className={styles.sectionHeader}>
            <span className={styles.sectionTitle}>Axiom</span>
            <span className={styles.sectionStats}>{axiomStats.commits} commits</span>
          </div>
          <div className={styles.axiomStats}>
            <div className={styles.axiomRow}>
              <span className={styles.axiomLabel}>Storage</span>
              <span
                className={
                  axiomStats.storage_ready ? styles.axiomValueSuccess : styles.axiomValueWarning
                }
              >
                {axiomStats.storage_ready ? 'Ready' : 'Not initialized'}
              </span>
            </div>
            <div className={styles.axiomRow}>
              <span className={styles.axiomLabel}>Events</span>
              <span className={styles.axiomValue}>{axiomStats.events}</span>
            </div>
            <div className={styles.axiomRow}>
              <span className={styles.axiomLabel}>Persisted</span>
              <span className={styles.axiomValue}>{axiomStats.persisted}</span>
            </div>
            <div className={styles.axiomRow}>
              <span className={styles.axiomLabel}>Pending</span>
              <span className={styles.axiomValue}>{axiomStats.pending}</span>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
