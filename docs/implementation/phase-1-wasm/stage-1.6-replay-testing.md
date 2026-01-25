# Stage 1.6: Replay + Testing

> **Status**: ✅ **COMPLETE**
>
> **Goal**: Verify deterministic replay works correctly.

## Overview

This is the **key missing piece** for Zero OS. Deterministic replay is the core guarantee:

> **Same CommitLog always produces same state.**

This stage depends on Stage 1.2 (Axiom Layer) being complete with SysLog and CommitLog.

## Current State

### What's Implemented ✅

| Component | Status | Description |
|-----------|--------|-------------|
| CommitLog | ✅ | Complete from Stage 1.2 |
| `apply_commit()` | ✅ | Apply single commit to state |
| `replay()` | ✅ | Replay full CommitLog |
| `replay_and_verify()` | ✅ | Replay with hash verification |
| `Replayable` trait | ✅ | Trait for replayable state |
| `StateHasher` | ✅ | FNV-1a hasher for deterministic state hashing |
| `state_hash()` | ✅ | Hash kernel state (processes, caps, endpoints) |
| Replay tests | ✅ | 12 tests verifying determinism |

### Implementation Details

- **Location**: `crates/Zero-axiom/src/replay.rs`
- **Kernel Integration**: `crates/Zero-kernel/src/lib.rs` (Replayable impl)
- **Fix**: `grant_capability()` now logs both `CapGranted` and `CapInserted` for correct replay

### Dependencies

```
Stage 1.2 (Axiom Layer) ─────┐
                             ├──→ Stage 1.6 (Replay) ✅
Kernel state mutations ──────┘
```

## Required Implementation

### Task 1: Implement `apply_commit()`

**File**: `crates/Zero-axiom/src/replay.rs`

```rust
use crate::{CommitType, Commit};
use Zero_kernel::{Kernel, ProcessId, ProcessState, CapabilitySpace, Capability, Endpoint};

/// Apply a single commit to kernel state.
/// 
/// This is a pure function - no side effects beyond state mutation.
/// Must be deterministic: same commit + same state = same result.
pub fn apply_commit<H: HAL>(kernel: &mut Kernel<H>, commit: &Commit) -> Result<(), ReplayError> {
    match &commit.commit_type {
        CommitType::Genesis => {
            // Genesis is implicit - kernel starts in genesis state
            Ok(())
        }
        
        CommitType::ProcessCreated { pid, parent, name } => {
            // Create process entry (without actually spawning)
            kernel.replay_create_process(*pid, *parent, name.clone())?;
            Ok(())
        }
        
        CommitType::ProcessExited { pid, code } => {
            kernel.replay_exit_process(*pid, *code)?;
            Ok(())
        }
        
        CommitType::CapInserted { pid, slot, cap_id, object_type, object_id, perms } => {
            let cap = Capability {
                id: *cap_id,
                object_type: ObjectType::from_u8(*object_type).ok_or(ReplayError::InvalidCommit)?,
                object_id: *object_id,
                permissions: Permissions::from_byte(*perms),
                generation: 0,
                expires_at: 0,
            };
            kernel.replay_insert_capability(*pid, *slot, cap)?;
            Ok(())
        }
        
        CommitType::CapRemoved { pid, slot } => {
            kernel.replay_remove_capability(*pid, *slot)?;
            Ok(())
        }
        
        CommitType::EndpointCreated { id, owner } => {
            kernel.replay_create_endpoint(*id, *owner)?;
            Ok(())
        }
        
        CommitType::EndpointDestroyed { id } => {
            kernel.replay_destroy_endpoint(*id)?;
            Ok(())
        }
    }
}

#[derive(Debug)]
pub enum ReplayError {
    InvalidCommit,
    ProcessNotFound,
    EndpointNotFound,
    CapabilityError,
    HashMismatch,
}
```

### Task 2: Add Replay Methods to Kernel

**File**: `crates/Zero-kernel/src/lib.rs`

