//! Neural key recovery handlers

extern crate alloc;

use alloc::format;
use alloc::vec::Vec;

use crate::services::identity::utils::bytes_to_hex;
use crate::services::identity::pending::{PendingKeystoreOp, RequestContext};
use crate::services::identity::response;
use crate::services::identity::{check_user_authorization, log_denial, AuthResult, IdentityService};
use zos_identity::crypto::{
    combine_shards_verified, derive_identity_signing_keypair, split_neural_key,
    ZidNeuralShard,
};
use zos_apps::syscall;
use zos_apps::{AppError, Message};
use zos_identity::ipc::{
    GetIdentityKeyRequest, NeuralKeyGenerated, NeuralShard, PublicIdentifiers,
    RecoverNeuralKeyRequest,
};
use zos_identity::keystore::LocalKeyStore;
use zos_identity::KeyError;
use uuid::Uuid;

pub fn handle_recover_neural_key(
    service: &mut IdentityService,
    msg: &Message,
) -> Result<(), AppError> {
    syscall::debug("IdentityService: Handling recover neural key request");

    // Rule 1: Parse request - return InvalidRequest on parse failure
    let request: RecoverNeuralKeyRequest = match serde_json::from_slice(&msg.data) {
        Ok(r) => r,
        Err(e) => {
            syscall::debug(&format!("IdentityService: Failed to parse request: {}", e));
            return response::send_recover_key_error(
                msg.from_pid,
                &msg.cap_slots,
                KeyError::InvalidRequest(format!("JSON parse error: {}", e)),
            );
        }
    };

    // Rule 4: Authorization check (FAIL-CLOSED)
    if check_user_authorization(msg.from_pid, request.user_id) == AuthResult::Denied {
        log_denial("recover_neural_key", msg.from_pid, request.user_id);
        return response::send_recover_key_error(
            msg.from_pid,
            &msg.cap_slots,
            KeyError::Unauthorized,
        );
    }

    if request.shards.len() < 3 {
        return response::send_recover_key_error(
            msg.from_pid,
            &msg.cap_slots,
            KeyError::InsufficientShards,
        );
    }

    // Convert IPC shards to zid-crypto format
    let zid_shards: Result<Vec<ZidNeuralShard>, _> = request
        .shards
        .iter()
        .map(|s| ZidNeuralShard::from_hex(&s.hex))
        .collect();

    let zid_shards = match zid_shards {
        Ok(shards) => shards,
        Err(e) => {
            return response::send_recover_key_error(
                msg.from_pid,
                &msg.cap_slots,
                KeyError::InvalidShard(format!("Invalid shard format: {:?}", e)),
            )
        }
    };

    // SECURITY: Read the existing LocalKeyStore to get the stored identity public key
    // for verification. This prevents attacks where arbitrary shards could be used
    // to reconstruct an unauthorized identity.
    let key_path = LocalKeyStore::storage_path(request.user_id);
    syscall::debug(&format!(
        "IdentityService: RecoverNeuralKey - reading existing identity from: {}",
        key_path
    ));
    let ctx = RequestContext::new(msg.from_pid, msg.cap_slots.clone());
    // Invariant 32: /keys/ paths use Keystore IPC, not VFS
    service.start_keystore_read(
        &key_path,
        PendingKeystoreOp::ReadIdentityForRecovery {
            ctx,
            user_id: request.user_id,
            zid_shards,
        },
    )
}

