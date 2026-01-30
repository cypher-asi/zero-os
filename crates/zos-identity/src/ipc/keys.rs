//! Key Management Request/Response Types
//!
//! Neural Key, Identity Key, and Machine Key operations.

use alloc::string::String;
use alloc::vec::Vec;
use serde::{Deserialize, Serialize};

use crate::error::KeyError;
use crate::keystore::{KeyScheme, LocalKeyStore, MachineKeyCapabilities, MachineKeyRecord};
use crate::serde_helpers::u128_hex_string;
use crate::types::UserId;

extern crate alloc;

// ============================================================================
// Neural Key Generation Request/Response Types
// ============================================================================

/// A Shamir shard for Neural Key backup.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NeuralShard {
    /// Shard index (1-5)
    pub index: u8,
    /// Shard data as hex string
    pub hex: String,
}

/// Public identifiers derived from the Neural Key.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PublicIdentifiers {
    /// Identity-level signing public key (Ed25519, hex string)
    pub identity_signing_pub_key: String,
    /// Machine-level signing public key (Ed25519, hex string)
    pub machine_signing_pub_key: String,
    /// Machine-level encryption public key (X25519, hex string)
    pub machine_encryption_pub_key: String,
}

/// Generate Neural Key request.
///
/// Triggers full key generation on the service:
/// 1. Generate 32 bytes of secure entropy
/// 2. Derive Ed25519/X25519 keypairs
/// 3. Split entropy into 5 Shamir shards
/// 4. Encrypt 2 shards with password, store to keystore
/// 5. Return 3 external shards + public identifiers
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GenerateNeuralKeyRequest {
    /// User ID to generate keys for
    #[serde(with = "u128_hex_string")]
    pub user_id: UserId,
    /// Password for encrypting 2 shards (minimum 12 characters)
    pub password: String,
}

/// Result of successful Neural Key generation.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NeuralKeyGenerated {
    /// The derived user ID (first 128 bits of SHA-256 of identity signing public key)
    #[serde(with = "u128_hex_string")]
    pub user_id: UserId,
    /// Public identifiers (stored server-side)
    pub public_identifiers: PublicIdentifiers,
    /// External Shamir shards (3 of 5) - returned to UI for backup, NOT stored
    /// The other 2 shards are encrypted with the password and stored in keystore.
    pub shards: Vec<NeuralShard>,
    /// Timestamp when the key was created
    pub created_at: u64,
}

/// Generate Neural Key response.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GenerateNeuralKeyResponse {
    /// Result containing the generated key info or an error
    pub result: Result<NeuralKeyGenerated, KeyError>,
}

/// Recover Neural Key from shards request.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RecoverNeuralKeyRequest {
    /// User ID to recover keys for
    #[serde(with = "u128_hex_string")]
    pub user_id: UserId,
    /// At least 3 shards required for recovery
    pub shards: Vec<NeuralShard>,
}

/// Recover Neural Key response.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RecoverNeuralKeyResponse {
    /// Result containing the recovered key info or an error
    pub result: Result<NeuralKeyGenerated, KeyError>,
}

// ============================================================================
// Identity Key Registration Request/Response Types
// ============================================================================

/// Register identity key request.
///
/// Registers the public keys derived from a Neural Key.
/// Private keys are stored client-side only.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RegisterIdentityKeyRequest {
    /// User ID to register keys for
    #[serde(with = "u128_hex_string")]
    pub user_id: UserId,
    /// Identity-level signing public key (Ed25519)
    pub identity_signing_public_key: [u8; 32],
    /// Machine-level signing public key (Ed25519)
    pub machine_signing_public_key: [u8; 32],
    /// Machine-level encryption public key (X25519)
    pub machine_encryption_public_key: [u8; 32],
}

/// Register identity key response.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RegisterIdentityKeyResponse {
    /// Result of the registration
    pub result: Result<(), KeyError>,
}

/// Get identity key request.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GetIdentityKeyRequest {
    /// User ID to get keys for
    #[serde(with = "u128_hex_string")]
    pub user_id: UserId,
}

/// Get identity key response.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GetIdentityKeyResponse {
    /// Result containing the key store (if it exists)
    pub result: Result<Option<LocalKeyStore>, KeyError>,
}

// ============================================================================
// Machine Key Request/Response Types
// ============================================================================

