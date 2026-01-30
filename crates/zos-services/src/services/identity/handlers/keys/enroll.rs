//! Combined Machine Key + ZID Enrollment Operations

extern crate alloc;

use alloc::format;
use alloc::vec::Vec;

use crate::services::identity::pending::{PendingKeystoreOp, RequestContext};
use crate::services::identity::response;
use crate::services::identity::{check_user_authorization, log_denial, AuthResult, IdentityService};
use zos_identity::crypto::{
    derive_identity_signing_keypair, derive_machine_encryption_seed, derive_machine_seed,
    derive_machine_signing_seed, KeyScheme as ZidKeyScheme, MachineKeyPair, NeuralKey,
    ZidMachineKeyCapabilities,
};
use super::shared::{
    collect_and_validate_shards, decrypt_shards_with_password, reconstruct_neural_key,
};
use zos_apps::syscall;
use zos_apps::{AppError, Message};
use zos_identity::ipc::CreateMachineKeyAndEnrollRequest;
use zos_identity::keystore::{EncryptedShardStore, KeyScheme, LocalKeyStore, MachineKeyRecord};
use uuid::Uuid;

/// Handle combined machine key creation and ZID enrollment.
///
/// This endpoint solves the signature mismatch problem by:
/// 1. Reconstructing the Neural Key from shards + password
/// 2. Deriving the machine keypair canonically
/// 3. Storing the machine key with SK seeds
/// 4. Enrolling with ZID using the SAME derived keypair
///
/// This ensures the keypair used for local storage matches the one registered with ZID.
pub fn handle_create_machine_key_and_enroll(
    service: &mut IdentityService,
    msg: &Message,
) -> Result<(), AppError> {
    syscall::debug("IdentityService: Handling create machine key AND enroll request");

    // Rule 1: Parse request - return InvalidRequest on parse failure
    let request: CreateMachineKeyAndEnrollRequest = match serde_json::from_slice(&msg.data) {
        Ok(r) => r,
        Err(e) => {
            syscall::debug(&format!("IdentityService: Failed to parse request: {}", e));
            return response::send_create_machine_key_and_enroll_error(
                msg.from_pid,
                &msg.cap_slots,
                zos_identity::error::ZidError::InvalidRequest(format!("JSON parse error: {}", e)),
            );
        }
    };

    // Rule 4: Authorization check (FAIL-CLOSED)
    if check_user_authorization(msg.from_pid, request.user_id) == AuthResult::Denied {
        log_denial("create_machine_key_and_enroll", msg.from_pid, request.user_id);
        return response::send_create_machine_key_and_enroll_error(
            msg.from_pid,
            &msg.cap_slots,
            zos_identity::error::ZidError::Unauthorized,
        );
    }

    // Read the LocalKeyStore to get the stored identity public key for verification
    let key_path = LocalKeyStore::storage_path(request.user_id);
    syscall::debug(&format!(
        "IdentityService: CreateMachineKeyAndEnroll - reading identity from: {}",
        key_path
    ));
    let ctx = RequestContext::new(msg.from_pid, msg.cap_slots.clone());
    // Invariant 32: /keys/ paths use Keystore IPC, not VFS
    service.start_keystore_read(
        &key_path,
        PendingKeystoreOp::ReadIdentityForMachineEnroll { ctx, request },
    )
}

