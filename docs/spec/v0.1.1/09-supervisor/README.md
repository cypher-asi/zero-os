# 09 - Web Supervisor

## Overview

The Web Supervisor (`zos-supervisor`) runs in the browser's main thread and acts as a boundary layer between user-space WASM processes and the kernel.

## Responsibilities

- **Kernel lifecycle**: Boot sequence, state management
- **Process spawning**: Request binaries, create Web Workers
- **Syscall dispatch**: Poll and dispatch syscalls to kernel
- **Console routing**: Route console output to UI callbacks
- **IPC message delivery**: Forward messages between processes

## What the Supervisor Does NOT Do

The supervisor is deliberately minimal:

- Does NOT execute application logic (that's in WASM processes)
- Does NOT manage windows (that's in `zos-desktop`)
- Does NOT handle UI events (that's in React)
- Does NOT store persistent state (that's in IndexedDB)

## Architecture

```
┌───────────────────────────────────────────────────────────────────┐
│                        Browser Main Thread                         │
│  ┌─────────────────────────────────────────────────────────────┐  │
│  │                        Supervisor                            │  │
│  │  ┌─────────────────┐  ┌─────────────────┐                   │  │
│  │  │     Kernel      │  │    WasmHal      │                   │  │
│  │  │  (zos-kernel)   │  │  (Web Workers)  │                   │  │
│  │  └─────────────────┘  └─────────────────┘                   │  │
│  │                                                              │  │
│  │  ┌─────────────────┐  ┌─────────────────┐                   │  │
│  │  │ Console Callbacks│  │ Spawn Callback  │                   │  │
│  │  │ (per-process)   │  │ (fetch WASM)    │                   │  │
│  │  └─────────────────┘  └─────────────────┘                   │  │
│  └─────────────────────────────────────────────────────────────┘  │
│                              │                                     │
│              ┌───────────────┼───────────────────┐                │
│              │               │                   │                │
│              ▼               ▼                   ▼                │
│  ┌─────────────────┐ ┌─────────────────┐ ┌─────────────────┐     │
│  │  Worker (P1)    │ │  Worker (P2)    │ │  Worker (P3)    │     │
│  │    (init)       │ │  (perm_mgr)     │ │  (terminal)     │     │
│  └─────────────────┘ └─────────────────┘ └─────────────────┘     │
└───────────────────────────────────────────────────────────────────┘
```

## Supervisor Structure

```rust
#[wasm_bindgen]
pub struct Supervisor {
    kernel: Kernel<WasmHal>,
    
    /// Per-process console callbacks
    console_callbacks: HashMap<u64, js_sys::Function>,
    
    /// Legacy single console callback
    console_callback: Option<js_sys::Function>,
    
    /// Spawn callback (JS fetches WASM)
    spawn_callback: Option<js_sys::Function>,
    
    /// Buffered console output
    console_buffer: Vec<String>,
    
    /// Axiom persistence state
    last_persisted_axiom_seq: u64,
    axiom_storage_ready: bool,
    
    /// Boot state
    init_spawned: bool,
    supervisor_pid: ProcessId,
    supervisor_initialized: bool,
}
```

## Boot Sequence

```rust
#[wasm_bindgen]
impl Supervisor {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        let hal = WasmHal::new();
        let kernel = Kernel::new(hal);
        
        Self {
            kernel,
            console_callbacks: HashMap::new(),
            // ...
        }
    }
    
    #[wasm_bindgen]
    pub fn boot(&mut self) {
        // 1. Register supervisor as PID 0
        self.supervisor_pid = self.kernel.register_supervisor();
        
        // 2. Create init endpoint
        let init_endpoint = self.kernel.create_init_endpoint();
        
        // 3. Request init spawn
        self.request_spawn("init", "init");
        
        self.supervisor_initialized = true;
    }
}
```

## Console Callback Routing

Each terminal registers its own console callback:

```rust
#[wasm_bindgen]
impl Supervisor {
    /// Register per-process console callback
    pub fn register_console_callback(&mut self, pid: u64, callback: js_sys::Function) {
        self.console_callbacks.insert(pid, callback);
    }
    
    /// Unregister callback when terminal closes
    pub fn unregister_console_callback(&mut self, pid: u64) {
        self.console_callbacks.remove(&pid);
    }
    
    /// Route console output to appropriate callback
    fn write_console_to_process(&mut self, pid: u64, text: &str) {
        // Try per-process callback first
        if let Some(callback) = self.console_callbacks.get(&pid) {
            let _ = callback.call1(&JsValue::null(), &JsValue::from_str(text));
            return;
        }
        
        // Fall back to legacy global callback
        if let Some(ref callback) = self.console_callback {
            let _ = callback.call1(&JsValue::null(), &JsValue::from_str(text));
        }
    }
}
```

## Syscall Polling

The supervisor polls syscalls from all processes:

```rust
#[wasm_bindgen]
impl Supervisor {
    pub fn poll_syscalls(&mut self) -> usize {
        // Get pending syscalls from HAL
        let pending = self.kernel.hal().poll_syscalls();
        let count = pending.len();
        
        for syscall_info in pending {
            let pid = ProcessId(syscall_info.pid);
            let data = self.kernel.hal().read_syscall_data(syscall_info.pid);
            
            // Process through Axiom gateway
            let result = self.process_syscall_internal(
                pid,
                syscall_info.syscall_num,
                syscall_info.args,
                &data,
            );
            
            // Complete syscall and wake worker
            self.kernel.hal().complete_syscall(syscall_info.pid, result);
        }
        
        // Drain console output
        self.drain_console_output();
        
        count
    }
    
    fn drain_console_output(&mut self) {
        let outputs = self.kernel.drain_console_output();
        for (pid, data) in outputs {
            if let Ok(text) = std::str::from_utf8(&data) {
                self.write_console_to_process(pid.0, text);
            }
        }
    }
}
```

## Process Spawning

Spawning is asynchronous—JS fetches the WASM binary:

```rust
#[wasm_bindgen]
impl Supervisor {
    /// Request spawn (JS will fetch and call complete_spawn)
    fn request_spawn(&mut self, proc_type: &str, name: &str) {
        if let Some(ref callback) = self.spawn_callback {
            let _ = callback.call2(
                &JsValue::null(),
                &JsValue::from_str(proc_type),
                &JsValue::from_str(name),
            );
        }
    }
    
    /// Complete spawn with fetched WASM binary
    #[wasm_bindgen]
    pub fn complete_spawn(&mut self, name: &str, binary: &[u8]) -> u64 {
        match self.kernel.spawn_process(name, binary) {
            Ok(pid) => pid.0,
            Err(_) => 0,
        }
    }
}
```

## Console Input Delivery

```rust
#[wasm_bindgen]
impl Supervisor {
    /// Send input to a specific terminal process
    pub fn send_input_to_process(&mut self, pid: u64, input: &str) {
        let process_id = ProcessId(pid);
        
        // Use privileged kernel API
        const TERMINAL_INPUT_SLOT: u32 = 1;
        let _ = self.kernel.deliver_console_input(
            process_id,
            TERMINAL_INPUT_SLOT,
            input.as_bytes(),
        );
    }
}
```

## Capability Revocation

```rust
#[wasm_bindgen]
impl Supervisor {
    /// Revoke a capability from any process (supervisor privilege)
    pub fn revoke_capability(&mut self, pid: u64, slot: u32) -> bool {
        const REVOKE_REASON_EXPLICIT: u8 = 1;
        
        match self.kernel.delete_capability_with_notification(
            ProcessId(pid),
            slot,
            REVOKE_REASON_EXPLICIT,
        ) {
            Ok(notification) => {
                // Deliver notification to affected process
                if notification.is_valid() {
                    let _ = self.kernel.deliver_revoke_notification(&notification);
                }
                true
            }
            Err(_) => false,
        }
    }
}
```

## Process Lifecycle

```rust
#[wasm_bindgen]
impl Supervisor {
    /// Kill a process
    pub fn kill_process(&mut self, pid: u64) {
        let process_id = ProcessId(pid);
        
        // Clean up supervisor state
        self.console_callbacks.remove(&pid);
        
        // Kill in kernel and HAL
        self.kernel.kill_process(process_id);
        let handle = WasmProcessHandle::new(pid);
        let _ = self.kernel.hal().kill_process(&handle);
    }
    
    /// Kill all processes
    pub fn kill_all_processes(&mut self) {
        let pids: Vec<ProcessId> = self.kernel
            .list_processes()
            .into_iter()
            .map(|(pid, _)| pid)
            .collect();
        
        for pid in pids {
            self.kill_process(pid.0);
        }
    }
}
```

## Debug Protocol

Init communicates with the supervisor via debug messages:

| Message | Purpose |
|---------|---------|
| `INIT:SPAWN:{name}` | Request process spawn |
| `INIT:GRANT:{pid}:{slot}:{type}:{id}:{perms}` | Grant capability |
| `INIT:REVOKE:{pid}:{slot}` | Revoke capability |
| `INIT:PERM_RESPONSE:{...}` | Permission operation result |

```rust
fn handle_sys_debug(&mut self, pid: ProcessId, data: &[u8]) -> i32 {
    if let Ok(s) = std::str::from_utf8(data) {
        if let Some(name) = s.strip_prefix("INIT:SPAWN:") {
            self.request_spawn(name, name);
        } else if s.starts_with("INIT:GRANT:") {
            syscall::handle_init_grant(&mut self.kernel, s);
        } else if s.starts_with("INIT:REVOKE:") {
            syscall::handle_init_revoke(&mut self.kernel, s);
        }
    }
    0
}
```

## JavaScript Integration

```typescript
// Create supervisor
const supervisor = new Supervisor();

// Set up callbacks
supervisor.set_console_callback((text: string) => {
    console.log(text);
});

supervisor.set_spawn_callback(async (type: string, name: string) => {
    const response = await fetch(`/processes/${type}.wasm`);
    const binary = await response.arrayBuffer();
    supervisor.complete_spawn(name, new Uint8Array(binary));
});

// Boot the system
supervisor.boot();

// Poll syscalls in animation frame
function tick() {
    supervisor.poll_syscalls();
    requestAnimationFrame(tick);
}
tick();
```

## Compliance Checklist

### Source Files
- `crates/zos-supervisor/src/supervisor/mod.rs` - Main supervisor
- `crates/zos-supervisor/src/supervisor/boot.rs` - Boot sequence
- `crates/zos-supervisor/src/supervisor/spawn.rs` - Process spawning
- `crates/zos-supervisor/src/syscall/*.rs` - Syscall handling
- `crates/zos-supervisor/src/hal.rs` - WasmHal

### Key Invariants
- [ ] All syscalls flow through Axiom gateway
- [ ] Console output routed to correct process callback
- [ ] Spawn completes asynchronously via callback
- [ ] Cleanup on process kill includes callback removal

### Differences from v0.1.0
- Per-process console callbacks for isolation
- Console output via SYS_CONSOLE_WRITE syscall
- Privileged kernel APIs for supervisor operations
- Debug message protocol for init communication
- Separated from desktop into own concern