/// Create machine key request.
///
/// Creates a new machine key record. Requires identity key to be registered first.
/// Machine keys are derived from the user's Neural Key using:
/// - 1 external shard (from paper backup)
/// - Password (to decrypt 2 stored shards from keystore)
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CreateMachineKeyRequest {
    /// User ID
    #[serde(with = "u128_hex_string")]
    pub user_id: UserId,
    /// Optional human-readable machine name
    pub machine_name: Option<String>,
    /// Machine key capabilities
    pub capabilities: MachineKeyCapabilities,
    /// Key scheme to use (defaults to Classical)
    #[serde(default)]
    pub key_scheme: KeyScheme,
    /// Single external Neural shard (from paper backup)
    pub external_shard: NeuralShard,
    /// Password to decrypt stored shards
    pub password: String,
}

/// Create machine key response.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CreateMachineKeyResponse {
    /// Result containing the created machine record
    pub result: Result<MachineKeyRecord, KeyError>,
}

/// List machine keys request.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ListMachineKeysRequest {
    /// User ID to list machines for
    #[serde(with = "u128_hex_string")]
    pub user_id: UserId,
}

/// List machine keys response.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ListMachineKeysResponse {
    /// List of machine key records
    pub machines: Vec<MachineKeyRecord>,
}

/// Get machine key request.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GetMachineKeyRequest {
    /// User ID
    #[serde(with = "u128_hex_string")]
    pub user_id: UserId,
    /// Machine ID to retrieve (hex string for JavaScript interop)
    #[serde(with = "u128_hex_string")]
    pub machine_id: u128,
}

/// Get machine key response.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GetMachineKeyResponse {
    /// Result containing the machine record (if it exists)
    pub result: Result<Option<MachineKeyRecord>, KeyError>,
}

/// Revoke machine key request.
///
/// Cannot revoke the primary/current machine key.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RevokeMachineKeyRequest {
    /// User ID
    #[serde(with = "u128_hex_string")]
    pub user_id: UserId,
    /// Machine ID to revoke (hex string for JavaScript interop)
    #[serde(with = "u128_hex_string")]
    pub machine_id: u128,
}

/// Revoke machine key response.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RevokeMachineKeyResponse {
    /// Result of the revocation
    pub result: Result<(), KeyError>,
}

/// Rotate machine key request.
///
/// Rotates the keys for a machine, incrementing the epoch.
/// The service will generate new keys using entropy - no public keys needed in request.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RotateMachineKeyRequest {
    /// User ID
    #[serde(with = "u128_hex_string")]
    pub user_id: UserId,
    /// Machine ID to rotate keys for (hex string for JavaScript interop)
    #[serde(with = "u128_hex_string")]
    pub machine_id: u128,
}

/// Rotate machine key response.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RotateMachineKeyResponse {
    /// Result containing the updated machine record
    pub result: Result<MachineKeyRecord, KeyError>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::string::ToString;

    #[test]
    fn test_key_request_types() {
        // Test RegisterIdentityKeyRequest
        let reg_req = RegisterIdentityKeyRequest {
            user_id: 1,
            identity_signing_public_key: [0u8; 32],
            machine_signing_public_key: [1u8; 32],
            machine_encryption_public_key: [2u8; 32],
        };
        assert_eq!(reg_req.user_id, 1);

        // Test GetIdentityKeyRequest
        let get_req = GetIdentityKeyRequest { user_id: 2 };
        assert_eq!(get_req.user_id, 2);

        // Test CreateMachineKeyRequest
        let create_req = CreateMachineKeyRequest {
            user_id: 3,
            machine_name: Some(String::from("My Laptop")),
            capabilities: MachineKeyCapabilities::default(),
            key_scheme: crate::keystore::KeyScheme::default(),
            external_shard: NeuralShard { index: 1, hex: "abc123".to_string() },
            password: "secure-password-123".to_string(),
        };
        assert_eq!(create_req.user_id, 3);
        assert!(create_req.machine_name.is_some());
        assert_eq!(create_req.external_shard.index, 1);

        // Test ListMachineKeysRequest
        let list_req = ListMachineKeysRequest { user_id: 4 };
        assert_eq!(list_req.user_id, 4);

        // Test RevokeMachineKeyRequest
        let revoke_req = RevokeMachineKeyRequest {
            user_id: 5,
            machine_id: 100,
        };
        assert_eq!(revoke_req.user_id, 5);
        assert_eq!(revoke_req.machine_id, 100);

        // Test RotateMachineKeyRequest
        let rotate_req = RotateMachineKeyRequest {
            user_id: 6,
            machine_id: 200,
        };
        assert_eq!(rotate_req.user_id, 6);
        assert_eq!(rotate_req.machine_id, 200);
    }
}
