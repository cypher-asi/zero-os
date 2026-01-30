//! Neural key generation handlers

extern crate alloc;

use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

use crate::services::identity::utils::bytes_to_hex;
use crate::services::identity::pending::{PendingKeystoreOp, PendingStorageOp, RequestContext};
use crate::services::identity::response;
use crate::services::identity::{check_user_authorization, log_denial, AuthResult, IdentityService};
use zos_identity::crypto::{
    create_kdf_params, derive_identity_signing_keypair, derive_key_from_password_public,
    encrypt_shard_with_key, select_shards_to_encrypt, split_neural_key, validate_password,
    NeuralKey,
};
use zos_apps::syscall;
use zos_apps::{AppError, Message};
use zos_identity::ipc::{GenerateNeuralKeyRequest, NeuralKeyGenerated, NeuralShard, PublicIdentifiers};
use zos_identity::keystore::{EncryptedShardStore, LocalKeyStore};
use zos_identity::KeyError;
use uuid::Uuid;

use super::derive_user_id_from_pubkey;

/// Continue neural key generation after checking if identity directory exists
pub fn continue_generate_after_directory_check(
    service: &mut IdentityService,
    client_pid: u32,
    user_id: u128,
    exists: bool,
    password: String,
    cap_slots: Vec<u32>,
) -> Result<(), AppError> {
    let ctx = RequestContext::new(client_pid, cap_slots);
    
    if exists {
        // Directory exists, proceed to check if key already exists (via Keystore)
        syscall::debug(&format!(
            "IdentityService: Identity directory exists for user {:032x}",
            user_id
        ));
        let key_path = LocalKeyStore::storage_path(user_id);
        // Invariant 32: /keys/ paths use Keystore IPC, not VFS
        return service.start_keystore_exists(
            &key_path,
            PendingKeystoreOp::CheckKeyExists { ctx, user_id, password },
        );
    }

    // Directory doesn't exist, create it with create_parents=true
    // This creates the entire directory structure in a single VFS operation
    syscall::debug(&format!(
        "IdentityService: Creating identity directory structure for user {}",
        user_id
    ));

    // Create the deepest directory path - VFS will create all parents
    let identity_dir = format!("/home/{}/.zos/identity", user_id);

    service.start_vfs_mkdir(
        &identity_dir,
        true, // create_parents = true - creates all parent directories
        PendingStorageOp::CreateIdentityDirectoryComplete {
            ctx,
            user_id,
            password,
        },
    )
}

/// Continue creating directories after VFS mkdir completes.
/// Creates directories one at a time since VFS does not support create_parents.
pub fn continue_create_directories(
    service: &mut IdentityService,
    client_pid: u32,
    user_id: u128,
    directories: Vec<String>,
    password: String,
    cap_slots: Vec<u32>,
) -> Result<(), AppError> {
    let ctx = RequestContext::new(client_pid, cap_slots);

    if directories.is_empty() {
        // All directories created, proceed to check if key already exists (via Keystore)
        let key_path = LocalKeyStore::storage_path(user_id);
        syscall::debug(&format!(
            "IdentityService: Directories created, checking if key exists at {}",
            key_path
        ));
        // Invariant 32: /keys/ paths use Keystore IPC, not VFS
        return service.start_keystore_exists(
            &key_path,
            PendingKeystoreOp::CheckKeyExists { ctx, user_id, password },
        );
    }

    // Create the next directory in the list
    let next_dir = directories[0].clone();
    let remaining_dirs: Vec<String> = directories[1..].to_vec();

    syscall::debug(&format!(
        "IdentityService: Creating directory {} ({} remaining)",
        next_dir,
        remaining_dirs.len()
    ));

    service.start_vfs_mkdir(
        &next_dir,
        false, // create_parents = false (not supported by VFS)
        PendingStorageOp::CreateIdentityDirectory {
            ctx,
            user_id,
            directories: remaining_dirs,
            password,
        },
    )
}

