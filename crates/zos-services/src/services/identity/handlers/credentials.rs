//! Credential management operations
//!
//! Handlers for:
//! - Attaching email credentials (with ZID verification)
//! - Unlinking credentials
//! - Retrieving credential lists
//!
//! # Safety Invariants (per zos-service.md Rule 0)
//!
//! ## Success Conditions
//! - Attach email: ZID verification succeeded, credential stored, response sent
//! - Unlink: Credential found and removed from store, response sent
//! - Get credentials: Credentials read (or empty list), response sent
//!
//! ## Acceptable Partial Failure
//! - Network failure during ZID verification (returns error, no state change)
//!
//! ## Forbidden States
//! - Storing credential without ZID verification
//! - Silent fallthrough on parse errors (must return InvalidRequest)
//! - Processing requests without authorization check

use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

use super::super::pending::{PendingNetworkOp, PendingStorageOp, RequestContext};
use super::super::response;
use super::super::{check_user_authorization, log_denial, AuthResult, IdentityService};
use zos_apps::syscall;
use zos_apps::{AppError, Message};
use zos_identity::error::CredentialError;
use zos_identity::ipc::{AttachEmailRequest, GetCredentialsRequest, UnlinkCredentialRequest};
use zos_identity::keystore::{CredentialStore, CredentialType, LinkedCredential};
use zos_network::HttpRequest;

// =============================================================================
// Email Credential Operations
// =============================================================================

pub fn handle_attach_email(service: &mut IdentityService, msg: &Message) -> Result<(), AppError> {
    // Rule 1: Parse request - return InvalidRequest on parse failure
    let request: AttachEmailRequest = match serde_json::from_slice(&msg.data) {
        Ok(r) => r,
        Err(e) => {
            syscall::debug(&format!("IdentityService: Failed to parse request: {}", e));
            return response::send_attach_email_error(
                msg.from_pid,
                &msg.cap_slots,
                CredentialError::InvalidRequest(format!("JSON parse error: {}", e)),
            );
        }
    };

    // Rule 4: Authorization check (FAIL-CLOSED)
    if check_user_authorization(msg.from_pid, request.user_id) == AuthResult::Denied {
        log_denial("attach_email", msg.from_pid, request.user_id);
        return response::send_attach_email_error(
            msg.from_pid,
            &msg.cap_slots,
            CredentialError::Unauthorized,
        );
    }

    if !request.email.contains('@') || request.email.len() < 5 {
        return response::send_attach_email_error(
            msg.from_pid,
            &msg.cap_slots,
            CredentialError::InvalidFormat,
        );
    }

    if request.password.len() < 12 {
        return response::send_attach_email_error(
            msg.from_pid,
            &msg.cap_slots,
            CredentialError::StorageError("Password must be at least 12 characters".into()),
        );
    }

    let body = format!(
        r#"{{"email":"{}","password":"{}"}}"#,
        request.email, request.password
    );
    let http_request = HttpRequest::post(format!("{}/v1/credentials/email", request.zid_endpoint))
        .with_header("Authorization", format!("Bearer {}", request.access_token))
        .with_json_body(body.into_bytes())
        .with_timeout(15_000);

    let ctx = RequestContext::new(msg.from_pid, msg.cap_slots.clone());
    service.start_network_fetch(
        &http_request,
        PendingNetworkOp::SubmitEmailToZid {
            ctx,
            user_id: request.user_id,
            email: request.email,
        },
    )
}

pub fn continue_attach_email_after_zid(
    service: &mut IdentityService,
    client_pid: u32,
    user_id: u128,
    email: String,
    cap_slots: Vec<u32>,
) -> Result<(), AppError> {
    let cred_path = CredentialStore::storage_path(user_id);
    let ctx = RequestContext::new(client_pid, cap_slots);
    service.start_vfs_read(
        &cred_path,
        PendingStorageOp::ReadCredentialsForAttach { ctx, user_id, email },
    )
}

pub fn continue_attach_email_after_read(
    service: &mut IdentityService,
    client_pid: u32,
    user_id: u128,
    email: String,
    existing_store: Option<CredentialStore>,
    cap_slots: Vec<u32>,
) -> Result<(), AppError> {
    let ctx = RequestContext::new(client_pid, cap_slots);
    
    let now = syscall::get_wallclock();
    let mut store = existing_store.unwrap_or_else(|| CredentialStore::new(user_id));
    store.credentials.retain(|c| {
        !(c.credential_type == CredentialType::Email && c.value == email && !c.verified)
    });
    store.credentials.push(LinkedCredential {
        credential_type: CredentialType::Email,
        value: email,
        verified: true,
        linked_at: now,
        verified_at: Some(now),
        is_primary: store.find_by_type(CredentialType::Email).is_empty(),
    });

    let cred_path = CredentialStore::storage_path(user_id);
    match serde_json::to_vec(&store) {
        Ok(json_bytes) => service.start_vfs_write(
            &cred_path,
            &json_bytes,
            PendingStorageOp::WriteEmailCredential {
                ctx,
                user_id,
                json_bytes: json_bytes.clone(),
            },
        ),
        Err(e) => response::send_attach_email_error(
            ctx.client_pid,
            &ctx.cap_slots,
            CredentialError::StorageError(format!("Serialization failed: {}", e)),
        ),
    }
}

