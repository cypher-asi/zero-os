# 01 - Hardware Abstraction Layer (HAL)

## Overview

The HAL trait provides a platform-agnostic interface for hardware operations. This allows the kernel to run unchanged across different targets:

- **WASM**: Web Workers for processes, browser APIs for time/entropy
- **QEMU**: Virtual hardware abstraction (Phase 2)
- **Bare Metal**: Direct hardware access (Phase 3)

## HAL Trait

```rust
pub trait HAL: Send + Sync + 'static {
    /// Handle to a spawned process
    type ProcessHandle: Clone + Send + Sync;

    // === Process Management ===
    fn spawn_process(&self, name: &str, binary: &[u8]) -> Result<Self::ProcessHandle, HalError>;
    fn kill_process(&self, handle: &Self::ProcessHandle) -> Result<(), HalError>;
    fn send_to_process(&self, handle: &Self::ProcessHandle, msg: &[u8]) -> Result<(), HalError>;
    fn is_process_alive(&self, handle: &Self::ProcessHandle) -> bool;
    fn get_process_memory_size(&self, handle: &Self::ProcessHandle) -> Result<usize, HalError>;

    // === Memory ===
    fn allocate(&self, size: usize, align: usize) -> Result<*mut u8, HalError>;
    fn deallocate(&self, ptr: *mut u8, size: usize, align: usize);

    // === Time & Entropy ===
    fn now_nanos(&self) -> u64;
    fn wallclock_ms(&self) -> u64;
    fn random_bytes(&self, buf: &mut [u8]) -> Result<(), HalError>;

    // === Debug ===
    fn debug_write(&self, msg: &str);

    // === Message Reception ===
    fn poll_messages(&self) -> Vec<(Self::ProcessHandle, Vec<u8>)>;
    fn set_message_callback(&self, callback: Option<MessageCallback<Self::ProcessHandle>>);
}
```

## Error Types

```rust
pub enum HalError {
    OutOfMemory,
    ProcessSpawnFailed,
    ProcessNotFound,
    InvalidMessage,
    NotSupported,
    IoError,
    InvalidArgument,
}
```

## Process Message Types

Communication between HAL and processes uses typed messages:

```rust
#[repr(u8)]
pub enum ProcessMessageType {
    Init = 0,           // Initialize process with PID and capabilities
    Syscall = 1,        // Syscall from process to supervisor
    SyscallResult = 2,  // Syscall result from supervisor to process
    IpcDeliver = 3,     // IPC message delivery
    Terminate = 4,      // Process termination request
    Ready = 5,          // Process ready notification
    Error = 6,          // Error notification
}
```

## WASM HAL Implementation

See [02-wasm-hal.md](./02-wasm-hal.md) for the detailed WASM-specific implementation.

### Key Characteristics

| Operation | WASM Implementation |
|-----------|---------------------|
| Process | Web Worker |
| Memory | WASM linear memory (64KB pages) |
| Monotonic time | `performance.now()` → nanoseconds |
| Wall clock | `Date.now()` → milliseconds |
| Entropy | `crypto.getRandomValues()` |
| Debug | `console.log()` |
| Syscalls | SharedArrayBuffer polling |

## Compliance Checklist

### Source Files
- `crates/zos-hal/src/lib.rs` - HAL trait definition

### Key Invariants
- [ ] HAL trait is `Send + Sync + 'static`
- [ ] ProcessHandle is `Clone + Send + Sync`
- [ ] All time values are monotonic (except wallclock)
- [ ] Random bytes are cryptographically secure

### Differences from v0.1.0
- Added `wallclock_ms()` for real time-of-day
- ProcessHandle is now a trait-associated type (not concrete)
- Message polling via SharedArrayBuffer (not postMessage)
