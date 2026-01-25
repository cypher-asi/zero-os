# Stage 1.7: Web UI

> **Status**: ✅ **COMPLETE**
>
> **Goal**: Browser interface to inspect system state.

## Implementation Status

This stage is **fully implemented** with a comprehensive dashboard UI.

### What's Implemented

| Component | Status | Location |
|-----------|--------|----------|
| Process viewer | ✅ | `index.html` Dashboard |
| Memory visualization | ✅ | Memory bar + per-process |
| Endpoint list | ✅ | With queue depth, stats |
| IPC traffic viewer | ✅ | Real-time message log |
| Axiom log viewer | ✅ | Recent capability mutations |
| IndexedDB persistence | ✅ | Axiom log to IndexedDB |
| System metrics | ✅ | Uptime, proc count, memory, IPC/s |
| Interactive terminal | ✅ | Command input + output |
| Process spawning | ✅ | Quick spawn buttons |
| Process killing | ✅ | Kill button per process |

### Screenshot of Current UI

```
┌─────────────────────────────────────────────────────────────────────┐
│ Zero OS KERNEL          Uptime: 42.123s  Procs: 4  Memory: 512KB│
├─────────────────────┬───────────────────────────────────────────────┤
│ Processes        4  │                                               │
│ ─────────────────── │ Zero OS Terminal                          │
│ 1 init      64KB    │ Type 'help' for available commands.          │
│ 2 terminal  64KB    │                                               │
│ 3 idle      64KB    │ Zero> ps                                  │
│ 4 memhog   256KB  x │ PID  STATE    NAME                           │
│                     │ ---  -----    ----                           │
│ [+ memhog] [+ idle] │ 1    Running  init                           │
│                     │ 2    Running  terminal                       │
│ Memory Map    448KB │ 3    Running  idle                           │
│ ─────────────────── │ 4    Running  memhog                         │
│ [████████████████]  │                                               │
│ ■init ■term ■idle   │ Zero> _                                   │
│                     │                                               │
│ IPC Endpoints    3  │                                               │
│ ─────────────────── │                                               │
│ #1 P1  0 msg       │                                               │
│ #2 P2  0 msg       │                                               │
│ #3 P4  5 msg       │                                               │
│                     │                                               │
│ IPC Traffic   12msg │                                               │
│ ─────────────────── │                                               │
│ P3→P4 0x0001   64B │                                               │
│ P4→P3 0x0002  128B │                                               │
│                     │                                               │
│ Axiom Log       8   │                                               │
│ ─────────────────── │                                               │
│ #7 Grant   P1      │                                               │
│ #6 Create  P4      │                                               │
│ Storage: IndexedDB │                                               │
└─────────────────────┴───────────────────────────────────────────────┘
```

### Key Implementation Details

#### Dashboard Update Loop

```javascript
// apps/zos-supervisor/www/index.html
setInterval(updateDashboard, 500);  // Update every 500ms

function updateDashboard() {
    if (!supervisor) return;
    
    // Update header stats
    const uptimeMs = supervisor.get_uptime_ms();
    uptimeEl.textContent = formatUptime(uptimeMs);
    
    const procCount = supervisor.get_process_count();
    procCountEl.textContent = procCount;
    
    const totalMem = supervisor.get_total_memory();
    totalMemEl.textContent = formatBytes(totalMem);
    
    // Update panels
    updateProcessList();
    updateMemoryBar();
    updateEndpointList();
    updateIpcTraffic();
    updateAxiomLog();
}
```

#### Process List with Worker IDs

```javascript
function updateProcessList() {
    const processes = JSON.parse(supervisor.get_process_list_json());
    processListEl.innerHTML = processes.map((p, i) => {
        const stateClass = `state-${p.state.toLowerCase()}`;
        const workerInfo = p.worker_id > 0 
            ? `<span class="worker-id">worker:${p.worker_id}</span>` 
            : '<span class="worker-id virtual">virtual</span>';
        return `
            <div class="process-item">
                <span class="process-pid">${p.pid}</span>
                <span class="process-name" style="color: ${COLORS[i]}">${p.name}</span>
                ${workerInfo}
                <span class="process-mem">${formatBytes(p.memory)}</span>
                <span class="process-state ${stateClass}">${p.state}</span>
                ${p.pid > 2 ? `<button class="btn-kill" onclick="killProcess(${p.pid})">x</button>` : ''}
            </div>
        `;
    }).join('');
}
```

#### Memory Bar Visualization

```javascript
function updateMemoryBar() {
    const processes = JSON.parse(supervisor.get_process_list_json());
    const totalMem = processes.reduce((sum, p) => sum + p.memory, 0);
    
    memBarEl.innerHTML = processes.map((p, i) => {
        const pct = (p.memory / totalMem * 100).toFixed(1);
        return `
            <div class="mem-segment" 
                 style="width: ${pct}%; background: ${COLORS[i]}"
                 data-tooltip="${p.name}: ${formatBytes(p.memory)}">
            </div>
        `;
    }).join('');
}
```

#### IndexedDB Axiom Persistence