pub fn handle_generate_neural_key(
    service: &mut IdentityService,
    msg: &Message,
) -> Result<(), AppError> {
    syscall::debug("IdentityService: Handling generate neural key request");

    // Rule 1: Parse request - return InvalidRequest on parse failure
    let request: GenerateNeuralKeyRequest = match serde_json::from_slice(&msg.data) {
        Ok(r) => r,
        Err(e) => {
            syscall::debug(&format!("IdentityService: Failed to parse request: {}", e));
            return response::send_neural_key_error(
                msg.from_pid,
                &msg.cap_slots,
                KeyError::InvalidRequest(format!("JSON parse error: {}", e)),
            );
        }
    };

    // Rule 4: Authorization check (FAIL-CLOSED)
    if check_user_authorization(msg.from_pid, request.user_id) == AuthResult::Denied {
        log_denial("generate_neural_key", msg.from_pid, request.user_id);
        return response::send_neural_key_error(
            msg.from_pid,
            &msg.cap_slots,
            KeyError::Unauthorized,
        );
    }

    // Validate password before proceeding
    if let Err(e) = validate_password(&request.password) {
        syscall::debug(&format!("IdentityService: Password validation failed: {:?}", e));
        return response::send_neural_key_error(msg.from_pid, &msg.cap_slots, e);
    }

    let user_id = request.user_id;
    let password = request.password;
    syscall::debug(&format!(
        "IdentityService: Generating Neural Key for user {:032x}",
        user_id
    ));

    // First, ensure the identity directory structure exists (via VFS)
    let identity_dir = format!("/home/{}/.zos/identity", user_id);
    let ctx = RequestContext::new(msg.from_pid, msg.cap_slots.clone());
    service.start_vfs_exists(
        &identity_dir,
        PendingStorageOp::CheckIdentityDirectory { ctx, user_id, password },
    )
}

pub fn continue_generate_after_exists_check(
    service: &mut IdentityService,
    client_pid: u32,
    user_id: u128,
    exists: bool,
    password: String,
    cap_slots: Vec<u32>,
) -> Result<(), AppError> {
    let ctx = RequestContext::new(client_pid, cap_slots);
    
    if exists {
        syscall::debug("IdentityService: Neural Key already exists");
        return response::send_neural_key_error(
            ctx.client_pid,
            &ctx.cap_slots,
            KeyError::IdentityKeyAlreadyExists,
        );
    }

    // Generate Neural Key and split into shards
    let (neural_key, identity_signing) = generate_neural_key_and_identity(user_id, &ctx)?;
    
    // Split into shards and select which to encrypt
    let (all_shards, encrypted_indices, external_indices) = 
        split_and_select_shards(&neural_key, &ctx)?;
    
    // Create KDF and derive encryption key
    let (kdf, derived_key) = create_kdf_and_derive_key(&password, &ctx)?;
    
    // Encrypt selected shards
    let encrypted_shards = encrypt_selected_shards(
        &all_shards, &encrypted_indices, &derived_key, &ctx
    )?;
    
    // Build result and prepare for storage
    let (result, key_store, encrypted_shard_store) = build_generation_result(
        user_id,
        &identity_signing,
        &all_shards,
        &external_indices,
        encrypted_shards,
        kdf,
    );
    
    // Serialize and initiate storage
    store_generated_keys(service, ctx, result, key_store, encrypted_shard_store)
}

// ============================================================================
// Helper functions for continue_generate_after_exists_check
// ============================================================================

