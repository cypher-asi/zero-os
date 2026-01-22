# HAL Traits Reference

## Core Trait: HAL

The `HAL` trait is the primary abstraction for platform-specific operations.

### Associated Types

```rust
type ProcessHandle: Clone + Send + Sync;
```

A platform-specific handle to a spawned process. On WASM, this is a reference to a Web Worker. On native platforms, this could be a process ID or memory region reference.

### Process Management

#### spawn_process

```rust
fn spawn_process(&self, name: &str, binary: &[u8]) -> Result<Self::ProcessHandle, HalError>;
```

Spawn a new process from a WASM binary.

- **name**: Human-readable process name for debugging
- **binary**: WASM binary bytes
- **Returns**: Handle to the spawned process, or `HalError::ProcessSpawnFailed`

#### kill_process

```rust
fn kill_process(&self, handle: &Self::ProcessHandle) -> Result<(), HalError>;
```

Terminate a process.

- **handle**: Handle to the process to terminate
- **Returns**: `Ok(())` or `HalError::ProcessNotFound`

#### send_to_process

```rust
fn send_to_process(&self, handle: &Self::ProcessHandle, msg: &[u8]) -> Result<(), HalError>;
```

Send a message to a process (e.g., IPC delivery, syscall results).

- **handle**: Target process handle
- **msg**: Message bytes
- **Returns**: `Ok(())` or error

#### is_process_alive

```rust
fn is_process_alive(&self, handle: &Self::ProcessHandle) -> bool;
```

Check if a process is still running.

#### get_process_memory_size

```rust
fn get_process_memory_size(&self, handle: &Self::ProcessHandle) -> Result<usize, HalError>;
```

Get the memory size of a process in bytes. On WASM, this returns the linear memory size (pages × 64KB).

### Memory Operations

#### allocate

```rust
fn allocate(&self, size: usize, align: usize) -> Result<*mut u8, HalError>;
```

Allocate memory within the current context (supervisor-side allocations).

- **size**: Number of bytes to allocate
- **align**: Alignment requirement
- **Returns**: Pointer to allocated memory, or `HalError::OutOfMemory`

#### deallocate

```rust
unsafe fn deallocate(&self, ptr: *mut u8, size: usize, align: usize);
```

Deallocate memory previously allocated by `allocate`.

**Safety**: The pointer must have been allocated by `allocate` with the same size and alignment.

### Time and Entropy

#### now_nanos

```rust
fn now_nanos(&self) -> u64;
```

Get current monotonic time in nanoseconds. Suitable for measuring durations and scheduling.

- On WASM: Uses `performance.now()` converted to nanoseconds

#### wallclock_ms

```rust
fn wallclock_ms(&self) -> u64;
```

Get wall-clock time in milliseconds since Unix epoch. This is real time-of-day (can jump due to NTP sync).

- On WASM: Uses `Date.now()`

#### random_bytes

```rust
fn random_bytes(&self, buf: &mut [u8]) -> Result<(), HalError>;
```

Fill buffer with cryptographically secure random bytes.

- On WASM: Uses `crypto.getRandomValues()`
- **Returns**: `Ok(())` or `HalError::NotSupported`

### Debug

#### debug_write

```rust
fn debug_write(&self, msg: &str);
```

Write a debug message to the platform's console/log.

- On WASM: Uses `console.log()`

### Message Reception

#### poll_messages

```rust
fn poll_messages(&self) -> Vec<(Self::ProcessHandle, Vec<u8>)>;
```

Poll for incoming messages from processes (non-blocking). Returns a list of (process_handle, message_bytes) pairs.

#### set_message_callback

```rust
fn set_message_callback(&self, callback: Option<MessageCallback<Self::ProcessHandle>>);
```

Register a callback for when messages arrive from processes. This is optional—implementations can use polling instead.

## Helper Type: NumericProcessHandle

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct NumericProcessHandle(pub u64);

impl NumericProcessHandle {
    pub fn new(id: u64) -> Self;
    pub fn id(&self) -> u64;
}
```

A simple process handle for platforms that use numeric IDs.

## Usage Example

```rust
// Generic kernel over any HAL
pub struct Kernel<H: HAL> {
    hal: H,
    // ...
}

impl<H: HAL> Kernel<H> {
    pub fn spawn_process(&mut self, name: &str, binary: &[u8]) -> Result<ProcessId, KernelError> {
        let handle = self.hal.spawn_process(name, binary)?;
        // Register process in kernel state
        // ...
    }
}
```

## Compliance Checklist

### Key Invariants
- [ ] `spawn_process` creates isolated process
- [ ] `kill_process` terminates and cleans up resources
- [ ] `now_nanos` is monotonically increasing
- [ ] `random_bytes` is cryptographically secure
- [ ] All methods are safe to call from any thread (`Send + Sync`)
