//! ZID registration handlers for managed identities
//!
//! Handlers for:
//! - Email/password registration
//! - OAuth registration (Google, X, Epic)
//! - Wallet registration (Ethereum, Solana, etc.)
//!
//! # Safety Invariants (per zos-service.md Rule 0)
//!
//! ## Success Conditions
//! - Registration succeeds on ZID server
//! - Tokens returned to caller
//! - Session stored in VFS (optional - tokens still returned if store fails)
//!
//! ## Acceptable Partial Failure
//! - Session write failure after successful registration (tokens still returned)
//!
//! ## Forbidden States
//! - Returning success before ZID server confirms registration
//! - Silent fallthrough on parse errors (must return InvalidRequest)

use alloc::format;
use alloc::string::String;

use super::super::pending::{PendingNetworkOp, RequestContext};
use super::super::response;
use super::super::IdentityService;
use zos_apps::syscall;
use zos_apps::{AppError, Message};
use zos_identity::error::ZidError;
use zos_identity::ipc::{
    InitOAuthRequest, InitWalletAuthRequest, OAuthCallbackRequest, RegisterEmailRequest,
    VerifyWalletRequest,
};
use zos_network::HttpRequest;

// =============================================================================
// Email/Password Registration
// =============================================================================

/// Handle email/password registration request.
///
/// Creates a new managed identity on ZID with email/password credentials.
pub fn handle_register_email(service: &mut IdentityService, msg: &Message) -> Result<(), AppError> {
    // Rule 1: Parse request
    let request: RegisterEmailRequest = match serde_json::from_slice(&msg.data) {
        Ok(r) => r,
        Err(e) => {
            syscall::debug(&format!(
                "IdentityService: Failed to parse RegisterEmailRequest: {}",
                e
            ));
            return response::send_register_email_error(
                msg.from_pid,
                &msg.cap_slots,
                ZidError::InvalidRequest(format!("JSON parse error: {}", e)),
            );
        }
    };

    syscall::debug(&format!(
        "IdentityService: Registering new identity with email: {}",
        request.email
    ));

    // Build JSON body
    let body = format!(
        r#"{{"email":"{}","password":"{}"}}"#,
        request.email, request.password
    );

    // Build HTTP request using builder API
    let http_request = HttpRequest::post(format!("{}/v1/identity/email", request.zid_endpoint))
        .with_json_body(body.into_bytes())
        .with_timeout(30_000);

    let ctx = RequestContext::new(msg.from_pid, msg.cap_slots.clone());
    service.start_network_fetch(
        &http_request,
        PendingNetworkOp::RegisterEmail {
            ctx,
            email: request.email,
            zid_endpoint: request.zid_endpoint,
        },
    )
}

// =============================================================================
// OAuth Registration
// =============================================================================

