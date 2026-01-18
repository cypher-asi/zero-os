# Stage 1.4: Process Management

> **Status**: ✅ **COMPLETE**
>
> **Goal**: Support multiple processes running in Web Workers.

## Implementation Status

This stage is **fully implemented** with Web Worker-based process isolation.

### What's Implemented

| Component | Status | Location |
|-----------|--------|----------|
| Process struct | ✅ | `crates/orbital-kernel/src/lib.rs:45-75` |
| ProcessState enum | ✅ | Running, Blocked, Zombie |
| ProcessMetrics | ✅ | Memory, IPC stats, syscall count |
| Process table | ✅ | `BTreeMap<ProcessId, Process>` |
| `register_process()` | ✅ | `crates/orbital-kernel/src/lib.rs:849-876` |
| `kill_process()` | ✅ | `crates/orbital-kernel/src/lib.rs:879-900` |
| Web Worker spawning | ✅ | `apps/orbital-web/www/worker.js` |
| SharedArrayBuffer syscalls | ✅ | Atomics-based mailbox |
| Sender verification | ✅ | PID from worker context |
| Test processes | ✅ | idle, memhog, sender, receiver, pingpong |

### Architecture

```
Browser Main Thread
├── Supervisor (orbital-web WASM)
│   ├── Kernel state (processes, capabilities, endpoints)
│   ├── Syscall mailbox polling (SharedArrayBuffer)
│   └── IPC message routing
│
├── Worker: terminal (PID 2)
│   ├── SharedArrayBuffer memory
│   ├── Atomics-based syscalls
│   └── Blocked waiting for input
│
├── Worker: idle (PID 3)
│   ├── SharedArrayBuffer memory
│   └── Loops calling yield
│
└── Worker: sender/receiver (PID 4+)
    ├── SharedArrayBuffer memory
    └── IPC messaging
```

### Key Implementation Details

#### Process Registration

```rust
// crates/orbital-kernel/src/lib.rs
pub fn register_process(&mut self, name: &str) -> ProcessId {
    let pid = ProcessId(self.next_pid);
    self.next_pid += 1;

    let now = self.uptime_nanos();
    let process = Process {
        pid,
        name: String::from(name),
        state: ProcessState::Running,
        metrics: ProcessMetrics {
            memory_size: 65536,  // Initial 64KB
            ipc_sent: 0,
            ipc_received: 0,
            ipc_bytes_sent: 0,
            ipc_bytes_received: 0,
            syscall_count: 0,
            last_active_ns: now,
            start_time_ns: now,
        },
    };
    self.processes.insert(pid, process);
    self.cap_spaces.insert(pid, CapabilitySpace::new());
    
    pid
}
```

#### Web Worker Bootstrap

```javascript
// apps/orbital-web/www/worker.js
self.onmessage = async (event) => {
    const { binary, pid } = event.data;
    
    // Create shared WASM memory
    const memory = new WebAssembly.Memory({
        initial: 256,   // 16MB initial
        maximum: 1024,  // 64MB max
        shared: true    // Enable SharedArrayBuffer
    });
    
    // Compile and instantiate WASM
    const instance = await WebAssembly.instantiate(module, {
        env: {
            memory: memory,
            orbital_syscall: orbital_syscall,
            orbital_send_bytes: orbital_send_bytes,
            orbital_recv_bytes: orbital_recv_bytes,
            orbital_yield: orbital_yield,
            orbital_get_pid: orbital_get_pid,
        }
    });
    
    // Send memory to supervisor for syscall polling
    self.postMessage({ type: 'memory', pid, buffer: memory.buffer });
    
    // Run process
    instance.exports._start();
};
```

#### Syscall Mailbox Layout

| Offset | Size | Field |
|--------|------|-------|
| 0 | 4 | status (0=idle, 1=pending, 2=ready) |
| 4 | 4 | syscall_num |
| 8 | 4 | arg0 |
| 12 | 4 | arg1 |
| 16 | 4 | arg2 |
| 20 | 4 | result |
| 24 | 4 | data_len |
| 28 | 4068 | data buffer |
| 56 | 4 | pid |

#### Syscall Flow

1. Process writes params to mailbox
2. Process sets status to PENDING
3. Process calls `Atomics.wait()` (blocks)
4. Supervisor polls mailboxes, finds PENDING
5. Supervisor processes syscall, writes result
6. Supervisor sets status to READY, calls `Atomics.notify()`
7. Process wakes up, reads result

### Test Processes

| Process | Purpose | File |
|---------|---------|------|
| idle | Continuous yield loop | `src/bin/idle.rs` |
| memhog | Allocates memory | `src/bin/memhog.rs` |
| sender | Sends IPC messages | `src/bin/sender.rs` |
| receiver | Receives IPC messages | `src/bin/receiver.rs` |
| pingpong | Bidirectional IPC | `src/bin/pingpong.rs` |

### Process Metrics Tracked

```rust
pub struct ProcessMetrics {
    pub memory_size: usize,          // WASM linear memory size
    pub ipc_sent: u64,               // Messages sent
    pub ipc_received: u64,           // Messages received
    pub ipc_bytes_sent: u64,         // Total bytes sent
    pub ipc_bytes_received: u64,     // Total bytes received
    pub syscall_count: u64,          // Total syscalls
    pub last_active_ns: u64,         // Last activity time
    pub start_time_ns: u64,          // Process start time
}
```

## Tests

All process management tests pass:

```bash
cargo test -p orbital-kernel
```

| Test | Description |
|------|-------------|
| `test_process_registration` | Register multiple processes |
| `test_process_kill` | Kill removes process and endpoints |
| `test_process_cleanup_removes_endpoints` | Cleanup on kill |
| `test_memory_allocation` | Allocate/free memory tracking |
| `test_system_metrics` | System-wide stats |
| `test_ipc_traffic_log` | Traffic monitoring |

## Invariants Verified

### 1. Process Isolation ✅

- ✅ Each process has unique PID
- ✅ Separate WASM linear memory per worker
- ✅ Capabilities per-process (CSpace)

### 2. Sender Verification ✅

- ✅ PID stored in mailbox by supervisor
- ✅ Process cannot modify its own PID
- ✅ Kernel trusts PID from supervisor

### 3. Message Routing ✅

- ✅ IPC messages routed via supervisor
- ✅ Capability checked before send/receive
- ✅ Metrics tracked per-process

## Usage in Browser

```javascript
// Spawn a new process
supervisor.send_input('spawn memhog');

// Kill a process
supervisor.send_input('kill 3');

// View process list via syscall
supervisor.get_process_list_json();
```

## Dashboard Integration

The web UI shows:
- Process list with PID, name, state, memory
- Worker memory context ID
- Kill button for user processes
- Memory bar visualization
- IPC traffic between processes

## No Modifications Needed

This stage is complete. The implementation includes:

- Full process lifecycle management
- Web Worker isolation
- SharedArrayBuffer + Atomics syscalls
- Comprehensive metrics
- Dashboard visualization

## Next Stage

Proceed to [Stage 1.5: Init + Services](stage-1.5-init-services.md).
