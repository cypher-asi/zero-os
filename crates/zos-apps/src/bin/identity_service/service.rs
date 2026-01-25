//! Identity Service core implementation
//!
//! Contains the IdentityService struct, storage/network syscall helpers,
//! and result dispatchers for async operations.
//!
//! # Storage Access
//!
//! This service is being migrated to use VFS IPC (async pattern) for storage.
//! New code should use the `start_vfs_*` helpers. Legacy code still uses
//! `start_storage_*` helpers until fully migrated.
//!
//! All storage operations flow through VFS Service (PID 4) per Invariant 31.

use alloc::collections::BTreeMap;
use alloc::format;

use zos_apps::identity::network_handlers::{self, NetworkHandlerResult};
use zos_apps::identity::pending::{PendingNetworkOp, PendingStorageOp};
use zos_apps::identity::response;
use zos_apps::identity::storage_handlers::{self, StorageHandlerResult};
use zos_apps::syscall;
use zos_apps::vfs_async;
use zos_apps::{AppError, Message};
use zos_identity::error::CredentialError;
use zos_identity::keystore::{CredentialStore, LocalKeyStore};
use zos_identity::KeyError;
use zos_network::{HttpRequest, HttpResponse, NetworkError};
use zos_process::storage_result;
use zos_vfs::ipc::vfs_msg;

/// IdentityService - manages user cryptographic identities
#[derive(Default)]
pub struct IdentityService {
    /// Whether we have registered with init
    pub registered: bool,
    /// Pending storage operations: request_id -> operation context
    /// Note: This is used by legacy storage syscalls. New VFS operations
    /// use `pending_vfs_ops` instead.
    pub pending_ops: BTreeMap<u32, PendingStorageOp>,
    /// Pending VFS operations: vfs_op_id -> operation context
    /// VFS IPC doesn't return request IDs, so we use our own counter.
    pub pending_vfs_ops: BTreeMap<u32, PendingStorageOp>,
    /// Counter for generating VFS operation IDs
    pub next_vfs_op_id: u32,
    /// Pending network operations: request_id -> operation context
    pub pending_net_ops: BTreeMap<u32, PendingNetworkOp>,
}

impl IdentityService {
    // =========================================================================
    // Storage syscall helpers (async, non-blocking)
    // =========================================================================

