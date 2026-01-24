//! Storage result handlers for identity service
//!
//! This module contains handlers for async storage operation results.
//! Each handler is a focused function that processes a specific pending operation type.

extern crate alloc;

use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use zos_identity::error::{CredentialError, ZidError};
use zos_identity::ipc::NeuralKeyGenerated;
use zos_identity::keystore::{CredentialStore, LocalKeyStore, MachineKeyRecord};
use zos_identity::KeyError;
use zos_process::storage_result;
use zos_vfs::{parent_path, Inode};

use crate::error::AppError;
use crate::identity::pending::PendingStorageOp;
use crate::identity::response;
use crate::syscall;

/// Result of handling a storage operation.
pub enum StorageHandlerResult {
    /// Operation complete, no further action needed
    Done(Result<(), AppError>),
    /// Need to start another storage write operation
    ContinueWrite {
        key: String,
        value: Vec<u8>,
        next_op: PendingStorageOp,
    },
    /// Need to start another storage read operation
    ContinueRead { key: String, next_op: PendingStorageOp },
    /// Need to start another storage delete operation
    ContinueDelete { key: String, next_op: PendingStorageOp },
}

// =============================================================================
// Neural Key handlers
// =============================================================================

/// Handle CheckKeyExists result for neural key generation.
pub fn handle_check_key_exists(
    client_pid: u32,
    user_id: u128,
    cap_slots: Vec<u32>,
    result_type: u8,
    data: &[u8],
) -> (bool, u32, u128, Vec<u32>) {
    let exists = if result_type == storage_result::EXISTS_OK {
        !data.is_empty() && data[0] == 1
    } else {
        false
    };
    (exists, client_pid, user_id, cap_slots)
}

/// Handle WriteKeyStoreContent result.
pub fn handle_write_key_store_content(
    client_pid: u32,
    user_id: u128,
    result: NeuralKeyGenerated,
    json_bytes: Vec<u8>,
    cap_slots: Vec<u32>,
    result_type: u8,
) -> StorageHandlerResult {
    if result_type != storage_result::WRITE_OK {
        return StorageHandlerResult::Done(response::send_neural_key_error(
            client_pid,
            &cap_slots,
            KeyError::StorageError("Content write failed".into()),
        ));
    }

    // Step 2: Write the inode
    let key_path = LocalKeyStore::storage_path(user_id);
    let now = syscall::get_wallclock();
    let inode = Inode::new_file(
        key_path.clone(),
        parent_path(&key_path).to_string(),
        key_path.rsplit('/').next().unwrap_or("keys.json").to_string(),
        Some(user_id),
        json_bytes.len() as u64,
        None,
        now,
    );

    match serde_json::to_vec(&inode) {
        Ok(inode_json) => StorageHandlerResult::ContinueWrite {
            key: format!("inode:{}", key_path),
            value: inode_json,
            next_op: PendingStorageOp::WriteKeyStoreInode {
                client_pid,
                result,
                cap_slots,
            },
        },
        Err(e) => {
            syscall::debug(&format!(
                "IdentityService: Failed to serialize inode: {}",
                e
            ));
            StorageHandlerResult::Done(response::send_neural_key_error(
                client_pid,
                &cap_slots,
                KeyError::StorageError(format!("Inode serialization failed: {}", e)),
            ))
        }
    }
}

/// Handle WriteKeyStoreInode result (final step).
pub fn handle_write_key_store_inode(
    client_pid: u32,
    result: NeuralKeyGenerated,
    cap_slots: Vec<u32>,
    result_type: u8,
) -> StorageHandlerResult {
    if result_type == storage_result::WRITE_OK {
        syscall::debug("IdentityService: Neural key stored (content + inode)");
        StorageHandlerResult::Done(response::send_neural_key_success(
            client_pid,
            &cap_slots,
            result,
        ))
    } else {
        StorageHandlerResult::Done(response::send_neural_key_error(
            client_pid,
            &cap_slots,
            KeyError::StorageError("Inode write failed".into()),
        ))
    }
}

