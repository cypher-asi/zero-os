//! Identity Service (PID 3)
//!
//! The IdentityService manages user cryptographic identities. It:
//! - Generates Neural Keys (entropy, key derivation, Shamir splitting)
//! - Stores public keys to VFS
//! - Handles key recovery from Shamir shards
//! - Manages machine key records
//!
//! # Protocol
//!
//! Apps communicate with IdentityService via IPC:
//!
//! - `MSG_GENERATE_NEURAL_KEY (0x7054)`: Generate a new Neural Key
//! - `MSG_RECOVER_NEURAL_KEY (0x7056)`: Recover from shards
//! - `MSG_GET_IDENTITY_KEY (0x7052)`: Get stored public keys
//! - `MSG_CREATE_MACHINE_KEY (0x7060)`: Create machine record
//! - `MSG_LIST_MACHINE_KEYS (0x7062)`: List all machines
//! - `MSG_REVOKE_MACHINE_KEY (0x7066)`: Delete machine record
//! - `MSG_ROTATE_MACHINE_KEY (0x7068)`: Update machine keys

#![cfg_attr(target_arch = "wasm32", no_main)]

extern crate alloc;

use alloc::format;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;
use zos_apps::manifest::IDENTITY_SERVICE_MANIFEST;
use zos_apps::syscall;
use zos_apps::{app_main, AppContext, AppError, AppManifest, ControlFlow, Message, ZeroApp};
use zos_identity::ipc::{
    key_msg, GenerateNeuralKeyRequest, GenerateNeuralKeyResponse, GetIdentityKeyRequest,
    GetIdentityKeyResponse, NeuralKeyGenerated, NeuralShard, PublicIdentifiers,
    RecoverNeuralKeyRequest, RecoverNeuralKeyResponse,
};
use zos_identity::keystore::LocalKeyStore;
use zos_identity::KeyError;

// =============================================================================
// Crypto Helpers (simplified for WASM - production would use proper crates)
// =============================================================================

/// Generate random bytes using the kernel's getrandom
fn generate_random_bytes(len: usize) -> Vec<u8> {
    // Use wallclock and PID for entropy source in WASM
    // In production, this would use the getrandom syscall
    let mut bytes = vec![0u8; len];
    let time = syscall::get_wallclock();
    let pid = syscall::get_pid();
    
    // Simple PRNG seeded with time and PID
    let mut state = time ^ ((pid as u64) << 32);
    for byte in bytes.iter_mut() {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        *byte = (state >> 56) as u8;
    }
    bytes
}

/// Convert bytes to hex string
fn bytes_to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

/// Convert hex string to bytes
fn hex_to_bytes(hex: &str) -> Result<Vec<u8>, &'static str> {
    if hex.len() % 2 != 0 {
        return Err("Invalid hex length");
    }
    (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).map_err(|_| "Invalid hex"))
        .collect()
}

/// Simple Shamir secret sharing (3-of-5) - mock implementation
/// Production would use a proper Shamir library
fn shamir_split(secret: &[u8], threshold: usize, shares: usize) -> Vec<NeuralShard> {
    let _ = threshold; // Would be used in real implementation
    let mut shards = Vec::with_capacity(shares);
    
    for i in 1..=shares {
        // Generate a shard by XORing secret with deterministic "random" data
        let mut shard_bytes = Vec::with_capacity(secret.len() + 1);
        shard_bytes.push(i as u8); // Shard index
        
        // Generate deterministic padding based on index
        let mut state = (i as u64).wrapping_mul(0x9E3779B97F4A7C15);
        for &byte in secret.iter() {
            state = state.wrapping_mul(6364136223846793005).wrapping_add(i as u64);
            shard_bytes.push(byte ^ (state >> 56) as u8);
        }
        
        shards.push(NeuralShard {
            index: i as u8,
            hex: bytes_to_hex(&shard_bytes),
        });
    }
    
    shards
}

