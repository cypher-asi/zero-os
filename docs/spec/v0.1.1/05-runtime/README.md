# 05 - Runtime Services

## Overview

Runtime services provide the environment for user-space processes. This section covers:

- Process runtime (entry point, syscall access)
- Console I/O model
- Permissions and capability management

## Process Runtime

### Entry Point

WASM processes define a `_start` export:

```rust
#[no_mangle]
pub extern "C" fn _start() {
    // Process main function
}
```

The Web Worker calls this after loading the WASM module.

### Syscall Access

Processes use imported functions for syscalls:

```rust
#[cfg(target_arch = "wasm32")]
extern "C" {
    fn zos_syscall(syscall_num: u32, arg1: u32, arg2: u32, arg3: u32) -> u32;
    fn zos_send_bytes(ptr: *const u8, len: u32);
    fn zos_recv_bytes(ptr: *mut u8, max_len: u32) -> u32;
    fn zos_yield();
    fn zos_get_pid() -> u32;
}
```

### Process Library (zos-process)

The `zos-process` crate provides ergonomic wrappers:

```rust
// Get process ID
let pid = zos_process::get_pid();

// Print debug message
zos_process::debug("Hello from process!");

// Write to console (visible in terminal)
zos_process::console_write("Command output\n");

// Send IPC message
zos_process::send(endpoint_slot, tag, &data)?;

// Receive IPC message (non-blocking)
if let Some(msg) = zos_process::receive(endpoint_slot) {
    // Handle message
}

// Exit process
zos_process::exit(0);
```

## Console I/O Model

Console I/O has two components:

### Console Output (Process → Terminal)

Uses `SYS_CONSOLE_WRITE` syscall:

```rust
pub fn console_write(text: &str) {
    let bytes = text.as_bytes();
    unsafe {
        zos_send_bytes(bytes.as_ptr(), bytes.len() as u32);
        zos_syscall(SYS_CONSOLE_WRITE, bytes.len() as u32, 0, 0);
    }
}
```

Flow:
```
Process                  Kernel                   Supervisor              UI
   │                        │                         │                   │
   │  SYS_CONSOLE_WRITE     │                         │                   │
   │───────────────────────▶│                         │                   │
   │                        │  buffer output          │                   │
   │                        │                         │                   │
   │                        │         poll_syscalls() │                   │
   │                        │◀────────────────────────│                   │
   │                        │                         │                   │
   │                        │  drain_console_output() │                   │
   │                        │────────────────────────▶│                   │
   │                        │                         │                   │
   │                        │                         │  console_callback │
   │                        │                         │──────────────────▶│
```

The supervisor routes output to process-specific callbacks for terminal isolation.

### Console Input (Terminal → Process)

Uses privileged kernel API (not a syscall):

```rust
// Supervisor delivers input to terminal process
supervisor.send_input_to_process(terminal_pid, "ls -la\n");
```

The kernel delivers this as an IPC message:

```
MSG_CONSOLE_INPUT (0x0002)
├── data: UTF-8 text bytes
```

Process receives via IPC:

```rust
loop {
    if let Some(msg) = receive(input_endpoint_slot) {
        if msg.tag == MSG_CONSOLE_INPUT {
            let input = std::str::from_utf8(&msg.data).unwrap();
            handle_input(input);
        }
    }
    yield_now();
}
```

## Permission Model

Permissions are managed through capabilities:

### Object Types

```rust
pub enum ObjectType {
    Endpoint = 1,  // IPC endpoint
    Console = 2,   // Console I/O
    Storage = 3,   // Persistent storage
    Network = 4,   // Network access
    Process = 5,   // Process management
    Memory = 6,    // Memory region
}
```

### Permission Bits

```rust
pub struct Permissions {
    pub read: bool,   // Can read/receive
    pub write: bool,  // Can write/send
    pub grant: bool,  // Can grant to others
}
```

### Granting Permissions

From supervisor (privileged):
```rust
supervisor.revoke_capability(pid, slot);
```

Via init debug channel:
```
INIT:GRANT:{to_pid}:{slot}:{object_type}:{object_id}:{perms}
INIT:REVOKE:{pid}:{slot}
```

### Revocation Notifications

When a capability is revoked, the process receives:

```
MSG_CAP_REVOKED (0x3010)
├── slot: u32
├── object_type: u8
├── object_id: u64
└── reason: u8
```

Revocation reasons:
- 1: Explicit (supervisor/user revoked)
- 2: Expired
- 3: Source process exited

## Memory Model

### Heap Allocation

WASM processes use a bump allocator:

```rust
#[global_allocator]
static ALLOCATOR: BumpAllocator = BumpAllocator::new();

const HEAP_START: usize = 0x10000;  // 64KB offset
const HEAP_SIZE: usize = 1024 * 1024;  // 1MB heap
```

### Memory Growth

WASM linear memory grows in 64KB pages. The supervisor tracks memory size and updates the kernel:

```rust
// Worker reports memory size
postMessage({ type: 'memory_update', memory_size: memory.buffer.byteLength });

// Supervisor updates kernel
kernel.update_process_memory(pid, memory_size);
```

## Panic Handling

WASM processes define a panic handler:

```rust
#[cfg(target_arch = "wasm32")]
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    let msg = format!("PANIC: {}", info.message());
    syscall::debug(&msg);
    syscall::exit(1);
}
```

## Compliance Checklist

### Source Files
- `crates/zos-process/src/lib.rs` - Process-side syscall library
- `crates/zos-supervisor-web/src/supervisor/mod.rs` - Console routing

### Key Invariants
- [ ] Console output routed to correct process callback
- [ ] Console input delivered via IPC (not syscall)
- [ ] Revocation notifications delivered
- [ ] Panic handler calls exit

### Differences from v0.1.0
- Console output via SYS_CONSOLE_WRITE (was IPC)
- Per-process console callbacks for isolation
- Privileged kernel API for console input delivery
- Revocation notification messages