/// Generate Neural Key and derive identity signing keypair
fn generate_neural_key_and_identity(
    user_id: u128,
    ctx: &RequestContext,
) -> Result<(NeuralKey, [u8; 32]), AppError> {
    syscall::debug("IdentityService: Calling NeuralKey::generate() - uses getrandom for entropy");
    let neural_key = match NeuralKey::generate() {
        Ok(key) => {
            // Rule 10: NEVER log key material. Only verify entropy quality.
            let bytes = key.as_bytes();
            let all_zeros = bytes.iter().all(|&b| b == 0);
            if all_zeros {
                syscall::debug("IdentityService: WARNING - NeuralKey::generate() returned all zeros! Entropy source may be broken");
            } else {
                syscall::debug("IdentityService: NeuralKey::generate() success - entropy validated");
            }
            key
        }
        Err(e) => {
            syscall::debug(&format!(
                "IdentityService: CRITICAL - NeuralKey::generate() FAILED! Error: {:?}",
                e
            ));
            syscall::debug("IdentityService: This usually means getrandom could not access crypto.getRandomValues");
            syscall::debug("IdentityService: Check browser console for wasm-bindgen import shim errors");
            response::send_neural_key_error(
                ctx.client_pid,
                &ctx.cap_slots,
                KeyError::CryptoError(format!("Neural Key generation failed: {:?}", e)),
            )?;
            return Err(AppError::Internal("Neural Key generation failed".into()));
        }
    };

    // Derive identity signing keypair (canonical way)
    let temp_identity_id = Uuid::from_u128(user_id);
    let (identity_signing, _identity_keypair) =
        match derive_identity_signing_keypair(&neural_key, &temp_identity_id) {
            Ok(keypair) => keypair,
            Err(e) => {
                response::send_neural_key_error(
                    ctx.client_pid,
                    &ctx.cap_slots,
                    KeyError::CryptoError(format!(
                        "Identity key derivation failed during generation: {:?}",
                        e
                    )),
                )?;
                return Err(AppError::Internal("Identity key derivation failed".into()));
            }
        };

    Ok((neural_key, identity_signing))
}

/// Split Neural Key into shards and select which to encrypt
fn split_and_select_shards(
    neural_key: &NeuralKey,
    ctx: &RequestContext,
) -> Result<(Vec<NeuralShard>, Vec<u8>, Vec<u8>), AppError> {
    // Split Neural Key into 5 shards (3-of-5 threshold)
    let zid_shards = match split_neural_key(neural_key) {
        Ok(shards) => shards,
        Err(e) => {
            response::send_neural_key_error(
                ctx.client_pid,
                &ctx.cap_slots,
                KeyError::CryptoError(format!("Shamir split failed: {:?}", e)),
            )?;
            return Err(AppError::Internal("Shamir split failed".into()));
        }
    };

    // Convert zid-crypto NeuralShard to our IPC NeuralShard format (all 5 shards)
    let all_shards: Vec<NeuralShard> = zid_shards
        .iter()
        .enumerate()
        .map(|(i, shard)| NeuralShard {
            index: (i + 1) as u8, // 1-indexed
            hex: shard.to_hex(),
        })
        .collect();

    // Select which 2 shards to encrypt and which 3 to return as external
    let (encrypted_indices, external_indices) = match select_shards_to_encrypt() {
        Ok(indices) => indices,
        Err(e) => {
            response::send_neural_key_error(
                ctx.client_pid,
                &ctx.cap_slots,
                e,
            )?;
            return Err(AppError::Internal("Shard selection failed".into()));
        }
    };

    syscall::debug(&format!(
        "IdentityService: Encrypting shards {:?}, external shards {:?}",
        encrypted_indices, external_indices
    ));

    Ok((all_shards, encrypted_indices, external_indices))
}

/// Create KDF parameters and derive encryption key
fn create_kdf_and_derive_key(
    password: &str,
    ctx: &RequestContext,
) -> Result<(zos_identity::keystore::KeyDerivation, zos_identity::crypto::DerivedKey), AppError> {
    let kdf = match create_kdf_params() {
        Ok(k) => k,
        Err(e) => {
            response::send_neural_key_error(
                ctx.client_pid,
                &ctx.cap_slots,
                e,
            )?;
            return Err(AppError::Internal("KDF creation failed".into()));
        }
    };

    // Derive encryption key ONCE (Argon2id is expensive in WASM)
    let derived_key = match derive_key_from_password_public(password, &kdf) {
        Ok(k) => k,
        Err(e) => {
            response::send_neural_key_error(
                ctx.client_pid,
                &ctx.cap_slots,
                e,
            )?;
            return Err(AppError::Internal("Key derivation failed".into()));
        }
    };

    Ok((kdf, derived_key))
}

