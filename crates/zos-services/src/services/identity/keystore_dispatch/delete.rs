//! Keystore delete response dispatch

extern crate alloc;

use alloc::format;
use alloc::string::String;

use crate::services::identity::pending::PendingKeystoreOp;
use crate::services::identity::{response, IdentityService};
use zos_apps::syscall;
use zos_apps::AppError;
use zos_identity::KeyError;

/// Dispatch keystore delete result to appropriate handler based on pending operation type.
pub fn dispatch_keystore_delete_result(
    _service: &mut IdentityService,
    op: PendingKeystoreOp,
    result: Result<(), String>,
) -> Result<(), AppError> {
    match op {
        PendingKeystoreOp::DeleteMachineKey { ctx, .. } => {
            if result.is_ok() {
                syscall::debug("IdentityService: Machine key deleted successfully via Keystore");
                response::send_revoke_machine_key_success(ctx.client_pid, &ctx.cap_slots)
            } else {
                response::send_revoke_machine_key_error(
                    ctx.client_pid,
                    &ctx.cap_slots,
                    KeyError::MachineKeyNotFound,
                )
            }
        }
        PendingKeystoreOp::DeleteIdentityKeyAfterShardFailure { ctx: _, user_id } => {
            if result.is_ok() {
                syscall::debug(&format!(
                    "IdentityService: Rolled back identity key store for user {:032x}",
                    user_id
                ));
            } else {
                syscall::debug(&format!(
                    "IdentityService: Failed to roll back identity key store for user {:032x}",
                    user_id
                ));
            }
            Ok(())
        }
        // Operations that should NOT receive a delete response
        PendingKeystoreOp::CheckKeyExists { ctx, .. }
        | PendingKeystoreOp::WriteKeyStore { ctx, .. }
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
        | PendingKeystoreOp::ReadMachineForRotate { ctx, .. }
        | PendingKeystoreOp::WriteRotatedMachineKey { ctx, .. }
        | PendingKeystoreOp::ReadSingleMachineKey { ctx }
        | PendingKeystoreOp::ReadMachineKeyForZidLogin { ctx, .. }
        | PendingKeystoreOp::ReadMachineKeyForZidEnroll { ctx, .. }
        | PendingKeystoreOp::ReadIdentityForMachineEnroll { ctx, .. }
        | PendingKeystoreOp::ReadEncryptedShardsForMachineEnroll { ctx, .. }
        | PendingKeystoreOp::WriteMachineKeyForEnroll { ctx, .. } => {
            syscall::debug(&format!(
                "IdentityService: STATE_MACHINE_ERROR - unexpected keystore delete result for non-delete op, client_pid={}",
                ctx.client_pid
            ));
            Err(AppError::Internal(
                "State machine error: unexpected keystore delete result for non-delete operation".into(),
            ))
        }
    }
}