/// Handle GetIdentityKey result.
pub fn handle_get_identity_key(
    client_pid: u32,
    cap_slots: Vec<u32>,
    result_type: u8,
    data: &[u8],
) -> StorageHandlerResult {
    if result_type == storage_result::READ_OK {
        match serde_json::from_slice::<LocalKeyStore>(data) {
            Ok(key_store) => StorageHandlerResult::Done(response::send_get_identity_key_success(
                client_pid,
                &cap_slots,
                Some(key_store),
            )),
            Err(e) => {
                syscall::debug(&format!(
                    "IdentityService: Failed to parse stored keys: {}",
                    e
                ));
                StorageHandlerResult::Done(response::send_get_identity_key_error(
                    client_pid,
                    &cap_slots,
                    KeyError::StorageError(format!("Parse failed: {}", e)),
                ))
            }
        }
    } else {
        // Key not found
        StorageHandlerResult::Done(response::send_get_identity_key_success(
            client_pid,
            &cap_slots,
            None,
        ))
    }
}

// =============================================================================
// Recovery handlers
// =============================================================================

/// Handle WriteRecoveredKeyStoreContent result.
pub fn handle_write_recovered_content(
    client_pid: u32,
    user_id: u128,
    result: NeuralKeyGenerated,
    json_bytes: Vec<u8>,
    cap_slots: Vec<u32>,
    result_type: u8,
) -> StorageHandlerResult {
    if result_type != storage_result::WRITE_OK {
        return StorageHandlerResult::Done(response::send_recover_key_error(
            client_pid,
            &cap_slots,
            KeyError::StorageError("Content write failed".into()),
        ));
    }

    let key_path = LocalKeyStore::storage_path(user_id);
    let now = syscall::get_wallclock();
    let inode = Inode::new_file(
        key_path.clone(),
        parent_path(&key_path).to_string(),
        key_path.rsplit('/').next().unwrap_or("keys.json").to_string(),
        Some(user_id),
        json_bytes.len() as u64,
        None,
        now,
    );

    match serde_json::to_vec(&inode) {
        Ok(inode_json) => StorageHandlerResult::ContinueWrite {
            key: format!("inode:{}", key_path),
            value: inode_json,
            next_op: PendingStorageOp::WriteRecoveredKeyStoreInode {
                client_pid,
                result,
                cap_slots,
            },
        },
        Err(e) => StorageHandlerResult::Done(response::send_recover_key_error(
            client_pid,
            &cap_slots,
            KeyError::StorageError(format!("Inode serialization failed: {}", e)),
        )),
    }
}

/// Handle WriteRecoveredKeyStoreInode result (final step).
pub fn handle_write_recovered_inode(
    client_pid: u32,
    result: NeuralKeyGenerated,
    cap_slots: Vec<u32>,
    result_type: u8,
) -> StorageHandlerResult {
    if result_type == storage_result::WRITE_OK {
        syscall::debug("IdentityService: Recovered key stored (content + inode)");
        StorageHandlerResult::Done(response::send_recover_key_success(
            client_pid,
            &cap_slots,
            result,
        ))
    } else {
        StorageHandlerResult::Done(response::send_recover_key_error(
            client_pid,
            &cap_slots,
            KeyError::StorageError("Inode write failed".into()),
        ))
    }
}

// =============================================================================
// Machine Key handlers
// =============================================================================