/// Continue combined machine key + enroll after reading encrypted shards from keystore.
///
/// This function:
/// 1. Decrypts the 2 stored shards using the password
/// 2. Combines with the 1 external shard (total 3)
/// 3. Reconstructs the Neural Key
/// 4. Verifies against stored identity public key
/// 5. Derives machine keypair (with SK seeds for enrollment signing)
/// 6. Stores machine key, then chains to ZID enrollment
///
/// # Arguments
/// * `derivation_user_id` - The user_id that was used to derive the identity signing keypair.
///   This may differ from `request.user_id` if the user_id was derived from the pubkey.
///   Verification must use this value to re-derive and compare the pubkey.
pub fn continue_create_machine_enroll_after_shards_read(
    service: &mut IdentityService,
    client_pid: u32,
    request: CreateMachineKeyAndEnrollRequest,
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
            syscall::debug(&format!("IdentityService: Shard decryption failed in combined flow: {:?}", e));
            return response::send_create_machine_key_and_enroll_error(
                ctx.client_pid,
                &ctx.cap_slots,
                zos_identity::error::ZidError::AuthenticationFailed,
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
            return response::send_create_machine_key_and_enroll_error(
                ctx.client_pid,
                &ctx.cap_slots,
                zos_identity::error::ZidError::InvalidRequest(format!("{:?}", e)),
            );
        }
    };
    
    // Step 3: Reconstruct and verify Neural Key
    // IMPORTANT: Use derivation_user_id (from key_store.user_id), not request.user_id
    let neural_key = match reconstruct_neural_key(&all_shards, derivation_user_id, &stored_identity_pubkey) {
        Ok(key) => {
            syscall::debug("IdentityService: Neural Key reconstructed for combined machine key + enroll");
            key
        }
        Err(e) => {
            syscall::debug(&format!(
                "IdentityService: Neural Key verification failed in combined flow: {:?} (derivation_user_id={:032x})",
                e, derivation_user_id
            ));
            return response::send_create_machine_key_and_enroll_error(
                ctx.client_pid,
                &ctx.cap_slots,
                zos_identity::error::ZidError::AuthenticationFailed,
            );
        }
    };
    
    // Step 4: Derive identity and machine keys
    let (identity_signing_public_key, identity_signing_sk, machine_id, machine_signing_sk, machine_encryption_sk, machine_keypair) = 
        derive_keys_for_enroll(&neural_key, request.user_id, &request.key_scheme, &ctx)?;
    
    // Step 5: Build and store machine record
    build_and_store_machine_record_for_enroll(
        service,
        ctx,
        request,
        machine_id,
        &machine_keypair,
        identity_signing_public_key,
        identity_signing_sk,
        machine_signing_sk,
        machine_encryption_sk,
    )
}

// ============================================================================
// Helper functions for continue_create_machine_enroll_after_shards_read
// ============================================================================

/// Derive identity and machine keys for enrollment
#[allow(clippy::type_complexity)]
fn derive_keys_for_enroll(
    neural_key: &NeuralKey,
    user_id: u128,
    key_scheme: &KeyScheme,
    ctx: &RequestContext,
) -> Result<([u8; 32], [u8; 32], u128, [u8; 32], [u8; 32], MachineKeyPair), AppError> {
    // Generate machine ID
    let machine_id_bytes = match NeuralKey::generate() {
        Ok(key) => {
            let bytes = key.as_bytes();
            [
                bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
                bytes[8], bytes[9], bytes[10], bytes[11], bytes[12], bytes[13], bytes[14], bytes[15],
            ]
        }
        Err(e) => {
            response::send_create_machine_key_and_enroll_error(
                ctx.client_pid,
                &ctx.cap_slots,
                zos_identity::error::ZidError::NetworkError(format!("Machine ID generation failed: {:?}", e)),
            )?;
            return Err(AppError::Internal("Machine ID generation failed".into()));
        }
    };
    let machine_id = u128::from_le_bytes(machine_id_bytes);

    // Create UUIDs for derivation
    let identity_id = Uuid::from_u128(user_id);
    let machine_uuid = Uuid::from_u128(machine_id);

    // Derive identity signing keypair (needed for ZID enrollment signature)
    let (identity_signing_public_key, identity_keypair) =
        match derive_identity_signing_keypair(neural_key, &identity_id) {
            Ok(keypair) => keypair,
            Err(e) => {
                response::send_create_machine_key_and_enroll_error(
                    ctx.client_pid,
                    &ctx.cap_slots,
                    zos_identity::error::ZidError::NetworkError(format!("Identity key derivation failed: {:?}", e)),
                )?;
                return Err(AppError::Internal("Identity key derivation failed".into()));
            }
        };
    
    // Extract identity signing seed for ZID enrollment authorization signature
    let identity_signing_sk = identity_keypair.seed_bytes();

    // Convert capabilities and key scheme
    let zid_capabilities = ZidMachineKeyCapabilities::FULL_DEVICE;
    let zid_scheme = match key_scheme {
        KeyScheme::Classical => ZidKeyScheme::Classical,
        KeyScheme::PqHybrid => ZidKeyScheme::PqHybrid,
    };

    // Derive the seeds first so we can store them
    // Step 1: Derive machine seed from Neural Key
    let machine_seed = match derive_machine_seed(neural_key, &identity_id, &machine_uuid, 1) {
        Ok(seed) => seed,
        Err(e) => {
            response::send_create_machine_key_and_enroll_error(
                ctx.client_pid,
                &ctx.cap_slots,
                zos_identity::error::ZidError::NetworkError(format!("Machine seed derivation failed: {:?}", e)),
            )?;
            return Err(AppError::Internal("Machine seed derivation failed".into()));
        }
    };

    // Step 2: Derive signing seed from machine seed
    let machine_signing_sk = match derive_machine_signing_seed(&machine_seed, &machine_uuid) {
        Ok(seed) => *seed,
        Err(e) => {
            response::send_create_machine_key_and_enroll_error(
                ctx.client_pid,
                &ctx.cap_slots,
                zos_identity::error::ZidError::NetworkError(format!("Signing seed derivation failed: {:?}", e)),
            )?;
            return Err(AppError::Internal("Signing seed derivation failed".into()));
        }
    };

    // Step 3: Derive encryption seed from machine seed
    let machine_encryption_sk = match derive_machine_encryption_seed(&machine_seed, &machine_uuid) {
        Ok(seed) => *seed,
        Err(e) => {
            response::send_create_machine_key_and_enroll_error(
                ctx.client_pid,
                &ctx.cap_slots,
                zos_identity::error::ZidError::NetworkError(format!("Encryption seed derivation failed: {:?}", e)),
            )?;
            return Err(AppError::Internal("Encryption seed derivation failed".into()));
        }
    };

    // Step 4: Create machine keypair from the derived seeds
    let machine_keypair = match MachineKeyPair::from_seeds_with_scheme(
        &machine_signing_sk,
        &machine_encryption_sk,
        None, // No PQ signing seed in WASM
        None, // No PQ encryption seed in WASM
        zid_capabilities,
        zid_scheme,
    ) {
        Ok(keypair) => keypair,
        Err(e) => {
            response::send_create_machine_key_and_enroll_error(
                ctx.client_pid,
                &ctx.cap_slots,
                zos_identity::error::ZidError::NetworkError(format!("Machine keypair creation failed: {:?}", e)),
            )?;
            return Err(AppError::Internal("Machine keypair creation failed".into()));
        }
    };

    syscall::debug(&format!(
        "IdentityService: Derived machine key {:032x} for combined flow",
        machine_id
    ));

    Ok((identity_signing_public_key, identity_signing_sk, machine_id, machine_signing_sk, machine_encryption_sk, machine_keypair))
}

