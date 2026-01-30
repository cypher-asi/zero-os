//! Keystore list response dispatch

extern crate alloc;

use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

use crate::services::identity::pending::{PendingKeystoreOp, RequestContext};
use crate::services::identity::{response, IdentityService};
use zos_apps::syscall;
use zos_apps::AppError;

/// Dispatch keystore list result to appropriate handler based on pending operation type.
pub fn dispatch_keystore_list_result(
    service: &mut IdentityService,
    op: PendingKeystoreOp,
    result: Result<Vec<String>, String>,
) -> Result<(), AppError> {
    match op {
        PendingKeystoreOp::ListMachineKeys { ctx, user_id } => {
            handle_list_machine_keys(service, ctx, user_id, result)
        }
        PendingKeystoreOp::ListMachineKeysForZidLogin { ctx, user_id, zid_endpoint } => {
            handle_list_machine_keys_for_zid_login(service, ctx, user_id, zid_endpoint, result)
        }
        PendingKeystoreOp::ListMachineKeysForZidEnroll { ctx, user_id, zid_endpoint } => {
            handle_list_machine_keys_for_zid_enroll(service, ctx, user_id, zid_endpoint, result)
        }
        // Operations that should NOT receive a list response
        PendingKeystoreOp::CheckKeyExists { ctx, .. }
        | PendingKeystoreOp::WriteKeyStore { ctx, .. }
        | PendingKeystoreOp::WriteEncryptedShards { ctx, .. }
        | PendingKeystoreOp::GetIdentityKey { ctx }
        | PendingKeystoreOp::ReadIdentityForRecovery { ctx, .. }
        | PendingKeystoreOp::WriteRecoveredKeyStore { ctx, .. }
        | PendingKeystoreOp::ReadIdentityForMachine { ctx, .. }
        | PendingKeystoreOp::ReadEncryptedShardsForMachine { ctx, .. }
        | PendingKeystoreOp::WriteMachineKey { ctx, .. }
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
                "IdentityService: STATE_MACHINE_ERROR - unexpected keystore list result for non-list op, client_pid={}",
                ctx.client_pid
            ));
            Err(AppError::Internal(
                "State machine error: unexpected keystore list result for non-list operation".into(),
            ))
        }
    }
}

fn handle_list_machine_keys(
    service: &mut IdentityService,
    ctx: RequestContext,
    user_id: u128,
    result: Result<Vec<String>, String>,
) -> Result<(), AppError> {
    match result {
        Ok(keys) => {
            // Convert key list to paths and start reading machine keys
            let paths: Vec<String> = keys
                .into_iter()
                .filter(|k| k.ends_with(".json"))
                .collect();

            if paths.is_empty() {
                response::send_list_machine_keys(ctx.client_pid, &ctx.cap_slots, alloc::vec![])
            } else {
                let mut remaining_paths = paths;
                let first_path = remaining_paths.remove(0);
                service.start_keystore_read(
                    &first_path,
                    PendingKeystoreOp::ReadMachineKey {
                        ctx: RequestContext::new(ctx.client_pid, ctx.cap_slots),
                        user_id,
                        remaining_paths,
                        records: alloc::vec![],
                    },
                )
            }
        }
        Err(_) => {
            // No machine keys or error - return empty list
            response::send_list_machine_keys(ctx.client_pid, &ctx.cap_slots, alloc::vec![])
        }
    }
}

fn handle_list_machine_keys_for_zid_login(
    service: &mut IdentityService,
    ctx: RequestContext,
    user_id: u128,
    zid_endpoint: String,
    result: Result<Vec<String>, String>,
) -> Result<(), AppError> {
    match result {
        Ok(keys) => {
            // Find first machine key for ZID login
            let path = keys.into_iter().find(|k| k.ends_with(".json"));
            match path {
                Some(p) => service.start_keystore_read(
                    &p,
                    PendingKeystoreOp::ReadMachineKeyForZidLogin { ctx, user_id, zid_endpoint },
                ),
                None => response::send_zid_login_error(
                    ctx.client_pid,
                    &ctx.cap_slots,
                    zos_identity::error::ZidError::MachineKeyNotFound,
                ),
            }
        }
        Err(_) => response::send_zid_login_error(
            ctx.client_pid,
            &ctx.cap_slots,
            zos_identity::error::ZidError::MachineKeyNotFound,
        ),
    }
}

fn handle_list_machine_keys_for_zid_enroll(
    service: &mut IdentityService,
    ctx: RequestContext,
    user_id: u128,
    zid_endpoint: String,
    result: Result<Vec<String>, String>,
) -> Result<(), AppError> {
    match result {
        Ok(keys) => {
            // Find first machine key for ZID enrollment
            let path = keys.into_iter().find(|k| k.ends_with(".json"));
            match path {
                Some(p) => service.start_keystore_read(
                    &p,
                    PendingKeystoreOp::ReadMachineKeyForZidEnroll { ctx, user_id, zid_endpoint },
                ),
                None => response::send_zid_enroll_error(
                    ctx.client_pid,
                    &ctx.cap_slots,
                    zos_identity::error::ZidError::MachineKeyNotFound,
                ),
            }
        }
        Err(_) => response::send_zid_enroll_error(
            ctx.client_pid,
            &ctx.cap_slots,
            zos_identity::error::ZidError::MachineKeyNotFound,
        ),
    }
}