/// Continue neural key recovery after reading the existing identity for verification.
///
/// SECURITY: This function uses `combine_shards_verified()` to ensure the reconstructed
/// Neural Key matches the stored identity public key. This prevents attacks where
/// arbitrary shards could be used to derive unauthorized machine keys.
///
/// # Arguments
/// * `derivation_user_id` - The ORIGINAL user_id used during generation for cryptographic
///   key derivation. This is stored in LocalKeyStore.user_id.
/// * `storage_user_id` - The derived_user_id (hash of pubkey) used for storage paths.
///   This is what the client sends and what was used for the storage path during generation.
pub fn continue_recover_after_identity_read(
    service: &mut IdentityService,
    client_pid: u32,
    derivation_user_id: u128,
    storage_user_id: u128,
    zid_shards: Vec<ZidNeuralShard>,
    stored_identity_pubkey: [u8; 32],
    cap_slots: Vec<u32>,
) -> Result<(), AppError> {
    let ctx = RequestContext::new(client_pid, cap_slots);
    
    // SECURITY: Reconstruct Neural Key from shards WITH VERIFICATION against stored identity.
    // This ensures the provided shards actually belong to this user's Neural Key.
    // CRITICAL: Must use derivation_user_id (the ORIGINAL user_id) for verification,
    // as that's what was used to derive the identity keypair during generation.
    let neural_key = match combine_shards_verified(&zid_shards, derivation_user_id, &stored_identity_pubkey) {
        Ok(key) => key,
        Err(e) => {
            syscall::debug(&format!(
                "IdentityService: Neural Key recovery verification failed: {:?}",
                e
            ));
            return response::send_recover_key_error(
                ctx.client_pid,
                &ctx.cap_slots,
                e,
            );
        }
    };

    syscall::debug("IdentityService: Neural Key recovered and verified against stored identity");

    // Derive keys using proper zid-crypto functions
    // CRITICAL: Must use derivation_user_id (the ORIGINAL) for key derivation
    let temp_identity_id = Uuid::from_u128(derivation_user_id);
    // The _identity_keypair is intentionally unused here - we only need the public key
    // for verification and storage. The full keypair would only be needed for signing
    // operations, which are performed elsewhere using the shards.
    let (identity_signing, _identity_keypair) =
        match derive_identity_signing_keypair(&neural_key, &temp_identity_id) {
            Ok(keypair) => keypair,
            Err(e) => {
                return response::send_recover_key_error(
                    ctx.client_pid,
                    &ctx.cap_slots,
                    KeyError::CryptoError(format!(
                        "Identity key derivation failed during recovery: {:?}",
                        e
                    )),
                )
            }
        };
    // Machine signing/encryption are placeholders - actual machine keys are created
    // separately via CreateMachineKey which derives them from the Neural Key
    let machine_signing = [0u8; 32];
    let machine_encryption = [0u8; 32];

    // Split the recovered neural key into new shards for backup
    let new_zid_shards = match split_neural_key(&neural_key) {
        Ok(shards) => shards,
        Err(e) => {
            return response::send_recover_key_error(
                ctx.client_pid,
                &ctx.cap_slots,
                KeyError::CryptoError(format!("Shamir split failed: {:?}", e)),
            )
        }
    };

    let new_shards: Vec<NeuralShard> = new_zid_shards
        .iter()
        .enumerate()
        .map(|(i, shard)| NeuralShard {
            index: (i + 1) as u8,
            hex: shard.to_hex(),
        })
        .collect();

    let public_identifiers = PublicIdentifiers {
        identity_signing_pub_key: format!("0x{}", bytes_to_hex(&identity_signing)),
        machine_signing_pub_key: format!("0x{}", bytes_to_hex(&machine_signing)),
        machine_encryption_pub_key: format!("0x{}", bytes_to_hex(&machine_encryption)),
    };

    let created_at = syscall::get_wallclock();
    // Store the ORIGINAL derivation_user_id in LocalKeyStore - this is needed for
    // future cryptographic operations (machine key creation, verification, etc.)
    let key_store = LocalKeyStore::new(
        derivation_user_id,
        identity_signing,
        machine_signing,
        machine_encryption,
        created_at,
    );

    // The response user_id should be the storage_user_id (derived from pubkey),
    // which is what the client uses for all subsequent API calls
    syscall::debug(&format!(
        "IdentityService: Recovered key - storage_user_id {:032x}, derivation_user_id {:032x}",
        storage_user_id, derivation_user_id
    ));

    let result = NeuralKeyGenerated {
        user_id: storage_user_id, // Client uses this for subsequent calls
        public_identifiers,
        shards: new_shards,
        created_at,
    };

    // CRITICAL: Use storage_user_id for path - this is where the key was originally stored
    let key_path = LocalKeyStore::storage_path(storage_user_id);
    match serde_json::to_vec(&key_store) {
        // Invariant 32: /keys/ paths use Keystore IPC, not VFS
        Ok(json_bytes) => service.start_keystore_write(
            &key_path,
            &json_bytes,
            PendingKeystoreOp::WriteRecoveredKeyStore {
                ctx,
                user_id: storage_user_id,
                result,
                json_bytes: json_bytes.clone(),
            },
        ),
        Err(e) => response::send_recover_key_error(
            ctx.client_pid,
            &ctx.cap_slots,
            KeyError::StorageError(format!("Serialization failed: {}", e)),
        ),
    }
}

pub fn handle_get_identity_key(
    service: &mut IdentityService,
    msg: &Message,
) -> Result<(), AppError> {
    // Rule 1: Parse request - return InvalidRequest on parse failure
    let request: GetIdentityKeyRequest = match serde_json::from_slice(&msg.data) {
        Ok(r) => r,
        Err(e) => {
            syscall::debug(&format!("IdentityService: Failed to parse request: {}", e));
            return response::send_get_identity_key_error(
                msg.from_pid,
                &msg.cap_slots,
                KeyError::InvalidRequest(format!("JSON parse error: {}", e)),
            );
        }
    };

    // Rule 4: Authorization check (FAIL-CLOSED)
    if check_user_authorization(msg.from_pid, request.user_id) == AuthResult::Denied {
        log_denial("get_identity_key", msg.from_pid, request.user_id);
        return response::send_get_identity_key_error(
            msg.from_pid,
            &msg.cap_slots,
            KeyError::Unauthorized,
        );
    }

    let key_path = LocalKeyStore::storage_path(request.user_id);
    let ctx = RequestContext::new(msg.from_pid, msg.cap_slots.clone());
    // Invariant 32: /keys/ paths use Keystore IPC, not VFS
    service.start_keystore_read(
        &key_path,
        PendingKeystoreOp::GetIdentityKey { ctx },
    )
}
