//! Keystore write response dispatch

extern crate alloc;

use alloc::format;
use alloc::string::String;

use crate::services::identity::handlers::session;
use crate::services::identity::pending::{PendingKeystoreOp, PendingStorageOp};
use crate::services::identity::{response, IdentityService};
use zos_apps::syscall;
use zos_apps::AppError;
use zos_identity::keystore::{EncryptedShardStore, LocalKeyStore};
use zos_identity::KeyError;

/// Dispatch keystore write result to appropriate handler based on pending operation type.
pub fn dispatch_keystore_write_result(
    service: &mut IdentityService,
    op: PendingKeystoreOp,
    result: Result<(), String>,
) -> Result<(), AppError> {
    match op {
        PendingKeystoreOp::WriteKeyStore { ctx, user_id, result: key_result, encrypted_shards_json, .. } => {
            handle_write_key_store(service, ctx, user_id, key_result, encrypted_shards_json, result)
        }
        PendingKeystoreOp::WriteEncryptedShards { ctx, user_id, result: key_result, .. } => {
            handle_write_encrypted_shards(service, ctx, user_id, key_result, result)
        }
        PendingKeystoreOp::WriteRecoveredKeyStore { ctx, result: key_result, .. } => {
            handle_write_recovered_key_store(ctx, key_result, result)
        }
        PendingKeystoreOp::WriteMachineKey { ctx, record, .. } => {
            handle_write_machine_key(ctx, record, result)
        }
        PendingKeystoreOp::WriteRotatedMachineKey { ctx, record, .. } => {
            handle_write_rotated_machine_key(ctx, record, result)
        }
        PendingKeystoreOp::WriteMachineKeyForEnroll {
            ctx, user_id, record, zid_endpoint,
            identity_signing_public_key, identity_signing_sk,
            machine_signing_sk, machine_encryption_sk, ..
        } => {
            handle_write_machine_key_for_enroll(
                service, ctx, user_id, record, zid_endpoint,
                identity_signing_public_key, identity_signing_sk,
                machine_signing_sk, machine_encryption_sk, result
            )
        }
        // Operations that should NOT receive a write response
        PendingKeystoreOp::CheckKeyExists { ctx, .. }
        | PendingKeystoreOp::GetIdentityKey { ctx }
        | PendingKeystoreOp::ReadIdentityForRecovery { ctx, .. }
        | PendingKeystoreOp::ReadIdentityForMachine { ctx, .. }
        | PendingKeystoreOp::ReadEncryptedShardsForMachine { ctx, .. }
        | PendingKeystoreOp::ListMachineKeys { ctx, .. }
        | PendingKeystoreOp::ListMachineKeysForZidLogin { ctx, .. }
        | PendingKeystoreOp::ListMachineKeysForZidEnroll { ctx, .. }
        | PendingKeystoreOp::ReadMachineKey { ctx, .. }
        | PendingKeystoreOp::DeleteMachineKey { ctx, .. }
        | PendingKeystoreOp::DeleteIdentityKeyAfterShardFailure { ctx, .. }
        | PendingKeystoreOp::ReadMachineForRotate { ctx, .. }
        | PendingKeystoreOp::ReadSingleMachineKey { ctx }
        | PendingKeystoreOp::ReadMachineKeyForZidLogin { ctx, .. }
        | PendingKeystoreOp::ReadMachineKeyForZidEnroll { ctx, .. }
        | PendingKeystoreOp::ReadIdentityForMachineEnroll { ctx, .. }
        | PendingKeystoreOp::ReadEncryptedShardsForMachineEnroll { ctx, .. } => {
            syscall::debug(&format!(
                "IdentityService: STATE_MACHINE_ERROR - unexpected keystore write result for non-write op, client_pid={}",
                ctx.client_pid
            ));
            Err(AppError::Internal(
                "State machine error: unexpected keystore write result for non-write operation".into(),
            ))
        }
    }
}

fn handle_write_key_store(
    service: &mut IdentityService,
    ctx: crate::services::identity::pending::RequestContext,
    user_id: u128,
    key_result: zos_identity::ipc::NeuralKeyGenerated,
    encrypted_shards_json: alloc::vec::Vec<u8>,
    result: Result<(), String>,
) -> Result<(), AppError> {
    match result {
        Ok(()) => {
            syscall::debug("IdentityService: Neural key stored successfully via Keystore, now writing encrypted shards");
            // Chain to write encrypted shards
            let shards_path = EncryptedShardStore::storage_path(user_id);
            service.start_keystore_write(
                &shards_path,
                &encrypted_shards_json,
                PendingKeystoreOp::WriteEncryptedShards {
                    ctx,
                    user_id,
                    result: key_result,
                },
            )
        }
        Err(e) => {
            syscall::debug(&format!(
                "IdentityService: WriteKeyStore failed - op=write_neural_key, error={}",
                e
            ));
            response::send_neural_key_error(
                ctx.client_pid,
                &ctx.cap_slots,
                KeyError::StorageError(format!("Keystore write failed for neural key: {}", e)),
            )
        }
    }
}

