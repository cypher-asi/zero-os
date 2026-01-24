//! Identity Service (PID 3)
//!
//! The IdentityService manages user cryptographic identities. It:
//! - Generates Neural Keys (entropy, key derivation, Shamir splitting)
//! - Stores public keys to VFS (via async storage syscalls)
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
//!
//! # Architecture
//!
//! This service uses modular components from `zos_apps::identity`:
//! - `crypto`: Key generation, Shamir splitting, signing
//! - `pending`: Async operation state tracking
//! - `response`: IPC response helpers
//! - `storage_handlers`: Async storage result processing
//! - `network_handlers`: Async network result processing

#![cfg_attr(target_arch = "wasm32", no_main)]

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;

use zos_apps::identity::crypto::{
    bytes_to_hex, derive_public_key, format_uuid, generate_random_bytes, shamir_reconstruct,
    shamir_split, sign_challenge, base64_decode,
};
use zos_apps::identity::network_handlers::{self, NetworkHandlerResult};
use zos_apps::identity::pending::{PendingNetworkOp, PendingStorageOp};
use zos_apps::identity::response;
use zos_apps::identity::storage_handlers::{self, StorageHandlerResult};
use zos_apps::manifest::IDENTITY_SERVICE_MANIFEST;
use zos_apps::syscall;
use zos_apps::{app_main, AppContext, AppError, AppManifest, ControlFlow, Message, ZeroApp};
use zos_identity::error::{CredentialError, ZidError};
use zos_identity::ipc::{
    AttachEmailRequest, CreateMachineKeyRequest, GenerateNeuralKeyRequest, GetCredentialsRequest,
    GetIdentityKeyRequest, GetMachineKeyRequest, ListMachineKeysRequest, NeuralKeyGenerated,
    PublicIdentifiers, RecoverNeuralKeyRequest, RevokeMachineKeyRequest, RotateMachineKeyRequest,
    UnlinkCredentialRequest, ZidLoginRequest, ZidSession, ZidTokens,
};
use zos_identity::keystore::{
    CredentialStore, CredentialType, KeyScheme, LinkedCredential, LocalKeyStore, MachineKeyRecord,
};
use zos_identity::KeyError;
use zos_network::{HttpRequest, HttpResponse, NetworkError};
use zos_process::{identity_cred, identity_key, identity_machine, identity_zid, net, storage_result, MSG_STORAGE_RESULT};
use zos_vfs::{parent_path, Inode};

// =============================================================================
// IdentityService Application
// =============================================================================

/// IdentityService - manages user cryptographic identities
pub struct IdentityService {
    /// Whether we have registered with init
    registered: bool,
    /// Pending storage operations: request_id -> operation context
    pending_ops: BTreeMap<u32, PendingStorageOp>,
    /// Pending network operations: request_id -> operation context
    pending_net_ops: BTreeMap<u32, PendingNetworkOp>,
}

impl Default for IdentityService {
    fn default() -> Self {
        Self {
            registered: false,
            pending_ops: BTreeMap::new(),
            pending_net_ops: BTreeMap::new(),
        }
    }
}

impl IdentityService {
    // =========================================================================
    // Storage syscall helpers (async, non-blocking)
    // =========================================================================

    fn start_storage_read(&mut self, key: &str, pending_op: PendingStorageOp) -> Result<(), AppError> {
        match syscall::storage_read_async(key) {
            Ok(request_id) => {
                syscall::debug(&format!(
                    "IdentityService: storage_read_async({}) -> request_id={}",
                    key, request_id
                ));
                self.pending_ops.insert(request_id, pending_op);
                Ok(())
            }
            Err(e) => {
                syscall::debug(&format!("IdentityService: storage_read_async failed: {}", e));
                Err(AppError::IpcError(format!("Storage read failed: {}", e)))
            }
        }
    }

    fn start_storage_write(&mut self, key: &str, value: &[u8], pending_op: PendingStorageOp) -> Result<(), AppError> {
        match syscall::storage_write_async(key, value) {
            Ok(request_id) => {
                syscall::debug(&format!(
                    "IdentityService: storage_write_async({}, {} bytes) -> request_id={}",
                    key, value.len(), request_id
                ));
                self.pending_ops.insert(request_id, pending_op);
                Ok(())
            }
            Err(e) => {
                syscall::debug(&format!("IdentityService: storage_write_async failed: {}", e));
                Err(AppError::IpcError(format!("Storage write failed: {}", e)))
            }
        }
    }

    fn start_storage_delete(&mut self, key: &str, pending_op: PendingStorageOp) -> Result<(), AppError> {
        match syscall::storage_delete_async(key) {
            Ok(request_id) => {
                syscall::debug(&format!(
                    "IdentityService: storage_delete_async({}) -> request_id={}",
                    key, request_id
                ));
                self.pending_ops.insert(request_id, pending_op);
                Ok(())
            }
            Err(e) => {
                syscall::debug(&format!("IdentityService: storage_delete_async failed: {}", e));
                Err(AppError::IpcError(format!("Storage delete failed: {}", e)))
            }
        }
    }

    fn start_storage_exists(&mut self, key: &str, pending_op: PendingStorageOp) -> Result<(), AppError> {
        match syscall::storage_exists_async(key) {
            Ok(request_id) => {
                syscall::debug(&format!(
                    "IdentityService: storage_exists_async({}) -> request_id={}",
                    key, request_id
                ));
                self.pending_ops.insert(request_id, pending_op);
                Ok(())
            }
            Err(e) => {
                syscall::debug(&format!("IdentityService: storage_exists_async failed: {}", e));
                Err(AppError::IpcError(format!("Storage exists failed: {}", e)))
            }
        }
    }

    fn start_storage_list(&mut self, prefix: &str, pending_op: PendingStorageOp) -> Result<(), AppError> {
        match syscall::storage_list_async(prefix) {
            Ok(request_id) => {
                syscall::debug(&format!(
                    "IdentityService: storage_list_async({}) -> request_id={}",
                    prefix, request_id
                ));
                self.pending_ops.insert(request_id, pending_op);
                Ok(())
            }
            Err(e) => {
                syscall::debug(&format!("IdentityService: storage_list_async failed: {}", e));
                Err(AppError::IpcError(format!("Storage list failed: {}", e)))
            }
        }
    }

    // =========================================================================
    // Network syscall helpers (async, non-blocking)
    // =========================================================================

    fn start_network_fetch(&mut self, request: &HttpRequest, pending_op: PendingNetworkOp) -> Result<(), AppError> {
        let request_json = match serde_json::to_vec(request) {
            Ok(json) => json,
            Err(e) => {
                syscall::debug(&format!("IdentityService: Failed to serialize HTTP request: {}", e));
                return Err(AppError::IpcError(format!("Request serialization failed: {}", e)));
            }
        };

        match syscall::network_fetch_async(&request_json) {
            Ok(request_id) => {
                syscall::debug(&format!(
                    "IdentityService: network_fetch_async({} {}) -> request_id={}",
                    request.method.as_str(), request.url, request_id
                ));
                self.pending_net_ops.insert(request_id, pending_op);
                Ok(())
            }
            Err(e) => {
                syscall::debug(&format!("IdentityService: network_fetch_async failed: {}", e));
                Err(AppError::IpcError(format!("Network fetch failed: {}", e)))
            }
        }
    }