```javascript
// apps/zos-supervisor/www/index.html
window.AxiomStorage = {
    DB_NAME: 'Zero-axiom',
    STORE_NAME: 'log',
    
    async init() {
        return new Promise((resolve, reject) => {
            const request = indexedDB.open(this.DB_NAME, 1);
            request.onupgradeneeded = (e) => {
                const db = e.target.result;
                db.createObjectStore(this.STORE_NAME, { keyPath: 'seq' });
            };
            request.onsuccess = () => { this.db = request.result; resolve(true); };
        });
    },
    
    async persistEntry(entry) { /* ... */ },
    async loadAll() { /* ... */ },
    async getCount() { /* ... */ },
};

// Sync every 2 seconds
setInterval(async () => {
    if (supervisor) await supervisor.sync_axiom_log();
}, 2000);
```

### Terminal Commands

| Command | Description |
|---------|-------------|
| `help` | Show available commands |
| `ps` | List running processes |
| `caps` | List current process capabilities |
| `echo <text>` | Echo text |
| `time` | Show system uptime |
| `clear` | Clear terminal |
| `exit` | Exit terminal |
| `spawn <type>` | Spawn new process |
| `kill <pid>` | Kill process |

### Quick Action Buttons

```html
<div class="quick-actions">
    <button class="quick-btn" onclick="spawnProcess('memhog')">+ memhog</button>
    <button class="quick-btn" onclick="spawnProcess('sender')">+ sender</button>
    <button class="quick-btn" onclick="spawnProcess('receiver')">+ receiver</button>
    <button class="quick-btn" onclick="spawnProcess('idle')">+ idle</button>
</div>
```

### API Methods (Rust → JavaScript)

```rust
// apps/zos-supervisor/src/lib.rs
#[wasm_bindgen]
impl Supervisor {
    pub fn get_uptime_ms(&self) -> f64;
    pub fn get_process_count(&self) -> u32;
    pub fn get_total_memory(&self) -> u32;
    pub fn get_endpoint_count(&self) -> u32;
    pub fn get_total_ipc_messages(&self) -> u64;
    
    pub fn get_process_list_json(&self) -> String;
    pub fn get_endpoint_list_json(&self) -> String;
    pub fn get_ipc_traffic_json(&self, count: u32) -> String;
    pub fn get_axiom_log_json(&self, count: u32) -> String;
    pub fn get_axiom_stats_json(&self) -> String;
    
    pub fn send_input(&mut self, line: &str);
    pub fn complete_spawn(&mut self, name: &str, binary: &[u8]) -> u32;
    
    pub async fn init_axiom_storage(&mut self);
    pub async fn sync_axiom_log(&mut self);
}
```

## CSS Design

The UI uses a dark theme with:
- Background: `#0a0a12` (near black)
- Panel background: `#12121e`, `#1a1a2e`
- Accent color: `#4ade80` (green)
- Text: `#e0e0e0` (light gray)
- Process colors: Rotating palette

## Responsive Design

```css
@media (max-width: 900px) {
    .dashboard { width: 280px; }
}

@media (max-width: 700px) {
    .dashboard { display: none; }
}
```

## Verification Checklist

- [x] UI loads without errors
- [x] Process list updates in real-time
- [x] Memory bar shows per-process usage
- [x] Endpoint list shows queue depth
- [x] IPC traffic shows recent messages
- [x] Axiom log shows capability mutations
- [x] IndexedDB persistence works
- [x] Terminal accepts commands
- [x] Process spawn buttons work
- [x] Process kill buttons work
- [x] Command history (up/down arrows)
- [x] Responsive on smaller screens

## No Modifications Needed

This stage is complete with a full-featured dashboard. Optional future enhancements:

- [ ] Export/import CommitLog (requires Stage 1.6)
- [ ] Replay controls (requires Stage 1.6)
- [ ] Capability graph visualization
- [ ] Process tree view
- [ ] Message content inspection

## Relationship to Stage 1.8 (Desktop Environment)

Stage 1.7's dashboard is a **developer/debug UI**. Stage 1.8 introduces the **user-facing desktop** with windows and workspaces. They coexist as follows:

| Component | Stage 1.7 Role | Stage 1.8 Role |
|-----------|----------------|----------------|
| Dashboard | Main UI | Developer tools panel (optional window) |
| Terminal | Embedded in dashboard | Runs in a desktop window |
| Process list | Dashboard panel | Dock/taskbar + developer tools |
| System metrics | Header bar | Status bar or developer tools |

**What 1.8 reuses from 1.7:**
- All supervisor APIs (`get_process_list_json()`, etc.)
- IndexedDB Axiom persistence
- Terminal process (wrapped in a window)

**What 1.8 adds:**
- WebGPU-based compositor
- Window management layer
- Input routing to windows
- React-based window content

The dashboard can become a "Developer Tools" window within the desktop, or remain available as a separate debug mode.

## Phase 1 Status

With Stage 1.7 complete, the remaining work for Phase 1:

| Stage | Status | Blocker |
|-------|--------|---------|
| 1.1 Minimal Kernel | ✅ Complete | - |
| 1.2 Axiom Layer | ⚠️ Partial | SysLog/CommitLog needed |
| 1.3 Capabilities + IPC | ✅ Complete | - |
| 1.4 Process Management | ✅ Complete | - |
| 1.5 Init + Services | ⚠️ Partial | Optional for Phase 1 |
| 1.6 Replay + Testing | ❌ TODO | Depends on 1.2 |
| 1.7 Web UI | ✅ Complete | - |
| 1.8 Desktop Environment | ❌ TODO | - |

## Next Stage

Proceed to [Stage 1.8: Desktop Environment](stage-1.8-desktop-environment.md) for the full desktop with infinite canvas, windows, and workspaces.
