# Stage 1.2: Axiom Layer (Logging)

> **Status**: ✅ **COMPLETE**
>
> **Goal**: Add SysLog and CommitLog infrastructure to record syscalls and state changes.

## Implementation Status

This stage is **fully implemented**. All objectives have been achieved.

### What's Implemented

| Component | Status | Location |
|-----------|--------|----------|
| AxiomLog for capability mutations (legacy) | ✅ | `crates/Zero-kernel/src/lib.rs` |
| Hash chain integrity | ✅ | `AxiomLog::verify_integrity()` |
| CapOperation enum | ✅ | Create, Grant, Revoke, Transfer, Delete |
| IndexedDB persistence | ✅ | `apps/zos-supervisor-web/www/index.html` |
| `axiom_check()` function | ✅ | `crates/Zero-kernel/src/lib.rs` |
| **SysLog** | ✅ | `crates/Zero-axiom/src/syslog.rs` |
| **CommitLog** | ✅ | `crates/Zero-axiom/src/commitlog.rs` |
| **AxiomGateway** | ✅ | `crates/Zero-axiom/src/gateway.rs` |
| **CommitType enum** | ✅ | ProcessCreated, ProcessExited, CapInserted, CapRemoved, CapGranted, EndpointCreated, EndpointDestroyed |
| **Commit hash chain** | ✅ | FNV-1a hash chain with integrity verification |
| **Kernel integration** | ✅ | All state mutations logged to CommitLog |

## Implementation Details

### Crate Structure

The `Zero-axiom` crate provides:

```
crates/Zero-axiom/
├── Cargo.toml
└── src/
    ├── lib.rs           # Module exports and integration tests
    ├── types.rs         # Common types (ProcessId, EventId, CommitId, Permissions, ObjectType)
    ├── syslog.rs        # SysLog for syscall audit trail
    ├── commitlog.rs     # CommitLog for deterministic state mutations
    └── gateway.rs       # AxiomGateway entry point
```

### SysLog

Records every syscall (request + response) for auditing:

```rust
pub struct SysEvent {
    pub id: EventId,
    pub sender: ProcessId,
    pub timestamp: u64,
    pub event_type: SysEventType,
}

pub enum SysEventType {
    Request { syscall_num: u32, args: [u32; 4] },
    Response { request_id: EventId, result: i64 },
}
```

### CommitLog

Records state mutations for deterministic replay:

```rust
pub struct Commit {
    pub id: CommitId,           // Hash of this commit
    pub prev_commit: CommitId,  // Hash chain
    pub seq: u64,               // Monotonic sequence
    pub timestamp: u64,
    pub commit_type: CommitType,
    pub caused_by: Option<EventId>,
}

pub enum CommitType {
    Genesis,
    ProcessCreated { pid, parent, name },
    ProcessExited { pid, code },
    CapInserted { pid, slot, cap_id, object_type, object_id, perms },
    CapRemoved { pid, slot },
    CapGranted { from_pid, to_pid, from_slot, to_slot, new_cap_id, perms },
    EndpointCreated { id, owner },
    EndpointDestroyed { id },
}
```

### AxiomGateway

Entry point that orchestrates logging:

```rust
impl AxiomGateway {
    pub fn syscall<F>(&mut self, sender, syscall_num, args, timestamp, kernel_fn)
        -> (i64, Vec<CommitId>)
    {
        // 1. Log syscall request to SysLog
        // 2. Execute kernel function
        // 3. Append commits to CommitLog
        // 4. Log syscall response to SysLog
    }
}
```

### Kernel Integration

The kernel uses `AxiomGateway` for all state mutations:

- `register_process()` → `ProcessCreated` commit
- `create_endpoint()` → `EndpointCreated` + `CapInserted` commits
- `grant_capability()` → `CapGranted` commit
- `revoke_capability()` → `CapRemoved` commit
- `delete_capability()` → `CapRemoved` commit
- `Syscall::Exit` → `ProcessExited` commit

## Required Modifications

### Task 1: Create `Zero-axiom` Crate

Create a new crate to hold the Axiom layer:

**File**: `crates/Zero-axiom/Cargo.toml`

