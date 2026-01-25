# Processes

## Overview

Processes are the fundamental unit of execution in Zero OS. Each process runs in isolated WASM linear memory with its own capability space.

## Process Structure

```rust
pub struct Process {
    /// Unique process ID
    pub pid: ProcessId,
    /// Parent process ID (0 for init/supervisor)
    pub parent: ProcessId,
    /// Human-readable name
    pub name: String,
    /// Current state
    pub state: ProcessState,
    /// Memory size in bytes
    pub memory_size: usize,
}

#[repr(u8)]
pub enum ProcessState {
    /// Process is running (scheduled)
    Running = 0,
    /// Process is waiting for IPC
    Waiting = 1,
    /// Process has exited
    Exited = 2,
}
```

## Process IDs

| PID | Process | Notes |
|-----|---------|-------|
| 0 | Supervisor | Not a real process, used for audit logging |
| 1 | Init | Service registry, bootstrap |
| 2 | PermissionService | Capability authority |
| 3+ | User processes | Terminal, apps, etc. |

## Lifecycle

```
                    ┌─────────────────┐
                    │  spawn_process  │
                    │  (kernel + HAL) │
                    └────────┬────────┘
                             │
                             ▼
┌─────────────────────────────────────────────────────────┐
│                       RUNNING                           │
│  • Scheduled for execution                             │
│  • Can make syscalls                                   │
│  • Can send/receive IPC                                │
└──────────────────────────┬──────────────────────────────┘
                           │
            ┌──────────────┼──────────────┐
            │              │              │
            ▼              ▼              ▼
      ┌──────────┐  ┌──────────┐   ┌──────────┐
      │ WAITING  │  │  EXITED  │   │  KILLED  │
      │ (IPC)    │  │ (normal) │   │ (forced) │
      └──────────┘  └──────────┘   └──────────┘
            │
            │ (message received)
            ▼
      ┌──────────┐
      │ RUNNING  │
      └──────────┘
```

## Spawning a Process

1. **Supervisor requests spawn**: Via init's boot sequence or user command
2. **HAL creates Worker**: `hal.spawn_process(name, binary)`
3. **Kernel registers process**: Assign PID, create CSpace
4. **Worker initializes**: Load WASM, call `_start()`
5. **Process signals ready**: Worker sends Ready message

```rust
// Kernel API
pub fn spawn_process(&mut self, name: &str, binary: &[u8]) -> Result<ProcessId, KernelError> {
    // Create HAL worker
    let handle = self.hal.spawn_process(name, binary)?;
    
    // Allocate PID
    let pid = self.next_pid;
    self.next_pid += 1;
    
    // Create process entry
    let process = Process {
        pid: ProcessId(pid),
        parent: ProcessId(0), // Set by supervisor
        name: name.to_string(),
        state: ProcessState::Running,
        memory_size: 0,
    };
    
    // Initialize CSpace with default capabilities
    self.cspaces.insert(pid, CSpace::new());
    
    // Record in Axiom
    self.axiom.commit(CommitType::ProcessCreated {
        pid,
        parent: 0,
        name: name.to_string(),
    });
    
    Ok(ProcessId(pid))
}
```

## Terminating a Process

### Voluntary Exit (SYS_EXIT)

```rust
pub fn exit_process(&mut self, pid: ProcessId, code: i32) {
    // Mark as exited
    if let Some(proc) = self.processes.get_mut(&pid.0) {
        proc.state = ProcessState::Exited;
    }
    
    // Clean up capabilities
    self.cspaces.remove(&pid.0);
    
    // Clean up owned endpoints
    self.cleanup_endpoints(pid);
    
    // Kill HAL worker
    if let Some(handle) = self.handles.remove(&pid.0) {
        self.hal.kill_process(&handle);
    }
    
    // Record in Axiom
    self.axiom.commit(CommitType::ProcessExited { pid: pid.0, code });
}
```

### Forced Kill

```rust
pub fn kill_process(&mut self, pid: ProcessId) {
    self.exit_process(pid, -1); // Exit code -1 indicates kill
}
```

## Memory Tracking

Process memory size is tracked for monitoring and resource limits:

```rust
pub fn update_process_memory(&mut self, pid: ProcessId, size: usize) {
    if let Some(proc) = self.processes.get_mut(&pid.0) {
        proc.memory_size = size;
    }
}
```

On WASM, memory size = WASM linear memory pages × 64KB.

## Process Queries

```rust
impl Kernel {
    /// Get process info
    pub fn get_process(&self, pid: ProcessId) -> Option<&Process>;
    
    /// List all processes
    pub fn list_processes(&self) -> Vec<(ProcessId, &Process)>;
    
    /// Find process by name
    pub fn find_process_by_name(&self, name: &str) -> Option<ProcessId>;
}
```

## Compliance Checklist

### Source Files
- `crates/zos-kernel/src/lib.rs` - Process management

### Key Invariants
- [ ] PIDs are unique and never reused
- [ ] Parent PID is valid at spawn time
- [ ] CSpace created at spawn, removed at exit
- [ ] All resources cleaned up on termination
- [ ] Process state transitions are recorded

### Differences from v0.1.0
- Process memory size tracked in kernel
- Parent process tracking for spawn hierarchy
- ProcessState is repr(u8) for serialization