/// Reconstruct secret from shards (mock implementation)
fn shamir_reconstruct(shards: &[NeuralShard]) -> Result<Vec<u8>, KeyError> {
    if shards.len() < 3 {
        return Err(KeyError::InsufficientShards);
    }
    
    // Use first shard to reconstruct (simplified - real Shamir uses polynomial interpolation)
    let shard = &shards[0];
    let shard_bytes = hex_to_bytes(&shard.hex)
        .map_err(|e| KeyError::InvalidShard(String::from(e)))?;
    
    if shard_bytes.is_empty() {
        return Err(KeyError::InvalidShard(String::from("Empty shard")));
    }
    
    let index = shard_bytes[0] as u64;
    let mut secret = Vec::with_capacity(shard_bytes.len() - 1);
    
    // Reverse the XOR operation
    let mut state = index.wrapping_mul(0x9E3779B97F4A7C15);
    for &byte in &shard_bytes[1..] {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(index);
        secret.push(byte ^ (state >> 56) as u8);
    }
    
    Ok(secret)
}

/// Derive a public key from entropy with a salt (mock Ed25519/X25519)
fn derive_public_key(entropy: &[u8], salt: &str) -> [u8; 32] {
    let mut combined = Vec::with_capacity(entropy.len() + salt.len());
    combined.extend_from_slice(entropy);
    combined.extend_from_slice(salt.as_bytes());
    
    // XOR fold to 32 bytes (mock derivation)
    let mut public_key = [0u8; 32];
    for (i, &byte) in combined.iter().enumerate() {
        public_key[i % 32] ^= byte;
    }
    
    // Add more mixing
    for i in 0..32 {
        let state = (public_key[i] as u64)
            .wrapping_mul(0x9E3779B97F4A7C15)
            .wrapping_add(i as u64);
        public_key[i] = (state >> 56) as u8;
    }
    
    public_key
}

// =============================================================================
// IdentityService Application
// =============================================================================

/// IdentityService - manages user cryptographic identities
#[derive(Default)]
pub struct IdentityService {
    /// Whether we have registered with init
    registered: bool,
}

impl IdentityService {
    /// Handle MSG_GENERATE_NEURAL_KEY
    fn handle_generate_neural_key(
        &mut self,
        ctx: &AppContext,
        msg: &Message,
    ) -> Result<(), AppError> {
        syscall::debug("IdentityService: Handling generate neural key request");

        // Parse request
        let request: GenerateNeuralKeyRequest = match serde_json::from_slice(&msg.data) {
            Ok(r) => r,
            Err(e) => {
                syscall::debug(&format!("IdentityService: Failed to parse request: {}", e));
                return self.send_neural_key_error(ctx, msg.from_pid, KeyError::DerivationFailed);
            }
        };

        let user_id = request.user_id;
        syscall::debug(&format!(
            "IdentityService: Generating Neural Key for user {:032x}",
            user_id
        ));

        // Check if key already exists
        let key_path = LocalKeyStore::storage_path(user_id);
        match syscall::vfs_exists(&key_path) {
            Ok(true) => {
                syscall::debug("IdentityService: Neural Key already exists");
                return self.send_neural_key_error(
                    ctx,
                    msg.from_pid,
                    KeyError::IdentityKeyAlreadyExists,
                );
            }
            Ok(false) => {}
            Err(e) => {
                syscall::debug(&format!("IdentityService: VFS exists check failed: {}", e));
                // Continue - might just mean the directory doesn't exist yet
            }
        }

        // Generate 32 bytes of entropy
        let entropy = generate_random_bytes(32);
        syscall::debug(&format!(
            "IdentityService: Generated {} bytes of entropy",
            entropy.len()
        ));

        // Derive keypairs
        let identity_signing = derive_public_key(&entropy, "identity-signing");
        let machine_signing = derive_public_key(&entropy, "machine-signing");
        let machine_encryption = derive_public_key(&entropy, "machine-encryption");

        // Split entropy into 5 Shamir shards with threshold 3
        let shards = shamir_split(&entropy, 3, 5);
        syscall::debug(&format!(
            "IdentityService: Split entropy into {} shards",
            shards.len()
        ));

        // Create public identifiers
        let public_identifiers = PublicIdentifiers {
            identity_signing_pub_key: format!("0x{}", bytes_to_hex(&identity_signing)),
            machine_signing_pub_key: format!("0x{}", bytes_to_hex(&machine_signing)),
            machine_encryption_pub_key: format!("0x{}", bytes_to_hex(&machine_encryption)),
        };

        // Create LocalKeyStore
        let key_store = LocalKeyStore::new(
            user_id,
            identity_signing,
            machine_signing,
            machine_encryption,
        );

        // Ensure directory exists
        let identity_dir = format!("/home/{:032x}/.zos/identity", user_id);
        let _ = syscall::vfs_mkdir(&format!("/home/{:032x}", user_id));
        let _ = syscall::vfs_mkdir(&format!("/home/{:032x}/.zos", user_id));
        let _ = syscall::vfs_mkdir(&identity_dir);

        // Store public keys to VFS
        match serde_json::to_vec(&key_store) {
            Ok(json_bytes) => {
                if let Err(e) = syscall::vfs_write(&key_path, &json_bytes) {
                    syscall::debug(&format!("IdentityService: Failed to write keys: {}", e));
                    return self.send_neural_key_error(
                        ctx,
                        msg.from_pid,
                        KeyError::StorageError(format!("VFS write failed: {}", e)),
                    );
                }
                syscall::debug(&format!(
                    "IdentityService: Stored public keys to {}",
                    key_path
                ));
            }
            Err(e) => {
                syscall::debug(&format!("IdentityService: Failed to serialize keys: {}", e));
                return self.send_neural_key_error(
                    ctx,
                    msg.from_pid,
                    KeyError::StorageError(format!("Serialization failed: {}", e)),
                );
            }
        }

        let created_at = syscall::get_wallclock();

        // Send success response with shards
        let result = NeuralKeyGenerated {
            public_identifiers,
            shards,
            created_at,
        };

        let response = GenerateNeuralKeyResponse { result: Ok(result) };
        self.send_response(ctx, msg.from_pid, key_msg::MSG_GENERATE_NEURAL_KEY_RESPONSE, &response)
    }

