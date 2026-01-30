//! Machine key operations (create, list, get, revoke, rotate)
//!
//! This module handles all machine key CRUD operations:
//! - `create` - Creating new machine keys derived from Neural Key
//! - `query` - Listing, getting, and revoking machine keys
//! - `rotate` - Rotating machine key cryptographic material
//!
//! # Invariant 32 Compliance
//!
//! All `/keys/` paths use Keystore IPC (via KeystoreService PID 7), NOT VFS.
//!
//! # Security
//!
//! Machine key creation requires:
//! 1. Valid authorization (FAIL-CLOSED pattern)
//! 2. Correct password to decrypt stored shards
//! 3. One external shard from user backup
//! 4. Verification that reconstructed Neural Key matches stored identity

mod create;
mod query;
mod rotate;

pub use create::{
    handle_create_machine_key,
    continue_create_machine_after_identity_read,
    continue_create_machine_after_shards_read,
};

pub use query::{
    handle_list_machine_keys,
    handle_revoke_machine_key,
    handle_get_machine_key,
};

pub use rotate::{
    handle_rotate_machine_key,
    continue_rotate_after_read,
};
