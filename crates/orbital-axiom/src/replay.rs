//! Deterministic Replay for Orbital OS
//!
//! This module provides the core replay functionality that ensures:
//!
//! > Same CommitLog always produces same state.
//!
//! # Architecture
//!
//! Replay works by applying commits in sequence to a fresh kernel:
//!
//! ```text
//! reduce(genesis, commits) -> state
//! ```
//!
//! Each commit is a pure state mutation with no side effects.

use alloc::string::String;

use crate::commitlog::{Commit, CommitType};
use crate::types::{CapSlot, EndpointId, Permissions, ProcessId};

/// Errors that can occur during replay.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReplayError {
    /// Commit references invalid or missing data
    InvalidCommit(String),
    /// Process not found during replay
    ProcessNotFound(ProcessId),
    /// Endpoint not found during replay
    EndpointNotFound(EndpointId),
    /// Capability operation failed
    CapabilityError(String),
    /// State hash mismatch after replay
    HashMismatch {
        expected: [u8; 32],
        actual: [u8; 32],
    },
    /// Unknown object type in commit
    UnknownObjectType(u8),
}

/// Result of applying a commit.
pub type ReplayResult<T> = Result<T, ReplayError>;

/// Trait for types that can be replayed.
///
/// This trait is implemented by the Kernel to support deterministic replay.
/// Each method corresponds to a CommitType and applies that mutation
/// without any side effects (no HAL calls, no IPC, etc.).
pub trait Replayable {
    /// Apply genesis commit (typically a no-op, kernel starts in genesis state).
    fn replay_genesis(&mut self) -> ReplayResult<()>;

    /// Create a process during replay.
    ///
    /// Unlike normal process creation, this:
    /// - Does NOT spawn an actual process/worker
    /// - Does NOT log to CommitLog (we're replaying)
    /// - Only updates internal state
    fn replay_create_process(
        &mut self,
        pid: ProcessId,
        parent: ProcessId,
        name: String,
    ) -> ReplayResult<()>;

    /// Exit a process during replay.
    fn replay_exit_process(&mut self, pid: ProcessId, code: i32) -> ReplayResult<()>;

    /// Insert a capability during replay.
    fn replay_insert_capability(
        &mut self,
        pid: ProcessId,
        slot: CapSlot,
        cap_id: u64,
        object_type: u8,
        object_id: u64,
        perms: u8,
    ) -> ReplayResult<()>;

    /// Remove a capability during replay.
    fn replay_remove_capability(&mut self, pid: ProcessId, slot: CapSlot) -> ReplayResult<()>;

    /// Record a capability grant during replay.
    ///
    /// Note: The actual capability insertion is done via replay_insert_capability.
    /// This is mainly for tracking the grant relationship.
    fn replay_cap_granted(
        &mut self,
        from_pid: ProcessId,
        to_pid: ProcessId,
        from_slot: CapSlot,
        to_slot: CapSlot,
        new_cap_id: u64,
        perms: Permissions,
    ) -> ReplayResult<()>;

    /// Create an endpoint during replay.
    fn replay_create_endpoint(&mut self, id: EndpointId, owner: ProcessId) -> ReplayResult<()>;

    /// Destroy an endpoint during replay.
    fn replay_destroy_endpoint(&mut self, id: EndpointId) -> ReplayResult<()>;

    /// Compute a deterministic hash of the current state.
    ///
    /// This hash covers:
    /// - Process table (PIDs, names, states)
    /// - Capability spaces (all capabilities)
    /// - Endpoints (IDs, owners)
    ///
    /// Does NOT include:
    /// - Message queues (volatile)
    /// - Metrics (non-deterministic)
    fn state_hash(&self) -> [u8; 32];
}

/// Apply a single commit to a replayable state.
///
/// This is a pure function - no side effects beyond state mutation.
/// Must be deterministic: same commit + same state = same result.
///
/// # Arguments
/// - `state`: Mutable reference to a Replayable state
/// - `commit`: The commit to apply
///
/// # Returns
/// - `Ok(())`: Commit applied successfully
/// - `Err(ReplayError)`: Error applying commit
pub fn apply_commit<R: Replayable>(state: &mut R, commit: &Commit) -> ReplayResult<()> {
    match &commit.commit_type {
        CommitType::Genesis => {
            // Genesis is implicit - kernel starts in genesis state
            state.replay_genesis()
        }

        CommitType::ProcessCreated { pid, parent, name } => {
            state.replay_create_process(*pid, *parent, name.clone())
        }

        CommitType::ProcessExited { pid, code } => state.replay_exit_process(*pid, *code),

        CommitType::CapInserted {
            pid,
            slot,
            cap_id,
            object_type,
            object_id,
            perms,
        } => state.replay_insert_capability(*pid, *slot, *cap_id, *object_type, *object_id, *perms),

        CommitType::CapRemoved { pid, slot } => state.replay_remove_capability(*pid, *slot),

        CommitType::CapGranted {
            from_pid,
            to_pid,
            from_slot,
            to_slot,
            new_cap_id,
            perms,
        } => state.replay_cap_granted(
            *from_pid,
            *to_pid,
            *from_slot,
            *to_slot,
            *new_cap_id,
            *perms,
        ),

        CommitType::EndpointCreated { id, owner } => state.replay_create_endpoint(*id, *owner),

        CommitType::EndpointDestroyed { id } => state.replay_destroy_endpoint(*id),
    }
}

