//! Keystore exists response dispatch

extern crate alloc;

use alloc::format;
use alloc::string::String;

use crate::services::identity::handlers::keys;
use crate::services::identity::pending::PendingKeystoreOp;
use crate::services::identity::{response, IdentityService};
use zos_apps::syscall;
use zos_apps::AppError;
use zos_identity::KeyError;

/// Dispatch keystore exists result to appropriate handler based on pending operation type.
pub fn dispatch_keystore_exists_result(
    service: &mut IdentityService,
    op: PendingKeystoreOp,
    result: Result<bool, String>,
) -> Result<(), AppError> {
    match op {
        PendingKeystoreOp::CheckKeyExists { ctx, user_id, password } => match result {
            Ok(exists) => keys::continue_generate_after_exists_check(
                service,
                ctx.client_pid,
                user_id,
                exists,
                password,
                ctx.cap_slots,
            ),
            Err(e) => {
                syscall::debug(&format!(
                    "IdentityService: Keystore exists check failed for key file: {}",
                    e
                ));
                response::send_neural_key_error(
                    ctx.client_pid,
                    &ctx.cap_slots,
                    KeyError::StorageError(format!("Key exists check failed: {}", e)),
                )
            }
        },
        // Operations that should NOT receive an exists response
        PendingKeystoreOp::WriteKeyStore { ctx, .. }
        | PendingKeystoreOp::WriteEncryptedShards { ctx, .. }
        | PendingKeystoreOp::GetIdentityKey { ctx }
        | PendingKeystoreOp::ReadIdentityForRecovery { ctx, .. }
        | PendingKeystoreOp::WriteRecoveredKeyStore { ctx, .. }
        | PendingKeystoreOp::ReadIdentityForMachine { ctx, .. }
        | PendingKeystoreOp::ReadEncryptedShardsForMachine { ctx, .. }
        | PendingKeystoreOp::WriteMachineKey { ctx, .. }
        | PendingKeystoreOp::ListMachineKeys { ctx, .. }
        | PendingKeystoreOp::ListMachineKeysForZidLogin { ctx, .. }
        | PendingKeystoreOp::ListMachineKeysForZidEnroll { ctx, .. }
        | PendingKeystoreOp::ReadMachineKey { ctx, .. }
        | PendingKeystoreOp::DeleteMachineKey { ctx, .. }
        | PendingKeystoreOp::DeleteIdentityKeyAfterShardFailure { ctx, .. }
        | PendingKeystoreOp::ReadMachineForRotate { ctx, .. }
        | PendingKeystoreOp::WriteRotatedMachineKey { ctx, .. }
        | PendingKeystoreOp::ReadSingleMachineKey { ctx }
        | PendingKeystoreOp::ReadMachineKeyForZidLogin { ctx, .. }
        | PendingKeystoreOp::ReadMachineKeyForZidEnroll { ctx, .. }
        | PendingKeystoreOp::ReadIdentityForMachineEnroll { ctx, .. }
        | PendingKeystoreOp::ReadEncryptedShardsForMachineEnroll { ctx, .. }
        | PendingKeystoreOp::WriteMachineKeyForEnroll { ctx, .. } => {
            syscall::debug(&format!(
                "IdentityService: STATE_MACHINE_ERROR - unexpected keystore exists result for non-exists op, client_pid={}",
                ctx.client_pid
            ));
            Err(AppError::Internal(
                "State machine error: unexpected keystore exists result for non-exists operation".into(),
            ))
        }
    }
}
