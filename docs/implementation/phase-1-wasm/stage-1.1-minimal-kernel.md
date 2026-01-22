# Stage 1.1: Minimal Kernel + Debug

> **Status**: ✅ **COMPLETE**
>
> **Goal**: Get the simplest possible syscall working - debug output to browser console.

## Implementation Status

This stage is **fully implemented**. All objectives have been achieved.

### What's Implemented

| Component | Status | Location |
|-----------|--------|----------|
| Kernel struct with HAL | ✅ | `crates/Zero-kernel/src/lib.rs` |
| Debug syscall (SYS_DEBUG) | ✅ | `crates/Zero-kernel/src/lib.rs:602` |
| HAL trait | ✅ | `crates/Zero-hal/src/lib.rs` |
| Mock HAL for testing | ✅ | `crates/Zero-kernel/src/lib.rs` (test module) |
| Process syscall library | ✅ | `crates/zos-process/src/lib.rs` |
| WASM runtime (JS atomics) | ✅ | `web/public/worker.js` |
| Web Worker bootstrap | ✅ | `web/public/worker.js` |
| Browser supervisor | ✅ | `crates/zos-supervisor-web/src/lib.rs` |
| HTML entry point | ✅ | `web/desktop/index.html` |

### Key Implementation Details

#### Kernel Structure

The kernel is generic over HAL implementation:

```rust
// crates/Zero-kernel/src/lib.rs
pub struct Kernel<H: HAL> {
    hal: H,
    processes: BTreeMap<ProcessId, Process>,
    cap_spaces: BTreeMap<ProcessId, CapabilitySpace>,
    endpoints: BTreeMap<EndpointId, Endpoint>,
    axiom_log: AxiomLog,
    // ... more fields
}
```

#### Syscall Numbers

Canonical syscall numbers are defined:

```rust
// crates/Zero-kernel/src/lib.rs
pub const SYS_DEBUG: u32 = 0x01;
pub const SYS_YIELD: u32 = 0x02;
pub const SYS_EXIT: u32 = 0x03;
pub const SYS_TIME: u32 = 0x04;
// ... more syscalls through 0x6F
```

#### HAL Trait

The HAL trait abstracts platform operations:

```rust
// crates/Zero-hal/src/lib.rs
pub trait HAL: Send + Sync + 'static {
    type ProcessHandle: Clone + Send + Sync;
    
    fn spawn_process(&self, name: &str, binary: &[u8]) -> Result<Self::ProcessHandle, HalError>;
    fn kill_process(&self, handle: &Self::ProcessHandle) -> Result<(), HalError>;
    fn send_to_process(&self, handle: &Self::ProcessHandle, msg: &[u8]) -> Result<(), HalError>;
    fn is_process_alive(&self, handle: &Self::ProcessHandle) -> bool;
    fn get_process_memory_size(&self, handle: &Self::ProcessHandle) -> Result<usize, HalError>;
    fn allocate(&self, size: usize, align: usize) -> Result<*mut u8, HalError>;
    fn deallocate(&self, ptr: *mut u8, size: usize, align: usize);
    fn now_nanos(&self) -> u64;
    fn random_bytes(&self, buf: &mut [u8]) -> Result<(), HalError>;
    fn debug_write(&self, msg: &str);
    fn poll_messages(&self) -> Vec<(Self::ProcessHandle, Vec<u8>)>;
}
```

#### WASM Syscall Mechanism

Syscalls use SharedArrayBuffer + Atomics:

1. Process writes syscall params to shared mailbox
2. Process sets status to PENDING
3. Process blocks with `memory.atomic.wait32`
4. Supervisor polls mailboxes, processes syscalls
5. Supervisor sets status to READY and calls `Atomics.notify()`
6. Process wakes up, reads result

## Tests

All tests pass:

```bash
cargo test -p Zero-kernel
```

Key tests:
- `test_kernel_creation` - Kernel initializes correctly
- `test_process_registration` - Processes can be registered
- `test_syscall_dispatch` - Syscalls route correctly

## Verification Checklist

- [x] Code compiles without warnings
- [x] WASM loads in browser without errors
- [x] `SYS_DEBUG` syscall works (message appears in console)
- [x] Both canonical (0x01) and legacy syscall numbers work
- [x] Unknown syscall returns error
- [x] Code formatted (`cargo fmt`)
- [x] Clippy clean (`cargo clippy`)
- [x] Tests pass (`cargo test`)

## No Modifications Needed

This stage is complete. Proceed to [Stage 1.2: Axiom Layer](stage-1.2-axiom-layer.md).