/// Handle WriteMachineKeyContent result.
pub fn handle_write_machine_key_content(
    client_pid: u32,
    user_id: u128,
    record: MachineKeyRecord,
    json_bytes: Vec<u8>,
    cap_slots: Vec<u32>,
    result_type: u8,
) -> StorageHandlerResult {
    if result_type != storage_result::WRITE_OK {
        return StorageHandlerResult::Done(response::send_create_machine_key_error(
            client_pid,
            &cap_slots,
            KeyError::StorageError("Content write failed".into()),
        ));
    }

    let machine_path = MachineKeyRecord::storage_path(user_id, record.machine_id);
    let now = syscall::get_wallclock();
    let inode = Inode::new_file(
        machine_path.clone(),
        parent_path(&machine_path).to_string(),
        machine_path
            .rsplit('/')
            .next()
            .unwrap_or("machine.json")
            .to_string(),
        Some(user_id),
        json_bytes.len() as u64,
        None,
        now,
    );

    match serde_json::to_vec(&inode) {
        Ok(inode_json) => StorageHandlerResult::ContinueWrite {
            key: format!("inode:{}", machine_path),
            value: inode_json,
            next_op: PendingStorageOp::WriteMachineKeyInode {
                client_pid,
                record,
                cap_slots,
            },
        },
        Err(e) => StorageHandlerResult::Done(response::send_create_machine_key_error(
            client_pid,
            &cap_slots,
            KeyError::StorageError(format!("Inode serialization failed: {}", e)),
        )),
    }
}

/// Handle WriteMachineKeyInode result (final step).
pub fn handle_write_machine_key_inode(
    client_pid: u32,
    record: MachineKeyRecord,
    cap_slots: Vec<u32>,
    result_type: u8,
) -> StorageHandlerResult {
    if result_type == storage_result::WRITE_OK {
        syscall::debug(&format!(
            "IdentityService: Stored machine key {:032x} (content + inode)",
            record.machine_id
        ));
        StorageHandlerResult::Done(response::send_create_machine_key_success(
            client_pid,
            &cap_slots,
            record,
        ))
    } else {
        StorageHandlerResult::Done(response::send_create_machine_key_error(
            client_pid,
            &cap_slots,
            KeyError::StorageError("Inode write failed".into()),
        ))
    }
}

/// Handle ListMachineKeys result.
pub fn handle_list_machine_keys(
    client_pid: u32,
    user_id: u128,
    cap_slots: Vec<u32>,
    result_type: u8,
    data: &[u8],
) -> StorageHandlerResult {
    if result_type != storage_result::LIST_OK {
        return StorageHandlerResult::Done(response::send_list_machine_keys(
            client_pid,
            &cap_slots,
            Vec::new(),
        ));
    }

    match serde_json::from_slice::<Vec<String>>(data) {
        Ok(paths) => {
            let json_paths: Vec<String> = paths
                .into_iter()
                .filter(|p| p.ends_with(".json"))
                .map(|p| format!("content:{}", p))
                .collect();

            if json_paths.is_empty() {
                return StorageHandlerResult::Done(response::send_list_machine_keys(
                    client_pid,
                    &cap_slots,
                    Vec::new(),
                ));
            }

            let mut remaining = json_paths;
            let first = remaining.remove(0);
            StorageHandlerResult::ContinueRead {
                key: first,
                next_op: PendingStorageOp::ReadMachineKey {
                    client_pid,
                    user_id,
                    remaining_paths: remaining,
                    records: Vec::new(),
                    cap_slots,
                },
            }
        }
        Err(_) => StorageHandlerResult::Done(response::send_list_machine_keys(
            client_pid,
            &cap_slots,
            Vec::new(),
        )),
    }
}

/// Handle ReadMachineKey result (iterative read).
pub fn handle_read_machine_key(
    client_pid: u32,
    user_id: u128,
    mut remaining_paths: Vec<String>,
    mut records: Vec<MachineKeyRecord>,
    cap_slots: Vec<u32>,
    result_type: u8,
    data: &[u8],
) -> StorageHandlerResult {
    if result_type == storage_result::READ_OK {
        if let Ok(record) = serde_json::from_slice::<MachineKeyRecord>(data) {
            records.push(record);
        }
    }

    if remaining_paths.is_empty() {
        syscall::debug(&format!(
            "IdentityService: Found {} machine keys",
            records.len()
        ));
        return StorageHandlerResult::Done(response::send_list_machine_keys(
            client_pid,
            &cap_slots,
            records,
        ));
    }

    let next = remaining_paths.remove(0);
    StorageHandlerResult::ContinueRead {
        key: next,
        next_op: PendingStorageOp::ReadMachineKey {
            client_pid,
            user_id,
            remaining_paths,
            records,
            cap_slots,
        },
    }
}