    /// Handle MSG_RECOVER_NEURAL_KEY
    fn handle_recover_neural_key(
        &mut self,
        ctx: &AppContext,
        msg: &Message,
    ) -> Result<(), AppError> {
        syscall::debug("IdentityService: Handling recover neural key request");

        // Parse request
        let request: RecoverNeuralKeyRequest = match serde_json::from_slice(&msg.data) {
            Ok(r) => r,
            Err(e) => {
                syscall::debug(&format!("IdentityService: Failed to parse request: {}", e));
                return self.send_recover_key_error(ctx, msg.from_pid, KeyError::DerivationFailed);
            }
        };

        let user_id = request.user_id;
        let shards = request.shards;

        syscall::debug(&format!(
            "IdentityService: Recovering Neural Key for user {:032x} with {} shards",
            user_id,
            shards.len()
        ));

        if shards.len() < 3 {
            return self.send_recover_key_error(ctx, msg.from_pid, KeyError::InsufficientShards);
        }

        // Reconstruct entropy from shards
        let entropy = match shamir_reconstruct(&shards) {
            Ok(e) => e,
            Err(e) => {
                syscall::debug(&format!("IdentityService: Shard reconstruction failed: {:?}", e));
                return self.send_recover_key_error(ctx, msg.from_pid, e);
            }
        };

        // Re-derive keypairs
        let identity_signing = derive_public_key(&entropy, "identity-signing");
        let machine_signing = derive_public_key(&entropy, "machine-signing");
        let machine_encryption = derive_public_key(&entropy, "machine-encryption");

        // Create public identifiers
        let public_identifiers = PublicIdentifiers {
            identity_signing_pub_key: format!("0x{}", bytes_to_hex(&identity_signing)),
            machine_signing_pub_key: format!("0x{}", bytes_to_hex(&machine_signing)),
            machine_encryption_pub_key: format!("0x{}", bytes_to_hex(&machine_encryption)),
        };

        // Create and store LocalKeyStore
        let key_store = LocalKeyStore::new(
            user_id,
            identity_signing,
            machine_signing,
            machine_encryption,
        );

        let key_path = LocalKeyStore::storage_path(user_id);

        // Ensure directory exists
        let identity_dir = format!("/home/{:032x}/.zos/identity", user_id);
        let _ = syscall::vfs_mkdir(&format!("/home/{:032x}", user_id));
        let _ = syscall::vfs_mkdir(&format!("/home/{:032x}/.zos", user_id));
        let _ = syscall::vfs_mkdir(&identity_dir);

        // Store public keys to VFS
        match serde_json::to_vec(&key_store) {
            Ok(json_bytes) => {
                if let Err(e) = syscall::vfs_write(&key_path, &json_bytes) {
                    syscall::debug(&format!("IdentityService: Failed to write keys: {}", e));
                    return self.send_recover_key_error(
                        ctx,
                        msg.from_pid,
                        KeyError::StorageError(format!("VFS write failed: {}", e)),
                    );
                }
            }
            Err(e) => {
                return self.send_recover_key_error(
                    ctx,
                    msg.from_pid,
                    KeyError::StorageError(format!("Serialization failed: {}", e)),
                );
            }
        }

        let created_at = syscall::get_wallclock();

        // Re-split entropy for new shards
        let new_shards = shamir_split(&entropy, 3, 5);

        let result = NeuralKeyGenerated {
            public_identifiers,
            shards: new_shards,
            created_at,
        };

        let response = RecoverNeuralKeyResponse { result: Ok(result) };
        self.send_response(ctx, msg.from_pid, key_msg::MSG_RECOVER_NEURAL_KEY_RESPONSE, &response)
    }

