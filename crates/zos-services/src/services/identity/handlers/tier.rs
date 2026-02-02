//! ZID tier status and upgrade handlers
//!
//! Handlers for:
//! - Getting current identity tier status
//! - Upgrading from managed to self-sovereign identity
//!
//! # Safety Invariants (per zos-service.md Rule 0)
//!
//! ## Success Conditions
//! - Tier status: ZID returns tier info, returned to caller
//! - Upgrade: ZID accepts new ISK, upgrade confirmed
//!
//! ## Acceptable Partial Failure
//! - None - tier operations should be atomic
//!
//! ## Forbidden States
//! - Returning success before ZID confirms upgrade
//! - Silent fallthrough on parse errors (must return InvalidRequest)
//! - Processing requests without authorization check

use alloc::format;

use super::super::pending::{PendingNetworkOp, RequestContext};
use super::super::response;
use super::super::{check_user_authorization, log_denial, AuthResult, IdentityService};
use zos_apps::syscall;
use zos_apps::{AppError, Message};
use zos_identity::error::ZidError;
use zos_identity::ipc::{GetTierStatusRequest, UpgradeToSelfSovereignRequest};
use zos_network::HttpRequest;

// =============================================================================
// Tier Status
// =============================================================================

/// Handle get tier status request.
///
/// Queries the ZID server for the current identity tier.
pub fn handle_get_tier_status(
    service: &mut IdentityService,
    msg: &Message,
) -> Result<(), AppError> {
    // Rule 1: Parse request
    let request: GetTierStatusRequest = match serde_json::from_slice(&msg.data) {
        Ok(r) => r,
        Err(e) => {
            syscall::debug(&format!(
                "IdentityService: Failed to parse GetTierStatusRequest: {}",
                e
            ));
            return response::send_get_tier_status_error(
                msg.from_pid,
                &msg.cap_slots,
                ZidError::InvalidRequest(format!("JSON parse error: {}", e)),
            );
        }
    };

    // Rule 4: Authorization check (FAIL-CLOSED)
    if check_user_authorization(msg.from_pid, request.user_id) == AuthResult::Denied {
        log_denial("get_tier_status", msg.from_pid, request.user_id);
        return response::send_get_tier_status_error(
            msg.from_pid,
            &msg.cap_slots,
            ZidError::Unauthorized,
        );
    }

    syscall::debug(&format!(
        "IdentityService: Getting tier status for user {:032x}",
        request.user_id
    ));

    // Build HTTP request using builder API
    let http_request = HttpRequest::get(format!("{}/v1/identity/tier", request.zid_endpoint))
        .with_bearer_token(&request.access_token)
        .with_timeout(30_000);

    let ctx = RequestContext::new(msg.from_pid, msg.cap_slots.clone());
    service.start_network_fetch(
        &http_request,
        PendingNetworkOp::GetTierStatus {
            ctx,
            user_id: request.user_id,
            zid_endpoint: request.zid_endpoint,
        },
    )
}

// =============================================================================
// Upgrade to Self-Sovereign
// =============================================================================

/// Handle upgrade to self-sovereign request.
///
/// Submits the new ISK public key to ZID server to transition
/// from managed to self-sovereign identity.
pub fn handle_upgrade_to_self_sovereign(
    service: &mut IdentityService,
    msg: &Message,
) -> Result<(), AppError> {
    // Rule 1: Parse request
    let request: UpgradeToSelfSovereignRequest = match serde_json::from_slice(&msg.data) {
        Ok(r) => r,
        Err(e) => {
            syscall::debug(&format!(
                "IdentityService: Failed to parse UpgradeToSelfSovereignRequest: {}",
                e
            ));
            return response::send_upgrade_to_self_sovereign_error(
                msg.from_pid,
                &msg.cap_slots,
                ZidError::InvalidRequest(format!("JSON parse error: {}", e)),
            );
        }
    };

    // Rule 4: Authorization check (FAIL-CLOSED)
    if check_user_authorization(msg.from_pid, request.user_id) == AuthResult::Denied {
        log_denial("upgrade_to_self_sovereign", msg.from_pid, request.user_id);
        return response::send_upgrade_to_self_sovereign_error(
            msg.from_pid,
            &msg.cap_slots,
            ZidError::Unauthorized,
        );
    }

    syscall::debug(&format!(
        "IdentityService: Upgrading user {:032x} to self-sovereign",
        request.user_id
    ));

    // Build JSON body
    let body = format!(
        r#"{{"new_isk_public":"{}","commitment":"{}","upgrade_signature":"{}"}}"#,
        request.new_isk_public, request.commitment, request.upgrade_signature
    );

    // Build HTTP request using builder API
    let http_request = HttpRequest::post(format!("{}/v1/identity/upgrade", request.zid_endpoint))
        .with_bearer_token(&request.access_token)
        .with_json_body(body.into_bytes())
        .with_timeout(30_000);

    let ctx = RequestContext::new(msg.from_pid, msg.cap_slots.clone());
    service.start_network_fetch(
        &http_request,
        PendingNetworkOp::UpgradeToSelfSovereign {
            ctx,
            user_id: request.user_id,
            zid_endpoint: request.zid_endpoint,
        },
    )
}
