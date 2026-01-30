//! Pending operation types for async storage and network tracking
//!
//! The identity service uses async storage, keystore, and network syscalls.
//! This module defines the state tracking for pending operations awaiting results.
//!
//! # Storage Strategy
//!
//! - **VFS operations**: Used for directory structure under `/home/{user_id}/.zos/`
//! - **Keystore operations**: Used for cryptographic key data under `/keys/{user_id}/`

mod storage;
mod keystore;
mod network;

pub use storage::{ExpectedVfsResponse, PendingStorageOp};
pub use keystore::PendingKeystoreOp;
pub use network::PendingNetworkOp;

extern crate alloc;

use alloc::vec::Vec;

/// Common context for all pending operations.
///
/// This struct captures the fields shared by most pending operations:
/// - `client_pid`: The PID of the client awaiting a response
/// - `cap_slots`: Capability slots for sending the response
///
/// Extracting these common fields reduces duplication in `PendingStorageOp`
/// and simplifies handler signatures from `(client_pid, cap_slots, ...)` to `(ctx, ...)`.
#[derive(Clone, Debug)]
pub struct RequestContext {
    /// PID of the client awaiting a response
    pub client_pid: u32,
    /// Capability slots for sending the response
    pub cap_slots: Vec<u32>,
}

impl RequestContext {
    /// Create a new request context
    pub fn new(client_pid: u32, cap_slots: Vec<u32>) -> Self {
        Self { client_pid, cap_slots }
    }
}
