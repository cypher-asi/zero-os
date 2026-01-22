# CommitLog - State Mutation Log

## Overview

CommitLog records all state mutations for deterministic replay. Each commit links to the previous via a hash chain, ensuring integrity.

## Core Invariant

```
reduce(genesis, commits) -> state
```

Replaying the same CommitLog always produces the same kernel state.

## Commit Structure

```rust
pub struct Commit {
    /// Hash of this commit (computed from contents + prev_commit)
    pub id: CommitId,
    /// Hash of the previous commit (chain integrity)
    pub prev_commit: CommitId,
    /// Sequence number (monotonic)
    pub seq: u64,
    /// Timestamp (nanos since boot)
    pub timestamp: u64,
    /// The type of state mutation
    pub commit_type: CommitType,
    /// Optional: the syscall event that caused this commit
    pub caused_by: Option<EventId>,
}
```

## Commit Types

```rust
pub enum CommitType {
    /// Genesis commit (system boot)
    Genesis,

    // === Process Lifecycle ===
    ProcessCreated { pid: ProcessId, parent: ProcessId, name: String },
    ProcessExited { pid: ProcessId, code: i32 },
    ProcessFaulted { pid: ProcessId, reason: u32, description: String },

    // === Capability Mutations ===
    CapInserted { pid: ProcessId, slot: CapSlot, cap_id: u64, object_type: u8, object_id: u64, perms: u8 },
    CapRemoved { pid: ProcessId, slot: CapSlot },
    CapGranted { from_pid: ProcessId, to_pid: ProcessId, from_slot: CapSlot, to_slot: CapSlot, new_cap_id: u64, perms: Permissions },

    // === Endpoint Lifecycle ===
    EndpointCreated { id: EndpointId, owner: ProcessId },
    EndpointDestroyed { id: EndpointId },

    // === IPC Events ===
    MessageSent { from_pid: ProcessId, to_endpoint: EndpointId, tag: u32, size: usize },
}
```

## Hash Chain

Each commit's hash is computed from:
- Previous commit hash (`prev_commit`)
- Sequence number
- Timestamp
- Commit type discriminant and data

```
┌─────────┐    ┌─────────┐    ┌─────────┐    ┌─────────┐
│ Genesis │───▶│Commit 1 │───▶│Commit 2 │───▶│Commit 3 │
│ hash: A │    │prev: A  │    │prev: B  │    │prev: C  │
│         │    │hash: B  │    │hash: C  │    │hash: D  │
└─────────┘    └─────────┘    └─────────┘    └─────────┘
```

## API

### Creation

```rust
impl CommitLog {
    /// Create new CommitLog with genesis commit
    pub fn new(timestamp: u64) -> Self;

    /// Append a commit, returns commit hash
    pub fn append(
        &mut self,
        commit_type: CommitType,
        caused_by: Option<EventId>,
        timestamp: u64,
    ) -> CommitId;
}
```

### Querying

```rust
impl CommitLog {
    /// All commits
    pub fn commits(&self) -> &[Commit];

    /// Commits in sequence range [start, end)
    pub fn get_range(&self, start_seq: u64, end_seq: u64) -> Vec<&Commit>;

    /// Most recent N commits
    pub fn get_recent(&self, count: usize) -> Vec<&Commit>;

    /// Head commit hash
    pub fn head(&self) -> CommitId;

    /// Current sequence number
    pub fn current_seq(&self) -> u64;

    /// Commit count
    pub fn len(&self) -> usize;
}
```

### Verification

```rust
impl CommitLog {
    /// Verify hash chain integrity
    pub fn verify_integrity(&self) -> bool;
}
```

## Hash Algorithm

Currently uses FNV-1a (64-bit) for `no_std` compatibility, expanded to 32 bytes:

```rust
fn compute_hash(commit: &Commit) -> CommitId {
    let mut hash = 0xcbf29ce484222325u64; // FNV offset basis
    const FNV_PRIME: u64 = 0x100000001b3;
    
    // Hash prev_commit, seq, timestamp, type, and type-specific data
    // ...
    
    // Expand to 32 bytes
    let mut result = [0u8; 32];
    // ...
    result
}
```

**Note**: Production deployments should use SHA-256 for cryptographic security.

## Memory Management

```rust
const MAX_COMMITLOG_ENTRIES: usize = 100000;
```

Old commits are trimmed from the front when exceeding capacity. For full history, commits should be persisted to IndexedDB (WASM) or disk.

## Persistence

On WASM, commits can be serialized to IndexedDB:

```rust
// Supervisor periodically persists new commits
let new_commits = commitlog.get_range(last_persisted_seq, commitlog.current_seq());
await indexedDB.put("commitlog", new_commits);
```

## Replay

See [03-replay.md](./03-replay.md) for deterministic state reconstruction from commits.

## Compliance Checklist

### Source Files
- `crates/zos-axiom/src/commitlog.rs`

### Key Invariants
- [ ] Genesis commit is always seq 0
- [ ] Sequence numbers are monotonically increasing
- [ ] Hash chain is valid (verify_integrity returns true)
- [ ] Each commit links to causing syscall when applicable
- [ ] CommitType covers all state mutations

### Differences from v0.1.0
- Added ProcessFaulted for crash recording
- Added CapInserted/CapRemoved for fine-grained cap tracking
- MessageSent records metadata only (not content) for privacy
- Causal linking via caused_by field