/// Handle DeleteMachineKey content result.
pub fn handle_delete_machine_key(
    client_pid: u32,
    user_id: u128,
    machine_id: u128,
    cap_slots: Vec<u32>,
    result_type: u8,
) -> StorageHandlerResult {
    if result_type == storage_result::WRITE_OK {
        syscall::debug("IdentityService: Machine key content deleted, now deleting inode");
        let machine_path = MachineKeyRecord::storage_path(user_id, machine_id);
        StorageHandlerResult::ContinueDelete {
            key: format!("inode:{}", machine_path),
            next_op: PendingStorageOp::DeleteMachineKeyInode { client_pid, cap_slots },
        }
    } else {
        StorageHandlerResult::Done(response::send_revoke_machine_key_error(
            client_pid,
            &cap_slots,
            KeyError::StorageError("Delete failed".into()),
        ))
    }
}

/// Handle DeleteMachineKeyInode result (final step).
pub fn handle_delete_machine_key_inode(
    client_pid: u32,
    cap_slots: Vec<u32>,
    result_type: u8,
) -> StorageHandlerResult {
    if result_type == storage_result::WRITE_OK {
        syscall::debug("IdentityService: Machine key deleted (content + inode)");
        StorageHandlerResult::Done(response::send_revoke_machine_key_success(
            client_pid, &cap_slots,
        ))
    } else {
        StorageHandlerResult::Done(response::send_revoke_machine_key_error(
            client_pid,
            &cap_slots,
            KeyError::StorageError("Inode delete failed".into()),
        ))
    }
}

/// Handle ReadSingleMachineKey result.
pub fn handle_read_single_machine_key(
    client_pid: u32,
    cap_slots: Vec<u32>,
    result_type: u8,
    data: &[u8],
) -> StorageHandlerResult {
    if result_type == storage_result::READ_OK {
        match serde_json::from_slice::<MachineKeyRecord>(data) {
            Ok(record) => StorageHandlerResult::Done(response::send_get_machine_key_success(
                client_pid,
                &cap_slots,
                Some(record),
            )),
            Err(e) => {
                syscall::debug(&format!(
                    "IdentityService: Failed to parse machine key: {}",
                    e
                ));
                StorageHandlerResult::Done(response::send_get_machine_key_error(
                    client_pid,
                    &cap_slots,
                    KeyError::StorageError(format!("Parse failed: {}", e)),
                ))
            }
        }
    } else {
        StorageHandlerResult::Done(response::send_get_machine_key_success(
            client_pid,
            &cap_slots,
            None,
        ))
    }
}

/// Handle WriteRotatedMachineKeyContent result.
pub fn handle_write_rotated_content(
    client_pid: u32,
    user_id: u128,
    record: MachineKeyRecord,
    json_bytes: Vec<u8>,
    cap_slots: Vec<u32>,
    result_type: u8,
) -> StorageHandlerResult {
    if result_type != storage_result::WRITE_OK {
        return StorageHandlerResult::Done(response::send_rotate_machine_key_error(
            client_pid,
            &cap_slots,
            KeyError::StorageError("Content write failed".into()),
        ));
    }

    let machine_path = MachineKeyRecord::storage_path(user_id, record.machine_id);
    let now = syscall::get_wallclock();
    let inode = Inode::new_file(
        machine_path.clone(),
        parent_path(&machine_path).to_string(),
        machine_path
            .rsplit('/')
            .next()
            .unwrap_or("machine.json")
            .to_string(),
        Some(user_id),
        json_bytes.len() as u64,
        None,
        now,
    );

    match serde_json::to_vec(&inode) {
        Ok(inode_json) => StorageHandlerResult::ContinueWrite {
            key: format!("inode:{}", machine_path),
            value: inode_json,
            next_op: PendingStorageOp::WriteRotatedMachineKeyInode {
                client_pid,
                record,
                cap_slots,
            },
        },
        Err(e) => StorageHandlerResult::Done(response::send_rotate_machine_key_error(
            client_pid,
            &cap_slots,
            KeyError::StorageError(format!("Inode serialization failed: {}", e)),
        )),
    }
}