/// Replay a sequence of commits to reconstruct state.
///
/// This function applies commits in order, starting from genesis.
///
/// # Arguments
/// - `state`: Mutable reference to a Replayable state (should be fresh/empty)
/// - `commits`: Slice of commits to replay
///
/// # Returns
/// - `Ok(())`: All commits applied successfully
/// - `Err(ReplayError)`: Error during replay
///
/// # Example
///
/// ```ignore
/// let mut kernel = Kernel::new_for_replay();
/// replay(&mut kernel, commitlog.commits())?;
/// let hash = kernel.state_hash();
/// ```
pub fn replay<R: Replayable>(state: &mut R, commits: &[Commit]) -> ReplayResult<()> {
    for commit in commits {
        apply_commit(state, commit)?;
    }
    Ok(())
}

/// Replay commits and verify the final state hash.
///
/// This is the primary verification function for deterministic replay.
///
/// # Arguments
/// - `state`: Mutable reference to a Replayable state (should be fresh/empty)
/// - `commits`: Slice of commits to replay
/// - `expected_hash`: Expected state hash after replay
///
/// # Returns
/// - `Ok(())`: Replay successful and hash matches
/// - `Err(ReplayError::HashMismatch)`: Replay successful but hash differs
/// - `Err(ReplayError)`: Error during replay
pub fn replay_and_verify<R: Replayable>(
    state: &mut R,
    commits: &[Commit],
    expected_hash: [u8; 32],
) -> ReplayResult<()> {
    replay(state, commits)?;

    let actual_hash = state.state_hash();
    if actual_hash != expected_hash {
        return Err(ReplayError::HashMismatch {
            expected: expected_hash,
            actual: actual_hash,
        });
    }

    Ok(())
}

/// FNV-1a hasher for state hashing.
///
/// This is a simple, deterministic hasher suitable for no_std environments.
/// The hash is expanded to 32 bytes for compatibility with cryptographic hashes.
pub struct StateHasher {
    hash: u64,
}

impl StateHasher {
    const FNV_OFFSET: u64 = 0xcbf29ce484222325;
    const FNV_PRIME: u64 = 0x100000001b3;

    /// Create a new hasher.
    pub fn new() -> Self {
        Self {
            hash: Self::FNV_OFFSET,
        }
    }

    /// Write a single byte.
    pub fn write_u8(&mut self, v: u8) {
        self.hash ^= v as u64;
        self.hash = self.hash.wrapping_mul(Self::FNV_PRIME);
    }

    /// Write a u32.
    pub fn write_u32(&mut self, v: u32) {
        for b in v.to_le_bytes() {
            self.write_u8(b);
        }
    }

    /// Write a u64.
    pub fn write_u64(&mut self, v: u64) {
        for b in v.to_le_bytes() {
            self.write_u8(b);
        }
    }

    /// Write a string.
    pub fn write_str(&mut self, s: &str) {
        // Write length first for unambiguous hashing
        self.write_u64(s.len() as u64);
        for b in s.bytes() {
            self.write_u8(b);
        }
    }

    /// Write a byte slice.
    pub fn write_bytes(&mut self, bytes: &[u8]) {
        self.write_u64(bytes.len() as u64);
        for b in bytes {
            self.write_u8(*b);
        }
    }

    /// Finalize and return a 32-byte hash.
    ///
    /// The 64-bit FNV hash is expanded to 32 bytes by iteratively
    /// multiplying by the FNV prime.
    pub fn finalize(&self) -> [u8; 32] {
        let mut result = [0u8; 32];
        let mut h = self.hash;

        for chunk in result.chunks_mut(8) {
            let bytes = h.to_le_bytes();
            chunk.copy_from_slice(&bytes[..chunk.len()]);
            h = h.wrapping_mul(Self::FNV_PRIME);
        }

        result
    }
}

impl Default for StateHasher {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::format;

    #[test]
    fn test_state_hasher_deterministic() {
        let mut h1 = StateHasher::new();
        let mut h2 = StateHasher::new();

        h1.write_u64(42);
        h1.write_str("test");
        h1.write_u8(0xff);

        h2.write_u64(42);
        h2.write_str("test");
        h2.write_u8(0xff);

        assert_eq!(h1.finalize(), h2.finalize());
    }

    #[test]
    fn test_state_hasher_different_inputs() {
        let mut h1 = StateHasher::new();
        let mut h2 = StateHasher::new();

        h1.write_u64(42);
        h2.write_u64(43);

        assert_ne!(h1.finalize(), h2.finalize());
    }

    #[test]
    fn test_state_hasher_order_matters() {
        let mut h1 = StateHasher::new();
        let mut h2 = StateHasher::new();

        h1.write_u64(1);
        h1.write_u64(2);

        h2.write_u64(2);
        h2.write_u64(1);

        assert_ne!(h1.finalize(), h2.finalize());
    }

    #[test]
    fn test_replay_error_display() {
        let err = ReplayError::ProcessNotFound(123);
        assert_eq!(format!("{:?}", err), "ProcessNotFound(123)");

        let err = ReplayError::HashMismatch {
            expected: [0u8; 32],
            actual: [1u8; 32],
        };
        assert!(format!("{:?}", err).contains("HashMismatch"));
    }
}