    pub fn start_storage_read(
        &mut self,
        key: &str,
        pending_op: PendingStorageOp,
    ) -> Result<(), AppError> {
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
                syscall::debug(&format!(
                    "IdentityService: storage_read_async failed: {}",
                    e
                ));
                Err(AppError::IpcError(format!("Storage read failed: {}", e)))
            }
        }
    }

    pub fn start_storage_write(
        &mut self,
        key: &str,
        value: &[u8],
        pending_op: PendingStorageOp,
    ) -> Result<(), AppError> {
        match syscall::storage_write_async(key, value) {
            Ok(request_id) => {
                syscall::debug(&format!(
                    "IdentityService: storage_write_async({}, {} bytes) -> request_id={}",
                    key,
                    value.len(),
                    request_id
                ));
                self.pending_ops.insert(request_id, pending_op);
                Ok(())
            }
            Err(e) => {
                syscall::debug(&format!(
                    "IdentityService: storage_write_async failed: {}",
                    e
                ));
                Err(AppError::IpcError(format!("Storage write failed: {}", e)))
            }
        }
    }

    pub fn start_storage_delete(
        &mut self,
        key: &str,
        pending_op: PendingStorageOp,
    ) -> Result<(), AppError> {
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
                syscall::debug(&format!(
                    "IdentityService: storage_delete_async failed: {}",
                    e
                ));
                Err(AppError::IpcError(format!("Storage delete failed: {}", e)))
            }
        }
    }

    pub fn start_storage_exists(
        &mut self,
        key: &str,
        pending_op: PendingStorageOp,
    ) -> Result<(), AppError> {
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
                syscall::debug(&format!(
                    "IdentityService: storage_exists_async failed: {}",
                    e
                ));
                Err(AppError::IpcError(format!("Storage exists failed: {}", e)))
            }
        }
    }

    pub fn start_storage_list(
        &mut self,
        prefix: &str,
        pending_op: PendingStorageOp,
    ) -> Result<(), AppError> {
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
                syscall::debug(&format!(
                    "IdentityService: storage_list_async failed: {}",
                    e
                ));
                Err(AppError::IpcError(format!("Storage list failed: {}", e)))
            }
        }
    }

    // =========================================================================
    // VFS IPC helpers (async, non-blocking) - Invariant 31 compliant
    // =========================================================================
    //
    // These helpers route storage operations through VFS Service (PID 4) via IPC.
    // Use these instead of the legacy storage syscall helpers for new code.

    /// Start async VFS read and track the pending operation.
    /// Uses VFS IPC instead of direct storage syscalls per Invariant 31.
    pub fn start_vfs_read(
        &mut self,
        path: &str,
        pending_op: PendingStorageOp,
    ) -> Result<(), AppError> {
        let op_id = self.next_vfs_op_id;
        self.next_vfs_op_id += 1;
        
        syscall::debug(&format!(
            "IdentityService: vfs_read({}) -> op_id={}",
            path, op_id
        ));
        
        vfs_async::send_read_request(path)?;
        self.pending_vfs_ops.insert(op_id, pending_op);
        Ok(())
    }

    /// Start async VFS write and track the pending operation.
    /// Uses VFS IPC instead of direct storage syscalls per Invariant 31.
    pub fn start_vfs_write(
        &mut self,
        path: &str,
        value: &[u8],
        pending_op: PendingStorageOp,
    ) -> Result<(), AppError> {
        let op_id = self.next_vfs_op_id;
        self.next_vfs_op_id += 1;
        
        syscall::debug(&format!(
            "IdentityService: vfs_write({}, {} bytes) -> op_id={}",
            path, value.len(), op_id
        ));
        
        vfs_async::send_write_request(path, value)?;
        self.pending_vfs_ops.insert(op_id, pending_op);
        Ok(())
    }

    /// Start async VFS delete (unlink) and track the pending operation.
    /// Uses VFS IPC instead of direct storage syscalls per Invariant 31.
    pub fn start_vfs_delete(
        &mut self,
        path: &str,
        pending_op: PendingStorageOp,
    ) -> Result<(), AppError> {
        let op_id = self.next_vfs_op_id;
        self.next_vfs_op_id += 1;
        
        syscall::debug(&format!(
            "IdentityService: vfs_unlink({}) -> op_id={}",
            path, op_id
        ));
        
        vfs_async::send_unlink_request(path)?;
        self.pending_vfs_ops.insert(op_id, pending_op);
        Ok(())
    }

    /// Start async VFS exists check and track the pending operation.
    /// Uses VFS IPC instead of direct storage syscalls per Invariant 31.
    pub fn start_vfs_exists(
        &mut self,
        path: &str,
        pending_op: PendingStorageOp,
    ) -> Result<(), AppError> {
        let op_id = self.next_vfs_op_id;
        self.next_vfs_op_id += 1;
        
        syscall::debug(&format!(
            "IdentityService: vfs_exists({}) -> op_id={}",
            path, op_id
        ));
        
        vfs_async::send_exists_request(path)?;
        self.pending_vfs_ops.insert(op_id, pending_op);
        Ok(())
    }

    /// Start async VFS readdir (list directory) and track the pending operation.
    /// Uses VFS IPC instead of direct storage syscalls per Invariant 31.
    pub fn start_vfs_readdir(
        &mut self,
        path: &str,
        pending_op: PendingStorageOp,
    ) -> Result<(), AppError> {
        let op_id = self.next_vfs_op_id;
        self.next_vfs_op_id += 1;
        
        syscall::debug(&format!(
            "IdentityService: vfs_readdir({}) -> op_id={}",
            path, op_id
        ));
        
        vfs_async::send_readdir_request(path)?;
        self.pending_vfs_ops.insert(op_id, pending_op);
        Ok(())
    }

    /// Start async VFS mkdir and track the pending operation.
    /// Uses VFS IPC instead of direct storage syscalls per Invariant 31.
    pub fn start_vfs_mkdir(
        &mut self,
        path: &str,
        create_parents: bool,
        pending_op: PendingStorageOp,
    ) -> Result<(), AppError> {
        let op_id = self.next_vfs_op_id;
        self.next_vfs_op_id += 1;
        
        syscall::debug(&format!(
            "IdentityService: vfs_mkdir({}, create_parents={}) -> op_id={}",
            path, create_parents, op_id
        ));
        
        vfs_async::send_mkdir_request(path, create_parents)?;
        self.pending_vfs_ops.insert(op_id, pending_op);
        Ok(())
    }

    // =========================================================================
    // Network syscall helpers (async, non-blocking)
    // =========================================================================

    pub fn start_network_fetch(
        &mut self,
        request: &HttpRequest,
        pending_op: PendingNetworkOp,
    ) -> Result<(), AppError> {
        let request_json = match serde_json::to_vec(request) {
            Ok(json) => json,
            Err(e) => {
                syscall::debug(&format!(
                    "IdentityService: Failed to serialize HTTP request: {}",
                    e
                ));
                return Err(AppError::IpcError(format!(
                    "Request serialization failed: {}",
                    e
                )));
            }
        };

        match syscall::network_fetch_async(&request_json) {
            Ok(request_id) => {
                syscall::debug(&format!(
                    "IdentityService: network_fetch_async({} {}) -> request_id={}",
                    request.method.as_str(),
                    request.url,
                    request_id
                ));
                self.pending_net_ops.insert(request_id, pending_op);
                Ok(())
            }
            Err(e) => {
                syscall::debug(&format!(
                    "IdentityService: network_fetch_async failed: {}",
                    e
                ));
                Err(AppError::IpcError(format!("Network fetch failed: {}", e)))
            }
        }
    }

    // =========================================================================
    // Storage result handler (dispatches to storage_handlers module)
    // =========================================================================

    pub fn handle_storage_result(&mut self, msg: &Message) -> Result<(), AppError> {
        if msg.data.len() < 9 {
            return Ok(());
        }

        let request_id = u32::from_le_bytes([msg.data[0], msg.data[1], msg.data[2], msg.data[3]]);
        let result_type = msg.data[4];
        let data_len =
            u32::from_le_bytes([msg.data[5], msg.data[6], msg.data[7], msg.data[8]]) as usize;
        let data = if data_len > 0 && msg.data.len() >= 9 + data_len {
            &msg.data[9..9 + data_len]
        } else {
            &[]
        };

        let pending_op = match self.pending_ops.remove(&request_id) {
            Some(op) => op,
            None => return Ok(()),
        };

        self.dispatch_storage_result(pending_op, result_type, data)
    }

    fn dispatch_storage_result(
        &mut self,
        op: PendingStorageOp,
        result_type: u8,
        data: &[u8],
    ) -> Result<(), AppError> {
        use crate::handlers::{credentials, keys, session};

        match op {
            PendingStorageOp::CheckIdentityDirectory {
                client_pid,
                user_id,
                cap_slots,
            } => {
                let exists =
                    result_type == storage_result::EXISTS_OK && !data.is_empty() && data[0] == 1;
                keys::continue_generate_after_directory_check(
                    self, client_pid, user_id, exists, cap_slots,
                )
            }
            PendingStorageOp::CreateIdentityDirectory {
                client_pid,
                user_id,
                cap_slots,
                directories,
            } => {
                if result_type == storage_result::WRITE_OK {
                    // Directory created successfully, continue with remaining directories
                    keys::continue_create_directories(self, client_pid, user_id, directories, cap_slots)
                } else {
                    // Directory creation failed
                    syscall::debug("IdentityService: Failed to create identity directory");
                    response::send_neural_key_error(
                        client_pid,
                        &cap_slots,
                        KeyError::StorageError("Failed to create identity directory".into()),
                    )
                }
            }
            PendingStorageOp::CheckKeyExists {
                client_pid,
                user_id,
                cap_slots,
            } => {
                let exists =
                    result_type == storage_result::EXISTS_OK && !data.is_empty() && data[0] == 1;
                keys::continue_generate_after_exists_check(
                    self, client_pid, user_id, exists, cap_slots,
                )
            }
            PendingStorageOp::WriteKeyStoreContent {
                client_pid,
                user_id,
                result,
                json_bytes,
                cap_slots,
            } => self.handle_storage_handler_result(
                storage_handlers::handle_write_key_store_content(
                    client_pid,
                    user_id,
                    result,
                    json_bytes,
                    cap_slots,
                    result_type,
                ),
            ),
            PendingStorageOp::WriteKeyStoreInode {
                client_pid,
                result,
                cap_slots,
            } => {
                self.handle_storage_handler_result(storage_handlers::handle_write_key_store_inode(
                    client_pid,
                    result,
                    cap_slots,
                    result_type,
                ))
            }
            PendingStorageOp::GetIdentityKey {
                client_pid,
                cap_slots,
            } => self.handle_storage_handler_result(storage_handlers::handle_get_identity_key(
                client_pid,
                cap_slots,
                result_type,
                data,
            )),
            PendingStorageOp::ReadIdentityForRecovery {
                client_pid,
                user_id,
                zid_shards,
                cap_slots,
            } => {
                // SECURITY: Parse the LocalKeyStore to get the stored identity public key
                // for verification before reconstructing the Neural Key from shards.
                syscall::debug(&format!(
                    "IdentityService: ReadIdentityForRecovery result_type={}, data_len={}",
                    result_type, data.len()
                ));
                if result_type != storage_result::READ_OK || data.is_empty() {
                    syscall::debug(&format!(
                        "IdentityService: Identity read for recovery failed - result_type={} (expected {}), data_empty={}",
                        result_type, storage_result::READ_OK, data.is_empty()
                    ));
                    return response::send_recover_key_error(
                        client_pid,
                        &cap_slots,
                        KeyError::IdentityKeyRequired,
                    );
                }
                let key_store: LocalKeyStore = match serde_json::from_slice(data) {
                    Ok(ks) => ks,
                    Err(e) => {
                        syscall::debug(&format!(
                            "IdentityService: Failed to parse LocalKeyStore for recovery: {}",
                            e
                        ));
                        return response::send_recover_key_error(
                            client_pid,
                            &cap_slots,
                            KeyError::StorageError("Corrupted identity key store".into()),
                        );
                    }
                };
                keys::continue_recover_after_identity_read(
                    self,
                    client_pid,
                    user_id,
                    zid_shards,
                    key_store.identity_signing_public_key,
                    cap_slots,
                )
            }
            PendingStorageOp::WriteRecoveredKeyStoreContent {
                client_pid,
                user_id,
                result,
                json_bytes,
                cap_slots,
            } => self.handle_storage_handler_result(
                storage_handlers::handle_write_recovered_content(
                    client_pid,
                    user_id,
                    result,
                    json_bytes,
                    cap_slots,
                    result_type,
                ),
            ),
            PendingStorageOp::WriteRecoveredKeyStoreInode {
                client_pid,
                result,
                cap_slots,
            } => {
                self.handle_storage_handler_result(storage_handlers::handle_write_recovered_inode(
                    client_pid,
                    result,
                    cap_slots,
                    result_type,
                ))
            }
            PendingStorageOp::ReadIdentityForMachine {
                client_pid,
                request,
                cap_slots,
            } => {
                // Parse the LocalKeyStore to get the stored identity public key
                syscall::debug(&format!(
                    "IdentityService: ReadIdentityForMachine result_type={}, data_len={}",
                    result_type, data.len()
                ));
                if result_type != storage_result::READ_OK || data.is_empty() {
                    syscall::debug(&format!(
                        "IdentityService: Identity read failed - result_type={} (expected {}), data_empty={}",
                        result_type, storage_result::READ_OK, data.is_empty()
                    ));
                    return response::send_create_machine_key_error(
                        client_pid,
                        &cap_slots,
                        KeyError::IdentityKeyRequired,
                    );
                }
                let key_store: LocalKeyStore = match serde_json::from_slice(&data) {
                    Ok(ks) => ks,
                    Err(e) => {
                        syscall::debug(&format!(
                            "IdentityService: Failed to parse LocalKeyStore: {}",
                            e
                        ));
                        return response::send_create_machine_key_error(
                            client_pid,
                            &cap_slots,
                            KeyError::StorageError("Corrupted identity key store".into()),
                        );
                    }
                };
                keys::continue_create_machine_after_identity_read(
                    self,
                    client_pid,
                    request,
                    key_store.identity_signing_public_key,
                    cap_slots,
                )
            }
            PendingStorageOp::WriteMachineKeyContent {
                client_pid,
                user_id,
                record,
                json_bytes,
                cap_slots,
            } => self.handle_storage_handler_result(
                storage_handlers::handle_write_machine_key_content(
                    client_pid,
                    user_id,
                    record,
                    json_bytes,
                    cap_slots,
                    result_type,
                ),
            ),
            PendingStorageOp::WriteMachineKeyInode {
                client_pid,
                record,
                cap_slots,
            } => self.handle_storage_handler_result(
                storage_handlers::handle_write_machine_key_inode(
                    client_pid,
                    record,
                    cap_slots,
                    result_type,
                ),
            ),
            PendingStorageOp::ListMachineKeys {
                client_pid,
                user_id,
                cap_slots,
            } => self.handle_storage_handler_result(storage_handlers::handle_list_machine_keys(
                client_pid,
                user_id,
                cap_slots,
                result_type,
                data,
            )),
            PendingStorageOp::ReadMachineKey {
                client_pid,
                user_id,
                remaining_paths,
                records,
                cap_slots,
            } => self.handle_storage_handler_result(storage_handlers::handle_read_machine_key(
                client_pid,
                user_id,
                remaining_paths,
                records,
                cap_slots,
                result_type,
                data,
            )),
            PendingStorageOp::DeleteMachineKey {
                client_pid,
                user_id,
                machine_id,
                cap_slots,
            } => self.handle_storage_handler_result(storage_handlers::handle_delete_machine_key(
                client_pid,
                user_id,
                machine_id,
                cap_slots,
                result_type,
            )),
            PendingStorageOp::DeleteMachineKeyInode {
                client_pid,
                cap_slots,
            } => self.handle_storage_handler_result(
                storage_handlers::handle_delete_machine_key_inode(
                    client_pid,
                    cap_slots,
                    result_type,
                ),
            ),
            PendingStorageOp::ReadMachineForRotate {
                client_pid,
                user_id,
                machine_id,
                cap_slots,
            } => {
                if result_type == storage_result::READ_OK {
                    keys::continue_rotate_after_read(
                        self, client_pid, user_id, machine_id, data, cap_slots,
                    )
                } else {
                    response::send_rotate_machine_key_error(
                        client_pid,
                        &cap_slots,
                        zos_identity::KeyError::MachineKeyNotFound,
                    )
                }
            }
            PendingStorageOp::WriteRotatedMachineKeyContent {
                client_pid,
                user_id,
                record,
                json_bytes,
                cap_slots,
            } => {
                self.handle_storage_handler_result(storage_handlers::handle_write_rotated_content(
                    client_pid,
                    user_id,
                    record,
                    json_bytes,
                    cap_slots,
                    result_type,
                ))
            }
            PendingStorageOp::WriteRotatedMachineKeyInode {
                client_pid,
                record,
                cap_slots,
            } => self.handle_storage_handler_result(storage_handlers::handle_write_rotated_inode(
                client_pid,
                record,
                cap_slots,
                result_type,
            )),
            PendingStorageOp::ReadSingleMachineKey {
                client_pid,
                cap_slots,
            } => self.handle_storage_handler_result(
                storage_handlers::handle_read_single_machine_key(
                    client_pid,
                    cap_slots,
                    result_type,
                    data,
                ),
            ),
            PendingStorageOp::ReadCredentialsForAttach {
                client_pid,
                user_id,
                email,
                cap_slots,
            } => {
                let existing_store = if result_type == storage_result::READ_OK && !data.is_empty() {
                    serde_json::from_slice::<CredentialStore>(data).ok()
                } else {
                    None
                };
                credentials::continue_attach_email_after_read(
                    self,
                    client_pid,
                    user_id,
                    email,
                    existing_store,
                    cap_slots,
                )
            }
            PendingStorageOp::GetCredentials {
                client_pid,
                cap_slots,
            } => self.handle_storage_handler_result(storage_handlers::handle_get_credentials(
                client_pid,
                cap_slots,
                result_type,
                data,
            )),
            PendingStorageOp::ReadCredentialsForUnlink {
                client_pid,
                user_id,
                credential_type,
                cap_slots,
            } => {
                if result_type == storage_result::READ_OK && !data.is_empty() {
                    credentials::continue_unlink_credential_after_read(
                        self,
                        client_pid,
                        user_id,
                        credential_type,
                        data,
                        cap_slots,
                    )
                } else {
                    response::send_unlink_credential_error(
                        client_pid,
                        &cap_slots,
                        CredentialError::NotFound,
                    )
                }
            }
            PendingStorageOp::WriteUnlinkedCredentialContent {
                client_pid,
                user_id,
                json_bytes,
                cap_slots,
            } => {
                self.handle_storage_handler_result(storage_handlers::handle_write_unlinked_content(
                    client_pid,
                    user_id,
                    json_bytes,
                    cap_slots,
                    result_type,
                ))
            }
            PendingStorageOp::WriteUnlinkedCredentialInode {
                client_pid,
                cap_slots,
            } => self.handle_storage_handler_result(storage_handlers::handle_write_unlinked_inode(
                client_pid,
                cap_slots,
                result_type,
            )),
            PendingStorageOp::WriteEmailCredentialContent {
                client_pid,
                user_id,
                json_bytes,
                cap_slots,
            } => self.handle_storage_handler_result(
                storage_handlers::handle_write_email_cred_content(
                    client_pid,
                    user_id,
                    json_bytes,
                    cap_slots,
                    result_type,
                ),
            ),
            PendingStorageOp::WriteEmailCredentialInode {
                client_pid,
                cap_slots,
            } => self.handle_storage_handler_result(
                storage_handlers::handle_write_email_cred_inode(client_pid, cap_slots, result_type),
            ),
            PendingStorageOp::ReadMachineKeyForZidLogin {
                client_pid,
                user_id,
                zid_endpoint,
                cap_slots,
            } => {
                match storage_handlers::handle_read_machine_for_zid_login(
                    client_pid,
                    user_id,
                    zid_endpoint,
                    cap_slots,
                    result_type,
                    data,
                ) {
                    Ok(storage_handlers::ZidLoginReadResult::PathList {
                        paths,
                        client_pid,
                        user_id,
                        zid_endpoint,
                        cap_slots,
                    }) => session::continue_zid_login_after_list(
                        self,
                        client_pid,
                        user_id,
                        zid_endpoint,
                        paths,
                        cap_slots,
                    ),
                    Ok(storage_handlers::ZidLoginReadResult::MachineKeyData {
                        data,
                        client_pid,
                        user_id,
                        zid_endpoint,
                        cap_slots,
                    }) => session::continue_zid_login_after_read(
                        self,
                        client_pid,
                        user_id,
                        zid_endpoint,
                        &data,
                        cap_slots,
                    ),
                    Err(result) => self.handle_storage_handler_result(*result),
                }
            }
            PendingStorageOp::WriteZidSessionContent {
                client_pid,
                user_id,
                tokens,
                json_bytes,
                cap_slots,
            } => session::continue_zid_login_after_write_content(
                self,
                client_pid,
                user_id,
                tokens,
                json_bytes,
                cap_slots,
                result_type,
            ),
            PendingStorageOp::WriteZidSessionInode {
                client_pid,
                tokens,
                cap_slots,
            } => self.handle_storage_handler_result(
                storage_handlers::handle_write_zid_session_inode(
                    client_pid,
                    tokens,
                    cap_slots,
                    result_type,
                ),
            ),
            PendingStorageOp::ReadMachineKeyForZidEnroll {
                client_pid,
                user_id,
                zid_endpoint,
                cap_slots,
            } => {
                match storage_handlers::handle_read_machine_for_zid_enroll(
                    client_pid,
                    user_id,
                    zid_endpoint,
                    cap_slots,
                    result_type,
                    data,
                ) {
                    Ok(storage_handlers::ZidEnrollReadResult::PathList {
                        paths,
                        client_pid,
                        user_id,
                        zid_endpoint,
                        cap_slots,
                    }) => session::continue_zid_enroll_after_list(
                        self,
                        client_pid,
                        user_id,
                        zid_endpoint,
                        paths,
                        cap_slots,
                    ),
                    Ok(storage_handlers::ZidEnrollReadResult::MachineKeyData {
                        data,
                        client_pid,
                        user_id,
                        zid_endpoint,
                        cap_slots,
                    }) => session::continue_zid_enroll_after_read(
                        self,
                        client_pid,
                        user_id,
                        zid_endpoint,
                        &data,
                        cap_slots,
                    ),
                    Err(result) => self.handle_storage_handler_result(*result),
                }
            }
            PendingStorageOp::WriteZidEnrollSessionContent {
                client_pid,
                user_id,
                tokens,
                json_bytes,
                cap_slots,
            } => {
                if result_type == storage_result::WRITE_OK {
                    let session_path =
                        format!("/home/{}/.zos/identity/zid_session.json", user_id);
                    self.start_storage_write(
                        &format!("inode:{}", session_path),
                        &json_bytes,
                        PendingStorageOp::WriteZidEnrollSessionInode {
                            client_pid,
                            tokens,
                            cap_slots,
                        },
                    )
                } else {
                    response::send_zid_enroll_error(
                        client_pid,
                        &cap_slots,
                        zos_identity::error::ZidError::EnrollmentFailed(
                            "Session write failed".into(),
                        ),
                    )
                }
            }
            PendingStorageOp::WriteZidEnrollSessionInode {
                client_pid,
                tokens,
                cap_slots,
            } => self.handle_storage_handler_result(
                storage_handlers::handle_write_zid_enroll_session_inode(
                    client_pid,
                    tokens,
                    cap_slots,
                    result_type,
                ),
            ),
            PendingStorageOp::ReadIdentityPreferences {
                client_pid,
                user_id,
                cap_slots,
            } => self.handle_storage_handler_result(
                storage_handlers::handle_read_identity_preferences(
                    client_pid,
                    user_id,
                    cap_slots,
                    result_type,
                    data,
                ),
            ),
            PendingStorageOp::ReadPreferencesForUpdate {
                client_pid,
                user_id,
                new_key_scheme,
                cap_slots,
            } => {
                // Read existing prefs or use default, then write with new scheme
                let mut preferences = if result_type == storage_result::READ_OK && !data.is_empty() {
                    serde_json::from_slice::<zos_identity::ipc::IdentityPreferences>(data)
                        .unwrap_or_default()
                } else {
                    zos_identity::ipc::IdentityPreferences::default()
                };
                
                preferences.default_key_scheme = new_key_scheme;
                
                match serde_json::to_vec(&preferences) {
                    Ok(json_bytes) => {
                        let prefs_path = zos_identity::ipc::IdentityPreferences::storage_path(user_id);
                        self.start_storage_write(
                            &format!("content:{}", prefs_path),
                            &json_bytes,
                            PendingStorageOp::WritePreferencesContent {
                                client_pid,
                                user_id,
                                json_bytes: json_bytes.clone(),
                                cap_slots,
                            },
                        )
                    }
                    Err(_) => response::send_set_default_key_scheme_error(
                        client_pid,
                        &cap_slots,
                        zos_identity::KeyError::StorageError("Serialization failed".into()),
                    ),
                }
            }
            PendingStorageOp::WritePreferencesContent {
                client_pid,
                user_id,
                json_bytes,
                cap_slots,
            } => {
                if result_type == storage_result::WRITE_OK {
                    let prefs_path = zos_identity::ipc::IdentityPreferences::storage_path(user_id);
                    self.start_storage_write(
                        &format!("inode:{}", prefs_path),
                        &json_bytes,
                        PendingStorageOp::WritePreferencesInode {
                            client_pid,
                            cap_slots,
                        },
                    )
                } else {
                    response::send_set_default_key_scheme_error(
                        client_pid,
                        &cap_slots,
                        zos_identity::KeyError::StorageError("Content write failed".into()),
                    )
                }
            }
            PendingStorageOp::WritePreferencesInode {
                client_pid,
                cap_slots,
            } => self.handle_storage_handler_result(
                storage_handlers::handle_write_preferences_inode(
                    client_pid,
                    cap_slots,
                    result_type,
                ),
            ),
        }
    }

    fn handle_storage_handler_result(
        &mut self,
        result: StorageHandlerResult,
    ) -> Result<(), AppError> {
        match result {
            StorageHandlerResult::Done(r) => r,
            StorageHandlerResult::ContinueWrite {
                key,
                value,
                next_op,
            } => self.start_storage_write(&key, &value, next_op),
            StorageHandlerResult::ContinueRead { key, next_op } => {
                self.start_storage_read(&key, next_op)
            }
            StorageHandlerResult::ContinueDelete { key, next_op } => {
                self.start_storage_delete(&key, next_op)
            }
        }
    }

    // =========================================================================
    // VFS result handler (dispatches to per-response-type handlers)
    // =========================================================================

    /// Handle VFS IPC responses. Routes to specific handlers based on response type.
    ///
    /// VFS IPC doesn't use request IDs, so we process pending operations in FIFO order.
    pub fn handle_vfs_result(&mut self, msg: &Message) -> Result<(), AppError> {
        syscall::debug(&format!(
            "IdentityService: handle_vfs_result tag=0x{:x}, data_len={}",
            msg.tag, msg.data.len()
        ));

        match msg.tag {
            vfs_msg::MSG_VFS_READ_RESPONSE => self.handle_vfs_read_response(msg),
            vfs_msg::MSG_VFS_WRITE_RESPONSE => self.handle_vfs_write_response(msg),
            vfs_msg::MSG_VFS_EXISTS_RESPONSE => self.handle_vfs_exists_response(msg),
            vfs_msg::MSG_VFS_MKDIR_RESPONSE => self.handle_vfs_mkdir_response(msg),
            vfs_msg::MSG_VFS_READDIR_RESPONSE => self.handle_vfs_readdir_response(msg),
            vfs_msg::MSG_VFS_UNLINK_RESPONSE => self.handle_vfs_unlink_response(msg),
            _ => {
                syscall::debug(&format!(
                    "IdentityService: Unhandled VFS response tag 0x{:x}",
                    msg.tag
                ));
                Ok(())
            }
        }
    }

    /// Take the next pending VFS operation (FIFO order).
    /// Returns None if no operations are pending.
    fn take_next_pending_vfs_op(&mut self) -> Option<PendingStorageOp> {
        // Get the smallest key (oldest operation)
        let key = *self.pending_vfs_ops.keys().next()?;
        self.pending_vfs_ops.remove(&key)
    }

    /// Handle VFS read response (MSG_VFS_READ_RESPONSE)
    fn handle_vfs_read_response(&mut self, msg: &Message) -> Result<(), AppError> {
        let pending_op = match self.take_next_pending_vfs_op() {
            Some(op) => op,
            None => {
                syscall::debug("IdentityService: VFS read response but no pending operation");
                return Ok(());
            }
        };

        // Parse VFS response
        let result = vfs_async::parse_read_response(&msg.data);

        // Dispatch based on operation type
        self.dispatch_vfs_read_result(pending_op, result)
    }

    /// Handle VFS write response (MSG_VFS_WRITE_RESPONSE)
    fn handle_vfs_write_response(&mut self, msg: &Message) -> Result<(), AppError> {
        let pending_op = match self.take_next_pending_vfs_op() {
            Some(op) => op,
            None => {
                syscall::debug("IdentityService: VFS write response but no pending operation");
                return Ok(());
            }
        };

        // Parse VFS response
        let result = vfs_async::parse_write_response(&msg.data);

        // Dispatch based on operation type
        self.dispatch_vfs_write_result(pending_op, result)
    }

    /// Handle VFS exists response (MSG_VFS_EXISTS_RESPONSE)
    fn handle_vfs_exists_response(&mut self, msg: &Message) -> Result<(), AppError> {
        let pending_op = match self.take_next_pending_vfs_op() {
            Some(op) => op,
            None => {
                syscall::debug("IdentityService: VFS exists response but no pending operation");
                return Ok(());
            }
        };

        // Parse VFS response
        let result = vfs_async::parse_exists_response(&msg.data);

        // Dispatch based on operation type
        self.dispatch_vfs_exists_result(pending_op, result)
    }

    /// Handle VFS mkdir response (MSG_VFS_MKDIR_RESPONSE)
    fn handle_vfs_mkdir_response(&mut self, msg: &Message) -> Result<(), AppError> {
        let pending_op = match self.take_next_pending_vfs_op() {
            Some(op) => op,
            None => {
                syscall::debug("IdentityService: VFS mkdir response but no pending operation");
                return Ok(());
            }
        };

        // Parse VFS response
        let result = vfs_async::parse_mkdir_response(&msg.data);

        // Dispatch based on operation type
        self.dispatch_vfs_mkdir_result(pending_op, result)
    }

    /// Handle VFS readdir response (MSG_VFS_READDIR_RESPONSE)
    fn handle_vfs_readdir_response(&mut self, msg: &Message) -> Result<(), AppError> {
        let pending_op = match self.take_next_pending_vfs_op() {
            Some(op) => op,
            None => {
                syscall::debug("IdentityService: VFS readdir response but no pending operation");
                return Ok(());
            }
        };

        // Parse VFS response
        let result = vfs_async::parse_readdir_response(&msg.data);

        // Dispatch based on operation type
        self.dispatch_vfs_readdir_result(pending_op, result)
    }

    /// Handle VFS unlink response (MSG_VFS_UNLINK_RESPONSE)
    fn handle_vfs_unlink_response(&mut self, msg: &Message) -> Result<(), AppError> {
        let pending_op = match self.take_next_pending_vfs_op() {
            Some(op) => op,
            None => {
                syscall::debug("IdentityService: VFS unlink response but no pending operation");
                return Ok(());
            }
        };

        // Parse VFS response
        let result = vfs_async::parse_unlink_response(&msg.data);

        // Dispatch based on operation type
        self.dispatch_vfs_unlink_result(pending_op, result)
    }

    // =========================================================================
    // VFS result dispatchers (placeholder - will be populated as handlers migrate)
    // =========================================================================

    /// Dispatch VFS read result to appropriate handler based on pending operation type.
    fn dispatch_vfs_read_result(
        &mut self,
        op: PendingStorageOp,
        result: Result<alloc::vec::Vec<u8>, alloc::string::String>,
    ) -> Result<(), AppError> {
        // TODO: Implement as handlers are migrated from storage syscalls to VFS IPC
        syscall::debug(&format!(
            "IdentityService: dispatch_vfs_read_result for {:?}, success={}",
            core::mem::discriminant(&op),
            result.is_ok()
        ));
        Ok(())
    }

    /// Dispatch VFS write result to appropriate handler based on pending operation type.
    fn dispatch_vfs_write_result(
        &mut self,
        op: PendingStorageOp,
        result: Result<(), alloc::string::String>,
    ) -> Result<(), AppError> {
        // TODO: Implement as handlers are migrated from storage syscalls to VFS IPC
        syscall::debug(&format!(
            "IdentityService: dispatch_vfs_write_result for {:?}, success={}",
            core::mem::discriminant(&op),
            result.is_ok()
        ));
        Ok(())
    }

    /// Dispatch VFS exists result to appropriate handler based on pending operation type.
    fn dispatch_vfs_exists_result(
        &mut self,
        op: PendingStorageOp,
        result: Result<bool, alloc::string::String>,
    ) -> Result<(), AppError> {
        // TODO: Implement as handlers are migrated from storage syscalls to VFS IPC
        syscall::debug(&format!(
            "IdentityService: dispatch_vfs_exists_result for {:?}, result={:?}",
            core::mem::discriminant(&op),
            result
        ));
        Ok(())
    }

    /// Dispatch VFS mkdir result to appropriate handler based on pending operation type.
    fn dispatch_vfs_mkdir_result(
        &mut self,
        op: PendingStorageOp,
        result: Result<(), alloc::string::String>,
    ) -> Result<(), AppError> {
        // TODO: Implement as handlers are migrated from storage syscalls to VFS IPC
        syscall::debug(&format!(
            "IdentityService: dispatch_vfs_mkdir_result for {:?}, success={}",
            core::mem::discriminant(&op),
            result.is_ok()
        ));
        Ok(())
    }

    /// Dispatch VFS readdir result to appropriate handler based on pending operation type.
    fn dispatch_vfs_readdir_result(
        &mut self,
        op: PendingStorageOp,
        result: Result<alloc::vec::Vec<zos_vfs::types::DirEntry>, alloc::string::String>,
    ) -> Result<(), AppError> {
        // TODO: Implement as handlers are migrated from storage syscalls to VFS IPC
        syscall::debug(&format!(
            "IdentityService: dispatch_vfs_readdir_result for {:?}, success={}",
            core::mem::discriminant(&op),
            result.is_ok()
        ));
        Ok(())
    }

    /// Dispatch VFS unlink result to appropriate handler based on pending operation type.
    fn dispatch_vfs_unlink_result(
        &mut self,
        op: PendingStorageOp,
        result: Result<(), alloc::string::String>,
    ) -> Result<(), AppError> {
        // TODO: Implement as handlers are migrated from storage syscalls to VFS IPC
        syscall::debug(&format!(
            "IdentityService: dispatch_vfs_unlink_result for {:?}, success={}",
            core::mem::discriminant(&op),
            result.is_ok()
        ));
        Ok(())
    }

    // =========================================================================
    // Network result handler
    // =========================================================================

    pub fn handle_net_result(&mut self, msg: &Message) -> Result<(), AppError> {
        if msg.data.len() < 9 {
            return Ok(());
        }

        let request_id = u32::from_le_bytes([msg.data[0], msg.data[1], msg.data[2], msg.data[3]]);
        let result_type = msg.data[4];
        let data_len =
            u32::from_le_bytes([msg.data[5], msg.data[6], msg.data[7], msg.data[8]]) as usize;
        let data = if data_len > 0 && msg.data.len() >= 9 + data_len {
            &msg.data[9..9 + data_len]
        } else {
            &[]
        };

        let pending_op = match self.pending_net_ops.remove(&request_id) {
            Some(op) => op,
            None => return Ok(()),
        };

        let http_response: HttpResponse = if result_type == 0 && !data.is_empty() {
            serde_json::from_slice(data)
                .unwrap_or_else(|_| HttpResponse::err(NetworkError::Other("Parse error".into())))
        } else {
            HttpResponse::err(NetworkError::Other("Network error".into()))
        };

        self.dispatch_network_result(pending_op, http_response)
    }

    fn dispatch_network_result(
        &mut self,
        op: PendingNetworkOp,
        http_response: HttpResponse,
    ) -> Result<(), AppError> {
        use crate::handlers::{credentials, session};

        match op {
            PendingNetworkOp::RequestZidChallenge {
                client_pid,
                user_id,
                zid_endpoint,
                machine_key,
                cap_slots,
            } => {
                match network_handlers::handle_zid_challenge_result(
                    client_pid,
                    user_id,
                    zid_endpoint,
                    *machine_key,
                    cap_slots,
                    http_response,
                ) {
                    NetworkHandlerResult::Done(r) => r,
                    NetworkHandlerResult::ContinueZidLoginWithChallenge {
                        client_pid,
                        user_id,
                        zid_endpoint,
                        machine_key,
                        challenge_response,
                        cap_slots,
                    } => session::continue_zid_login_after_challenge(
                        self,
                        client_pid,
                        user_id,
                        zid_endpoint,
                        *machine_key,
                        challenge_response,
                        cap_slots,
                    ),
                    _ => Ok(()),
                }
            }
            PendingNetworkOp::SubmitZidLogin {
                client_pid,
                user_id,
                zid_endpoint,
                cap_slots,
            } => {
                match network_handlers::handle_zid_login_result(
                    client_pid,
                    user_id,
                    zid_endpoint,
                    cap_slots,
                    http_response,
                ) {
                    NetworkHandlerResult::Done(r) => r,
                    NetworkHandlerResult::ContinueZidLoginWithTokens {
                        client_pid,
                        user_id,
                        zid_endpoint,
                        login_response,
                        cap_slots,
                    } => session::continue_zid_login_after_login(
                        self,
                        client_pid,
                        user_id,
                        zid_endpoint,
                        login_response,
                        cap_slots,
                    ),
                    _ => Ok(()),
                }
            }
            PendingNetworkOp::SubmitEmailToZid {
                client_pid,
                user_id,
                email,
                cap_slots,
            } => {
                match network_handlers::handle_email_to_zid_result(
                    client_pid,
                    user_id,
                    email,
                    cap_slots,
                    http_response,
                ) {
                    NetworkHandlerResult::Done(r) => r,
                    NetworkHandlerResult::ContinueAttachEmail {
                        client_pid,
                        user_id,
                        email,
                        cap_slots,
                    } => credentials::continue_attach_email_after_zid(
                        self, client_pid, user_id, email, cap_slots,
                    ),
                    _ => Ok(()),
                }
            }
            PendingNetworkOp::SubmitZidEnroll {
                client_pid,
                user_id,
                zid_endpoint,
                cap_slots,
                identity_id,
                machine_id,
                identity_signing_public_key,
                machine_signing_public_key,
                machine_encryption_public_key,
                machine_signing_sk,
                machine_encryption_sk,
            } => {
                match network_handlers::handle_zid_enroll_result(
                    client_pid,
                    user_id,
                    zid_endpoint.clone(),
                    cap_slots.clone(),
                    http_response,
                ) {
                    NetworkHandlerResult::Done(r) => r,
                    NetworkHandlerResult::ContinueZidEnroll {
                        client_pid: _,
                        user_id: _,
                        zid_endpoint: _,
                        enroll_response,
                        cap_slots: _,
                    } => session::continue_zid_enroll_after_submit(
                        self,
                        client_pid,
                        user_id,
                        zid_endpoint,
                        enroll_response,
                        cap_slots,
                        identity_id,
                        machine_id,
                        identity_signing_public_key,
                        machine_signing_public_key,
                        machine_encryption_public_key,
                        machine_signing_sk,
                        machine_encryption_sk,
                    ),
                    _ => Ok(()),
                }
            }
            // Chained login flow after identity creation
            PendingNetworkOp::RequestZidChallengeAfterEnroll {
                client_pid,
                user_id,
                zid_endpoint,
                cap_slots,
                machine_id,
                identity_signing_public_key,
                machine_signing_public_key,
                machine_encryption_public_key,
                machine_signing_sk,
                machine_encryption_sk,
            } => {
                match network_handlers::handle_zid_challenge_after_enroll_result(
                    client_pid,
                    user_id,
                    zid_endpoint.clone(),
                    machine_id,
                    identity_signing_public_key,
                    machine_signing_public_key,
                    machine_encryption_public_key,
                    machine_signing_sk,
                    machine_encryption_sk,
                    cap_slots.clone(),
                    http_response,
                ) {
                    NetworkHandlerResult::Done(r) => r,
                    NetworkHandlerResult::ContinueZidEnrollWithChallenge {
                        client_pid,
                        user_id,
                        zid_endpoint,
                        machine_id,
                        identity_signing_public_key,
                        machine_signing_public_key,
                        machine_encryption_public_key,
                        machine_signing_sk,
                        machine_encryption_sk,
                        challenge_response,
                        cap_slots,
                    } => session::continue_zid_enroll_after_challenge(
                        self,
                        client_pid,
                        user_id,
                        zid_endpoint,
                        machine_id,
                        identity_signing_public_key,
                        machine_signing_public_key,
                        machine_encryption_public_key,
                        machine_signing_sk,
                        machine_encryption_sk,
                        challenge_response,
                        cap_slots,
                    ),
                    _ => Ok(()),
                }
            }
            PendingNetworkOp::SubmitZidLoginAfterEnroll {
                client_pid,
                user_id,
                zid_endpoint,
                cap_slots,
                machine_id,
                identity_signing_public_key,
                machine_signing_public_key,
                machine_encryption_public_key,
                machine_signing_sk,
                machine_encryption_sk,
            } => {
                match network_handlers::handle_zid_login_after_enroll_result(
                    client_pid,
                    user_id,
                    zid_endpoint.clone(),
                    machine_id,
                    identity_signing_public_key,
                    machine_signing_public_key,
                    machine_encryption_public_key,
                    machine_signing_sk,
                    machine_encryption_sk,
                    cap_slots.clone(),
                    http_response,
                ) {
                    NetworkHandlerResult::Done(r) => r,
                    NetworkHandlerResult::ContinueZidEnrollWithTokens {
                        client_pid,
                        user_id,
                        zid_endpoint,
                        machine_id,
                        identity_signing_public_key,
                        machine_signing_public_key,
                        machine_encryption_public_key,
                        machine_signing_sk,
                        machine_encryption_sk,
                        login_response,
                        cap_slots,
                    } => session::continue_zid_enroll_after_login(
                        self,
                        client_pid,
                        user_id,
                        zid_endpoint,
                        machine_id,
                        identity_signing_public_key,
                        machine_signing_public_key,
                        machine_encryption_public_key,
                        machine_signing_sk,
                        machine_encryption_sk,
                        login_response,
                        cap_slots,
                    ),
                    _ => Ok(()),
                }
            }
        }
    }
}