```toml
[package]
name = "Zero-axiom"
version.workspace = true
edition.workspace = true

[lib]
crate-type = ["rlib"]

[dependencies]
serde = { workspace = true }
```

**File**: `crates/Zero-axiom/src/lib.rs`

```rust
#![no_std]
extern crate alloc;

pub mod types;
pub mod syslog;
pub mod commitlog;
pub mod gateway;

pub use types::*;
pub use syslog::{SysLog, SysEvent, SysEventType};
pub use commitlog::{CommitLog, Commit, CommitType};
pub use gateway::AxiomGateway;
```

### Task 2: Implement SysLog

**File**: `crates/Zero-axiom/src/syslog.rs`

```rust
use crate::types::*;
use alloc::vec::Vec;
use serde::{Deserialize, Serialize};

/// System event (syscall request or response).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SysEvent {
    pub id: EventId,
    pub sender: ProcessId,
    pub timestamp: u64,
    pub event_type: SysEventType,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum SysEventType {
    Request { syscall_num: u32, args: [u32; 4] },
    Response { request_id: EventId, result: i64 },
}

pub struct SysLog {
    events: Vec<SysEvent>,
    next_id: EventId,
}

impl SysLog {
    pub fn new() -> Self {
        Self { events: Vec::new(), next_id: 0 }
    }

    pub fn log_request(&mut self, sender: ProcessId, syscall_num: u32, args: [u32; 4], timestamp: u64) -> EventId {
        let id = self.next_id;
        self.next_id += 1;
        self.events.push(SysEvent {
            id,
            sender,
            timestamp,
            event_type: SysEventType::Request { syscall_num, args },
        });
        id
    }

    pub fn log_response(&mut self, sender: ProcessId, request_id: EventId, result: i64, timestamp: u64) {
        let id = self.next_id;
        self.next_id += 1;
        self.events.push(SysEvent {
            id,
            sender,
            timestamp,
            event_type: SysEventType::Response { request_id, result },
        });
    }

    pub fn events(&self) -> &[SysEvent] { &self.events }
}
```

### Task 3: Implement CommitLog

**File**: `crates/Zero-axiom/src/commitlog.rs`

```rust
use crate::types::*;
use alloc::{string::String, vec::Vec};
use serde::{Deserialize, Serialize};

/// A state mutation record.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Commit {
    pub id: CommitId,
    pub prev_commit: CommitId,
    pub seq: u64,
    pub timestamp: u64,
    pub commit_type: CommitType,
    pub caused_by: Option<EventId>,
}

/// Types of state mutations (deterministic replay).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum CommitType {
    Genesis,
    
    // Process lifecycle
    ProcessCreated { pid: ProcessId, parent: ProcessId, name: String },
    ProcessExited { pid: ProcessId, code: i32 },
    
    // Capability mutations
    CapInserted { pid: ProcessId, slot: CapSlot, cap_id: u64, object_type: u8, object_id: u64, perms: u8 },
    CapRemoved { pid: ProcessId, slot: CapSlot },
    
    // Endpoint lifecycle
    EndpointCreated { id: EndpointId, owner: ProcessId },
    EndpointDestroyed { id: EndpointId },
}

pub struct CommitLog {
    commits: Vec<Commit>,
    next_seq: u64,
    last_hash: CommitId,
}

impl CommitLog {
    pub fn new(timestamp: u64) -> Self {
        let genesis = Commit {
            id: [0u8; 32],
            prev_commit: [0u8; 32],
            seq: 0,
            timestamp,
            commit_type: CommitType::Genesis,
            caused_by: None,
        };
        let id = Self::compute_hash(&genesis);
        let genesis = Commit { id, ..genesis };
        
        Self {
            commits: vec![genesis],
            next_seq: 1,
            last_hash: id,
        }
    }

    pub fn append(&mut self, commit_type: CommitType, caused_by: Option<EventId>, timestamp: u64) -> CommitId {
        let commit = Commit {
            id: [0u8; 32],
            prev_commit: self.last_hash,
            seq: self.next_seq,
            timestamp,
            commit_type,
            caused_by,
        };
        let id = Self::compute_hash(&commit);
        let commit = Commit { id, ..commit };
        
        self.last_hash = id;
        self.next_seq += 1;
        self.commits.push(commit);
        id
    }

    fn compute_hash(commit: &Commit) -> CommitId {
        // FNV-1a hash (same as current AxiomLog)
        let mut hash = 0xcbf29ce484222325u64;
        const FNV_PRIME: u64 = 0x100000001b3;
        
        for byte in commit.prev_commit { hash ^= byte as u64; hash = hash.wrapping_mul(FNV_PRIME); }
        for byte in commit.seq.to_le_bytes() { hash ^= byte as u64; hash = hash.wrapping_mul(FNV_PRIME); }
        for byte in commit.timestamp.to_le_bytes() { hash ^= byte as u64; hash = hash.wrapping_mul(FNV_PRIME); }
        
        let mut result = [0u8; 32];
        let mut h = hash;
        for chunk in result.chunks_mut(8) {
            let bytes = h.to_le_bytes();
            chunk.copy_from_slice(&bytes[..chunk.len()]);
            h = h.wrapping_mul(FNV_PRIME);
        }
        result
    }

    pub fn commits(&self) -> &[Commit] { &self.commits }
    pub fn head(&self) -> CommitId { self.last_hash }
    pub fn current_seq(&self) -> u64 { self.next_seq - 1 }
}
```

