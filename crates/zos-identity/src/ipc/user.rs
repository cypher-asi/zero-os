//! User Service Request/Response Types

use alloc::string::String;
use alloc::vec::Vec;
use serde::{Deserialize, Serialize};

use crate::error::UserError;
use crate::serde_helpers::u128_hex_string;
use crate::types::{User, UserId, UserStatus};

extern crate alloc;

/// Create user request.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CreateUserRequest {
    /// Display name for the new user
    pub display_name: String,
}

/// Create user response.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CreateUserResponse {
    /// Result containing the created user or an error
    pub result: Result<User, UserError>,
}

/// Get user request.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GetUserRequest {
    /// User ID to retrieve
    #[serde(with = "u128_hex_string")]
    pub user_id: UserId,
}

/// Get user response.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GetUserResponse {
    /// Result containing the user or an error
    pub result: Result<Option<User>, UserError>,
}

/// List users request.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ListUsersRequest {
    /// Optional status filter
    pub status_filter: Option<UserStatus>,
}

/// List users response.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ListUsersResponse {
    /// List of users
    pub users: Vec<User>,
}

/// Delete user request.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DeleteUserRequest {
    /// User ID to delete
    #[serde(with = "u128_hex_string")]
    pub user_id: UserId,
    /// Whether to delete the home directory
    pub delete_home: bool,
}

/// Delete user response.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DeleteUserResponse {
    /// Result of the operation
    pub result: Result<(), UserError>,
}