/// Encrypt the selected shards using pre-derived key
fn encrypt_selected_shards(
    all_shards: &[NeuralShard],
    encrypted_indices: &[u8],
    derived_key: &zos_identity::crypto::DerivedKey,
    ctx: &RequestContext,
) -> Result<Vec<zos_identity::keystore::EncryptedShard>, AppError> {
    let mut encrypted_shards = Vec::new();
    for &idx in encrypted_indices {
        let shard = &all_shards[(idx - 1) as usize];
        match encrypt_shard_with_key(&shard.hex, idx, derived_key) {
            Ok(encrypted) => encrypted_shards.push(encrypted),
            Err(e) => {
                response::send_neural_key_error(
                    ctx.client_pid,
                    &ctx.cap_slots,
                    e,
                )?;
                return Err(AppError::Internal("Shard encryption failed".into()));
            }
        }
    }
    Ok(encrypted_shards)
}

/// Build the generation result structures
fn build_generation_result(
    user_id: u128,
    identity_signing: &[u8; 32],
    all_shards: &[NeuralShard],
    external_indices: &[u8],
    encrypted_shards: Vec<zos_identity::keystore::EncryptedShard>,
    kdf: zos_identity::keystore::KeyDerivation,
) -> (NeuralKeyGenerated, LocalKeyStore, EncryptedShardStore) {
    // Machine signing and encryption keys are placeholders - derived via CreateMachineKey
    let machine_signing = [0u8; 32];
    let machine_encryption = [0u8; 32];

    // Filter to only external shards (the 3 that user will backup)
    let external_shards: Vec<NeuralShard> = external_indices
        .iter()
        .map(|&idx| all_shards[(idx - 1) as usize].clone())
        .collect();

    let created_at = syscall::get_wallclock();

    // Derive the canonical user ID from the identity signing public key
    let derived_user_id = derive_user_id_from_pubkey(identity_signing);
    syscall::debug(&format!(
        "IdentityService: Derived user_id {:032x} from identity signing key (original: {:032x})",
        derived_user_id, user_id
    ));

    // Create encrypted shard store - use ORIGINAL user_id because that's what was used
    // for identity signing key derivation and will be needed for verification
    let encrypted_shard_store = EncryptedShardStore {
        user_id,
        encrypted_shards,
        external_shard_indices: external_indices.to_vec(),
        kdf,
        created_at,
    };

    let public_identifiers = PublicIdentifiers {
        identity_signing_pub_key: format!("0x{}", bytes_to_hex(identity_signing)),
        machine_signing_pub_key: format!("0x{}", bytes_to_hex(&machine_signing)),
        machine_encryption_pub_key: format!("0x{}", bytes_to_hex(&machine_encryption)),
    };

    // Create key store - use ORIGINAL user_id because it was used for identity key derivation
    let key_store = LocalKeyStore::new(
        user_id,
        *identity_signing,
        machine_signing,
        machine_encryption,
        created_at,
    );

    // Result contains only the 3 external shards
    let result = NeuralKeyGenerated {
        user_id: derived_user_id,
        public_identifiers,
        shards: external_shards,
        created_at,
    };

    (result, key_store, encrypted_shard_store)
}

/// Serialize and initiate storage of generated keys
fn store_generated_keys(
    service: &mut IdentityService,
    ctx: RequestContext,
    result: NeuralKeyGenerated,
    key_store: LocalKeyStore,
    encrypted_shard_store: EncryptedShardStore,
) -> Result<(), AppError> {
    let derived_user_id = result.user_id;
    
    let key_json = match serde_json::to_vec(&key_store) {
        Ok(json) => json,
        Err(e) => {
            return response::send_neural_key_error(
                ctx.client_pid,
                &ctx.cap_slots,
                KeyError::StorageError(format!("Key store serialization failed: {}", e)),
            );
        }
    };

    let encrypted_shards_json = match serde_json::to_vec(&encrypted_shard_store) {
        Ok(json) => json,
        Err(e) => {
            return response::send_neural_key_error(
                ctx.client_pid,
                &ctx.cap_slots,
                KeyError::StorageError(format!("Encrypted shards serialization failed: {}", e)),
            );
        }
    };

    // Store under the DERIVED user_id so subsequent operations can find it
    let key_path = LocalKeyStore::storage_path(derived_user_id);
    // Invariant 32: /keys/ paths use Keystore IPC, not VFS
    service.start_keystore_write(
        &key_path,
        &key_json,
        PendingKeystoreOp::WriteKeyStore {
            ctx,
            user_id: derived_user_id,
            result,
            json_bytes: key_json.clone(),
            encrypted_shards_json,
        },
    )
}
