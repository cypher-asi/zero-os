//! Neural key and machine key operations
//!
//! Handlers for:
//! - Neural key generation and recovery
//! - Machine key CRUD operations (create, list, get, revoke, rotate)
//!
//! # Dual User ID Pattern (CRITICAL)
//!
//! This module uses TWO different user ID concepts that must not be confused:
//!
//! ## 1. Derivation User ID (Original)
//!
//! - **Source**: The user_id from the initial `generateNeuralKey` request
//! - **Stored in**: `LocalKeyStore.user_id`, `EncryptedShardStore.user_id`
//! - **Purpose**: Used for cryptographic key derivation via `derive_identity_signing_keypair()`
//! - **Important**: The identity keypair is derived using `Uuid::from_u128(derivation_user_id)`
//!
//! ## 2. Storage User ID (Derived)
//!
//! - **Source**: `derive_user_id_from_pubkey(identity_signing_public_key)` - SHA-256 hash of pubkey
//! - **Returned to client**: In `NeuralKeyGenerated.user_id` after generation/recovery
//! - **Purpose**: Used for storage paths via `LocalKeyStore::storage_path(storage_user_id)`
//! - **Client uses**: All subsequent API calls (machine key creation, recovery, etc.)
//!
//! ## Why Two User IDs?
//!
//! The derived user_id provides a canonical, deterministic identifier that:
//! - Is cryptographically tied to the identity signing key
//! - Can be independently verified by any party with the public key
//! - Prevents confusion when the same user registers with different initial IDs
//!
//! ## Data Flow Example
//!
//! ```text
//! Generation:
//!   1. Client calls generateNeuralKey(REQUEST_ID, password)
//!   2. Backend derives identity_signing_pubkey using REQUEST_ID
//!   3. Backend computes derived_user_id = hash(identity_signing_pubkey)
//!   4. Backend stores LocalKeyStore at path(derived_user_id), with user_id = REQUEST_ID
//!   5. Backend returns NeuralKeyGenerated { user_id: derived_user_id, ... }
//!
//! Machine Key Creation:
//!   1. Client calls createMachineKey(derived_user_id, shard, password)
//!   2. Backend reads LocalKeyStore from path(derived_user_id)
//!   3. Backend uses stored REQUEST_ID for verification in combine_shards_verified()
//!   4. Backend derives machine keys using REQUEST_ID
//!
//! Recovery:
//!   1. Client calls recoverNeuralKey(derived_user_id, shards)
//!   2. Backend reads LocalKeyStore from path(derived_user_id)
//!   3. Backend extracts REQUEST_ID from LocalKeyStore.user_id
//!   4. Backend uses REQUEST_ID for verification in combine_shards_verified()
//!   5. Backend writes back to path(derived_user_id)
//! ```
//!
//! ## Common Pitfalls
//!
//! - **DO NOT** use the request user_id for cryptographic verification - use stored derivation_user_id
//! - **DO NOT** use derivation_user_id for storage paths - use storage_user_id (derived)
//! - **ALWAYS** store the derivation_user_id in LocalKeyStore.user_id for future operations
//!
//! # Invariant 32 Compliance
//!
//! All `/keys/` paths use Keystore IPC (via KeystoreService PID 7), NOT VFS.
//! Directory operations (e.g., `/home/{user_id}/.zos/identity/`) still use VFS.
//!
//! # Safety Invariants (per zos-service.md Rule 0)
//!
//! ## Success Conditions
//! - Neural key generation: Key generated, split into shards, stored to Keystore, response sent
//! - Machine key creation: Neural Key verified against stored identity, keypair derived, stored
//! - Key rotation: Existing key read, new keys generated, stored atomically
//!
//! ## Acceptable Partial Failure
//! - Orphan content if write fails (cleanup handled)
//!
//! ## Forbidden States
//! - Returning shards before key is persisted
//! - Creating machine key without verifying Neural Key ownership
//! - Silent fallthrough on parse errors (must return InvalidRequest)
//! - Processing requests without authorization check

mod generate;
mod recover;
mod machine;
mod enroll;
mod shared;

// Re-export all public handlers
pub use generate::{
    continue_generate_after_directory_check,
    continue_create_directories,
    handle_generate_neural_key,
    continue_generate_after_exists_check,
};

pub use recover::{
    handle_recover_neural_key,
    continue_recover_after_identity_read,
    handle_get_identity_key,
};

pub use machine::{
    handle_create_machine_key,
    continue_create_machine_after_identity_read,
    continue_create_machine_after_shards_read,
    handle_list_machine_keys,
    handle_revoke_machine_key,
    handle_rotate_machine_key,
    continue_rotate_after_read,
    handle_get_machine_key,
};

pub use enroll::{
    handle_create_machine_key_and_enroll,
    continue_create_machine_enroll_after_shards_read,
};

// ============================================================================
// Shared utilities
// ============================================================================

use sha2::{Sha256, Digest};
use zos_identity::types::UserId;

/// Derive the "storage user ID" from the identity signing public key.
///
/// Takes the first 128 bits (16 bytes) of SHA-256 hash of the public key.
/// This creates a deterministic, unique user ID from the cryptographic identity.
///
/// # Dual User ID Pattern
///
/// This function produces the **storage_user_id** (also called derived_user_id):
/// - Used for storage paths: `LocalKeyStore::storage_path(storage_user_id)`
/// - Returned to client: `NeuralKeyGenerated.user_id = storage_user_id`
/// - Client uses this for all subsequent API calls
///
/// This is DIFFERENT from the **derivation_user_id** (original user_id):
/// - Stored in `LocalKeyStore.user_id`
/// - Used for cryptographic key derivation
///
/// See module-level documentation for complete explanation.
pub(crate) fn derive_user_id_from_pubkey(identity_signing_public_key: &[u8; 32]) -> UserId {
    let mut hasher = Sha256::new();
    hasher.update(identity_signing_public_key);
    let hash = hasher.finalize();
    
    // Take first 16 bytes (128 bits) as user ID
    let mut id_bytes = [0u8; 16];
    id_bytes.copy_from_slice(&hash[..16]);
    u128::from_be_bytes(id_bytes)
}

extern crate alloc;
