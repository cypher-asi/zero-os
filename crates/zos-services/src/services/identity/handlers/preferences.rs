//! Identity preference handlers
//!
//! Handles get/set operations for identity preferences stored in VFS
//! at `/home/{user_id}/.zos/identity/preferences.json`.

use alloc::format;
use zos_apps::{AppError, Message};
use zos_identity::ipc::{
    GetIdentityPreferencesRequest, IdentityPreferences,
    SetDefaultKeySchemeRequest,
};

use super::super::IdentityService;
use super::super::pending::{PendingStorageOp, RequestContext};

/// Handle get preferences - read from VFS
pub fn handle_get_preferences(
    service: &mut IdentityService,
    msg: &Message,
) -> Result<(), AppError> {
    let request: GetIdentityPreferencesRequest = serde_json::from_slice(&msg.data)
        .map_err(|e| AppError::Internal(format!("Failed to parse request: {:?}", e)))?;
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
    let request: SetDefaultKeySchemeRequest = serde_json::from_slice(&msg.data)
        .map_err(|e| AppError::Internal(format!("Failed to parse request: {:?}", e)))?;
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
