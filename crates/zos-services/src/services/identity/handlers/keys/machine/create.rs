//! Machine key creation operations

extern crate alloc;

use alloc::format;
use alloc::vec::Vec;

use crate::services::identity::pending::{PendingKeystoreOp, RequestContext};
use crate::services::identity::response;
use crate::services::identity::{check_user_authorization, log_denial, AuthResult, IdentityService};
use zos_identity::crypto::{
    derive_machine_keypair_with_scheme, KeyScheme as ZidKeyScheme, MachineKeyPair,
    NeuralKey, ZidMachineKeyCapabilities,
};
use super::super::shared::{
    collect_and_validate_shards, decrypt_shards_with_password, reconstruct_neural_key,
};
use zos_apps::syscall;
use zos_apps::{AppError, Message};
use zos_identity::ipc::CreateMachineKeyRequest;
use zos_identity::keystore::{EncryptedShardStore, KeyScheme, LocalKeyStore, MachineKeyRecord};
use zos_identity::KeyError;
use uuid::Uuid;

pub fn handle_create_machine_key(
    service: &mut IdentityService,
    msg: &Message,
) -> Result<(), AppError> {
    // Rule 1: Parse request - return InvalidRequest on parse failure
    let request: CreateMachineKeyRequest = match serde_json::from_slice(&msg.data) {
        Ok(r) => r,
        Err(e) => {
            syscall::debug(&format!("IdentityService: Failed to parse request: {}", e));
            return response::send_create_machine_key_error(
                msg.from_pid,
                &msg.cap_slots,
                KeyError::InvalidRequest(format!("JSON parse error: {}", e)),
            );
        }
    };

    // Rule 4: Authorization check (FAIL-CLOSED)
    if check_user_authorization(msg.from_pid, request.user_id) == AuthResult::Denied {
        log_denial("create_machine_key", msg.from_pid, request.user_id);
        return response::send_create_machine_key_error(
            msg.from_pid,
            &msg.cap_slots,
            KeyError::Unauthorized,
        );
    }

    // Read the LocalKeyStore to get the stored identity public key for verification
    let key_path = LocalKeyStore::storage_path(request.user_id);
    syscall::debug(&format!(
        "IdentityService: CreateMachineKey - reading identity from: {}",
        key_path
    ));
    let ctx = RequestContext::new(msg.from_pid, msg.cap_slots.clone());
    // Invariant 32: /keys/ paths use Keystore IPC, not VFS
    service.start_keystore_read(
        &key_path,
        PendingKeystoreOp::ReadIdentityForMachine { ctx, request },
    )
}

/// Legacy function - now just a stub that should not be called directly.
/// Machine key creation now goes through continue_create_machine_after_shards_read.
pub fn continue_create_machine_after_identity_read(
    _service: &mut IdentityService,
    client_pid: u32,
    _request: CreateMachineKeyRequest,
    _stored_identity_pubkey: [u8; 32],
    cap_slots: Vec<u32>,
) -> Result<(), AppError> {
    // This should not be called directly anymore - see keystore_dispatch.rs
    // which now chains to ReadEncryptedShardsForMachine
    response::send_create_machine_key_error(
        client_pid,
        &cap_slots,
        KeyError::StorageError("Internal error: legacy path invoked".into()),
    )
}

/// Continue machine key creation after reading encrypted shards from keystore.
///
/// This function:
/// 1. Decrypts the 2 stored shards using the password
/// 2. Combines with the 1 external shard (total 3)
/// 3. Reconstructs the Neural Key
/// 4. Verifies against stored identity public key
/// 5. Derives machine keypair
///
/// # Arguments
/// * `derivation_user_id` - The user_id that was used to derive the identity signing keypair.
///   This may differ from `request.user_id` if the user_id was derived from the pubkey.
///   Verification must use this value to re-derive and compare the pubkey.
pub fn continue_create_machine_after_shards_read(
    service: &mut IdentityService,
    client_pid: u32,
    request: CreateMachineKeyRequest,
    stored_identity_pubkey: [u8; 32],
    derivation_user_id: u128,
    encrypted_store: EncryptedShardStore,
    cap_slots: Vec<u32>,
) -> Result<(), AppError> {
    let ctx = RequestContext::new(client_pid, cap_slots);

    // Step 1: Decrypt shards (derives key ONCE, decrypts each shard)
    let decrypted_shard_hexes = match decrypt_shards_with_password(&encrypted_store, &request.password) {
        Ok(hexes) => hexes,
        Err(e) => {
            syscall::debug(&format!("IdentityService: Shard decryption failed: {:?}", e));
            return response::send_create_machine_key_error(
                ctx.client_pid,
                &ctx.cap_slots,
                e,
            );
        }
    };
    
    // Step 2: Validate and collect all shards
    let all_shards = match collect_and_validate_shards(
        &request.external_shard,
        &decrypted_shard_hexes,
        &encrypted_store,
    ) {
        Ok(shards) => shards,
        Err(e) => {
            return response::send_create_machine_key_error(
                ctx.client_pid,
                &ctx.cap_slots,
                e,
            );
        }
    };
    
    // Step 3: Reconstruct and verify Neural Key
    // IMPORTANT: Use derivation_user_id (from key_store.user_id), not request.user_id
    // The identity keypair was derived using derivation_user_id during generation
    let neural_key = match reconstruct_neural_key(&all_shards, derivation_user_id, &stored_identity_pubkey) {
        Ok(key) => key,
        Err(e) => {
            syscall::debug(&format!(
                "IdentityService: Neural Key verification failed: {:?} (derivation_user_id={:032x})",
                e, derivation_user_id
            ));
            return response::send_create_machine_key_error(
                ctx.client_pid,
                &ctx.cap_slots,
                e,
            );
        }
    };
    
    // Step 4: Generate machine ID and derive keypair
    let (machine_id, machine_keypair) = derive_machine_keypair(
        &neural_key,
        request.user_id,
        &request.key_scheme,
        &ctx,
    )?;
    
    // Step 5: Build and store machine record
    build_and_store_machine_record(
        service,
        ctx,
        request,
        machine_id,
        &machine_keypair,
    )
}

