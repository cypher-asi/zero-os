//! Shared utilities for shard decryption and Neural Key reconstruction.
//!
//! This module provides common functions used by both `create.rs` (machine key creation)
//! and `enroll.rs` (combined machine key + ZID enrollment). By centralizing this logic,
//! we ensure consistent behavior and reduce code duplication.
//!
//! # Performance
//!
//! The `decrypt_shards_with_password` function derives the encryption key ONCE using
//! Argon2id, then decrypts each shard with the pre-derived key. This avoids running
//! Argon2id multiple times, which is especially important in WASM where Argon2id
//! runs 10-100x slower than native.

extern crate alloc;

use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

use zos_apps::syscall;
use zos_identity::crypto::{
    combine_shards_verified, decrypt_shard_with_key, derive_key_from_password_public,
    NeuralKey, ZidNeuralShard,
};
use zos_identity::ipc::NeuralShard;
use zos_identity::keystore::EncryptedShardStore;
use zos_identity::KeyError;

/// Decrypt stored shards using a password.
///
/// Derives the encryption key ONCE from the password, then decrypts each shard.
/// This avoids running Argon2id multiple times (expensive in WASM).
///
/// # Arguments
/// * `encrypted_store` - The encrypted shard store containing shards and KDF params
/// * `password` - User-provided password for decryption
///
/// # Returns
/// * `Ok(Vec<(index, hex)>)` - Vector of (shard_index, decrypted_hex) tuples
/// * `Err(KeyError)` - If key derivation or decryption fails
pub fn decrypt_shards_with_password(
    encrypted_store: &EncryptedShardStore,
    password: &str,
) -> Result<Vec<(u8, String)>, KeyError> {
    // Derive encryption key ONCE (runs Argon2id - expensive in WASM)
    let derived_key = derive_key_from_password_public(password, &encrypted_store.kdf)?;

    // Decrypt each shard using the pre-derived key (no Argon2id per shard)
    let mut decrypted_shard_hexes = Vec::new();
    for encrypted_shard in &encrypted_store.encrypted_shards {
        let hex = decrypt_shard_with_key(encrypted_shard, &derived_key)?;
        decrypted_shard_hexes.push((encrypted_shard.index, hex));
    }

    syscall::debug(&format!(
        "IdentityService: Successfully decrypted {} stored shards",
        decrypted_shard_hexes.len()
    ));

    Ok(decrypted_shard_hexes)
}

/// Collect and validate all shards (1 external + 2 decrypted).
///
/// Validates that:
/// - External shard index is in the expected list
/// - All 3 shard indices are unique
/// - All shards can be parsed as valid NeuralShards
///
/// # Arguments
/// * `external_shard` - The external shard provided by the user
/// * `decrypted_shard_hexes` - Decrypted shards from `decrypt_shards_with_password`
/// * `encrypted_store` - The encrypted shard store (for validation)
///
/// # Returns
/// * `Ok(Vec<ZidNeuralShard>)` - All 3 shards in zid-crypto format
/// * `Err(KeyError)` - If validation fails or shards are malformed
pub fn collect_and_validate_shards(
    external_shard: &NeuralShard,
    decrypted_shard_hexes: &[(u8, String)],
    encrypted_store: &EncryptedShardStore,
) -> Result<Vec<ZidNeuralShard>, KeyError> {
    // Validate external shard index is expected
    if !encrypted_store
        .external_shard_indices
        .contains(&external_shard.index)
    {
        syscall::debug(&format!(
            "IdentityService: Invalid external shard index {}. Expected one of {:?}",
            external_shard.index, encrypted_store.external_shard_indices
        ));
        return Err(KeyError::InvalidShard(format!(
            "Invalid shard index {}. Your backup shards have indices {:?}. Please check your shard and enter the correct index.",
            external_shard.index, encrypted_store.external_shard_indices
        )));
    }

    // Collect and validate shard indices are unique
    let mut shard_indices = Vec::new();
    shard_indices.push(external_shard.index);
    shard_indices.extend(encrypted_store.encrypted_shards.iter().map(|s| s.index));
    shard_indices.sort_unstable();
    shard_indices.dedup();
    if shard_indices.len() != 3 {
        return Err(KeyError::InvalidShard(
            "Shard indices must be unique (3 total)".into(),
        ));
    }

    // Convert all shards to zid-crypto format
    let mut all_shards = Vec::new();

    // Add external shard
    let external = ZidNeuralShard::from_hex(&external_shard.hex).map_err(|e| {
        syscall::debug(&format!(
            "IdentityService: Invalid external shard format: {:?}",
            e
        ));
        KeyError::InvalidShard(format!("Invalid external shard format: {:?}", e))
    })?;
    all_shards.push(external);

    // Add decrypted shards
    for (_idx, hex) in decrypted_shard_hexes {
        let shard = ZidNeuralShard::from_hex(hex).map_err(|e| {
            syscall::debug(&format!(
                "IdentityService: Invalid decrypted shard format: {:?}",
                e
            ));
            KeyError::InvalidShard(format!("Invalid decrypted shard format: {:?}", e))
        })?;
        all_shards.push(shard);
    }

    syscall::debug(&format!(
        "IdentityService: Total shards for reconstruction: {}",
        all_shards.len()
    ));

    Ok(all_shards)
}

/// Reconstruct Neural Key from shards with identity verification.
///
/// This wraps `combine_shards_verified` to provide consistent logging.
///
/// # Arguments
/// * `all_shards` - At least 3 shards from `collect_and_validate_shards`
/// * `user_id` - The user ID for identity key derivation
/// * `stored_identity_pubkey` - The stored identity public key for verification
///
/// # Returns
/// * `Ok(NeuralKey)` - The reconstructed and verified Neural Key
/// * `Err(KeyError)` - If reconstruction or verification fails
pub fn reconstruct_neural_key(
    all_shards: &[ZidNeuralShard],
    user_id: u128,
    stored_identity_pubkey: &[u8; 32],
) -> Result<NeuralKey, KeyError> {
    match combine_shards_verified(all_shards, user_id, stored_identity_pubkey) {
        Ok(key) => {
            syscall::debug(
                "IdentityService: Neural Key reconstructed and verified against stored identity",
            );
            Ok(key)
        }
        Err(e) => {
            syscall::debug(&format!(
                "IdentityService: Neural Key verification failed: {:?}",
                e
            ));
            Err(e)
        }
    }
}