/// Build machine record and initiate storage for enrollment
#[allow(clippy::too_many_arguments)]
fn build_and_store_machine_record_for_enroll(
    service: &mut IdentityService,
    ctx: RequestContext,
    request: CreateMachineKeyAndEnrollRequest,
    machine_id: u128,
    machine_keypair: &MachineKeyPair,
    identity_signing_public_key: [u8; 32],
    identity_signing_sk: [u8; 32],
    machine_signing_sk: [u8; 32],
    machine_encryption_sk: [u8; 32],
) -> Result<(), AppError> {
    // Extract public keys
    let signing_key = machine_keypair.signing_public_key();
    let encryption_key = machine_keypair.encryption_public_key();
    let now = syscall::get_wallclock();

    // Create machine key record WITH SK seeds (needed for ZID enrollment signing)
    let record = MachineKeyRecord {
        machine_id,
        signing_public_key: signing_key,
        encryption_public_key: encryption_key,
        signing_sk: Some(machine_signing_sk),
        encryption_sk: Some(machine_encryption_sk),
        authorized_at: now,
        authorized_by: request.user_id,
        capabilities: request.capabilities,
        machine_name: request.machine_name,
        last_seen_at: now,
        epoch: 1,
        key_scheme: request.key_scheme,
        pq_signing_public_key: None,
        pq_encryption_public_key: None,
    };

    // Store machine key first, then chain to ZID enrollment
    let machine_path = MachineKeyRecord::storage_path(request.user_id, machine_id);
    match serde_json::to_vec(&record) {
        Ok(json_bytes) => service.start_keystore_write(
            &machine_path,
            &json_bytes,
            PendingKeystoreOp::WriteMachineKeyForEnroll {
                ctx,
                user_id: request.user_id,
                record,
                json_bytes: json_bytes.clone(),
                zid_endpoint: request.zid_endpoint,
                identity_signing_public_key,
                identity_signing_sk,
                machine_signing_sk,
                machine_encryption_sk,
            },
        ),
        Err(e) => response::send_create_machine_key_and_enroll_error(
            ctx.client_pid,
            &ctx.cap_slots,
            zos_identity::error::ZidError::NetworkError(format!("Serialization failed: {}", e)),
        ),
    }
}