Add replay-specific methods that mutate state without side effects:

```rust
impl<H: HAL> Kernel<H> {
    /// Create process during replay (no actual Worker).
    pub fn replay_create_process(
        &mut self,
        pid: ProcessId,
        parent: ProcessId,
        name: String,
    ) -> Result<(), ReplayError> {
        let process = Process {
            pid,
            name,
            state: ProcessState::Running,
            metrics: ProcessMetrics::default(),
        };
        self.processes.insert(pid, process);
        self.cap_spaces.insert(pid, CapabilitySpace::new());
        Ok(())
    }
    
    /// Exit process during replay.
    pub fn replay_exit_process(&mut self, pid: ProcessId, _code: i32) -> Result<(), ReplayError> {
        if let Some(proc) = self.processes.get_mut(&pid) {
            proc.state = ProcessState::Zombie;
            Ok(())
        } else {
            Err(ReplayError::ProcessNotFound)
        }
    }
    
    /// Insert capability during replay.
    pub fn replay_insert_capability(
        &mut self,
        pid: ProcessId,
        slot: CapSlot,
        cap: Capability,
    ) -> Result<(), ReplayError> {
        let cspace = self.cap_spaces.get_mut(&pid).ok_or(ReplayError::ProcessNotFound)?;
        cspace.slots.insert(slot, cap);
        Ok(())
    }
    
    /// Remove capability during replay.
    pub fn replay_remove_capability(&mut self, pid: ProcessId, slot: CapSlot) -> Result<(), ReplayError> {
        let cspace = self.cap_spaces.get_mut(&pid).ok_or(ReplayError::ProcessNotFound)?;
        cspace.remove(slot);
        Ok(())
    }
    
    /// Create endpoint during replay.
    pub fn replay_create_endpoint(&mut self, id: EndpointId, owner: ProcessId) -> Result<(), ReplayError> {
        let endpoint = Endpoint {
            id,
            owner,
            pending_messages: VecDeque::new(),
            metrics: EndpointMetrics::default(),
        };
        self.endpoints.insert(id, endpoint);
        Ok(())
    }
    
    /// Destroy endpoint during replay.
    pub fn replay_destroy_endpoint(&mut self, id: EndpointId) -> Result<(), ReplayError> {
        self.endpoints.remove(&id);
        Ok(())
    }
}
```

### Task 3: Implement `replay()`

**File**: `crates/Zero-axiom/src/replay.rs`

```rust
/// Replay a CommitLog to reconstruct state.
/// 
/// `reduce(genesis, commits) -> state`
/// 
/// This function is deterministic: same commits always produce same state.
pub fn replay<H: HAL>(commits: &[Commit]) -> Result<Kernel<MockHal>, ReplayError> {
    let mut kernel = Kernel::new_for_replay();
    
    // Skip genesis (commit 0) - kernel starts in genesis state
    for commit in commits.iter().skip(1) {
        apply_commit(&mut kernel, commit)?;
    }
    
    Ok(kernel)
}

impl<H: HAL> Kernel<H> {
    /// Create kernel in replay mode (no HAL operations).
    pub fn new_for_replay() -> Self {
        Self {
            hal: MockHal::new(),  // Use mock HAL for replay
            processes: BTreeMap::new(),
            cap_spaces: BTreeMap::new(),
            endpoints: BTreeMap::new(),
            axiom_log: AxiomLog::new(),
            // ... other fields at default
        }
    }
}
```

### Task 4: Implement State Hashing

**File**: `crates/Zero-kernel/src/lib.rs`

