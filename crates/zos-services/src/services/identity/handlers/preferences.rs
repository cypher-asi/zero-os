//! Identity preference handlers
//!
//! Handles get/set operations for identity preferences stored in VFS
//! at `/home/{user_id}/.zos/identity/preferences.json`.
//!
//! # Safety Invariants (per zos-service.md Rule 0)
//!
//! ## Success Conditions
//! - Get preferences: Preferences read (or default), response sent
//! - Set key scheme: Preferences read, updated, written, response sent
//!
//! ## Acceptable Partial Failure
//! - Read failure returns default preferences (not an error)
//!
//! ## Forbidden States
//! - Returning success before preferences are written
//! - Silent fallthrough on parse errors (must return InvalidRequest)
//! - Processing requests without authorization check

use alloc::format;
use zos_apps::{syscall, AppError, Message};
use zos_identity::ipc::{
    GetIdentityPreferencesRequest, IdentityPreferences,
    SetDefaultKeySchemeRequest,
};
use zos_identity::KeyError;

use super::super::IdentityService;
use super::super::pending::{PendingStorageOp, RequestContext};
use super::super::response;
use super::super::{check_user_authorization, log_denial, AuthResult};

/// Handle get preferences - read from VFS
pub fn handle_get_preferences(
    service: &mut IdentityService,
    msg: &Message,
) -> Result<(), AppError> {
    // Rule 1: Parse request - return InvalidRequest on parse failure
    let request: GetIdentityPreferencesRequest = match serde_json::from_slice(&msg.data) {
        Ok(r) => r,
        Err(e) => {
            syscall::debug(&format!("IdentityService: Failed to parse request: {}", e));
            return response::send_set_default_key_scheme_error(
                msg.from_pid,
                &msg.cap_slots,
                KeyError::InvalidRequest(format!("JSON parse error: {}", e)),
            );
        }
    };

    // Rule 4: Authorization check (FAIL-CLOSED)
    if check_user_authorization(msg.from_pid, request.user_id) == AuthResult::Denied {
        log_denial("get_preferences", msg.from_pid, request.user_id);
        return response::send_set_default_key_scheme_error(
            msg.from_pid,
            &msg.cap_slots,
            KeyError::Unauthorized,
        );
    }

    let prefs_path = IdentityPreferences::storage_path(request.user_id);

    let ctx = RequestContext::new(msg.from_pid, msg.cap_slots.clone());
    service.start_vfs_read(
        &prefs_path,
        PendingStorageOp::ReadIdentityPreferences { ctx, user_id: request.user_id },
    )
}

/// Handle set default key scheme - write to VFS
pub fn handle_set_default_key_scheme(
    service: &mut IdentityService,
    msg: &Message,
) -> Result<(), AppError> {
    // Rule 1: Parse request - return InvalidRequest on parse failure
    let request: SetDefaultKeySchemeRequest = match serde_json::from_slice(&msg.data) {
        Ok(r) => r,
        Err(e) => {
            syscall::debug(&format!("IdentityService: Failed to parse request: {}", e));
            return response::send_set_default_key_scheme_error(
                msg.from_pid,
                &msg.cap_slots,
                KeyError::InvalidRequest(format!("JSON parse error: {}", e)),
            );
        }
    };

    // Rule 4: Authorization check (FAIL-CLOSED)
    if check_user_authorization(msg.from_pid, request.user_id) == AuthResult::Denied {
        log_denial("set_default_key_scheme", msg.from_pid, request.user_id);
        return response::send_set_default_key_scheme_error(
            msg.from_pid,
            &msg.cap_slots,
            KeyError::Unauthorized,
        );
    }

    let prefs_path = IdentityPreferences::storage_path(request.user_id);

    // Read existing preferences first (or use default)
    let ctx = RequestContext::new(msg.from_pid, msg.cap_slots.clone());
    service.start_vfs_read(
        &prefs_path,
        PendingStorageOp::ReadPreferencesForUpdate {
            ctx,
            user_id: request.user_id,
            new_key_scheme: request.key_scheme,
        },
    )
}
