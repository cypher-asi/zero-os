# WASM HAL Implementation

## Overview

The WASM HAL runs in the browser, using Web Workers as isolated processes and SharedArrayBuffer for efficient syscall communication.

## Process Model

Each process is a Web Worker running a WASM binary:

```
┌─────────────────────────────────────────────────────────┐
│                    Main Thread                          │
│  ┌───────────────────────────────────────────────────┐ │
│  │               WasmHal                              │ │
│  │  • Workers: HashMap<u64, Worker>                  │ │
│  │  • Syscall buffers: HashMap<u64, SharedArrayBuffer>│ │
│  └───────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────┘
           │                    │                    │
           ▼                    ▼                    ▼
┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐
│   Worker (P1)   │  │   Worker (P2)   │  │   Worker (P3)   │
│  ┌───────────┐  │  │  ┌───────────┐  │  │  ┌───────────┐  │
│  │WASM Module│  │  │  │WASM Module│  │  │  │WASM Module│  │
│  │  (init)   │  │  │  │(terminal) │  │  │  │  (clock)  │  │
│  └───────────┘  │  │  └───────────┘  │  │  └───────────┘  │
└─────────────────┘  └─────────────────┘  └─────────────────┘
```

## Syscall Mechanism

### SharedArrayBuffer Layout

Each process has a dedicated SharedArrayBuffer for syscall communication:

```
Offset   Size    Field
──────────────────────────────────
0        4       status (0=idle, 1=pending, 2=complete)
4        4       syscall_num
8        4       arg0
12       4       arg1
16       4       arg2
20       4       result (i32)
24       N       data buffer (for variable-length syscall data)
```

### Syscall Flow

```
Process (Worker)                    Supervisor (Main Thread)
     │                                       │
     │  1. Write syscall to SharedArrayBuffer│
     │  2. Set status = 1 (pending)         │
     │  3. Atomics.wait()                   │
     │                                       │
     │<────── poll_syscalls() finds pending ─┤
     │                                       │
     │       4. Process syscall             │
     │       5. Write result                │
     │       6. Set status = 2 (complete)   │
     │       7. Atomics.notify()            │
     │                                       │
     │  8. Wake from wait                   │
     │  9. Read result                      │
     │                                       ▼
```

### Process-Side Imports

WASM modules import these functions from the JavaScript host:

```rust
extern "C" {
    /// Make a syscall to the kernel
    fn zos_syscall(syscall_num: u32, arg1: u32, arg2: u32, arg3: u32) -> u32;

    /// Send bytes to the kernel (for syscall data)
    fn zos_send_bytes(ptr: *const u8, len: u32);

    /// Get bytes from the kernel (for syscall results)
    fn zos_recv_bytes(ptr: *mut u8, max_len: u32) -> u32;

    /// Yield to allow other processes to run
    fn zos_yield();

    /// Get the process's assigned PID
    fn zos_get_pid() -> u32;
}
```

## Memory Model

- Each process has isolated WASM linear memory
- Memory grows in 64KB pages
- Memory size tracked by supervisor via `MemoryUpdate` messages
- No shared memory between processes (isolation)

## Time Sources

| Function | Source | Resolution | Notes |
|----------|--------|------------|-------|
| `now_nanos()` | `performance.now()` | ~μs | Monotonic, for durations |
| `wallclock_ms()` | `Date.now()` | 1ms | Wall clock, can jump |

## Worker Script

The worker script (`worker.js`) provides the JavaScript glue:

```javascript
// Simplified worker.js structure
let wasmInstance;
let syscallBuffer;
let pid;

onmessage = async (e) => {
    if (e.data.type === 'init') {
        pid = e.data.pid;
        syscallBuffer = e.data.syscallBuffer;
        
        const imports = {
            env: {
                zos_syscall: (num, a1, a2, a3) => {
                    // Write to SharedArrayBuffer
                    // Atomics.wait for completion
                    // Return result
                },
                zos_send_bytes: (ptr, len) => { /* ... */ },
                zos_recv_bytes: (ptr, maxLen) => { /* ... */ },
                zos_yield: () => { /* Atomics.wait(0) */ },
                zos_get_pid: () => pid,
            }
        };
        
        wasmInstance = await WebAssembly.instantiate(e.data.binary, imports);
        wasmInstance.exports._start();
    }
};
```

## Legacy postMessage Path

For compatibility, the HAL also supports the legacy postMessage path:

```javascript
// Worker -> Supervisor
postMessage({ type: 'syscall', syscall_num, args, data });

// Supervisor -> Worker
postMessage({ type: 'syscall_result', result, data });
```

This path is slower but works without SharedArrayBuffer support.

## Compliance Checklist

### Source Files
- `crates/zos-supervisor-web/src/hal.rs` - WasmHal implementation
- `crates/zos-supervisor-web/src/worker.rs` - Worker message types
- `web/public/worker.js` - JavaScript worker script

### Key Invariants
- [ ] Each process has exactly one Web Worker
- [ ] SharedArrayBuffer is isolated per-process
- [ ] Syscall buffer status transitions: 0→1→2→0
- [ ] Atomics operations use correct memory ordering

### Differences from v0.1.0
- SharedArrayBuffer replaces postMessage for syscalls
- Added wallclock_ms for real time-of-day
- Worker script handles both SharedArrayBuffer and legacy paths