// =============================================================================
// Credential Retrieval
// =============================================================================

pub fn handle_get_credentials(
    service: &mut IdentityService,
    msg: &Message,
) -> Result<(), AppError> {
    // Rule 1: Parse request - return InvalidRequest on parse failure (NOT empty list)
    let request: GetCredentialsRequest = match serde_json::from_slice(&msg.data) {
        Ok(r) => r,
        Err(e) => {
            syscall::debug(&format!("IdentityService: Failed to parse request: {}", e));
            return response::send_get_credentials_error(
                msg.from_pid,
                &msg.cap_slots,
                CredentialError::InvalidRequest(format!("JSON parse error: {}", e)),
            );
        }
    };

    // Rule 4: Authorization check (FAIL-CLOSED)
    if check_user_authorization(msg.from_pid, request.user_id) == AuthResult::Denied {
        log_denial("get_credentials", msg.from_pid, request.user_id);
        return response::send_get_credentials_error(
            msg.from_pid,
            &msg.cap_slots,
            CredentialError::Unauthorized,
        );
    }

    let cred_path = CredentialStore::storage_path(request.user_id);
    let ctx = RequestContext::new(msg.from_pid, msg.cap_slots.clone());
    service.start_vfs_read(
        &cred_path,
        PendingStorageOp::GetCredentials { ctx },
    )
}

// =============================================================================
// Credential Unlinking
// =============================================================================

pub fn handle_unlink_credential(
    service: &mut IdentityService,
    msg: &Message,
) -> Result<(), AppError> {
    // Rule 1: Parse request - return InvalidRequest on parse failure
    let request: UnlinkCredentialRequest = match serde_json::from_slice(&msg.data) {
        Ok(r) => r,
        Err(e) => {
            syscall::debug(&format!("IdentityService: Failed to parse request: {}", e));
            return response::send_unlink_credential_error(
                msg.from_pid,
                &msg.cap_slots,
                CredentialError::InvalidRequest(format!("JSON parse error: {}", e)),
            );
        }
    };

    // Rule 4: Authorization check (FAIL-CLOSED)
    if check_user_authorization(msg.from_pid, request.user_id) == AuthResult::Denied {
        log_denial("unlink_credential", msg.from_pid, request.user_id);
        return response::send_unlink_credential_error(
            msg.from_pid,
            &msg.cap_slots,
            CredentialError::Unauthorized,
        );
    }

    let cred_path = CredentialStore::storage_path(request.user_id);
    let ctx = RequestContext::new(msg.from_pid, msg.cap_slots.clone());
    service.start_vfs_read(
        &cred_path,
        PendingStorageOp::ReadCredentialsForUnlink {
            ctx,
            user_id: request.user_id,
            credential_type: request.credential_type,
        },
    )
}

pub fn continue_unlink_credential_after_read(
    service: &mut IdentityService,
    client_pid: u32,
    user_id: u128,
    credential_type: CredentialType,
    data: &[u8],
    cap_slots: Vec<u32>,
) -> Result<(), AppError> {
    let ctx = RequestContext::new(client_pid, cap_slots);
    
    let mut store: CredentialStore = match serde_json::from_slice(data) {
        Ok(s) => s,
        Err(e) => {
            return response::send_unlink_credential_error(
                ctx.client_pid,
                &ctx.cap_slots,
                CredentialError::StorageError(format!("Parse failed: {}", e)),
            )
        }
    };

    let original_len = store.credentials.len();
    store
        .credentials
        .retain(|c| c.credential_type != credential_type);

    if store.credentials.len() == original_len {
        return response::send_unlink_credential_error(
            ctx.client_pid,
            &ctx.cap_slots,
            CredentialError::NotFound,
        );
    }

    let cred_path = CredentialStore::storage_path(user_id);
    match serde_json::to_vec(&store) {
        Ok(json_bytes) => service.start_vfs_write(
            &cred_path,
            &json_bytes,
            PendingStorageOp::WriteUnlinkedCredential {
                ctx,
                user_id,
                json_bytes: json_bytes.clone(),
            },
        ),
        Err(e) => response::send_unlink_credential_error(
            ctx.client_pid,
            &ctx.cap_slots,
            CredentialError::StorageError(format!("Serialization failed: {}", e)),
        ),
    }
}
