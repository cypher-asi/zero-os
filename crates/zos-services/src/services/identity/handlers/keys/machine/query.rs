//! Machine key query operations (list, get, revoke)

extern crate alloc;

use alloc::format;

use crate::services::identity::pending::{PendingKeystoreOp, RequestContext};
use crate::services::identity::response;
use crate::services::identity::{check_user_authorization, log_denial, AuthResult, IdentityService};
use zos_apps::syscall;
use zos_apps::{AppError, Message};
use zos_identity::ipc::{
    GetMachineKeyRequest, ListMachineKeysRequest, RevokeMachineKeyRequest,
};
use zos_identity::keystore::MachineKeyRecord;
use zos_identity::KeyError;

pub fn handle_list_machine_keys(
    service: &mut IdentityService,
    msg: &Message,
) -> Result<(), AppError> {
    // Rule 1: Parse request - return InvalidRequest on parse failure (NOT empty list)
    let request: ListMachineKeysRequest = match serde_json::from_slice(&msg.data) {
        Ok(r) => r,
        Err(e) => {
            syscall::debug(&format!("IdentityService: Failed to parse request: {}", e));
            return response::send_list_machine_keys_error(
                msg.from_pid,
                &msg.cap_slots,
                KeyError::InvalidRequest(format!("JSON parse error: {}", e)),
            );
        }
    };

    // Rule 4: Authorization check (FAIL-CLOSED)
    if check_user_authorization(msg.from_pid, request.user_id) == AuthResult::Denied {
        log_denial("list_machine_keys", msg.from_pid, request.user_id);
        return response::send_list_machine_keys_error(
            msg.from_pid,
            &msg.cap_slots,
            KeyError::Unauthorized,
        );
    }

    // Invariant 32: /keys/ paths use Keystore IPC, not VFS
    // Use keystore list with prefix to find all machine keys
    let machine_prefix = format!("/keys/{}/identity/machine/", request.user_id);
    let ctx = RequestContext::new(msg.from_pid, msg.cap_slots.clone());
    service.start_keystore_list(
        &machine_prefix,
        PendingKeystoreOp::ListMachineKeys { ctx, user_id: request.user_id },
    )
}

pub fn handle_revoke_machine_key(
    service: &mut IdentityService,
    msg: &Message,
) -> Result<(), AppError> {
    // Rule 1: Parse request - return InvalidRequest on parse failure
    let request: RevokeMachineKeyRequest = match serde_json::from_slice(&msg.data) {
        Ok(r) => r,
        Err(e) => {
            syscall::debug(&format!("IdentityService: Failed to parse request: {}", e));
            return response::send_revoke_machine_key_error(
                msg.from_pid,
                &msg.cap_slots,
                KeyError::InvalidRequest(format!("JSON parse error: {}", e)),
            );
        }
    };

    // Rule 4: Authorization check (FAIL-CLOSED)
    if check_user_authorization(msg.from_pid, request.user_id) == AuthResult::Denied {
        log_denial("revoke_machine_key", msg.from_pid, request.user_id);
        return response::send_revoke_machine_key_error(
            msg.from_pid,
            &msg.cap_slots,
            KeyError::Unauthorized,
        );
    }

    let machine_path = MachineKeyRecord::storage_path(request.user_id, request.machine_id);
    let ctx = RequestContext::new(msg.from_pid, msg.cap_slots.clone());
    // Invariant 32: /keys/ paths use Keystore IPC, not VFS
    service.start_keystore_delete(
        &machine_path,
        PendingKeystoreOp::DeleteMachineKey {
            ctx,
            user_id: request.user_id,
            machine_id: request.machine_id,
        },
    )
}

pub fn handle_get_machine_key(
    service: &mut IdentityService,
    msg: &Message,
) -> Result<(), AppError> {
    // Rule 1: Parse request - return InvalidRequest on parse failure
    let request: GetMachineKeyRequest = match serde_json::from_slice(&msg.data) {
        Ok(r) => r,
        Err(e) => {
            syscall::debug(&format!("IdentityService: Failed to parse request: {}", e));
            return response::send_get_machine_key_error(
                msg.from_pid,
                &msg.cap_slots,
                KeyError::InvalidRequest(format!("JSON parse error: {}", e)),
            );
        }
    };

    // Rule 4: Authorization check (FAIL-CLOSED)
    if check_user_authorization(msg.from_pid, request.user_id) == AuthResult::Denied {
        log_denial("get_machine_key", msg.from_pid, request.user_id);
        return response::send_get_machine_key_error(
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
        PendingKeystoreOp::ReadSingleMachineKey { ctx },
    )
}