// ============================================================================
// Helper functions for continue_create_machine_after_shards_read
// ============================================================================

/// Generate machine ID and derive keypair from Neural Key
fn derive_machine_keypair(
    neural_key: &NeuralKey,
    user_id: u128,
    key_scheme: &KeyScheme,
    ctx: &RequestContext,
) -> Result<(u128, MachineKeyPair), AppError> {
    // Generate machine ID using entropy
    syscall::debug("IdentityService: Generating machine ID via NeuralKey::generate()");
    let machine_id_bytes = match NeuralKey::generate() {
        Ok(key) => {
            let bytes = key.as_bytes();
            let all_zeros = bytes[..16].iter().all(|&b| b == 0);
            if all_zeros {
                syscall::debug("IdentityService: WARNING - machine ID entropy returned all zeros!");
            }
            [
                bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
                bytes[8], bytes[9], bytes[10], bytes[11], bytes[12], bytes[13], bytes[14],
                bytes[15],
            ]
        }
        Err(e) => {
            syscall::debug(&format!(
                "IdentityService: CRITICAL - Machine ID generation FAILED! Error: {:?}",
                e
            ));
            response::send_create_machine_key_error(
                ctx.client_pid,
                &ctx.cap_slots,
                KeyError::CryptoError("Failed to generate machine ID".into()),
            )?;
            return Err(AppError::Internal("Machine ID generation failed".into()));
        }
    };
    let machine_id = u128::from_le_bytes(machine_id_bytes);

    // Create UUIDs for derivation
    let identity_id = Uuid::from_u128(user_id);
    let machine_uuid = Uuid::from_u128(machine_id);

    // Convert capabilities to zid-crypto format
    let zid_capabilities = ZidMachineKeyCapabilities::FULL_DEVICE;

    // Convert key scheme
    let zid_scheme = match key_scheme {
        KeyScheme::Classical => ZidKeyScheme::Classical,
        KeyScheme::PqHybrid => ZidKeyScheme::PqHybrid,
    };

    // Derive machine keypair from Neural Key using zid-crypto
    let machine_keypair = match derive_machine_keypair_with_scheme(
        neural_key,
        &identity_id,
        &machine_uuid,
        1, // epoch
        zid_capabilities,
        zid_scheme,
    ) {
        Ok(keypair) => keypair,
        Err(e) => {
            syscall::debug(&format!(
                "IdentityService: Machine keypair derivation failed: {:?}",
                e
            ));
            response::send_create_machine_key_error(
                ctx.client_pid,
                &ctx.cap_slots,
                KeyError::CryptoError(format!("Machine keypair derivation failed: {:?}", e)),
            )?;
            return Err(AppError::Internal("Machine keypair derivation failed".into()));
        }
    };

    syscall::debug(&format!(
        "IdentityService: Derived machine key {:032x} from Neural Key",
        machine_id
    ));

    Ok((machine_id, machine_keypair))
}

/// Build machine record and initiate storage
fn build_and_store_machine_record(
    service: &mut IdentityService,
    ctx: RequestContext,
    request: CreateMachineKeyRequest,
    machine_id: u128,
    machine_keypair: &MachineKeyPair,
) -> Result<(), AppError> {
    // Extract public keys
    let signing_key = machine_keypair.signing_public_key();
    let encryption_key = machine_keypair.encryption_public_key();
    let now = syscall::get_wallclock();

    // Get PQ keys if available
    let (pq_signing_public_key, pq_encryption_public_key) = 
        if request.key_scheme == KeyScheme::PqHybrid {
            // For now, PQ keys are not available in WASM
            syscall::debug(&format!(
                "IdentityService: PQ-Hybrid requested for machine {:032x}, but not yet supported in WASM",
                machine_id
            ));
            (None, None)
        } else {
            (None, None)
        };

    let record = MachineKeyRecord {
        machine_id,
        signing_public_key: signing_key,
        encryption_public_key: encryption_key,
        signing_sk: None, // Seeds not stored - derived from Neural Key
        encryption_sk: None,
        authorized_at: now,
        authorized_by: request.user_id,
        capabilities: request.capabilities,
        machine_name: request.machine_name,
        last_seen_at: now,
        epoch: 1,
        key_scheme: request.key_scheme,
        pq_signing_public_key,
        pq_encryption_public_key,
    };

    let machine_path = MachineKeyRecord::storage_path(request.user_id, machine_id);
    match serde_json::to_vec(&record) {
        // Invariant 32: /keys/ paths use Keystore IPC, not VFS
        Ok(json_bytes) => service.start_keystore_write(
            &machine_path,
            &json_bytes,
            PendingKeystoreOp::WriteMachineKey {
                ctx,
                user_id: request.user_id,
                record,
                json_bytes: json_bytes.clone(),
            },
        ),
        Err(e) => response::send_create_machine_key_error(
            ctx.client_pid,
            &ctx.cap_slots,
            KeyError::StorageError(format!("Serialization failed: {}", e)),
        ),
    }
}