    /// Handle MSG_GET_IDENTITY_KEY
    fn handle_get_identity_key(
        &mut self,
        ctx: &AppContext,
        msg: &Message,
    ) -> Result<(), AppError> {
        syscall::debug("IdentityService: Handling get identity key request");

        // Parse request
        let request: GetIdentityKeyRequest = match serde_json::from_slice(&msg.data) {
            Ok(r) => r,
            Err(e) => {
                syscall::debug(&format!("IdentityService: Failed to parse request: {}", e));
                let response = GetIdentityKeyResponse {
                    result: Err(KeyError::DerivationFailed),
                };
                return self.send_response(
                    ctx,
                    msg.from_pid,
                    key_msg::MSG_GET_IDENTITY_KEY_RESPONSE,
                    &response,
                );
            }
        };

        let user_id = request.user_id;
        let key_path = LocalKeyStore::storage_path(user_id);

        syscall::debug(&format!(
            "IdentityService: Getting identity key for user {:032x}",
            user_id
        ));

        // Read from VFS
        let response = match syscall::vfs_read(&key_path) {
            Ok(data) => {
                match serde_json::from_slice::<LocalKeyStore>(&data) {
                    Ok(key_store) => GetIdentityKeyResponse {
                        result: Ok(Some(key_store)),
                    },
                    Err(e) => {
                        syscall::debug(&format!("IdentityService: Failed to parse stored keys: {}", e));
                        GetIdentityKeyResponse {
                            result: Err(KeyError::StorageError(format!("Parse failed: {}", e))),
                        }
                    }
                }
            }
            Err(_) => {
                // Key not found
                GetIdentityKeyResponse {
                    result: Ok(None),
                }
            }
        };

        self.send_response(ctx, msg.from_pid, key_msg::MSG_GET_IDENTITY_KEY_RESPONSE, &response)
    }

    /// Send a serialized response
    fn send_response<T: serde::Serialize>(
        &self,
        ctx: &AppContext,
        _to_pid: u32,
        tag: u32,
        response: &T,
    ) -> Result<(), AppError> {
        match serde_json::to_vec(response) {
            Ok(data) => {
                if let Some(slot) = ctx.ui_endpoint {
                    syscall::send(slot, tag, &data)
                        .map_err(|e| AppError::IpcError(format!("Send failed: {}", e)))?;
                }
                // Also send via debug for supervisor to route
                let hex: String = data.iter().map(|b| format!("{:02x}", b)).collect();
                syscall::debug(&format!("IDENTITY:RESPONSE:{}:{:08x}:{}", _to_pid, tag, hex));
                Ok(())
            }
            Err(e) => {
                syscall::debug(&format!("IdentityService: Failed to serialize response: {}", e));
                Err(AppError::IpcError(format!("Serialization failed: {}", e)))
            }
        }
    }

