# Replay - Deterministic State Reconstruction

## Overview

Replay reconstructs kernel state from CommitLog. This enables:

- **Testing**: Verify implementation correctness
- **Debugging**: Reproduce bugs from commit history
- **Recovery**: Restore state after restart

## Core Principle

```
apply_commit(state, commit) -> new_state
```

Given the same starting state and commit, the result is always identical.

## Replayable Trait

```rust
/// Types that can have commits applied to them
pub trait Replayable {
    /// Apply a single commit
    fn apply(&mut self, commit: &Commit) -> Result<(), ReplayError>;
}
```

## State Hasher

For verification, state can be hashed after each commit:

```rust
/// Types that can be hashed for verification
pub trait StateHasher {
    /// Compute hash of current state
    fn hash(&self) -> [u8; 32];
}
```

## Replay Functions

### Basic Replay

```rust
/// Replay commits to reconstruct state
pub fn replay<S: Replayable>(
    state: &mut S,
    commits: &[Commit],
) -> Result<(), ReplayError>;
```

### Verified Replay

```rust
/// Replay with verification at each step
pub fn replay_and_verify<S: Replayable + StateHasher>(
    state: &mut S,
    commits: &[Commit],
    expected_hashes: &[[u8; 32]],
) -> Result<ReplayResult, ReplayError>;

pub struct ReplayResult {
    pub commits_applied: usize,
    pub final_hash: [u8; 32],
}
```

### Single Commit Application

```rust
/// Apply a single commit
pub fn apply_commit<S: Replayable>(
    state: &mut S,
    commit: &Commit,
) -> Result<(), ReplayError>;
```

## Replay Errors

```rust
pub enum ReplayError {
    /// Commit references non-existent entity
    InvalidReference(String),
    /// Commit would create duplicate entity
    DuplicateEntity(String),
    /// Commit fails verification
    VerificationFailed { seq: u64, expected: [u8; 32], actual: [u8; 32] },
    /// Application-specific error
    ApplicationError(String),
}
```

## Kernel Replay Implementation

The kernel implements `Replayable`:

```rust
impl Replayable for Kernel {
    fn apply(&mut self, commit: &Commit) -> Result<(), ReplayError> {
        match &commit.commit_type {
            CommitType::Genesis => Ok(()),
            
            CommitType::ProcessCreated { pid, parent, name } => {
                self.create_process_direct(*pid, *parent, name.clone())?;
                Ok(())
            }
            
            CommitType::ProcessExited { pid, code } => {
                self.exit_process_direct(*pid, *code)?;
                Ok(())
            }
            
            CommitType::EndpointCreated { id, owner } => {
                self.create_endpoint_direct(*id, *owner)?;
                Ok(())
            }
            
            CommitType::CapInserted { pid, slot, cap_id, object_type, object_id, perms } => {
                self.insert_cap_direct(*pid, *slot, *cap_id, *object_type, *object_id, *perms)?;
                Ok(())
            }
            
            // ... etc
        }
    }
}
```

## Use Cases

### Cold Start Recovery

```rust
// Load commits from IndexedDB
let commits = load_commits_from_storage().await;

// Create fresh kernel
let mut kernel = Kernel::new(hal);

// Replay to restore state
replay(&mut kernel, &commits)?;
```

### Testing Determinism

```rust
#[test]
fn test_replay_determinism() {
    // Execute sequence of operations
    let mut kernel1 = Kernel::new(hal);
    // ... perform operations ...
    let commits = kernel1.axiom().commitlog().commits().to_vec();
    let hash1 = kernel1.state_hash();
    
    // Replay on fresh kernel
    let mut kernel2 = Kernel::new(hal);
    replay(&mut kernel2, &commits).unwrap();
    let hash2 = kernel2.state_hash();
    
    assert_eq!(hash1, hash2);
}
```

### Bug Reproduction

```rust
// Load commit log from bug report
let commits = load_bug_commits();

// Replay up to the problematic commit
let mut kernel = Kernel::new(hal);
for (i, commit) in commits.iter().enumerate() {
    println!("Applying commit {}: {:?}", i, commit.commit_type);
    if let Err(e) = apply_commit(&mut kernel, commit) {
        println!("Error at commit {}: {:?}", i, e);
        break;
    }
}
```

## Limitations

1. **Non-deterministic operations**: Replay cannot reproduce operations that depend on:
   - Real-time clocks (use monotonic time instead)
   - External I/O (network, user input)
   - Random numbers (seed must be recorded)

2. **Message content**: IPC message content is not stored in CommitLog for privacy/size. Only metadata (from, to, tag, size) is recorded.

3. **Memory layout**: WASM memory layout may differ between original and replay (allocator details).

## Compliance Checklist

### Source Files
- `crates/zos-axiom/src/replay.rs`

### Key Invariants
- [ ] Same commits always produce same state hash
- [ ] Replay never modifies commits
- [ ] Verification catches any divergence
- [ ] Errors provide actionable information

### Differences from v0.1.0
- Added verified replay with hash checking
- Added ReplayResult for detailed feedback
- Replayable trait is now public for external implementations
