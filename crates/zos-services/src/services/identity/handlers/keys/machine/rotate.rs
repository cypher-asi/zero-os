//! Machine key rotation operations

extern crate alloc;

use alloc::format;
use alloc::vec::Vec;

use crate::services::identity::pending::{PendingKeystoreOp, RequestContext};
use crate::services::identity::response;
use crate::services::identity::{check_user_authorization, log_denial, AuthResult, IdentityService};
use zos_identity::crypto::{
    KeyScheme as ZidKeyScheme, MachineKeyPair, NeuralKey, ZidMachineKeyCapabilities,
};
use zos_apps::syscall;
use zos_apps::{AppError, Message};
use zos_identity::ipc::RotateMachineKeyRequest;
use zos_identity::keystore::{KeyScheme, MachineKeyRecord};
use zos_identity::KeyError;

pub fn handle_rotate_machine_key(
    service: &mut IdentityService,
    msg: &Message,
) -> Result<(), AppError> {
    // Rule 1: Parse request - return InvalidRequest on parse failure
    let request: RotateMachineKeyRequest = match serde_json::from_slice(&msg.data) {
        Ok(r) => r,
        Err(e) => {
            syscall::debug(&format!("IdentityService: Failed to parse request: {}", e));
            return response::send_rotate_machine_key_error(
                msg.from_pid,
                &msg.cap_slots,
                KeyError::InvalidRequest(format!("JSON parse error: {}", e)),
            );
        }
    };

    // Rule 4: Authorization check (FAIL-CLOSED)
    if check_user_authorization(msg.from_pid, request.user_id) == AuthResult::Denied {
        log_denial("rotate_machine_key", msg.from_pid, request.user_id);
        return response::send_rotate_machine_key_error(
            msg.from_pid,
            &msg.cap_slots,
            KeyError::Unauthorized,
        );
    }

    let machine_path = MachineKeyRecord::storage_path(request.user_id, request.machine_id);
    let ctx = RequestContext::new(msg.from_pid, msg.cap_slots.clone());
    // Invariant 32: /keys/ paths use Keystore IPC, not VFS
    service.start_keystore_read(
        &machine_path,
        PendingKeystoreOp::ReadMachineForRotate {
            ctx,
            user_id: request.user_id,
            machine_id: request.machine_id,
        },
    )
}

pub fn continue_rotate_after_read(
    service: &mut IdentityService,
    client_pid: u32,
    user_id: u128,
    machine_id: u128,
    data: &[u8],
    cap_slots: Vec<u32>,
) -> Result<(), AppError> {
    let ctx = RequestContext::new(client_pid, cap_slots);
    
    let mut record: MachineKeyRecord = match serde_json::from_slice(data) {
        Ok(r) => r,
        Err(e) => {
            return response::send_rotate_machine_key_error(
                ctx.client_pid,
                &ctx.cap_slots,
                KeyError::StorageError(format!("Parse failed: {}", e)),
            )
        }
    };

    // Generate new keys and update record
    let machine_keypair = generate_rotated_keypair(&record, &ctx)?;
    
    // Update record with new keys
    update_record_with_rotated_keys(&mut record, &machine_keypair, machine_id);

    // Store updated record
    store_rotated_record(service, ctx, user_id, machine_id, record)
}

// ============================================================================
// Helper functions for continue_rotate_after_read
// ============================================================================