    /// Send error response for generate neural key
    fn send_neural_key_error(
        &self,
        ctx: &AppContext,
        to_pid: u32,
        error: KeyError,
    ) -> Result<(), AppError> {
        let response = GenerateNeuralKeyResponse { result: Err(error) };
        self.send_response(ctx, to_pid, key_msg::MSG_GENERATE_NEURAL_KEY_RESPONSE, &response)
    }

    /// Send error response for recover neural key
    fn send_recover_key_error(
        &self,
        ctx: &AppContext,
        to_pid: u32,
        error: KeyError,
    ) -> Result<(), AppError> {
        let response = RecoverNeuralKeyResponse { result: Err(error) };
        self.send_response(ctx, to_pid, key_msg::MSG_RECOVER_NEURAL_KEY_RESPONSE, &response)
    }
}

impl ZeroApp for IdentityService {
    fn manifest() -> &'static AppManifest {
        &IDENTITY_SERVICE_MANIFEST
    }

    fn init(&mut self, ctx: &AppContext) -> Result<(), AppError> {
        syscall::debug(&format!("IdentityService starting (PID {})", ctx.pid));

        // Register with init as "identity" service
        let service_name = "identity";
        let name_bytes = service_name.as_bytes();
        let mut data = Vec::with_capacity(1 + name_bytes.len() + 8);
        data.push(name_bytes.len() as u8);
        data.extend_from_slice(name_bytes);
        // Endpoint ID (placeholder - would be actual endpoint)
        data.extend_from_slice(&0u32.to_le_bytes());
        data.extend_from_slice(&0u32.to_le_bytes());

        // Send to init's endpoint (slot 2 is typically init)
        let _ = syscall::send(syscall::INIT_ENDPOINT_SLOT, syscall::MSG_REGISTER_SERVICE, &data);
        self.registered = true;

        syscall::debug("IdentityService: Registered with init");

        Ok(())
    }

    fn update(&mut self, _ctx: &AppContext) -> ControlFlow {
        ControlFlow::Yield
    }

    fn on_message(&mut self, ctx: &AppContext, msg: Message) -> Result<(), AppError> {
        syscall::debug(&format!(
            "IdentityService: Received message tag 0x{:x} from PID {}",
            msg.tag, msg.from_pid
        ));

        match msg.tag {
            key_msg::MSG_GENERATE_NEURAL_KEY => self.handle_generate_neural_key(ctx, &msg),
            key_msg::MSG_RECOVER_NEURAL_KEY => self.handle_recover_neural_key(ctx, &msg),
            key_msg::MSG_GET_IDENTITY_KEY => self.handle_get_identity_key(ctx, &msg),
            // TODO: Implement machine key handlers
            key_msg::MSG_CREATE_MACHINE_KEY => {
                syscall::debug("IdentityService: MSG_CREATE_MACHINE_KEY not yet implemented");
                Ok(())
            }
            key_msg::MSG_LIST_MACHINE_KEYS => {
                syscall::debug("IdentityService: MSG_LIST_MACHINE_KEYS not yet implemented");
                Ok(())
            }
            key_msg::MSG_REVOKE_MACHINE_KEY => {
                syscall::debug("IdentityService: MSG_REVOKE_MACHINE_KEY not yet implemented");
                Ok(())
            }
            key_msg::MSG_ROTATE_MACHINE_KEY => {
                syscall::debug("IdentityService: MSG_ROTATE_MACHINE_KEY not yet implemented");
                Ok(())
            }
            _ => {
                syscall::debug(&format!(
                    "IdentityService: Unknown message tag 0x{:x} from PID {}",
                    msg.tag, msg.from_pid
                ));
                Ok(())
            }
        }
    }

    fn shutdown(&mut self, _ctx: &AppContext) {
        syscall::debug("IdentityService: shutting down");
    }
}

// Entry point
app_main!(IdentityService);

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    println!("IdentityService is meant to run as WASM in Zero OS");
}