/// Handle WriteRotatedMachineKeyInode result (final step).
pub fn handle_write_rotated_inode(
    client_pid: u32,
    record: MachineKeyRecord,
    cap_slots: Vec<u32>,
    result_type: u8,
) -> StorageHandlerResult {
    if result_type == storage_result::WRITE_OK {
        syscall::debug(&format!(
            "IdentityService: Rotated keys for machine {:032x} (epoch {}, content + inode)",
            record.machine_id, record.epoch
        ));
        StorageHandlerResult::Done(response::send_rotate_machine_key_success(
            client_pid,
            &cap_slots,
            record,
        ))
    } else {
        StorageHandlerResult::Done(response::send_rotate_machine_key_error(
            client_pid,
            &cap_slots,
            KeyError::StorageError("Inode write failed".into()),
        ))
    }
}

// =============================================================================
// Credential handlers
// =============================================================================

/// Handle GetCredentials result.
pub fn handle_get_credentials(
    client_pid: u32,
    cap_slots: Vec<u32>,
    result_type: u8,
    data: &[u8],
) -> StorageHandlerResult {
    if result_type == storage_result::READ_OK && !data.is_empty() {
        match serde_json::from_slice::<CredentialStore>(data) {
            Ok(store) => StorageHandlerResult::Done(response::send_get_credentials(
                client_pid,
                &cap_slots,
                store.credentials,
            )),
            Err(e) => {
                syscall::debug(&format!(
                    "IdentityService: Failed to parse credentials: {}",
                    e
                ));
                StorageHandlerResult::Done(response::send_get_credentials(
                    client_pid,
                    &cap_slots,
                    Vec::new(),
                ))
            }
        }
    } else {
        StorageHandlerResult::Done(response::send_get_credentials(
            client_pid,
            &cap_slots,
            Vec::new(),
        ))
    }
}

/// Handle WriteUnlinkedCredentialContent result.
pub fn handle_write_unlinked_content(
    client_pid: u32,
    user_id: u128,
    json_bytes: Vec<u8>,
    cap_slots: Vec<u32>,
    result_type: u8,
) -> StorageHandlerResult {
    if result_type != storage_result::WRITE_OK {
        return StorageHandlerResult::Done(response::send_unlink_credential_error(
            client_pid,
            &cap_slots,
            CredentialError::StorageError("Content write failed".into()),
        ));
    }

    let cred_path = CredentialStore::storage_path(user_id);
    let now = syscall::get_wallclock();
    let inode = Inode::new_file(
        cred_path.clone(),
        parent_path(&cred_path).to_string(),
        cred_path
            .rsplit('/')
            .next()
            .unwrap_or("credentials.json")
            .to_string(),
        Some(user_id),
        json_bytes.len() as u64,
        None,
        now,
    );

    match serde_json::to_vec(&inode) {
        Ok(inode_json) => StorageHandlerResult::ContinueWrite {
            key: format!("inode:{}", cred_path),
            value: inode_json,
            next_op: PendingStorageOp::WriteUnlinkedCredentialInode { client_pid, cap_slots },
        },
        Err(e) => StorageHandlerResult::Done(response::send_unlink_credential_error(
            client_pid,
            &cap_slots,
            CredentialError::StorageError(format!("Inode serialization failed: {}", e)),
        )),
    }
}