fn handle_write_encrypted_shards(
    service: &mut IdentityService,
    ctx: crate::services::identity::pending::RequestContext,
    user_id: u128,
    key_result: zos_identity::ipc::NeuralKeyGenerated,
    result: Result<(), String>,
) -> Result<(), AppError> {
    match result {
        Ok(()) => {
            // Keystore writes complete. Now create the VFS directory for the derived user_id.
            syscall::debug(&format!(
                "IdentityService: Encrypted shards stored, creating VFS directory for derived user {}",
                user_id
            ));
            let identity_dir = alloc::format!("/home/{}/.zos/identity", user_id);
            service.start_vfs_mkdir(
                &identity_dir,
                true, // create_parents = true
                PendingStorageOp::CreateDerivedUserDirectory {
                    ctx,
                    derived_user_id: user_id,
                    result: key_result,
                },
            )
        }
        Err(e) => {
            syscall::debug(&format!(
                "IdentityService: WriteEncryptedShards failed - op=write_encrypted_shards, error={}",
                e
            ));
            let key_path = LocalKeyStore::storage_path(user_id);
            syscall::debug(&format!(
                "IdentityService: Rolling back identity key store at {}",
                key_path
            ));
            if let Err(err) = service.start_keystore_delete(
                &key_path,
                PendingKeystoreOp::DeleteIdentityKeyAfterShardFailure {
                    ctx: ctx.clone(),
                    user_id,
                },
            ) {
                syscall::debug(&format!(
                    "IdentityService: Failed to schedule rollback delete: {:?}",
                    err
                ));
            }
            response::send_neural_key_error(
                ctx.client_pid,
                &ctx.cap_slots,
                KeyError::StorageError(format!("Keystore write failed for encrypted shards: {}", e)),
            )
        }
    }
}

fn handle_write_recovered_key_store(
    ctx: crate::services::identity::pending::RequestContext,
    key_result: zos_identity::ipc::NeuralKeyGenerated,
    result: Result<(), String>,
) -> Result<(), AppError> {
    match result {
        Ok(()) => {
            syscall::debug("IdentityService: Recovered key stored successfully via Keystore");
            response::send_recover_key_success(ctx.client_pid, &ctx.cap_slots, key_result)
        }
        Err(e) => {
            syscall::debug(&format!(
                "IdentityService: WriteRecoveredKeyStore failed - op=recover_neural_key, error={}",
                e
            ));
            response::send_recover_key_error(
                ctx.client_pid,
                &ctx.cap_slots,
                KeyError::StorageError(format!("Keystore write failed for recovered key: {}", e)),
            )
        }
    }
}

fn handle_write_machine_key(
    ctx: crate::services::identity::pending::RequestContext,
    record: zos_identity::keystore::MachineKeyRecord,
    result: Result<(), String>,
) -> Result<(), AppError> {
    match result {
        Ok(()) => {
            syscall::debug(&format!(
                "IdentityService: Machine key {:032x} stored successfully via Keystore",
                record.machine_id
            ));
            response::send_create_machine_key_success(ctx.client_pid, &ctx.cap_slots, record)
        }
        Err(e) => {
            syscall::debug(&format!(
                "IdentityService: WriteMachineKey failed - op=create_machine_key, machine_id={:032x}, error={}",
                record.machine_id, e
            ));
            response::send_create_machine_key_error(
                ctx.client_pid,
                &ctx.cap_slots,
                KeyError::StorageError(format!("Keystore write failed for machine key: {}", e)),
            )
        }
    }
}

fn handle_write_rotated_machine_key(
    ctx: crate::services::identity::pending::RequestContext,
    record: zos_identity::keystore::MachineKeyRecord,
    result: Result<(), String>,
) -> Result<(), AppError> {
    match result {
        Ok(()) => {
            syscall::debug(&format!(
                "IdentityService: Rotated machine key {:032x} stored successfully via Keystore",
                record.machine_id
            ));
            response::send_rotate_machine_key_success(ctx.client_pid, &ctx.cap_slots, record)
        }
        Err(e) => {
            syscall::debug(&format!(
                "IdentityService: WriteRotatedMachineKey failed - op=rotate_machine_key, machine_id={:032x}, error={}",
                record.machine_id, e
            ));
            response::send_rotate_machine_key_error(
                ctx.client_pid,
                &ctx.cap_slots,
                KeyError::StorageError(format!("Keystore write failed for rotated key: {}", e)),
            )
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn handle_write_machine_key_for_enroll(
    service: &mut IdentityService,
    ctx: crate::services::identity::pending::RequestContext,
    user_id: u128,
    record: zos_identity::keystore::MachineKeyRecord,
    zid_endpoint: String,
    identity_signing_public_key: [u8; 32],
    identity_signing_sk: [u8; 32],
    machine_signing_sk: [u8; 32],
    machine_encryption_sk: [u8; 32],
    result: Result<(), String>,
) -> Result<(), AppError> {
    match result {
        Ok(()) => {
            syscall::debug(&format!(
                "IdentityService: Machine key {:032x} stored, now enrolling with ZID",
                record.machine_id
            ));
            // Chain to ZID enrollment with the stored machine key
            session::continue_combined_enroll_after_machine_write(
                service,
                ctx.client_pid,
                user_id,
                zid_endpoint,
                record,
                identity_signing_public_key,
                identity_signing_sk,
                machine_signing_sk,
                machine_encryption_sk,
                ctx.cap_slots,
            )
        }
        Err(e) => {
            syscall::debug(&format!(
                "IdentityService: WriteMachineKeyForEnroll failed - machine_id={:032x}, error={}",
                record.machine_id, e
            ));
            response::send_create_machine_key_and_enroll_error(
                ctx.client_pid,
                &ctx.cap_slots,
                zos_identity::error::ZidError::NetworkError(format!("Machine key storage failed: {}", e)),
            )
        }
    }
}