### Task 4: Implement Axiom Gateway

**File**: `crates/Zero-axiom/src/gateway.rs`

```rust
use crate::{SysLog, CommitLog, CommitType, types::*};
use alloc::vec::Vec;

/// Axiom gateway: Entry point for all syscalls.
pub struct AxiomGateway {
    syslog: SysLog,
    commitlog: CommitLog,
}

impl AxiomGateway {
    pub fn new(timestamp: u64) -> Self {
        Self {
            syslog: SysLog::new(),
            commitlog: CommitLog::new(timestamp),
        }
    }

    /// Process a syscall through Axiom.
    pub fn syscall<F>(
        &mut self,
        sender: ProcessId,
        syscall_num: u32,
        args: [u32; 4],
        timestamp: u64,
        mut kernel_fn: F,
    ) -> (i64, Vec<CommitId>)
    where
        F: FnMut(u32, [u32; 4]) -> (i64, Vec<CommitType>),
    {
        // 1. Log syscall request
        let request_id = self.syslog.log_request(sender, syscall_num, args, timestamp);

        // 2. Execute kernel operation
        let (result, commit_types) = kernel_fn(syscall_num, args);

        // 3. Append commits to CommitLog
        let commit_ids: Vec<CommitId> = commit_types
            .into_iter()
            .map(|ct| self.commitlog.append(ct, Some(request_id), timestamp))
            .collect();

        // 4. Log syscall response
        self.syslog.log_response(sender, request_id, result, timestamp);

        (result, commit_ids)
    }

    pub fn syslog(&self) -> &SysLog { &self.syslog }
    pub fn commitlog(&self) -> &CommitLog { &self.commitlog }
}
```

### Task 5: Integrate into Kernel

Modify `crates/Zero-kernel/src/lib.rs`:

```rust
use Zero_axiom::{AxiomGateway, CommitType};

pub struct Kernel<H: HAL> {
    hal: H,
    axiom: AxiomGateway,  // Add this field
    // ... existing fields ...
}

impl<H: HAL> Kernel<H> {
    pub fn new(hal: H) -> Self {
        let boot_time = hal.now_nanos();
        Self {
            hal,
            axiom: AxiomGateway::new(boot_time),  // Initialize
            // ... existing initialization ...
        }
    }

    /// Handle syscall through Axiom gateway.
    pub fn handle_syscall_via_axiom(&mut self, from_pid: ProcessId, syscall_num: u32, args: [u32; 4]) -> i64 {
        let timestamp = self.hal.now_nanos();
        
        let (result, _commits) = self.axiom.syscall(
            from_pid.0 as u32,
            syscall_num,
            args,
            timestamp,
            |num, args| self.kernel_exec(num, args, from_pid),
        );
        
        result
    }

    /// Execute kernel operation, returning commits.
    fn kernel_exec(&mut self, syscall_num: u32, args: [u32; 4], from_pid: ProcessId) -> (i64, Vec<CommitType>) {
        // Route to existing syscall handler but collect commits
        // ... implementation details ...
    }
}
```