/// Handle WriteUnlinkedCredentialInode result (final step).
pub fn handle_write_unlinked_inode(
    client_pid: u32,
    cap_slots: Vec<u32>,
    result_type: u8,
) -> StorageHandlerResult {
    if result_type == storage_result::WRITE_OK {
        syscall::debug("IdentityService: Credential unlinked (content + inode)");
        StorageHandlerResult::Done(response::send_unlink_credential_success(
            client_pid, &cap_slots,
        ))
    } else {
        StorageHandlerResult::Done(response::send_unlink_credential_error(
            client_pid,
            &cap_slots,
            CredentialError::StorageError("Inode write failed".into()),
        ))
    }
}

/// Handle WriteEmailCredentialContent result.
pub fn handle_write_email_cred_content(
    client_pid: u32,
    user_id: u128,
    json_bytes: Vec<u8>,
    cap_slots: Vec<u32>,
    result_type: u8,
) -> StorageHandlerResult {
    if result_type != storage_result::WRITE_OK {
        return StorageHandlerResult::Done(response::send_attach_email_error(
            client_pid,
            &cap_slots,
            CredentialError::StorageError("Content write failed".into()),
        ));
    }

    let cred_path = CredentialStore::storage_path(user_id);
    let now = syscall::get_wallclock();
    let inode = Inode::new_file(
        cred_path.clone(),
        parent_path(&cred_path).to_string(),
        cred_path
            .rsplit('/')
            .next()
            .unwrap_or("credentials.json")
            .to_string(),
        Some(user_id),
        json_bytes.len() as u64,
        None,
        now,
    );

    match serde_json::to_vec(&inode) {
        Ok(inode_json) => StorageHandlerResult::ContinueWrite {
            key: format!("inode:{}", cred_path),
            value: inode_json,
            next_op: PendingStorageOp::WriteEmailCredentialInode { client_pid, cap_slots },
        },
        Err(e) => StorageHandlerResult::Done(response::send_attach_email_error(
            client_pid,
            &cap_slots,
            CredentialError::StorageError(format!("Inode serialization failed: {}", e)),
        )),
    }
}

/// Handle WriteEmailCredentialInode result (final step).
pub fn handle_write_email_cred_inode(
    client_pid: u32,
    cap_slots: Vec<u32>,
    result_type: u8,
) -> StorageHandlerResult {
    if result_type == storage_result::WRITE_OK {
        syscall::debug("IdentityService: Email credential stored via ZID (content + inode)");
        StorageHandlerResult::Done(response::send_attach_email_success(client_pid, &cap_slots))
    } else {
        StorageHandlerResult::Done(response::send_attach_email_error(
            client_pid,
            &cap_slots,
            CredentialError::StorageError("Inode write failed".into()),
        ))
    }
}

// =============================================================================
// ZID Session handlers
// =============================================================================

/// Handle WriteZidSessionInode result (final step).
pub fn handle_write_zid_session_inode(
    client_pid: u32,
    tokens: zos_identity::ipc::ZidTokens,
    cap_slots: Vec<u32>,
    result_type: u8,
) -> StorageHandlerResult {
    if result_type == storage_result::WRITE_OK {
        syscall::debug("IdentityService: ZID session stored successfully");
        StorageHandlerResult::Done(response::send_zid_login_success(
            client_pid, &cap_slots, tokens,
        ))
    } else {
        StorageHandlerResult::Done(response::send_zid_login_error(
            client_pid,
            &cap_slots,
            ZidError::NetworkError("Session inode write failed".into()),
        ))
    }
}

/// Handle WriteZidEnrollSessionInode result (final step).
pub fn handle_write_zid_enroll_session_inode(
    client_pid: u32,
    tokens: zos_identity::ipc::ZidTokens,
    cap_slots: Vec<u32>,
    result_type: u8,
) -> StorageHandlerResult {
    if result_type == storage_result::WRITE_OK {
        syscall::debug("IdentityService: ZID enrollment session stored successfully");
        StorageHandlerResult::Done(response::send_zid_enroll_success(
            client_pid, &cap_slots, tokens,
        ))
    } else {
        StorageHandlerResult::Done(response::send_zid_enroll_error(
            client_pid,
            &cap_slots,
            ZidError::EnrollmentFailed("Session inode write failed".into()),
        ))
    }
}

