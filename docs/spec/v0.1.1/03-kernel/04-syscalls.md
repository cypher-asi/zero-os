# Syscall ABI Reference

## Overview

This document defines the canonical syscall ABI for Zero OS v0.1.1.

## Syscall Invocation

On WASM, syscalls are made via imported functions:

```rust
extern "C" {
    fn zos_syscall(syscall_num: u32, arg1: u32, arg2: u32, arg3: u32) -> u32;
    fn zos_send_bytes(ptr: *const u8, len: u32);
    fn zos_recv_bytes(ptr: *mut u8, max_len: u32) -> u32;
}
```

## Syscall Number Ranges

| Range | Category |
|-------|----------|
| 0x00-0x0F | Misc (debug, info, time) |
| 0x10-0x1F | Thread (create, exit, yield, sleep) |
| 0x20-0x2F | Memory (map, unmap, protect) |
| 0x30-0x3F | Capability (grant, revoke, transfer) |
| 0x40-0x4F | IPC (send, receive, call, reply) |
| 0x50-0x5F | IRQ (register, ack, mask) |
| 0x60-0x6F | I/O (port read/write) |

## Syscall Reference

### Misc (0x00-0x0F)

#### SYS_DEBUG (0x01)

Print debug message to console.

```
Args:    len (data length)
Data:    message bytes (UTF-8)
Returns: 0 on success
```

#### SYS_GET_TIME (0x02)

Get monotonic uptime in nanoseconds.

```
Args:    part (0=low32, 1=high32)
Returns: Time part
```

#### SYS_GET_PID (0x03)

Get current process ID.

```
Args:    (none)
Returns: PID
```

#### SYS_LIST_CAPS (0x04)

List capabilities in own CSpace.

```
Args:    (none)
Returns: 0 on success
Data:    [count: u32, entries: [slot: u32, type: u8, object_id: u64]...]
```

#### SYS_LIST_PROCS (0x05)

List all processes.

```
Args:    (none)
Returns: 0 on success
Data:    [count: u32, entries: [pid: u32, name_len: u16, name: bytes]...]
```

#### SYS_GET_WALLCLOCK (0x06)

Get wall-clock time in milliseconds since Unix epoch.

```
Args:    part (0=low32, 1=high32)
Returns: Time part
```

#### SYS_CONSOLE_WRITE (0x07)

Write to console output (for terminal display).

```
Args:    len (data length)
Data:    text bytes (UTF-8)
Returns: 0 on success
```

The supervisor receives a callback with the output text.

### Thread (0x10-0x1F)

#### SYS_THREAD_CREATE (0x10)

Create a new thread (not implemented in WASM phase).

```
Args:    entry_point, stack_ptr, arg
Returns: thread_id or error
```

#### SYS_EXIT (0x11)

Exit the current process.

```
Args:    exit_code
Returns: (never returns)
```

#### SYS_YIELD (0x12)

Yield CPU to other processes.

```
Args:    (none)
Returns: 0
```

#### SYS_SLEEP (0x13)

Sleep for specified duration.

```
Args:    nanoseconds_low, nanoseconds_high
Returns: 0 on success
```

### Capability (0x30-0x3F)

#### SYS_CAP_GRANT (0x30)

Grant a capability to another process.

```
Args:    from_slot, to_pid, perms
Returns: to_slot in target's CSpace, or error (high bit set)
```

#### SYS_CAP_REVOKE (0x31)

Revoke a capability (requires grant permission).

```
Args:    slot
Returns: 0 on success, or error
```

#### SYS_CAP_DELETE (0x32)

Delete a capability from own CSpace.

```
Args:    slot
Returns: 0 on success, or error
```

#### SYS_CAP_INSPECT (0x33)

Inspect a capability.

```
Args:    slot
Returns: 1 if exists, 0 if empty
Data:    [object_type: u8, perms: u8, pad: u16, slot: u32, object_id: u64]
```

#### SYS_CAP_DERIVE (0x34)

Derive a capability with reduced permissions.

```
Args:    slot, new_perms
Returns: new_slot, or error (high bit set)
```

#### SYS_EP_CREATE (0x35)

Create an IPC endpoint.

```
Args:    part (0=first call returns slot|ep_low, 1=returns ep_high)
Returns: See args
```

### IPC (0x40-0x4F)

#### SYS_SEND (0x40)

Send a message to an endpoint.

```
Args:    endpoint_slot, tag, data_len
Data:    message bytes
Returns: 0 on success, or error
```

#### SYS_RECEIVE (0x41)

Receive a message (non-blocking).

```
Args:    endpoint_slot
Returns: message_len (0 if no message), or error (negative)
Data:    [from_pid: u32, tag: u32, data: bytes]
```

#### SYS_CALL (0x42)

Send and wait for reply (RPC).

```
Args:    endpoint_slot, tag, data_len
Data:    request bytes
Returns: reply_len
Data:    reply bytes
```

#### SYS_REPLY (0x43)

Reply to a call.

```
Args:    caller_pid, tag, data_len
Data:    reply bytes
Returns: 0 on success
```

#### SYS_SEND_CAP (0x44)

Send message with capability transfer.

```
Args:    endpoint_slot, tag, data_len | (cap_count << 16)
Data:    [message_data, cap_slots: [u32]...]
Returns: 0 on success
```

### IRQ (0x50-0x5F) - Not Implemented in WASM

Reserved for QEMU/bare metal phases.

### I/O (0x60-0x6F) - Not Implemented in WASM

Reserved for QEMU/bare metal phases.

## Error Codes

```rust
pub const E_OK: u32 = 0;      // Success
pub const E_PERM: u32 = 1;    // Permission denied
pub const E_NOENT: u32 = 2;   // Object not found
pub const E_INVAL: u32 = 3;   // Invalid argument
pub const E_NOSYS: u32 = 4;   // Syscall not implemented
pub const E_AGAIN: u32 = 5;   // Would block (try again)
pub const E_NOMEM: u32 = 6;   // Out of memory
pub const E_BADF: u32 = 7;    // Invalid capability slot
pub const E_BUSY: u32 = 8;    // Resource busy
pub const E_EXIST: u32 = 9;   // Already exists
pub const E_OVERFLOW: u32 = 10; // Buffer overflow
```

## Data Buffer Convention

Variable-length data is passed via `zos_send_bytes` before the syscall:

```rust
// Sending data
let data = b"Hello, World!";
zos_send_bytes(data.as_ptr(), data.len() as u32);
zos_syscall(SYS_SEND, endpoint_slot, tag, data.len() as u32);
```

Results are retrieved via `zos_recv_bytes` after the syscall:

```rust
// Receiving data
let result = zos_syscall(SYS_RECEIVE, endpoint_slot, 0, 0);
if result > 0 {
    let mut buffer = [0u8; 4096];
    let len = zos_recv_bytes(buffer.as_mut_ptr(), buffer.len() as u32);
    // Parse buffer[..len]
}
```

## Compliance Checklist

### Source Files
- `crates/zos-process/src/lib.rs` - Syscall numbers and wrappers
- `crates/zos-kernel/src/lib.rs` - Syscall handlers

### Key Invariants
- [ ] Syscall numbers match this document
- [ ] Error codes are consistent
- [ ] Data buffer cleared between syscalls
- [ ] All syscalls logged to SysLog

### Differences from v0.1.0
- Added SYS_CONSOLE_WRITE (0x07)
- Added SYS_GET_WALLCLOCK (0x06)
- Added SYS_CAP_DERIVE (0x34)
- Added SYS_SEND_CAP (0x44)
- Canonical syscall ranges defined