    // =========================================================================
    // Request handlers
    // =========================================================================

    fn handle_generate_neural_key(&mut self, msg: &Message) -> Result<(), AppError> {
        syscall::debug("IdentityService: Handling generate neural key request");

        let request: GenerateNeuralKeyRequest = match serde_json::from_slice(&msg.data) {
            Ok(r) => r,
            Err(e) => {
                syscall::debug(&format!("IdentityService: Failed to parse request: {}", e));
                return response::send_neural_key_error(msg.from_pid, &msg.cap_slots, KeyError::DerivationFailed);
            }
        };

        let user_id = request.user_id;
        syscall::debug(&format!("IdentityService: Generating Neural Key for user {:032x}", user_id));

        let key_path = LocalKeyStore::storage_path(user_id);
        self.start_storage_exists(
            &format!("inode:{}", key_path),
            PendingStorageOp::CheckKeyExists {
                client_pid: msg.from_pid,
                user_id,
                cap_slots: msg.cap_slots.clone(),
            },
        )
    }

    fn continue_generate_after_exists_check(&mut self, client_pid: u32, user_id: u128, exists: bool, cap_slots: Vec<u32>) -> Result<(), AppError> {
        if exists {
            syscall::debug("IdentityService: Neural Key already exists");
            return response::send_neural_key_error(client_pid, &cap_slots, KeyError::IdentityKeyAlreadyExists);
        }

        let entropy = generate_random_bytes(32);
        let identity_signing = derive_public_key(&entropy, "identity-signing");
        let machine_signing = derive_public_key(&entropy, "machine-signing");
        let machine_encryption = derive_public_key(&entropy, "machine-encryption");

        let shards = shamir_split(&entropy, 3, 5);
        let public_identifiers = PublicIdentifiers {
            identity_signing_pub_key: format!("0x{}", bytes_to_hex(&identity_signing)),
            machine_signing_pub_key: format!("0x{}", bytes_to_hex(&machine_signing)),
            machine_encryption_pub_key: format!("0x{}", bytes_to_hex(&machine_encryption)),
        };

        let created_at = syscall::get_wallclock();
        let key_store = LocalKeyStore::new(user_id, identity_signing, machine_signing, machine_encryption, created_at);
        let result = NeuralKeyGenerated { public_identifiers, shards, created_at };

        let key_path = LocalKeyStore::storage_path(user_id);
        match serde_json::to_vec(&key_store) {
            Ok(json_bytes) => self.start_storage_write(
                &format!("content:{}", key_path),
                &json_bytes.clone(),
                PendingStorageOp::WriteKeyStoreContent { client_pid, user_id, result, json_bytes, cap_slots },
            ),
            Err(e) => response::send_neural_key_error(client_pid, &cap_slots, KeyError::StorageError(format!("Serialization failed: {}", e))),
        }
    }

    fn handle_recover_neural_key(&mut self, msg: &Message) -> Result<(), AppError> {
        syscall::debug("IdentityService: Handling recover neural key request");

        let request: RecoverNeuralKeyRequest = match serde_json::from_slice(&msg.data) {
            Ok(r) => r,
            Err(e) => {
                syscall::debug(&format!("IdentityService: Failed to parse request: {}", e));
                return response::send_recover_key_error(msg.from_pid, &msg.cap_slots, KeyError::DerivationFailed);
            }
        };

        if request.shards.len() < 3 {
            return response::send_recover_key_error(msg.from_pid, &msg.cap_slots, KeyError::InsufficientShards);
        }

        let entropy = match shamir_reconstruct(&request.shards) {
            Ok(e) => e,
            Err(e) => return response::send_recover_key_error(msg.from_pid, &msg.cap_slots, e),
        };

        let identity_signing = derive_public_key(&entropy, "identity-signing");
        let machine_signing = derive_public_key(&entropy, "machine-signing");
        let machine_encryption = derive_public_key(&entropy, "machine-encryption");

        let public_identifiers = PublicIdentifiers {
            identity_signing_pub_key: format!("0x{}", bytes_to_hex(&identity_signing)),
            machine_signing_pub_key: format!("0x{}", bytes_to_hex(&machine_signing)),
            machine_encryption_pub_key: format!("0x{}", bytes_to_hex(&machine_encryption)),
        };

        let created_at = syscall::get_wallclock();
        let key_store = LocalKeyStore::new(request.user_id, identity_signing, machine_signing, machine_encryption, created_at);
        let new_shards = shamir_split(&entropy, 3, 5);
        let result = NeuralKeyGenerated { public_identifiers, shards: new_shards, created_at };

        let key_path = LocalKeyStore::storage_path(request.user_id);
        match serde_json::to_vec(&key_store) {
            Ok(json_bytes) => self.start_storage_write(
                &format!("content:{}", key_path),
                &json_bytes.clone(),
                PendingStorageOp::WriteRecoveredKeyStoreContent {
                    client_pid: msg.from_pid,
                    user_id: request.user_id,
                    result,
                    json_bytes,
                    cap_slots: msg.cap_slots.clone(),
                },
            ),
            Err(e) => response::send_recover_key_error(msg.from_pid, &msg.cap_slots, KeyError::StorageError(format!("Serialization failed: {}", e))),
        }
    }

    fn handle_get_identity_key(&mut self, msg: &Message) -> Result<(), AppError> {
        let request: GetIdentityKeyRequest = match serde_json::from_slice(&msg.data) {
            Ok(r) => r,
            Err(e) => {
                syscall::debug(&format!("IdentityService: Failed to parse request: {}", e));
                return response::send_get_identity_key_error(msg.from_pid, &msg.cap_slots, KeyError::DerivationFailed);
            }
        };

        let key_path = LocalKeyStore::storage_path(request.user_id);
        self.start_storage_read(&format!("content:{}", key_path), PendingStorageOp::GetIdentityKey {
            client_pid: msg.from_pid,
            cap_slots: msg.cap_slots.clone(),
        })
    }

    fn handle_create_machine_key(&mut self, msg: &Message) -> Result<(), AppError> {
        let request: CreateMachineKeyRequest = match serde_json::from_slice(&msg.data) {
            Ok(r) => r,
            Err(e) => {
                syscall::debug(&format!("IdentityService: Failed to parse request: {}", e));
                return response::send_create_machine_key_error(msg.from_pid, &msg.cap_slots, KeyError::DerivationFailed);
            }
        };

        let key_path = LocalKeyStore::storage_path(request.user_id);
        self.start_storage_exists(&format!("inode:{}", key_path), PendingStorageOp::CheckIdentityForMachine {
            client_pid: msg.from_pid,
            request,
            cap_slots: msg.cap_slots.clone(),
        })
    }