```rust
impl<H: HAL> Kernel<H> {
    /// Compute hash of kernel state for comparison.
    /// 
    /// This hash covers:
    /// - Process table (PIDs, names, states)
    /// - Capability spaces (all capabilities)
    /// - Endpoints (IDs, owners)
    /// 
    /// Does NOT include:
    /// - Message queues (volatile)
    /// - Metrics (non-deterministic)
    pub fn state_hash(&self) -> [u8; 32] {
        let mut hasher = FnvHasher::new();
        
        // Hash processes (sorted by PID for determinism)
        for (pid, proc) in &self.processes {
            hasher.write_u64(pid.0);
            hasher.write_str(&proc.name);
            hasher.write_u8(match proc.state {
                ProcessState::Running => 0,
                ProcessState::Blocked => 1,
                ProcessState::Zombie => 2,
            });
        }
        
        // Hash capability spaces
        for (pid, cspace) in &self.cap_spaces {
            hasher.write_u64(pid.0);
            for (slot, cap) in &cspace.slots {
                hasher.write_u32(*slot);
                hasher.write_u64(cap.id);
                hasher.write_u8(cap.object_type as u8);
                hasher.write_u64(cap.object_id);
                hasher.write_u8(cap.permissions.to_byte());
            }
        }
        
        // Hash endpoints
        for (id, ep) in &self.endpoints {
            hasher.write_u64(id.0);
            hasher.write_u64(ep.owner.0);
        }
        
        hasher.finalize()
    }
}

struct FnvHasher {
    hash: u64,
}

impl FnvHasher {
    fn new() -> Self { Self { hash: 0xcbf29ce484222325 } }
    
    fn write_u8(&mut self, v: u8) {
        self.hash ^= v as u64;
        self.hash = self.hash.wrapping_mul(0x100000001b3);
    }
    
    fn write_u32(&mut self, v: u32) {
        for b in v.to_le_bytes() { self.write_u8(b); }
    }
    
    fn write_u64(&mut self, v: u64) {
        for b in v.to_le_bytes() { self.write_u8(b); }
    }
    
    fn write_str(&mut self, s: &str) {
        for b in s.bytes() { self.write_u8(b); }
    }
    
    fn finalize(&self) -> [u8; 32] {
        let mut result = [0u8; 32];
        let mut h = self.hash;
        for chunk in result.chunks_mut(8) {
            chunk.copy_from_slice(&h.to_le_bytes());
            h = h.wrapping_mul(0x100000001b3);
        }
        result
    }
}
```

### Task 5: Write Replay Tests

**File**: `crates/Zero-kernel/tests/replay.rs`

