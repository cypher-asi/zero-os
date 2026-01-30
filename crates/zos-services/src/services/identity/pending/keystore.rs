//! Keystore pending operations

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;
use zos_identity::ipc::{CreateMachineKeyAndEnrollRequest, CreateMachineKeyRequest, NeuralKeyGenerated};
use zos_identity::keystore::MachineKeyRecord;

use super::RequestContext;

/// Tracks pending keystore operations awaiting results.
///
/// These operations communicate with KeystoreService (PID 7) for secure
/// cryptographic key storage, keeping keys isolated from the general filesystem.
#[derive(Clone)]
pub enum PendingKeystoreOp {
    // =========================================================================
    // Identity Key operations (stored in keystore)
    // =========================================================================
    /// Check if identity key exists (for generate)
    CheckKeyExists {
        ctx: RequestContext,
        user_id: u128,
        /// Password for encrypting shards (new field for encrypted shard flow)
        password: String,
    },
    /// Write identity key store to keystore
    WriteKeyStore {
        ctx: RequestContext,
        user_id: u128,
        result: NeuralKeyGenerated,
        json_bytes: Vec<u8>,
        /// Encrypted shard store to write after key store succeeds
        encrypted_shards_json: Vec<u8>,
    },
    /// Write encrypted shards after key store write succeeds
    WriteEncryptedShards {
        ctx: RequestContext,
        user_id: u128,
        result: NeuralKeyGenerated,
    },
    /// Best-effort rollback if encrypted shard write fails
    DeleteIdentityKeyAfterShardFailure {
        ctx: RequestContext,
        user_id: u128,
    },
    /// Get identity key from keystore
    GetIdentityKey {
        ctx: RequestContext,
    },
    /// Read existing identity for recovery verification (SECURITY)
    ReadIdentityForRecovery {
        ctx: RequestContext,
        user_id: u128,
        zid_shards: Vec<zos_identity::crypto::ZidNeuralShard>,
    },
    /// Write recovered key store to keystore
    WriteRecoveredKeyStore {
        ctx: RequestContext,
        user_id: u128,
        result: NeuralKeyGenerated,
        json_bytes: Vec<u8>,
    },

    // =========================================================================
    // Machine Key operations (stored in keystore)
    // =========================================================================
    /// Read identity key for create machine key
    ReadIdentityForMachine {
        ctx: RequestContext,
        request: CreateMachineKeyRequest,
    },
    /// Read encrypted shards for machine key creation
    ReadEncryptedShardsForMachine {
        ctx: RequestContext,
        request: CreateMachineKeyRequest,
        /// Stored identity public key for verification
        stored_identity_pubkey: [u8; 32],
    },
    /// Write machine key to keystore
    WriteMachineKey {
        ctx: RequestContext,
        user_id: u128,
        record: MachineKeyRecord,
        json_bytes: Vec<u8>,
    },
    /// List machine keys from keystore
    ListMachineKeys {
        ctx: RequestContext,
        user_id: u128,
    },
    /// Read individual machine key record from keystore
    ReadMachineKey {
        ctx: RequestContext,
        user_id: u128,
        remaining_paths: Vec<String>,
        records: Vec<MachineKeyRecord>,
    },
    /// Delete machine key from keystore
    DeleteMachineKey {
        ctx: RequestContext,
        user_id: u128,
        machine_id: u128,
    },
    /// Read machine key for rotation
    ReadMachineForRotate {
        ctx: RequestContext,
        user_id: u128,
        machine_id: u128,
    },
    /// Write rotated machine key to keystore
    WriteRotatedMachineKey {
        ctx: RequestContext,
        user_id: u128,
        record: MachineKeyRecord,
        json_bytes: Vec<u8>,
    },
    /// Read single machine key by ID
    ReadSingleMachineKey {
        ctx: RequestContext,
    },

    // =========================================================================
    // ZID session operations (stored in keystore)
    // =========================================================================
    /// List machine keys for ZID login
    ListMachineKeysForZidLogin {
        ctx: RequestContext,
        user_id: u128,
        zid_endpoint: String,
    },
    /// Read machine key for ZID login
    ReadMachineKeyForZidLogin {
        ctx: RequestContext,
        user_id: u128,
        zid_endpoint: String,
    },
    /// List machine keys for ZID enrollment
    ListMachineKeysForZidEnroll {
        ctx: RequestContext,
        user_id: u128,
        zid_endpoint: String,
    },
    /// Read machine key for ZID enrollment
    ReadMachineKeyForZidEnroll {
        ctx: RequestContext,
        user_id: u128,
        zid_endpoint: String,
    },

    // =========================================================================
    // Combined Machine Key + ZID Enrollment operations
    // =========================================================================
    /// Read identity key for combined machine key creation + enrollment
    ReadIdentityForMachineEnroll {
        ctx: RequestContext,
        request: CreateMachineKeyAndEnrollRequest,
    },
    /// Read encrypted shards for combined machine key creation + enrollment
    ReadEncryptedShardsForMachineEnroll {
        ctx: RequestContext,
        request: CreateMachineKeyAndEnrollRequest,
        /// Stored identity public key for verification
        stored_identity_pubkey: [u8; 32],
        /// User ID used for identity key derivation (may differ from request.user_id)
        /// Required for verification to re-derive the same pubkey
        derivation_user_id: u128,
    },
    /// Write machine key after derivation, before ZID enrollment
    WriteMachineKeyForEnroll {
        ctx: RequestContext,
        user_id: u128,
        record: MachineKeyRecord,
        json_bytes: Vec<u8>,
        /// ZID endpoint for enrollment after write succeeds
        zid_endpoint: String,
        /// Identity signing public key (for ZID enrollment)
        identity_signing_public_key: [u8; 32],
        /// Identity signing SK seed (for ZID enrollment authorization signature)
        identity_signing_sk: [u8; 32],
        /// Machine signing SK (for ZID enrollment signing)
        machine_signing_sk: [u8; 32],
        /// Machine encryption SK (for storage)
        machine_encryption_sk: [u8; 32],
    },
}