    fn continue_create_machine_after_identity_check(&mut self, client_pid: u32, request: CreateMachineKeyRequest, exists: bool, cap_slots: Vec<u32>) -> Result<(), AppError> {
        if !exists {
            return response::send_create_machine_key_error(client_pid, &cap_slots, KeyError::IdentityKeyRequired);
        }

        let machine_entropy = generate_random_bytes(32);
        let machine_id_bytes = generate_random_bytes(16);
        let machine_id = u128::from_le_bytes(machine_id_bytes[..16].try_into().unwrap_or([0u8; 16]));

        // Always generate classical keys (Ed25519/X25519)
        let signing_key = derive_public_key(&machine_entropy, "machine-signing");
        let encryption_key = derive_public_key(&machine_entropy, "machine-encryption");
        let now = syscall::get_wallclock();

        // Generate PQ keys if PqHybrid scheme is requested
        let (pq_signing_public_key, pq_encryption_public_key) = if request.key_scheme == KeyScheme::PqHybrid {
            // For WASM target, we generate deterministic placeholder keys using HKDF
            // Real PQ key generation would use ML-DSA-65 (1952 bytes) and ML-KEM-768 (1184 bytes)
            // TODO: Integrate actual PQ crypto library when WASM-compatible version is available
            let pq_sign_seed = derive_public_key(&machine_entropy, "cypher:shared:machine:pq-sign:v1");
            let pq_kem_seed = derive_public_key(&machine_entropy, "cypher:shared:machine:pq-kem:v1");
            
            // Create placeholder public keys with correct sizes
            // ML-DSA-65 public key: 1952 bytes
            // ML-KEM-768 public key: 1184 bytes
            let mut pq_sign_pk = vec![0u8; 1952];
            pq_sign_pk[..32].copy_from_slice(&pq_sign_seed);
            
            let mut pq_kem_pk = vec![0u8; 1184];
            pq_kem_pk[..32].copy_from_slice(&pq_kem_seed);
            
            syscall::debug(&format!(
                "IdentityService: Generated PQ-Hybrid keys (placeholder) for machine {:032x}",
                machine_id
            ));
            
            (Some(pq_sign_pk), Some(pq_kem_pk))
        } else {
            (None, None)
        };

        let record = MachineKeyRecord {
            machine_id,
            signing_public_key: signing_key,
            encryption_public_key: encryption_key,
            authorized_at: now,
            authorized_by: request.user_id,
            capabilities: request.capabilities,
            machine_name: request.machine_name,
            last_seen_at: now,
            epoch: 1,
            key_scheme: request.key_scheme,
            pq_signing_public_key,
            pq_encryption_public_key,
        };

        let machine_path = MachineKeyRecord::storage_path(request.user_id, machine_id);
        match serde_json::to_vec(&record) {
            Ok(json_bytes) => self.start_storage_write(
                &format!("content:{}", machine_path),
                &json_bytes.clone(),
                PendingStorageOp::WriteMachineKeyContent {
                    client_pid,
                    user_id: request.user_id,
                    record,
                    json_bytes,
                    cap_slots,
                },
            ),
            Err(e) => response::send_create_machine_key_error(client_pid, &cap_slots, KeyError::StorageError(format!("Serialization failed: {}", e))),
        }
    }

    fn handle_list_machine_keys(&mut self, msg: &Message) -> Result<(), AppError> {
        let request: ListMachineKeysRequest = match serde_json::from_slice(&msg.data) {
            Ok(r) => r,
            Err(_) => return response::send_list_machine_keys(msg.from_pid, &msg.cap_slots, vec![]),
        };

        let machine_dir = format!("/home/{:032x}/.zos/identity/machine", request.user_id);
        self.start_storage_list(&format!("inode:{}", machine_dir), PendingStorageOp::ListMachineKeys {
            client_pid: msg.from_pid,
            user_id: request.user_id,
            cap_slots: msg.cap_slots.clone(),
        })
    }

    fn handle_revoke_machine_key(&mut self, msg: &Message) -> Result<(), AppError> {
        let request: RevokeMachineKeyRequest = match serde_json::from_slice(&msg.data) {
            Ok(r) => r,
            Err(e) => {
                syscall::debug(&format!("IdentityService: Failed to parse request: {}", e));
                return response::send_revoke_machine_key_error(msg.from_pid, &msg.cap_slots, KeyError::DerivationFailed);
            }
        };

        let machine_path = MachineKeyRecord::storage_path(request.user_id, request.machine_id);
        self.start_storage_delete(&format!("content:{}", machine_path), PendingStorageOp::DeleteMachineKey {
            client_pid: msg.from_pid,
            user_id: request.user_id,
            machine_id: request.machine_id,
            cap_slots: msg.cap_slots.clone(),
        })
    }

    fn handle_rotate_machine_key(&mut self, msg: &Message) -> Result<(), AppError> {
        let request: RotateMachineKeyRequest = match serde_json::from_slice(&msg.data) {
            Ok(r) => r,
            Err(e) => {
                syscall::debug(&format!("IdentityService: Failed to parse request: {}", e));
                return response::send_rotate_machine_key_error(msg.from_pid, &msg.cap_slots, KeyError::DerivationFailed);
            }
        };

        let machine_path = MachineKeyRecord::storage_path(request.user_id, request.machine_id);
        self.start_storage_read(&format!("content:{}", machine_path), PendingStorageOp::ReadMachineForRotate {
            client_pid: msg.from_pid,
            user_id: request.user_id,
            machine_id: request.machine_id,
            cap_slots: msg.cap_slots.clone(),
        })
    }

    fn continue_rotate_after_read(&mut self, client_pid: u32, user_id: u128, machine_id: u128, data: &[u8], cap_slots: Vec<u32>) -> Result<(), AppError> {
        let mut record: MachineKeyRecord = match serde_json::from_slice(data) {
            Ok(r) => r,
            Err(e) => return response::send_rotate_machine_key_error(client_pid, &cap_slots, KeyError::StorageError(format!("Parse failed: {}", e))),
        };

        let new_entropy = generate_random_bytes(32);
        record.signing_public_key = derive_public_key(&new_entropy, "machine-signing");
        record.encryption_public_key = derive_public_key(&new_entropy, "machine-encryption");
        record.epoch += 1;
        record.last_seen_at = syscall::get_wallclock();

        // Regenerate PQ keys if PqHybrid scheme
        if record.key_scheme == KeyScheme::PqHybrid {
            let pq_sign_seed = derive_public_key(&new_entropy, "cypher:shared:machine:pq-sign:v1");
            let pq_kem_seed = derive_public_key(&new_entropy, "cypher:shared:machine:pq-kem:v1");
            
            let mut pq_sign_pk = vec![0u8; 1952];
            pq_sign_pk[..32].copy_from_slice(&pq_sign_seed);
            
            let mut pq_kem_pk = vec![0u8; 1184];
            pq_kem_pk[..32].copy_from_slice(&pq_kem_seed);
            
            record.pq_signing_public_key = Some(pq_sign_pk);
            record.pq_encryption_public_key = Some(pq_kem_pk);
            
            syscall::debug(&format!(
                "IdentityService: Rotated PQ-Hybrid keys for machine {:032x} (epoch {})",
                machine_id, record.epoch
            ));
        }

        let machine_path = MachineKeyRecord::storage_path(user_id, machine_id);
        match serde_json::to_vec(&record) {
            Ok(json_bytes) => self.start_storage_write(
                &format!("content:{}", machine_path),
                &json_bytes.clone(),
                PendingStorageOp::WriteRotatedMachineKeyContent { client_pid, user_id, record, json_bytes, cap_slots },
            ),
            Err(e) => response::send_rotate_machine_key_error(client_pid, &cap_slots, KeyError::StorageError(format!("Serialization failed: {}", e))),
        }
    }