/// Handle ReadMachineKeyForZidLogin - can be LIST or READ result.
pub fn handle_read_machine_for_zid_login(
    client_pid: u32,
    user_id: u128,
    zid_endpoint: String,
    cap_slots: Vec<u32>,
    result_type: u8,
    data: &[u8],
) -> Result<ZidLoginReadResult, StorageHandlerResult> {
    if result_type == storage_result::LIST_OK {
        let paths: Vec<String> = if !data.is_empty() {
            serde_json::from_slice(data).unwrap_or_default()
        } else {
            Vec::new()
        };
        Ok(ZidLoginReadResult::PathList {
            paths,
            client_pid,
            user_id,
            zid_endpoint,
            cap_slots,
        })
    } else if result_type == storage_result::READ_OK && !data.is_empty() {
        Ok(ZidLoginReadResult::MachineKeyData {
            data: data.to_vec(),
            client_pid,
            user_id,
            zid_endpoint,
            cap_slots,
        })
    } else if result_type == storage_result::NOT_FOUND {
        Err(StorageHandlerResult::Done(response::send_zid_login_error(
            client_pid,
            &cap_slots,
            ZidError::MachineKeyNotFound,
        )))
    } else {
        Err(StorageHandlerResult::Done(response::send_zid_login_error(
            client_pid,
            &cap_slots,
            ZidError::NetworkError("Storage read failed".into()),
        )))
    }
}

/// Result of reading machine key for ZID login.
pub enum ZidLoginReadResult {
    PathList {
        paths: Vec<String>,
        client_pid: u32,
        user_id: u128,
        zid_endpoint: String,
        cap_slots: Vec<u32>,
    },
    MachineKeyData {
        data: Vec<u8>,
        client_pid: u32,
        user_id: u128,
        zid_endpoint: String,
        cap_slots: Vec<u32>,
    },
}

/// Handle ReadMachineKeyForZidEnroll - can be LIST or READ result.
pub fn handle_read_machine_for_zid_enroll(
    client_pid: u32,
    user_id: u128,
    zid_endpoint: String,
    cap_slots: Vec<u32>,
    result_type: u8,
    data: &[u8],
) -> Result<ZidEnrollReadResult, StorageHandlerResult> {
    if result_type == storage_result::LIST_OK {
        let paths: Vec<String> = if !data.is_empty() {
            serde_json::from_slice(data).unwrap_or_default()
        } else {
            Vec::new()
        };
        Ok(ZidEnrollReadResult::PathList {
            paths,
            client_pid,
            user_id,
            zid_endpoint,
            cap_slots,
        })
    } else if result_type == storage_result::READ_OK && !data.is_empty() {
        Ok(ZidEnrollReadResult::MachineKeyData {
            data: data.to_vec(),
            client_pid,
            user_id,
            zid_endpoint,
            cap_slots,
        })
    } else if result_type == storage_result::NOT_FOUND {
        Err(StorageHandlerResult::Done(response::send_zid_enroll_error(
            client_pid,
            &cap_slots,
            ZidError::MachineKeyNotFound,
        )))
    } else {
        Err(StorageHandlerResult::Done(response::send_zid_enroll_error(
            client_pid,
            &cap_slots,
            ZidError::NetworkError("Storage read failed".into()),
        )))
    }
}

/// Result of reading machine key for ZID enrollment.
pub enum ZidEnrollReadResult {
    PathList {
        paths: Vec<String>,
        client_pid: u32,
        user_id: u128,
        zid_endpoint: String,
        cap_slots: Vec<u32>,
    },
    MachineKeyData {
        data: Vec<u8>,
        client_pid: u32,
        user_id: u128,
        zid_endpoint: String,
        cap_slots: Vec<u32>,
    },
}