### Task 6: Update Cargo.toml

Add to workspace `Cargo.toml`:

```toml
[workspace]
members = [
    # ... existing members ...
    "crates/Zero-axiom",
]

[workspace.dependencies]
Zero-axiom = { path = "crates/Zero-axiom" }
```

Add to `crates/Zero-kernel/Cargo.toml`:

```toml
[dependencies]
Zero-axiom = { workspace = true }
```

## Migration Strategy

The current `AxiomLog` (capability mutations only) should be **kept** and renamed to clarify its purpose. It can coexist with the new SysLog/CommitLog:

1. Keep existing `AxiomLog` as `CapabilityAuditLog` for backward compatibility
2. Add new `SysLog` and `CommitLog` via `AxiomGateway`
3. Gradually migrate syscall dispatch to use `AxiomGateway`
4. Tests can verify both old and new logging

## Test Criteria

### New Tests to Add

```rust
#[test]
fn test_syslog_records_request_and_response() {
    let mut gateway = AxiomGateway::new(0);
    
    gateway.syscall(1, 0x01, [0, 0, 0, 0], 1000, |_, _| (0, vec![]));
    
    let events = gateway.syslog().events();
    assert_eq!(events.len(), 2); // Request + Response
}

#[test]
fn test_commitlog_starts_with_genesis() {
    let gateway = AxiomGateway::new(0);
    
    assert_eq!(gateway.commitlog().current_seq(), 0);
    assert_eq!(gateway.commitlog().commits().len(), 1);
}

#[test]
fn test_commitlog_records_mutations() {
    let mut gateway = AxiomGateway::new(0);
    
    gateway.syscall(1, 0x35, [0, 0, 0, 0], 1000, |_, _| {
        (0, vec![CommitType::EndpointCreated { id: 1, owner: 1 }])
    });
    
    assert_eq!(gateway.commitlog().current_seq(), 1);
}
```

## Verification Checklist

- [x] `Zero-axiom` crate created and compiles
- [x] SysLog records request + response for each syscall
- [x] CommitLog starts with Genesis commit
- [x] Hash chain verification passes
- [x] Kernel integrated with AxiomGateway
- [x] All existing tests still pass (37 kernel tests)
- [x] New tests for SysLog/CommitLog pass (24 axiom tests)
- [x] Code formatted (`cargo fmt`)
- [x] Clippy clean (`cargo clippy`)

## Test Results

```
running 24 tests (Zero-axiom)
test commitlog::tests::test_commitlog_creation ... ok
test commitlog::tests::test_commitlog_append ... ok
test commitlog::tests::test_commitlog_integrity ... ok
test commitlog::tests::test_commitlog_get_range ... ok
test commitlog::tests::test_commitlog_get_recent ... ok
test commitlog::tests::test_commitlog_hash_determinism ... ok
test gateway::tests::test_gateway_creation ... ok
test gateway::tests::test_gateway_syscall_no_commits ... ok
test gateway::tests::test_gateway_syscall_with_commits ... ok
test gateway::tests::test_gateway_multiple_syscalls ... ok
test gateway::tests::test_gateway_state_summary ... ok
test gateway::tests::test_gateway_internal_commit ... ok
test syslog::tests::test_syslog_creation ... ok
test syslog::tests::test_syslog_request_response ... ok
test syslog::tests::test_syslog_get_recent ... ok
test syslog::tests::test_syslog_get_range ... ok
test types::tests::test_permissions_byte_roundtrip ... ok
test types::tests::test_object_type_from_u8 ... ok
... (6 integration tests)

test result: ok. 24 passed; 0 failed
```

## Actual Changes

| File | Change Type | Lines |
|------|-------------|-------|
| `crates/Zero-axiom/` | New crate | ~550 |
| `crates/Zero-kernel/src/lib.rs` | Modify | ~80 |
| `crates/Zero-kernel/Cargo.toml` | Modify | 1 |
| `Cargo.toml` | Modify | 2 |

## Next Stage

This stage is complete. Proceed to [Stage 1.6: Replay + Testing](stage-1.6-replay-testing.md) to implement the replay functions that use the CommitLog.