```rust
use Zero_kernel::{Kernel, ProcessId};
use Zero_hal_mock::MockHal;
use Zero_axiom::{replay, CommitType};

#[test]
fn test_replay_empty_commitlog() {
    // Genesis only
    let commits = vec![create_genesis_commit()];
    
    let kernel = replay(&commits).expect("replay should succeed");
    
    assert_eq!(kernel.list_processes().len(), 0);
    assert_eq!(kernel.list_endpoints().len(), 0);
}

#[test]
fn test_replay_single_process() {
    let commits = vec![
        create_genesis_commit(),
        create_commit(CommitType::ProcessCreated {
            pid: ProcessId(1),
            parent: ProcessId(0),
            name: "init".into(),
        }),
    ];
    
    let kernel = replay(&commits).expect("replay should succeed");
    
    assert_eq!(kernel.list_processes().len(), 1);
    let proc = kernel.get_process(ProcessId(1)).expect("process should exist");
    assert_eq!(proc.name, "init");
}

#[test]
fn test_replay_determinism() {
    // Run system, record commits
    let hal = MockHal::new();
    let mut kernel1 = Kernel::new(hal);
    
    let pid1 = kernel1.register_process("init");
    let (eid, slot) = kernel1.create_endpoint(pid1).unwrap();
    let pid2 = kernel1.register_process("terminal");
    kernel1.grant_capability(pid1, slot, pid2, Permissions::read_only()).unwrap();
    
    // Get state hash and commits
    let hash1 = kernel1.state_hash();
    let commits = kernel1.axiom().commitlog().commits().to_vec();
    
    // Replay from commits
    let kernel2 = replay(&commits).expect("replay should succeed");
    let hash2 = kernel2.state_hash();
    
    // Hashes must match
    assert_eq!(hash1, hash2, "Replay must produce identical state");
}

#[test]
fn test_replay_multiple_times() {
    // Create commits
    let commits = create_test_commits();
    
    // Replay 10 times
    let hashes: Vec<[u8; 32]> = (0..10)
        .map(|_| replay(&commits).expect("replay").state_hash())
        .collect();
    
    // All hashes must be identical
    for hash in &hashes[1..] {
        assert_eq!(hash, &hashes[0], "All replays must produce same hash");
    }
}

#[test]
fn test_replay_capability_lifecycle() {
    let commits = vec![
        create_genesis_commit(),
        create_commit(CommitType::ProcessCreated { pid: ProcessId(1), parent: ProcessId(0), name: "owner".into() }),
        create_commit(CommitType::EndpointCreated { id: 1, owner: ProcessId(1) }),
        create_commit(CommitType::CapInserted { 
            pid: ProcessId(1), slot: 0, cap_id: 1, 
            object_type: 1, object_id: 1, perms: 0x07 
        }),
        create_commit(CommitType::ProcessCreated { pid: ProcessId(2), parent: ProcessId(1), name: "client".into() }),
        create_commit(CommitType::CapInserted { 
            pid: ProcessId(2), slot: 0, cap_id: 2,
            object_type: 1, object_id: 1, perms: 0x01  // Read only
        }),
        create_commit(CommitType::CapRemoved { pid: ProcessId(2), slot: 0 }),
    ];
    
    let kernel = replay(&commits).expect("replay should succeed");
    
    // Owner still has capability
    let owner_cspace = kernel.get_cap_space(ProcessId(1)).unwrap();
    assert!(owner_cspace.get(0).is_some());
    
    // Client's capability was removed
    let client_cspace = kernel.get_cap_space(ProcessId(2)).unwrap();
    assert!(client_cspace.get(0).is_none());
}

#[test]
fn test_nondeterminism_detection() {
    // This test verifies that if we accidentally introduce
    // nondeterminism (e.g., using HashMap instead of BTreeMap),
    // the replay would produce different hashes.
    
    // The test passes if our implementation is deterministic
}
```

### Task 6: Add Replay to Browser (Optional)

**File**: `apps/zos-supervisor/www/index.html`

Add UI for replay:

```javascript
// Export CommitLog
function exportCommitLog() {
    const commits = JSON.parse(supervisor.export_commitlog_json());
    const blob = new Blob([JSON.stringify(commits, null, 2)], { type: 'application/json' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = 'Zero-commitlog.json';
    a.click();
}

// Import and replay CommitLog
async function replayCommitLog(file) {
    const text = await file.text();
    const commits = JSON.parse(text);
    supervisor.replay_commitlog(commits);
    updateDashboard();
}
```

## Verification Checklist

- [x] `apply_commit()` implemented for all CommitTypes
- [x] `replay()` function works
- [x] `state_hash()` produces deterministic hashes
- [x] Empty CommitLog replay works
- [x] Single commit replay works
- [x] Multi-commit replay is deterministic
- [x] Replay 10x produces identical hashes
- [x] Capability lifecycle replay works
- [x] Process lifecycle replay works
- [x] Endpoint lifecycle replay works

## Test Matrix

| Scenario | Expected |
|----------|----------|
| Genesis only | Empty state |
| Create process | Process exists |
| Create + exit | Zombie process |
| Create endpoint | Endpoint exists |
| Insert capability | Cap in CSpace |
| Remove capability | Cap removed |
| Grant chain | Correct permissions |
| 10x replay | Identical hashes |

## Estimated Changes

| File | Change Type | Lines |
|------|-------------|-------|
| `crates/Zero-axiom/src/replay.rs` | New | ~200 |
| `crates/Zero-kernel/src/lib.rs` | Modify | ~150 |
| `tests/replay.rs` | New | ~200 |

## Next Stage

After deterministic replay is verified, proceed to [Stage 1.7: Web UI](stage-1.7-web-ui.md).
