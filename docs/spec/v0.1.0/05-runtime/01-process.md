# Process Manager Service

> User-space process lifecycle management.

## Overview

The Process Manager service handles:

1. **Process Creation**: Spawning new processes from binaries
2. **Capability Distribution**: Granting initial capabilities to new processes
3. **Process Queries**: Listing and inspecting processes
4. **Resource Limits**: Enforcing per-process resource quotas

Note: The *kernel* handles low-level process primitives (TCBs, scheduling). The Process Manager is a *policy layer* that decides what capabilities processes receive.

## Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                      Process Manager Service                         │
│                                                                     │
│  ┌────────────────────────────────────────────────────────────────┐│
│  │                    Capabilities                                 ││
│  │                                                                ││
│  │  • ProcessSpawn (kernel) - ability to spawn processes          ││
│  │  • CapabilityGrant (kernel) - ability to grant capabilities    ││
│  │  • Storage (read) - load binaries                              ││
│  │  • Init endpoint (write) - report status                       ││
│  └────────────────────────────────────────────────────────────────┘│
│                                                                     │
│  ┌────────────────────────────────────────────────────────────────┐│
│  │                    Process Table                                ││
│  │                                                                ││
│  │  PID │ Name     │ Parent │ State   │ Caps Granted             ││
│  │  ────┼──────────┼────────┼─────────┼────────────────          ││
│  │  1   │ init     │ 0      │ Running │ [full]                   ││
│  │  2   │ terminal │ 1      │ Running │ [console]                ││
│  │  3   │ app1     │ 2      │ Running │ [storage-ro, net]        ││
│  └────────────────────────────────────────────────────────────────┘│
│                                                                     │
│  Message Handlers:                                                   │
│  • SPAWN_REQUEST  → spawn_process()                                 │
│  • LIST_PROCESSES → list_processes()                                │
│  • KILL_PROCESS   → kill_process()                                  │
│  • GET_PROCESS    → get_process_info()                              │
└─────────────────────────────────────────────────────────────────────┘
```

## IPC Protocol

### Spawn Process

```rust
/// Spawn request message.
pub const MSG_SPAWN: u32 = 0x4000;
/// Spawn response message.
pub const MSG_SPAWN_RESPONSE: u32 = 0x4001;

/// Request to spawn a new process.
#[derive(Clone, Debug)]
pub struct SpawnRequest {
    /// Process name (for debugging)
    pub name: String,
    /// Binary to execute
    pub binary: BinarySource,
    /// Initial capabilities to grant
    pub capabilities: Vec<CapRequest>,
    /// Resource limits
    pub limits: ResourceLimits,
}

/// Source for process binary.
pub enum BinarySource {
    /// Load from storage path
    Path(String),
    /// Inline binary data
    Inline(Vec<u8>),
    /// Well-known service name
    WellKnown(String),
}

/// Request for a capability.
pub struct CapRequest {
    /// Type of capability needed
    pub cap_type: String,  // "storage", "network", "console"
    /// Requested permissions
    pub permissions: Permissions,
}

/// Resource limits for the process.
#[derive(Clone, Debug, Default)]
pub struct ResourceLimits {
    /// Maximum memory in bytes
    pub max_memory: Option<usize>,
    /// Maximum CPU time in nanoseconds (0 = unlimited)
    pub max_cpu_time: Option<u64>,
    /// Maximum open capabilities
    pub max_caps: Option<u32>,
    /// Maximum IPC message queue depth
    pub max_ipc_queue: Option<usize>,
}

/// Spawn response.
pub struct SpawnResponse {
    /// Success or error
    pub result: Result<ProcessId, SpawnError>,
}

#[derive(Clone, Debug)]
pub enum SpawnError {
    /// Binary not found
    BinaryNotFound,
    /// Insufficient permissions
    PermissionDenied,
    /// Resource limit exceeded
    ResourceLimitExceeded,
    /// Invalid binary format
    InvalidBinary,
}
```

### List Processes

```rust
/// List processes request.
pub const MSG_LIST_PROCESSES: u32 = 0x4002;
/// List processes response.
pub const MSG_PROCESS_LIST: u32 = 0x4003;

/// Process information.
#[derive(Clone, Debug)]
pub struct ProcessInfo {
    pub pid: ProcessId,
    pub name: String,
    pub parent_pid: ProcessId,
    pub state: ProcessState,
    pub memory_bytes: usize,
    pub cpu_time_ns: u64,
    pub start_time_ns: u64,
}
```

### Kill Process

```rust
/// Kill process request.
pub const MSG_KILL: u32 = 0x4004;
/// Kill response.
pub const MSG_KILL_RESPONSE: u32 = 0x4005;

/// Kill request.
pub struct KillRequest {
    /// Process to kill
    pub pid: ProcessId,
    /// Signal (for compatibility, Zero uses messages)
    pub signal: KillSignal,
}

