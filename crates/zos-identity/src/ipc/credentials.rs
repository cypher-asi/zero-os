//! Credential Request/Response Types

use alloc::string::String;
use alloc::vec::Vec;
use serde::{Deserialize, Serialize};

use crate::error::CredentialError;
use crate::serde_helpers::u128_hex_string;
use crate::types::UserId;

extern crate alloc;

/// Attach email credential request.
///
/// Calls ZID API to attach email credential. Requires active ZID session.
/// Password is hashed server-side with Argon2id.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AttachEmailRequest {
    /// User ID
    #[serde(with = "u128_hex_string")]
    pub user_id: UserId,
    /// Email address to attach
    pub email: String,
    /// Password for ZID account (hashed server-side with Argon2id)
    pub password: String,
    /// JWT access token from ZID login
    pub access_token: String,
    /// ZID API endpoint (e.g., "https://api.zero-id.io")
    pub zid_endpoint: String,
}

/// Attach email response.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AttachEmailResponse {
    /// Result of the operation (simplified - no verification needed with ZID)
    pub result: Result<(), CredentialError>,
}

/// Get credentials request.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GetCredentialsRequest {
    /// User ID
    #[serde(with = "u128_hex_string")]
    pub user_id: UserId,
    /// Optional filter by credential type
    pub credential_type: Option<crate::keystore::CredentialType>,
}

/// Get credentials response.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GetCredentialsResponse {
    /// List of credentials
    pub credentials: Vec<crate::keystore::LinkedCredential>,
}

/// Unlink credential request.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UnlinkCredentialRequest {
    /// User ID
    #[serde(with = "u128_hex_string")]
    pub user_id: UserId,
    /// Type of credential to unlink
    pub credential_type: crate::keystore::CredentialType,
}

/// Unlink credential response.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UnlinkCredentialResponse {
    /// Result of the operation
    pub result: Result<(), CredentialError>,
}