/// Generate new keypair for rotation
fn generate_rotated_keypair(
    record: &MachineKeyRecord,
    ctx: &RequestContext,
) -> Result<MachineKeyPair, AppError> {
    // Generate new secure random seeds for key rotation
    syscall::debug("IdentityService: Generating signing seed for key rotation");
    let signing_sk = match NeuralKey::generate() {
        Ok(key) => {
            let bytes = *key.as_bytes();
            let all_zeros = bytes.iter().all(|&b| b == 0);
            if all_zeros {
                syscall::debug("IdentityService: WARNING - signing seed returned all zeros!");
            }
            bytes
        }
        Err(e) => {
            syscall::debug(&format!(
                "IdentityService: CRITICAL - Signing seed generation FAILED! Error: {:?}",
                e
            ));
            response::send_rotate_machine_key_error(
                ctx.client_pid,
                &ctx.cap_slots,
                KeyError::CryptoError("Failed to generate signing seed".into()),
            )?;
            return Err(AppError::Internal("Signing seed generation failed".into()));
        }
    };

    syscall::debug("IdentityService: Generating encryption seed for key rotation");
    let encryption_sk = match NeuralKey::generate() {
        Ok(key) => {
            let bytes = *key.as_bytes();
            let all_zeros = bytes.iter().all(|&b| b == 0);
            if all_zeros {
                syscall::debug("IdentityService: WARNING - encryption seed returned all zeros!");
            }
            bytes
        }
        Err(e) => {
            syscall::debug(&format!(
                "IdentityService: CRITICAL - Encryption seed generation FAILED! Error: {:?}",
                e
            ));
            response::send_rotate_machine_key_error(
                ctx.client_pid,
                &ctx.cap_slots,
                KeyError::CryptoError("Failed to generate encryption seed".into()),
            )?;
            return Err(AppError::Internal("Encryption seed generation failed".into()));
        }
    };

    // Convert capabilities and key scheme to zid-crypto format
    let zid_capabilities = ZidMachineKeyCapabilities::FULL_DEVICE;
    let zid_scheme = match record.key_scheme {
        KeyScheme::Classical => ZidKeyScheme::Classical,
        KeyScheme::PqHybrid => ZidKeyScheme::PqHybrid,
    };

    // Create new machine keypair using zid-crypto
    match MachineKeyPair::from_seeds_with_scheme(
        &signing_sk,
        &encryption_sk,
        None, // No PQ seeds for now (WASM limitation)
        None, // No PQ seeds for now
        zid_capabilities,
        zid_scheme,
    ) {
        Ok(keypair) => Ok(keypair),
        Err(e) => {
            response::send_rotate_machine_key_error(
                ctx.client_pid,
                &ctx.cap_slots,
                KeyError::CryptoError(format!("Machine keypair rotation failed: {:?}", e)),
            )?;
            Err(AppError::Internal("Machine keypair rotation failed".into()))
        }
    }
}

/// Update machine record with rotated keys
fn update_record_with_rotated_keys(
    record: &mut MachineKeyRecord,
    machine_keypair: &MachineKeyPair,
    machine_id: u128,
) {
    record.signing_public_key = machine_keypair.signing_public_key();
    record.encryption_public_key = machine_keypair.encryption_public_key();
    record.epoch += 1;
    record.last_seen_at = syscall::get_wallclock();

    // Clear PQ keys if in PQ mode (not supported in WASM yet)
    if record.key_scheme == KeyScheme::PqHybrid {
        record.pq_signing_public_key = None;
        record.pq_encryption_public_key = None;

        syscall::debug(&format!(
            "IdentityService: Rotated keys for machine {:032x} (epoch {}), PQ mode not yet supported",
            machine_id, record.epoch
        ));
    }
}

/// Store the rotated machine record
fn store_rotated_record(
    service: &mut IdentityService,
    ctx: RequestContext,
    user_id: u128,
    machine_id: u128,
    record: MachineKeyRecord,
) -> Result<(), AppError> {
    let machine_path = MachineKeyRecord::storage_path(user_id, machine_id);
    match serde_json::to_vec(&record) {
        // Invariant 32: /keys/ paths use Keystore IPC, not VFS
        Ok(json_bytes) => service.start_keystore_write(
            &machine_path,
            &json_bytes,
            PendingKeystoreOp::WriteRotatedMachineKey {
                ctx,
                user_id,
                record,
                json_bytes: json_bytes.clone(),
            },
        ),
        Err(e) => response::send_rotate_machine_key_error(
            ctx.client_pid,
            &ctx.cap_slots,
            KeyError::StorageError(format!("Serialization failed: {}", e)),
        ),
    }
}