    fn handle_get_machine_key(&mut self, msg: &Message) -> Result<(), AppError> {
        let request: GetMachineKeyRequest = match serde_json::from_slice(&msg.data) {
            Ok(r) => r,
            Err(_e) => return response::send_get_machine_key_error(msg.from_pid, &msg.cap_slots, KeyError::DerivationFailed),
        };

        let machine_path = MachineKeyRecord::storage_path(request.user_id, request.machine_id);
        self.start_storage_read(&format!("content:{}", machine_path), PendingStorageOp::ReadSingleMachineKey {
            client_pid: msg.from_pid,
            cap_slots: msg.cap_slots.clone(),
        })
    }

    fn handle_attach_email(&mut self, msg: &Message) -> Result<(), AppError> {
        let request: AttachEmailRequest = match serde_json::from_slice(&msg.data) {
            Ok(r) => r,
            Err(_) => return response::send_attach_email_error(msg.from_pid, &msg.cap_slots, CredentialError::InvalidFormat),
        };

        if !request.email.contains('@') || request.email.len() < 5 {
            return response::send_attach_email_error(msg.from_pid, &msg.cap_slots, CredentialError::InvalidFormat);
        }

        if request.password.len() < 12 {
            return response::send_attach_email_error(msg.from_pid, &msg.cap_slots, CredentialError::StorageError("Password must be at least 12 characters".into()));
        }

        let body = format!(r#"{{"email":"{}","password":"{}"}}"#, request.email, request.password);
        let http_request = HttpRequest::post(format!("{}/v1/credentials/email", request.zid_endpoint))
            .with_header("Authorization", &format!("Bearer {}", request.access_token))
            .with_json_body(body.into_bytes())
            .with_timeout(15_000);

        self.start_network_fetch(&http_request, PendingNetworkOp::SubmitEmailToZid {
            client_pid: msg.from_pid,
            user_id: request.user_id,
            email: request.email,
            cap_slots: msg.cap_slots.clone(),
        })
    }

    fn handle_get_credentials(&mut self, msg: &Message) -> Result<(), AppError> {
        let request: GetCredentialsRequest = match serde_json::from_slice(&msg.data) {
            Ok(r) => r,
            Err(_) => return response::send_get_credentials(msg.from_pid, &msg.cap_slots, vec![]),
        };

        let cred_path = CredentialStore::storage_path(request.user_id);
        self.start_storage_read(&format!("content:{}", cred_path), PendingStorageOp::GetCredentials {
            client_pid: msg.from_pid,
            cap_slots: msg.cap_slots.clone(),
        })
    }

    fn handle_unlink_credential(&mut self, msg: &Message) -> Result<(), AppError> {
        let request: UnlinkCredentialRequest = match serde_json::from_slice(&msg.data) {
            Ok(r) => r,
            Err(_) => return response::send_unlink_credential_error(msg.from_pid, &msg.cap_slots, CredentialError::InvalidFormat),
        };

        let cred_path = CredentialStore::storage_path(request.user_id);
        self.start_storage_read(&format!("content:{}", cred_path), PendingStorageOp::ReadCredentialsForUnlink {
            client_pid: msg.from_pid,
            user_id: request.user_id,
            credential_type: request.credential_type,
            cap_slots: msg.cap_slots.clone(),
        })
    }

    fn handle_zid_login(&mut self, msg: &Message) -> Result<(), AppError> {
        let request: ZidLoginRequest = match serde_json::from_slice(&msg.data) {
            Ok(r) => r,
            Err(e) => return response::send_zid_login_error(msg.from_pid, &msg.cap_slots, ZidError::NetworkError(format!("Invalid request: {}", e))),
        };

        let machine_dir = format!("/home/{:032x}/.zos/identity/machine", request.user_id);
        self.start_storage_list(&machine_dir, PendingStorageOp::ReadMachineKeyForZidLogin {
            client_pid: msg.from_pid,
            user_id: request.user_id,
            zid_endpoint: request.zid_endpoint,
            cap_slots: msg.cap_slots.clone(),
        })
    }

    fn handle_zid_enroll_machine(&mut self, msg: &Message) -> Result<(), AppError> {
        let request: ZidLoginRequest = match serde_json::from_slice(&msg.data) {
            Ok(r) => r,
            Err(e) => return response::send_zid_enroll_error(msg.from_pid, &msg.cap_slots, ZidError::NetworkError(format!("Invalid request: {}", e))),
        };

        let machine_dir = format!("/home/{:032x}/.zos/identity/machine", request.user_id);
        self.start_storage_list(&machine_dir, PendingStorageOp::ReadMachineKeyForZidEnroll {
            client_pid: msg.from_pid,
            user_id: request.user_id,
            zid_endpoint: request.zid_endpoint,
            cap_slots: msg.cap_slots.clone(),
        })
    }

    // =========================================================================
    // Storage result handler (dispatches to storage_handlers module)
    // =========================================================================

    fn handle_storage_result(&mut self, msg: &Message) -> Result<(), AppError> {
        if msg.data.len() < 9 {
            return Ok(());
        }

        let request_id = u32::from_le_bytes([msg.data[0], msg.data[1], msg.data[2], msg.data[3]]);
        let result_type = msg.data[4];
        let data_len = u32::from_le_bytes([msg.data[5], msg.data[6], msg.data[7], msg.data[8]]) as usize;
        let data = if data_len > 0 && msg.data.len() >= 9 + data_len { &msg.data[9..9 + data_len] } else { &[] };

        let pending_op = match self.pending_ops.remove(&request_id) {
            Some(op) => op,
            None => return Ok(()),
        };

        self.dispatch_storage_result(pending_op, result_type, data)
    }

    fn dispatch_storage_result(&mut self, op: PendingStorageOp, result_type: u8, data: &[u8]) -> Result<(), AppError> {
        match op {
            PendingStorageOp::CheckKeyExists { client_pid, user_id, cap_slots } => {
                let exists = result_type == storage_result::EXISTS_OK && !data.is_empty() && data[0] == 1;
                self.continue_generate_after_exists_check(client_pid, user_id, exists, cap_slots)
            }
            PendingStorageOp::WriteKeyStoreContent { client_pid, user_id, result, json_bytes, cap_slots } => {
                self.handle_storage_handler_result(storage_handlers::handle_write_key_store_content(client_pid, user_id, result, json_bytes, cap_slots, result_type))
            }
            PendingStorageOp::WriteKeyStoreInode { client_pid, result, cap_slots } => {
                self.handle_storage_handler_result(storage_handlers::handle_write_key_store_inode(client_pid, result, cap_slots, result_type))
            }
            PendingStorageOp::GetIdentityKey { client_pid, cap_slots } => {
                self.handle_storage_handler_result(storage_handlers::handle_get_identity_key(client_pid, cap_slots, result_type, data))
            }
            PendingStorageOp::WriteRecoveredKeyStoreContent { client_pid, user_id, result, json_bytes, cap_slots } => {
                self.handle_storage_handler_result(storage_handlers::handle_write_recovered_content(client_pid, user_id, result, json_bytes, cap_slots, result_type))
            }
            PendingStorageOp::WriteRecoveredKeyStoreInode { client_pid, result, cap_slots } => {
                self.handle_storage_handler_result(storage_handlers::handle_write_recovered_inode(client_pid, result, cap_slots, result_type))
            }
            PendingStorageOp::CheckIdentityForMachine { client_pid, request, cap_slots } => {
                let exists = result_type == storage_result::EXISTS_OK && !data.is_empty() && data[0] == 1;
                self.continue_create_machine_after_identity_check(client_pid, request, exists, cap_slots)
            }
            PendingStorageOp::WriteMachineKeyContent { client_pid, user_id, record, json_bytes, cap_slots } => {
                self.handle_storage_handler_result(storage_handlers::handle_write_machine_key_content(client_pid, user_id, record, json_bytes, cap_slots, result_type))
            }
            PendingStorageOp::WriteMachineKeyInode { client_pid, record, cap_slots } => {
                self.handle_storage_handler_result(storage_handlers::handle_write_machine_key_inode(client_pid, record, cap_slots, result_type))
            }
            PendingStorageOp::ListMachineKeys { client_pid, user_id, cap_slots } => {
                self.handle_storage_handler_result(storage_handlers::handle_list_machine_keys(client_pid, user_id, cap_slots, result_type, data))
            }
            PendingStorageOp::ReadMachineKey { client_pid, user_id, remaining_paths, records, cap_slots } => {
                self.handle_storage_handler_result(storage_handlers::handle_read_machine_key(client_pid, user_id, remaining_paths, records, cap_slots, result_type, data))
            }
            PendingStorageOp::DeleteMachineKey { client_pid, user_id, machine_id, cap_slots } => {
                self.handle_storage_handler_result(storage_handlers::handle_delete_machine_key(client_pid, user_id, machine_id, cap_slots, result_type))
            }
            PendingStorageOp::DeleteMachineKeyInode { client_pid, cap_slots } => {
                self.handle_storage_handler_result(storage_handlers::handle_delete_machine_key_inode(client_pid, cap_slots, result_type))
            }
            PendingStorageOp::ReadMachineForRotate { client_pid, user_id, machine_id, cap_slots } => {
                if result_type == storage_result::READ_OK {
                    self.continue_rotate_after_read(client_pid, user_id, machine_id, data, cap_slots)
                } else {
                    response::send_rotate_machine_key_error(client_pid, &cap_slots, KeyError::MachineKeyNotFound)
                }
            }
            PendingStorageOp::WriteRotatedMachineKeyContent { client_pid, user_id, record, json_bytes, cap_slots } => {
                self.handle_storage_handler_result(storage_handlers::handle_write_rotated_content(client_pid, user_id, record, json_bytes, cap_slots, result_type))
            }
            PendingStorageOp::WriteRotatedMachineKeyInode { client_pid, record, cap_slots } => {
                self.handle_storage_handler_result(storage_handlers::handle_write_rotated_inode(client_pid, record, cap_slots, result_type))
            }
            PendingStorageOp::ReadSingleMachineKey { client_pid, cap_slots } => {
                self.handle_storage_handler_result(storage_handlers::handle_read_single_machine_key(client_pid, cap_slots, result_type, data))
            }
            PendingStorageOp::ReadCredentialsForAttach { client_pid, user_id, email, cap_slots } => {
                let existing_store = if result_type == storage_result::READ_OK && !data.is_empty() {
                    serde_json::from_slice::<CredentialStore>(data).ok()
                } else { None };
                self.continue_attach_email_after_read(client_pid, user_id, email, existing_store, cap_slots)
            }
            PendingStorageOp::GetCredentials { client_pid, cap_slots } => {
                self.handle_storage_handler_result(storage_handlers::handle_get_credentials(client_pid, cap_slots, result_type, data))
            }
            PendingStorageOp::ReadCredentialsForUnlink { client_pid, user_id, credential_type, cap_slots } => {
                if result_type == storage_result::READ_OK && !data.is_empty() {
                    self.continue_unlink_credential_after_read(client_pid, user_id, credential_type, data, cap_slots)
                } else {
                    response::send_unlink_credential_error(client_pid, &cap_slots, CredentialError::NotFound)
                }
            }
            PendingStorageOp::WriteUnlinkedCredentialContent { client_pid, user_id, json_bytes, cap_slots } => {
                self.handle_storage_handler_result(storage_handlers::handle_write_unlinked_content(client_pid, user_id, json_bytes, cap_slots, result_type))
            }
            PendingStorageOp::WriteUnlinkedCredentialInode { client_pid, cap_slots } => {
                self.handle_storage_handler_result(storage_handlers::handle_write_unlinked_inode(client_pid, cap_slots, result_type))
            }
            PendingStorageOp::WriteEmailCredentialContent { client_pid, user_id, json_bytes, cap_slots } => {
                self.handle_storage_handler_result(storage_handlers::handle_write_email_cred_content(client_pid, user_id, json_bytes, cap_slots, result_type))
            }
            PendingStorageOp::WriteEmailCredentialInode { client_pid, cap_slots } => {
                self.handle_storage_handler_result(storage_handlers::handle_write_email_cred_inode(client_pid, cap_slots, result_type))
            }
            PendingStorageOp::ReadMachineKeyForZidLogin { client_pid, user_id, zid_endpoint, cap_slots } => {
                match storage_handlers::handle_read_machine_for_zid_login(client_pid, user_id, zid_endpoint, cap_slots, result_type, data) {
                    Ok(storage_handlers::ZidLoginReadResult::PathList { paths, client_pid, user_id, zid_endpoint, cap_slots }) => {
                        self.continue_zid_login_after_list(client_pid, user_id, zid_endpoint, paths, cap_slots)
                    }
                    Ok(storage_handlers::ZidLoginReadResult::MachineKeyData { data, client_pid, user_id, zid_endpoint, cap_slots }) => {
                        self.continue_zid_login_after_read(client_pid, user_id, zid_endpoint, &data, cap_slots)
                    }
                    Err(result) => self.handle_storage_handler_result(result),
                }
            }
            PendingStorageOp::WriteZidSessionContent { client_pid, user_id, tokens, json_bytes, cap_slots } => {
                self.continue_zid_login_after_write_content(client_pid, user_id, tokens, json_bytes, cap_slots, result_type)
            }
            PendingStorageOp::WriteZidSessionInode { client_pid, tokens, cap_slots } => {
                self.handle_storage_handler_result(storage_handlers::handle_write_zid_session_inode(client_pid, tokens, cap_slots, result_type))
            }
            PendingStorageOp::ReadMachineKeyForZidEnroll { client_pid, user_id, zid_endpoint, cap_slots } => {
                match storage_handlers::handle_read_machine_for_zid_enroll(client_pid, user_id, zid_endpoint, cap_slots, result_type, data) {
                    Ok(storage_handlers::ZidEnrollReadResult::PathList { paths, client_pid, user_id, zid_endpoint, cap_slots }) => {
                        self.continue_zid_enroll_after_list(client_pid, user_id, zid_endpoint, paths, cap_slots)
                    }
                    Ok(storage_handlers::ZidEnrollReadResult::MachineKeyData { data, client_pid, user_id, zid_endpoint, cap_slots }) => {
                        self.continue_zid_enroll_after_read(client_pid, user_id, zid_endpoint, &data, cap_slots)
                    }
                    Err(result) => self.handle_storage_handler_result(result),
                }
            }
            PendingStorageOp::WriteZidEnrollSessionContent { client_pid, user_id, tokens, json_bytes, cap_slots } => {
                if result_type == storage_result::WRITE_OK {
                    let session_path = format!("/home/{:032x}/.zos/identity/zid_session.json", user_id);
                    self.start_storage_write(&format!("inode:{}", session_path), &json_bytes, PendingStorageOp::WriteZidEnrollSessionInode { client_pid, tokens, cap_slots })
                } else {
                    response::send_zid_enroll_error(client_pid, &cap_slots, ZidError::EnrollmentFailed("Session write failed".into()))
                }
            }
            PendingStorageOp::WriteZidEnrollSessionInode { client_pid, tokens, cap_slots } => {
                self.handle_storage_handler_result(storage_handlers::handle_write_zid_enroll_session_inode(client_pid, tokens, cap_slots, result_type))
            }
        }
    }

    fn handle_storage_handler_result(&mut self, result: StorageHandlerResult) -> Result<(), AppError> {
        match result {
            StorageHandlerResult::Done(r) => r,
            StorageHandlerResult::ContinueWrite { key, value, next_op } => {
                self.start_storage_write(&key, &value, next_op)
            }
            StorageHandlerResult::ContinueRead { key, next_op } => {
                self.start_storage_read(&key, next_op)
            }
            StorageHandlerResult::ContinueDelete { key, next_op } => {
                self.start_storage_delete(&key, next_op)
            }
        }
    }

    // =========================================================================
    // Network result handler
    // =========================================================================

    fn handle_net_result(&mut self, msg: &Message) -> Result<(), AppError> {
        if msg.data.len() < 9 { return Ok(()); }

        let request_id = u32::from_le_bytes([msg.data[0], msg.data[1], msg.data[2], msg.data[3]]);
        let result_type = msg.data[4];
        let data_len = u32::from_le_bytes([msg.data[5], msg.data[6], msg.data[7], msg.data[8]]) as usize;
        let data = if data_len > 0 && msg.data.len() >= 9 + data_len { &msg.data[9..9 + data_len] } else { &[] };

        let pending_op = match self.pending_net_ops.remove(&request_id) {
            Some(op) => op,
            None => return Ok(()),
        };

        let http_response: HttpResponse = if result_type == 0 && !data.is_empty() {
            serde_json::from_slice(data).unwrap_or_else(|_| HttpResponse::err(NetworkError::Other("Parse error".into())))
        } else {
            HttpResponse::err(NetworkError::Other("Network error".into()))
        };

        self.dispatch_network_result(pending_op, http_response)
    }

    fn dispatch_network_result(&mut self, op: PendingNetworkOp, http_response: HttpResponse) -> Result<(), AppError> {
        match op {
            PendingNetworkOp::RequestZidChallenge { client_pid, user_id, zid_endpoint, machine_key, cap_slots } => {
                match network_handlers::handle_zid_challenge_result(client_pid, user_id, zid_endpoint, machine_key, cap_slots, http_response) {
                    NetworkHandlerResult::Done(r) => r,
                    NetworkHandlerResult::ContinueZidLoginWithChallenge { client_pid, user_id, zid_endpoint, machine_key, challenge_response, cap_slots } => {
                        self.continue_zid_login_after_challenge(client_pid, user_id, zid_endpoint, machine_key, challenge_response, cap_slots)
                    }
                    _ => Ok(()),
                }
            }
            PendingNetworkOp::SubmitZidLogin { client_pid, user_id, zid_endpoint, cap_slots } => {
                match network_handlers::handle_zid_login_result(client_pid, user_id, zid_endpoint, cap_slots, http_response) {
                    NetworkHandlerResult::Done(r) => r,
                    NetworkHandlerResult::ContinueZidLoginWithTokens { client_pid, user_id, zid_endpoint, login_response, cap_slots } => {
                        self.continue_zid_login_after_login(client_pid, user_id, zid_endpoint, login_response, cap_slots)
                    }
                    _ => Ok(()),
                }
            }
            PendingNetworkOp::SubmitEmailToZid { client_pid, user_id, email, cap_slots } => {
                match network_handlers::handle_email_to_zid_result(client_pid, user_id, email, cap_slots, http_response) {
                    NetworkHandlerResult::Done(r) => r,
                    NetworkHandlerResult::ContinueAttachEmail { client_pid, user_id, email, cap_slots } => {
                        self.continue_attach_email_after_zid(client_pid, user_id, email, cap_slots)
                    }
                    _ => Ok(()),
                }
            }
            PendingNetworkOp::SubmitZidEnroll { client_pid, user_id, zid_endpoint, cap_slots } => {
                match network_handlers::handle_zid_enroll_result(client_pid, user_id, zid_endpoint, cap_slots, http_response) {
                    NetworkHandlerResult::Done(r) => r,
                    NetworkHandlerResult::ContinueZidEnroll { client_pid, user_id, zid_endpoint, enroll_response, cap_slots } => {
                        self.continue_zid_enroll_after_submit(client_pid, user_id, zid_endpoint, enroll_response, cap_slots)
                    }
                    _ => Ok(()),
                }
            }
        }
    }

    // =========================================================================
    // Continuation methods for ZID flows
    // =========================================================================

    fn continue_zid_login_after_list(&mut self, client_pid: u32, user_id: u128, zid_endpoint: String, paths: Vec<String>, cap_slots: Vec<u32>) -> Result<(), AppError> {
        let path = paths.into_iter().find(|p| p.ends_with(".json"));
        match path {
            Some(p) => self.start_storage_read(&format!("content:{}", p), PendingStorageOp::ReadMachineKeyForZidLogin { client_pid, user_id, zid_endpoint, cap_slots }),
            None => response::send_zid_login_error(client_pid, &cap_slots, ZidError::MachineKeyNotFound),
        }
    }

    fn continue_zid_login_after_read(&mut self, client_pid: u32, user_id: u128, zid_endpoint: String, data: &[u8], cap_slots: Vec<u32>) -> Result<(), AppError> {
        let machine_key: MachineKeyRecord = match serde_json::from_slice(data) {
            Ok(r) => r,
            Err(_) => return response::send_zid_login_error(client_pid, &cap_slots, ZidError::MachineKeyNotFound),
        };

        let machine_id_uuid = format_uuid(machine_key.machine_id);
        let challenge_request = HttpRequest::get(format!("{}/v1/auth/challenge?machine_id={}", zid_endpoint, machine_id_uuid)).with_timeout(10_000);
        self.start_network_fetch(&challenge_request, PendingNetworkOp::RequestZidChallenge { client_pid, user_id, zid_endpoint, machine_key, cap_slots })
    }

    fn continue_zid_login_after_challenge(&mut self, client_pid: u32, user_id: u128, zid_endpoint: String, machine_key: MachineKeyRecord, challenge_response: zos_network::HttpSuccess, cap_slots: Vec<u32>) -> Result<(), AppError> {
        #[derive(serde::Deserialize)]
        struct ChallengeResponse { challenge: String, challenge_id: String }

        let challenge: ChallengeResponse = match serde_json::from_slice(&challenge_response.body) {
            Ok(c) => c,
            Err(_) => return response::send_zid_login_error(client_pid, &cap_slots, ZidError::InvalidChallenge),
        };

        let challenge_bytes = match base64_decode(&challenge.challenge) {
            Ok(b) => b,
            Err(_) => return response::send_zid_login_error(client_pid, &cap_slots, ZidError::InvalidChallenge),
        };

        let signature = sign_challenge(&challenge_bytes, &machine_key.signing_public_key);
        let signature_hex = bytes_to_hex(&signature);
        let machine_id_uuid = format_uuid(machine_key.machine_id);
        let login_body = format!(r#"{{"challenge_id":"{}","machine_id":"{}","signature":"{}"}}"#, challenge.challenge_id, machine_id_uuid, signature_hex);

        let login_request = HttpRequest::post(format!("{}/v1/auth/login/machine", zid_endpoint)).with_json_body(login_body.into_bytes()).with_timeout(10_000);
        self.start_network_fetch(&login_request, PendingNetworkOp::SubmitZidLogin { client_pid, user_id, zid_endpoint, cap_slots })
    }

    fn continue_zid_login_after_login(&mut self, client_pid: u32, user_id: u128, zid_endpoint: String, login_response: zos_network::HttpSuccess, cap_slots: Vec<u32>) -> Result<(), AppError> {
        let tokens: ZidTokens = match serde_json::from_slice(&login_response.body) {
            Ok(t) => t,
            Err(_) => return response::send_zid_login_error(client_pid, &cap_slots, ZidError::AuthenticationFailed),
        };

        let now = syscall::get_wallclock();
        let session = ZidSession {
            zid_endpoint: zid_endpoint.clone(),
            access_token: tokens.access_token.clone(),
            refresh_token: tokens.refresh_token.clone(),
            session_id: tokens.session_id.clone(),
            expires_at: now + (tokens.expires_in * 1000),
            created_at: now,
        };

        let session_path = format!("/home/{:032x}/.zos/identity/zid_session.json", user_id);
        match serde_json::to_vec(&session) {
            Ok(json_bytes) => self.start_storage_write(&format!("content:{}", session_path), &json_bytes.clone(), PendingStorageOp::WriteZidSessionContent { client_pid, user_id, tokens, json_bytes, cap_slots }),
            Err(e) => response::send_zid_login_error(client_pid, &cap_slots, ZidError::NetworkError(format!("Serialization failed: {}", e))),
        }
    }

    fn continue_zid_login_after_write_content(&mut self, client_pid: u32, user_id: u128, tokens: ZidTokens, json_bytes: Vec<u8>, cap_slots: Vec<u32>, result_type: u8) -> Result<(), AppError> {
        if result_type != storage_result::WRITE_OK {
            return response::send_zid_login_error(client_pid, &cap_slots, ZidError::NetworkError("Session write failed".into()));
        }

        let session_path = format!("/home/{:032x}/.zos/identity/zid_session.json", user_id);
        let now = syscall::get_wallclock();
        let inode = Inode::new_file(session_path.clone(), parent_path(&session_path).to_string(), "zid_session.json".to_string(), Some(user_id), json_bytes.len() as u64, None, now);

        match serde_json::to_vec(&inode) {
            Ok(inode_json) => self.start_storage_write(&format!("inode:{}", session_path), &inode_json, PendingStorageOp::WriteZidSessionInode { client_pid, tokens, cap_slots }),
            Err(e) => response::send_zid_login_error(client_pid, &cap_slots, ZidError::NetworkError(format!("Inode serialization failed: {}", e))),
        }
    }

    fn continue_zid_enroll_after_list(&mut self, client_pid: u32, user_id: u128, zid_endpoint: String, paths: Vec<String>, cap_slots: Vec<u32>) -> Result<(), AppError> {
        let path = paths.into_iter().find(|p| p.ends_with(".json"));
        match path {
            Some(p) => self.start_storage_read(&format!("content:{}", p), PendingStorageOp::ReadMachineKeyForZidEnroll { client_pid, user_id, zid_endpoint, cap_slots }),
            None => response::send_zid_enroll_error(client_pid, &cap_slots, ZidError::MachineKeyNotFound),
        }
    }

    fn continue_zid_enroll_after_read(&mut self, client_pid: u32, user_id: u128, zid_endpoint: String, data: &[u8], cap_slots: Vec<u32>) -> Result<(), AppError> {
        let machine_key: MachineKeyRecord = match serde_json::from_slice(data) {
            Ok(r) => r,
            Err(_) => return response::send_zid_enroll_error(client_pid, &cap_slots, ZidError::MachineKeyNotFound),
        };

        let machine_id_uuid = format_uuid(machine_key.machine_id);
        let public_key_hex = bytes_to_hex(&machine_key.signing_public_key);
        let enroll_body = format!(r#"{{"machine_id":"{}","public_key":"{}"}}"#, machine_id_uuid, public_key_hex);
        let enroll_request = HttpRequest::post(format!("{}/v1/identity", zid_endpoint)).with_json_body(enroll_body.into_bytes()).with_timeout(10_000);
        self.start_network_fetch(&enroll_request, PendingNetworkOp::SubmitZidEnroll { client_pid, user_id, zid_endpoint, cap_slots })
    }

    fn continue_zid_enroll_after_submit(&mut self, client_pid: u32, user_id: u128, zid_endpoint: String, enroll_response: zos_network::HttpSuccess, cap_slots: Vec<u32>) -> Result<(), AppError> {
        #[derive(serde::Deserialize)]
        struct EnrollResponse { access_token: String, refresh_token: String, session_id: String, expires_in: u64 }

        let enroll: EnrollResponse = match serde_json::from_slice(&enroll_response.body) {
            Ok(e) => e,
            Err(e) => return response::send_zid_enroll_error(client_pid, &cap_slots, ZidError::EnrollmentFailed(format!("Invalid response: {}", e))),
        };

        let tokens = ZidTokens { access_token: enroll.access_token, refresh_token: enroll.refresh_token, session_id: enroll.session_id, expires_in: enroll.expires_in };
        let session = ZidSession {
            zid_endpoint: zid_endpoint.clone(),
            access_token: tokens.access_token.clone(),
            refresh_token: tokens.refresh_token.clone(),
            session_id: tokens.session_id.clone(),
            expires_at: syscall::get_wallclock() + tokens.expires_in * 1000,
            created_at: syscall::get_wallclock(),
        };

        let json_bytes = match serde_json::to_vec(&session) {
            Ok(b) => b,
            Err(_) => return response::send_zid_enroll_success(client_pid, &cap_slots, tokens),
        };

        let session_path = format!("/home/{:032x}/.zos/identity/zid_session.json", user_id);
        self.start_storage_write(&format!("content:{}", session_path), &json_bytes.clone(), PendingStorageOp::WriteZidEnrollSessionContent { client_pid, user_id, tokens, json_bytes, cap_slots })
    }

    fn continue_attach_email_after_zid(&mut self, client_pid: u32, user_id: u128, email: String, cap_slots: Vec<u32>) -> Result<(), AppError> {
        let cred_path = CredentialStore::storage_path(user_id);
        self.start_storage_read(&format!("content:{}", cred_path), PendingStorageOp::ReadCredentialsForAttach { client_pid, user_id, email, cap_slots })
    }

    fn continue_attach_email_after_read(&mut self, client_pid: u32, user_id: u128, email: String, existing_store: Option<CredentialStore>, cap_slots: Vec<u32>) -> Result<(), AppError> {
        let now = syscall::get_wallclock();
        let mut store = existing_store.unwrap_or_else(|| CredentialStore::new(user_id));
        store.credentials.retain(|c| !(c.credential_type == CredentialType::Email && c.value == email && !c.verified));
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
            Ok(json_bytes) => self.start_storage_write(&format!("content:{}", cred_path), &json_bytes.clone(), PendingStorageOp::WriteEmailCredentialContent { client_pid, user_id, json_bytes, cap_slots }),
            Err(e) => response::send_attach_email_error(client_pid, &cap_slots, CredentialError::StorageError(format!("Serialization failed: {}", e))),
        }
    }

    fn continue_unlink_credential_after_read(&mut self, client_pid: u32, user_id: u128, credential_type: CredentialType, data: &[u8], cap_slots: Vec<u32>) -> Result<(), AppError> {
        let mut store: CredentialStore = match serde_json::from_slice(data) {
            Ok(s) => s,
            Err(e) => return response::send_unlink_credential_error(client_pid, &cap_slots, CredentialError::StorageError(format!("Parse failed: {}", e))),
        };

        let original_len = store.credentials.len();
        store.credentials.retain(|c| c.credential_type != credential_type);

        if store.credentials.len() == original_len {
            return response::send_unlink_credential_error(client_pid, &cap_slots, CredentialError::NotFound);
        }

        let cred_path = CredentialStore::storage_path(user_id);
        match serde_json::to_vec(&store) {
            Ok(json_bytes) => self.start_storage_write(&format!("content:{}", cred_path), &json_bytes.clone(), PendingStorageOp::WriteUnlinkedCredentialContent { client_pid, user_id, json_bytes, cap_slots }),
            Err(e) => response::send_unlink_credential_error(client_pid, &cap_slots, CredentialError::StorageError(format!("Serialization failed: {}", e))),
        }
    }
}

impl ZeroApp for IdentityService {
    fn manifest() -> &'static AppManifest {
        &IDENTITY_SERVICE_MANIFEST
    }

    fn init(&mut self, _ctx: &AppContext) -> Result<(), AppError> {
        syscall::debug("IdentityService: init");
        Ok(())
    }

    fn update(&mut self, ctx: &AppContext) -> ControlFlow {
        if !self.registered {
            syscall::debug("IdentityService: Registering with init");
            let name = b"identity_service";
            // Input endpoint is always slot 1 for services
            let endpoint_slot: u64 = ctx.input_endpoint.unwrap_or(1) as u64;
            let mut data = Vec::with_capacity(1 + name.len() + 8);
            data.push(name.len() as u8);
            data.extend_from_slice(name);
            data.extend_from_slice(&endpoint_slot.to_le_bytes());
            let _ = syscall::send(0, zos_process::init::MSG_REGISTER_SERVICE, &data);
            self.registered = true;
        }
        ControlFlow::Yield
    }

    fn on_message(&mut self, _ctx: &AppContext, msg: Message) -> Result<(), AppError> {
        match msg.tag {
            identity_key::MSG_GENERATE_NEURAL_KEY => self.handle_generate_neural_key(&msg),
            identity_key::MSG_RECOVER_NEURAL_KEY => self.handle_recover_neural_key(&msg),
            identity_key::MSG_GET_IDENTITY_KEY => self.handle_get_identity_key(&msg),
            identity_machine::MSG_CREATE_MACHINE_KEY => self.handle_create_machine_key(&msg),
            identity_machine::MSG_LIST_MACHINE_KEYS => self.handle_list_machine_keys(&msg),
            identity_machine::MSG_REVOKE_MACHINE_KEY => self.handle_revoke_machine_key(&msg),
            identity_machine::MSG_ROTATE_MACHINE_KEY => self.handle_rotate_machine_key(&msg),
            identity_machine::MSG_GET_MACHINE_KEY => self.handle_get_machine_key(&msg),
            identity_cred::MSG_ATTACH_EMAIL => self.handle_attach_email(&msg),
            identity_cred::MSG_GET_CREDENTIALS => self.handle_get_credentials(&msg),
            identity_cred::MSG_UNLINK_CREDENTIAL => self.handle_unlink_credential(&msg),
            identity_zid::MSG_ZID_LOGIN => self.handle_zid_login(&msg),
            identity_zid::MSG_ZID_ENROLL_MACHINE => self.handle_zid_enroll_machine(&msg),
            MSG_STORAGE_RESULT => self.handle_storage_result(&msg),
            net::MSG_NET_RESULT => self.handle_net_result(&msg),
            _ => {
                syscall::debug(&format!("IdentityService: Unknown message tag 0x{:x}", msg.tag));
                Ok(())
            }
        }
    }

    fn shutdown(&mut self, _ctx: &AppContext) {
        syscall::debug("IdentityService: shutdown");
    }
}

app_main!(IdentityService);

// Provide a main function for non-WASM targets (used for cargo check)
#[cfg(not(target_arch = "wasm32"))]
fn main() {
    // This is never called - binaries run as WASM
    panic!("This binary is designed for WASM only");
}