/// Handle OAuth flow initiation request.
///
/// Returns the OAuth authorization URL for the user to visit.
pub fn handle_init_oauth(service: &mut IdentityService, msg: &Message) -> Result<(), AppError> {
    // Rule 1: Parse request
    let request: InitOAuthRequest = match serde_json::from_slice(&msg.data) {
        Ok(r) => r,
        Err(e) => {
            syscall::debug(&format!(
                "IdentityService: Failed to parse InitOAuthRequest: {}",
                e
            ));
            return response::send_init_oauth_error(
                msg.from_pid,
                &msg.cap_slots,
                ZidError::InvalidRequest(format!("JSON parse error: {}", e)),
            );
        }
    };

    let provider_str = match request.provider {
        zos_identity::ipc::OAuthProvider::Google => "google",
        zos_identity::ipc::OAuthProvider::X => "x",
        zos_identity::ipc::OAuthProvider::Epic => "epic",
    };

    syscall::debug(&format!(
        "IdentityService: Initiating OAuth flow for provider: {}",
        provider_str
    ));

    // Build JSON body
    let body = match &request.redirect_uri {
        Some(uri) => format!(r#"{{"redirect_uri":"{}"}}"#, uri),
        None => r#"{}"#.into(),
    };

    // Build HTTP request using builder API
    let http_request = HttpRequest::post(format!(
        "{}/v1/identity/oauth/{}/init",
        request.zid_endpoint, provider_str
    ))
    .with_json_body(body.into_bytes())
    .with_timeout(30_000);

    let ctx = RequestContext::new(msg.from_pid, msg.cap_slots.clone());
    service.start_network_fetch(
        &http_request,
        PendingNetworkOp::InitOAuth {
            ctx,
            provider: provider_str.into(),
            zid_endpoint: request.zid_endpoint,
        },
    )
}

/// Handle OAuth callback with authorization code.
///
/// Exchanges the authorization code for tokens.
pub fn handle_oauth_callback(
    service: &mut IdentityService,
    msg: &Message,
) -> Result<(), AppError> {
    // Rule 1: Parse request
    let request: OAuthCallbackRequest = match serde_json::from_slice(&msg.data) {
        Ok(r) => r,
        Err(e) => {
            syscall::debug(&format!(
                "IdentityService: Failed to parse OAuthCallbackRequest: {}",
                e
            ));
            return response::send_oauth_callback_error(
                msg.from_pid,
                &msg.cap_slots,
                ZidError::InvalidRequest(format!("JSON parse error: {}", e)),
            );
        }
    };

    let provider_str = match request.provider {
        zos_identity::ipc::OAuthProvider::Google => "google",
        zos_identity::ipc::OAuthProvider::X => "x",
        zos_identity::ipc::OAuthProvider::Epic => "epic",
    };

    syscall::debug(&format!(
        "IdentityService: Processing OAuth callback for provider: {}",
        provider_str
    ));

    // Build JSON body
    let body = format!(r#"{{"code":"{}","state":"{}"}}"#, request.code, request.state);

    // Build HTTP request using builder API
    let http_request = HttpRequest::post(format!(
        "{}/v1/identity/oauth/{}/callback",
        request.zid_endpoint, provider_str
    ))
    .with_json_body(body.into_bytes())
    .with_timeout(30_000);

    let ctx = RequestContext::new(msg.from_pid, msg.cap_slots.clone());
    service.start_network_fetch(
        &http_request,
        PendingNetworkOp::OAuthCallback {
            ctx,
            provider: provider_str.into(),
            zid_endpoint: request.zid_endpoint,
        },
    )
}

// =============================================================================
// Wallet Registration
// =============================================================================

/// Handle wallet auth initiation request.
///
/// Returns a challenge message for the wallet to sign.
pub fn handle_init_wallet(service: &mut IdentityService, msg: &Message) -> Result<(), AppError> {
    // Rule 1: Parse request
    let request: InitWalletAuthRequest = match serde_json::from_slice(&msg.data) {
        Ok(r) => r,
        Err(e) => {
            syscall::debug(&format!(
                "IdentityService: Failed to parse InitWalletAuthRequest: {}",
                e
            ));
            return response::send_init_wallet_error(
                msg.from_pid,
                &msg.cap_slots,
                ZidError::InvalidRequest(format!("JSON parse error: {}", e)),
            );
        }
    };

    let wallet_type_str = match request.wallet_type {
        zos_identity::ipc::WalletType::Ethereum => "ethereum",
        zos_identity::ipc::WalletType::Polygon => "polygon",
        zos_identity::ipc::WalletType::Arbitrum => "arbitrum",
        zos_identity::ipc::WalletType::Base => "base",
        zos_identity::ipc::WalletType::Solana => "solana",
    };

    syscall::debug(&format!(
        "IdentityService: Initiating wallet auth for {}: {}",
        wallet_type_str, request.address
    ));

    // Build JSON body
    let body = format!(
        r#"{{"wallet_type":"{}","address":"{}"}}"#,
        wallet_type_str, request.address
    );

    // Build HTTP request using builder API
    let http_request =
        HttpRequest::post(format!("{}/v1/identity/wallet/challenge", request.zid_endpoint))
            .with_json_body(body.into_bytes())
            .with_timeout(30_000);

    let ctx = RequestContext::new(msg.from_pid, msg.cap_slots.clone());
    service.start_network_fetch(
        &http_request,
        PendingNetworkOp::InitWallet {
            ctx,
            wallet_type: wallet_type_str.into(),
            address: request.address,
            zid_endpoint: request.zid_endpoint,
        },
    )
}

/// Handle wallet signature verification request.
///
/// Verifies the signature and creates/authenticates the identity.
pub fn handle_verify_wallet(service: &mut IdentityService, msg: &Message) -> Result<(), AppError> {
    // Rule 1: Parse request
    let request: VerifyWalletRequest = match serde_json::from_slice(&msg.data) {
        Ok(r) => r,
        Err(e) => {
            syscall::debug(&format!(
                "IdentityService: Failed to parse VerifyWalletRequest: {}",
                e
            ));
            return response::send_verify_wallet_error(
                msg.from_pid,
                &msg.cap_slots,
                ZidError::InvalidRequest(format!("JSON parse error: {}", e)),
            );
        }
    };

    let wallet_type_str = match request.wallet_type {
        zos_identity::ipc::WalletType::Ethereum => "ethereum",
        zos_identity::ipc::WalletType::Polygon => "polygon",
        zos_identity::ipc::WalletType::Arbitrum => "arbitrum",
        zos_identity::ipc::WalletType::Base => "base",
        zos_identity::ipc::WalletType::Solana => "solana",
    };

    syscall::debug(&format!(
        "IdentityService: Verifying {} wallet signature for challenge: {}, address: {}",
        wallet_type_str, request.challenge_id, request.address
    ));

    // Build JSON body with all required fields
    let namespace_part = request
        .namespace_name
        .map_or(String::new(), |n| format!(r#","namespace_name":"{}""#, n));

    let body = format!(
        r#"{{"challenge_id":"{}","wallet_type":"{}","address":"{}","signature":"{}"{}}}"#,
        request.challenge_id, wallet_type_str, request.address, request.signature, namespace_part
    );

    // Build HTTP request using builder API
    let http_request =
        HttpRequest::post(format!("{}/v1/identity/wallet/verify", request.zid_endpoint))
            .with_json_body(body.into_bytes())
            .with_timeout(30_000);

    let ctx = RequestContext::new(msg.from_pid, msg.cap_slots.clone());
    service.start_network_fetch(
        &http_request,
        PendingNetworkOp::VerifyWallet {
            ctx,
            zid_endpoint: request.zid_endpoint,
        },
    )
}