pub enum KillSignal {
    /// Request graceful termination
    Term,
    /// Force immediate termination
    Kill,
}
```

## Spawn Flow

```
   Client                  ProcessManager              Kernel
     │                          │                        │
     │  SpawnRequest            │                        │
     │  (name, binary, caps)    │                        │
     │─────────────────────────▶│                        │
     │                          │                        │
     │                          │  Load binary           │
     │                          │  (from storage)        │
     │                          │                        │
     │                          │  Check permissions     │
     │                          │  (can client spawn?)   │
     │                          │                        │
     │                          │  Syscall: spawn        │
     │                          │───────────────────────▶│
     │                          │                        │  Create process
     │                          │                        │  Allocate PID
     │                          │◀───────────────────────│
     │                          │  { pid: 5 }            │
     │                          │                        │
     │                          │  Grant capabilities    │
     │                          │  to new process        │
     │                          │───────────────────────▶│
     │                          │◀───────────────────────│
     │                          │                        │
     │  SpawnResponse           │                        │
     │  { pid: 5 }              │                        │
     │◀─────────────────────────│                        │
```

## Permission Checking

The Process Manager enforces spawn policies:

```rust
fn check_spawn_permission(
    &self,
    requester: ProcessId,
    request: &SpawnRequest,
) -> Result<(), SpawnError> {
    // 1. Check requester has spawn capability
    if !self.has_capability(requester, CAP_SPAWN) {
        return Err(SpawnError::PermissionDenied);
    }
    
    // 2. Check requester can grant requested capabilities
    for cap_req in &request.capabilities {
        if !self.can_grant(requester, cap_req) {
            return Err(SpawnError::PermissionDenied);
        }
    }
    
    // 3. Check resource limits
    if let Some(max_mem) = request.limits.max_memory {
        if max_mem > self.get_available_memory() {
            return Err(SpawnError::ResourceLimitExceeded);
        }
    }
    
    Ok(())
}
```

## Capability Distribution

New processes receive capabilities through the Process Manager:

```rust
fn grant_initial_capabilities(
    &mut self,
    pid: ProcessId,
    requests: &[CapRequest],
) -> Vec<CapSlot> {
    let mut granted = Vec::new();
    
    for req in requests {
        // Map cap_type to actual capability
        let source_cap = match req.cap_type.as_str() {
            "console" => self.console_cap,
            "storage" => self.storage_cap,
            "network" => self.network_cap,
            _ => continue,
        };
        
        // Attenuate permissions
        let perms = self.attenuate(source_cap, &req.permissions);
        
        // Grant to new process
        let slot = syscall_cap_grant(source_cap, pid, perms);
        granted.push(slot);
    }
    
    granted
}
```

## Resource Enforcement

The Process Manager tracks and enforces resource limits:

```rust
/// Per-process resource tracking.
struct ProcessResources {
    pid: ProcessId,
    limits: ResourceLimits,
    usage: ResourceUsage,
}

struct ResourceUsage {
    memory_bytes: usize,
    cpu_time_ns: u64,
    cap_count: u32,
    ipc_queue_depth: usize,
}

impl ProcessManager {
    fn check_resource_violation(&self, pid: ProcessId) -> Option<ResourceViolation> {
        let resources = self.resources.get(&pid)?;
        
        if let Some(max) = resources.limits.max_memory {
            if resources.usage.memory_bytes > max {
                return Some(ResourceViolation::Memory);
            }
        }
        
        if let Some(max) = resources.limits.max_cpu_time {
            if resources.usage.cpu_time_ns > max {
                return Some(ResourceViolation::CpuTime);
            }
        }
        
        None
    }
    
    fn handle_violation(&mut self, pid: ProcessId, violation: ResourceViolation) {
        debug(&format!("Resource violation: {:?} for PID {}", violation, pid.0));
        
        // Notify the process (give it a chance to clean up)
        send(pid_endpoint, MSG_RESOURCE_WARNING, &[violation as u8]);
        
        // If persistent, kill
        if self.violation_count(pid) > MAX_VIOLATIONS {
            self.kill_process(pid, KillSignal::Kill);
        }
    }
}
```

## WASM Implementation

```rust
// process_manager.rs

#![no_std]
extern crate alloc;
extern crate Zero_process;

use alloc::collections::BTreeMap;
use Zero_process::*;

static mut PROCESS_TABLE: Option<BTreeMap<u32, ProcessInfo>> = None;

#[no_mangle]
pub extern "C" fn _start() {
    debug("process_manager: starting");
    
    unsafe { PROCESS_TABLE = Some(BTreeMap::new()); }
    
    let service_ep = create_endpoint();
    register_service("process", service_ep);
    send_ready();
    
    loop {
        let msg = receive_blocking(service_ep);
        match msg.tag {
            MSG_SPAWN => handle_spawn(msg),
            MSG_LIST_PROCESSES => handle_list(msg),
            MSG_KILL => handle_kill(msg),
            MSG_GET_PROCESS => handle_get(msg),
            _ => debug("process_manager: unknown message"),
        }
    }
}

fn handle_spawn(msg: ReceivedMessage) {
    let request: SpawnRequest = decode(&msg.data);
    let reply_ep = msg.cap_slots.get(0);
    
    // Check permissions
    if let Err(e) = check_spawn_permission(msg.from, &request) {
        if let Some(ep) = reply_ep {
            send(*ep, MSG_SPAWN_RESPONSE, &encode_error(e));
        }
        return;
    }
    
    // Load binary and spawn
    match do_spawn(&request) {
        Ok(pid) => {
            if let Some(ep) = reply_ep {
                send(*ep, MSG_SPAWN_RESPONSE, &encode_success(pid));
            }
        }
        Err(e) => {
            if let Some(ep) = reply_ep {
                send(*ep, MSG_SPAWN_RESPONSE, &encode_error(e));
            }
        }
    }
}
```
