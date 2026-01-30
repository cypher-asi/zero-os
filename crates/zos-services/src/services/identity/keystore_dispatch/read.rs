//! Keystore read response dispatch

extern crate alloc;

use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

use crate::services::identity::handlers::{keys, session};
use crate::services::identity::pending::{PendingKeystoreOp, RequestContext};
use crate::services::identity::{response, IdentityService};
use zos_apps::syscall;
use zos_apps::AppError;
use zos_identity::keystore::{EncryptedShardStore, LocalKeyStore, MachineKeyRecord};
use zos_identity::KeyError;

/// Dispatch keystore read result to appropriate handler based on pending operation type.
pub fn dispatch_keystore_read_result(
    service: &mut IdentityService,
    op: PendingKeystoreOp,
    result: Result<Vec<u8>, String>,
) -> Result<(), AppError> {
    match op {
        PendingKeystoreOp::GetIdentityKey { ctx } => {
            handle_get_identity_key_read(ctx, result)
        }
        PendingKeystoreOp::ReadIdentityForRecovery { ctx, user_id, zid_shards } => {
            handle_read_identity_for_recovery(service, ctx, user_id, zid_shards, result)
        }
        PendingKeystoreOp::ReadIdentityForMachine { ctx, request } => {
            handle_read_identity_for_machine(service, ctx, request, result)
        }
        PendingKeystoreOp::ReadEncryptedShardsForMachine { ctx, request, stored_identity_pubkey, derivation_user_id } => {
            handle_read_encrypted_shards_for_machine(service, ctx, request, stored_identity_pubkey, derivation_user_id, result)
        }
        PendingKeystoreOp::ReadIdentityForMachineEnroll { ctx, request } => {
            handle_read_identity_for_machine_enroll(service, ctx, request, result)
        }
        PendingKeystoreOp::ReadEncryptedShardsForMachineEnroll { ctx, request, stored_identity_pubkey, derivation_user_id } => {
            handle_read_encrypted_shards_for_machine_enroll(service, ctx, request, stored_identity_pubkey, derivation_user_id, result)
        }
        PendingKeystoreOp::ReadMachineKey { ctx, user_id, remaining_paths, records } => {
            handle_read_machine_key(service, ctx, user_id, remaining_paths, records, result)
        }
        PendingKeystoreOp::ReadSingleMachineKey { ctx } => {
            handle_read_single_machine_key(ctx, result)
        }
        PendingKeystoreOp::ReadMachineForRotate { ctx, user_id, machine_id } => {
            handle_read_machine_for_rotate(service, ctx, user_id, machine_id, result)
        }
        PendingKeystoreOp::ReadMachineKeyForZidLogin { ctx, user_id, zid_endpoint } => {
            handle_read_machine_key_for_zid_login(service, ctx, user_id, zid_endpoint, result)
        }
        PendingKeystoreOp::ReadMachineKeyForZidEnroll { ctx, user_id, zid_endpoint } => {
            handle_read_machine_key_for_zid_enroll(service, ctx, user_id, zid_endpoint, result)
        }
        // Operations that should NOT receive a read response
        PendingKeystoreOp::CheckKeyExists { ctx, .. }
        | PendingKeystoreOp::WriteKeyStore { ctx, .. }
        | PendingKeystoreOp::WriteEncryptedShards { ctx, .. }
        | PendingKeystoreOp::WriteRecoveredKeyStore { ctx, .. }
        | PendingKeystoreOp::WriteMachineKey { ctx, .. }
        | PendingKeystoreOp::ListMachineKeys { ctx, .. }
        | PendingKeystoreOp::ListMachineKeysForZidLogin { ctx, .. }
        | PendingKeystoreOp::ListMachineKeysForZidEnroll { ctx, .. }
        | PendingKeystoreOp::DeleteMachineKey { ctx, .. }
        | PendingKeystoreOp::DeleteIdentityKeyAfterShardFailure { ctx, .. }
        | PendingKeystoreOp::WriteRotatedMachineKey { ctx, .. }
        | PendingKeystoreOp::WriteMachineKeyForEnroll { ctx, .. } => {
            syscall::debug(&format!(
                "IdentityService: STATE_MACHINE_ERROR - unexpected keystore read result for non-read op, client_pid={}",
                ctx.client_pid
            ));
            Err(AppError::Internal(
                "State machine error: unexpected keystore read result for non-read operation".into(),
            ))
        }
    }
}

fn handle_get_identity_key_read(
    ctx: RequestContext,
    result: Result<Vec<u8>, String>,
) -> Result<(), AppError> {
    match result {
        Ok(data) => match serde_json::from_slice::<LocalKeyStore>(&data) {
            Ok(key_store) => response::send_get_identity_key_success(
                ctx.client_pid,
                &ctx.cap_slots,
                Some(key_store),
            ),
            Err(e) => {
                syscall::debug(&format!(
                    "IdentityService: Failed to parse stored keys from keystore: {}",
                    e
                ));
                response::send_get_identity_key_error(
                    ctx.client_pid,
                    &ctx.cap_slots,
                    KeyError::StorageError(format!("Parse failed: {}", e)),
                )
            }
        },
        Err(_) => {
            // Key not found
            response::send_get_identity_key_success(ctx.client_pid, &ctx.cap_slots, None)
        }
    }
}

fn handle_read_identity_for_recovery(
    service: &mut IdentityService,
    ctx: RequestContext,
    user_id: u128,
    zid_shards: Vec<zos_identity::crypto::ZidNeuralShard>,
    result: Result<Vec<u8>, String>,
) -> Result<(), AppError> {
    match result {
        Ok(data) if !data.is_empty() => {
            match serde_json::from_slice::<LocalKeyStore>(&data) {
                Ok(key_store) => {
                    // CRITICAL: Two different user_id concepts here:
                    // 1. storage_user_id (request.user_id) - the derived_user_id used for storage paths
                    // 2. derivation_user_id (key_store.user_id) - the ORIGINAL user_id used for
                    //    cryptographic key derivation
                    //
                    // The identity keypair was derived using the ORIGINAL user_id during generation,
                    // so verification must use derivation_user_id. But storage paths must use
                    // storage_user_id (what the client sent) to write back to the same location.
                    let derivation_user_id = key_store.user_id;
                    let storage_user_id = user_id; // The derived_user_id from request
                    syscall::debug(&format!(
                        "IdentityService: Recovery - derivation_user_id {:032x}, storage_user_id {:032x}",
                        derivation_user_id, storage_user_id
                    ));
                    keys::continue_recover_after_identity_read(
                        service,
                        ctx.client_pid,
                        derivation_user_id,
                        storage_user_id,
                        zid_shards,
                        key_store.identity_signing_public_key,
                        ctx.cap_slots,
                    )
                }
                Err(e) => {
                    syscall::debug(&format!(
                        "IdentityService: Failed to parse LocalKeyStore for recovery: {}",
                        e
                    ));
                    response::send_recover_key_error(
                        ctx.client_pid,
                        &ctx.cap_slots,
                        KeyError::StorageError("Corrupted identity key store".into()),
                    )
                }
            }
        }
        _ => {
            syscall::debug("IdentityService: Identity read for recovery failed (keystore)");
            response::send_recover_key_error(
                ctx.client_pid,
                &ctx.cap_slots,
                KeyError::IdentityKeyRequired,
            )
        }
    }
}

fn handle_read_identity_for_machine(
    service: &mut IdentityService,
    ctx: RequestContext,
    request: zos_identity::ipc::CreateMachineKeyRequest,
    result: Result<Vec<u8>, String>,
) -> Result<(), AppError> {
    match result {
        Ok(data) if !data.is_empty() => {
            match serde_json::from_slice::<LocalKeyStore>(&data) {
                Ok(key_store) => {
                    // Chain to read encrypted shards
                    // CRITICAL: key_store.user_id is the derivation_user_id used for identity key derivation.
                    // This may differ from request.user_id if user_id was derived from the pubkey.
                    // Verification must use derivation_user_id to re-derive and compare the pubkey.
                    let derivation_user_id = key_store.user_id;
                    let shards_path = EncryptedShardStore::storage_path(request.user_id);
                    syscall::debug(&format!(
                        "IdentityService: Identity read success, reading encrypted shards (derivation_user_id={:032x})",
                        derivation_user_id
                    ));
                    service.start_keystore_read(
                        &shards_path,
                        PendingKeystoreOp::ReadEncryptedShardsForMachine {
                            ctx,
                            request,
                            stored_identity_pubkey: key_store.identity_signing_public_key,
                            derivation_user_id,
                        },
                    )
                }
                Err(e) => {
                    syscall::debug(&format!(
                        "IdentityService: Failed to parse LocalKeyStore: {}",
                        e
                    ));
                    response::send_create_machine_key_error(
                        ctx.client_pid,
                        &ctx.cap_slots,
                        KeyError::StorageError("Corrupted identity key store".into()),
                    )
                }
            }
        }
        _ => {
            syscall::debug("IdentityService: Identity read failed (keystore)");
            response::send_create_machine_key_error(
                ctx.client_pid,
                &ctx.cap_slots,
                KeyError::IdentityKeyRequired,
            )
        }
    }
}

fn handle_read_encrypted_shards_for_machine(
    service: &mut IdentityService,
    ctx: RequestContext,
    request: zos_identity::ipc::CreateMachineKeyRequest,
    stored_identity_pubkey: [u8; 32],
    derivation_user_id: u128,
    result: Result<Vec<u8>, String>,
) -> Result<(), AppError> {
    match result {
        Ok(data) if !data.is_empty() => {
            match serde_json::from_slice::<EncryptedShardStore>(&data) {
                Ok(encrypted_store) => keys::continue_create_machine_after_shards_read(
                    service,
                    ctx.client_pid,
                    request,
                    stored_identity_pubkey,
                    derivation_user_id,
                    encrypted_store,
                    ctx.cap_slots,
                ),
                Err(e) => {
                    syscall::debug(&format!(
                        "IdentityService: Failed to parse EncryptedShardStore: {}",
                        e
                    ));
                    response::send_create_machine_key_error(
                        ctx.client_pid,
                        &ctx.cap_slots,
                        KeyError::StorageError("Corrupted encrypted shard store".into()),
                    )
                }
            }
        }
        _ => {
            syscall::debug("IdentityService: Encrypted shards not found (keystore)");
            response::send_create_machine_key_error(
                ctx.client_pid,
                &ctx.cap_slots,
                KeyError::EncryptedShardsNotFound,
            )
        }
    }
}

fn handle_read_identity_for_machine_enroll(
    service: &mut IdentityService,
    ctx: RequestContext,
    request: zos_identity::ipc::CreateMachineKeyAndEnrollRequest,
    result: Result<Vec<u8>, String>,
) -> Result<(), AppError> {
    match result {
        Ok(data) if !data.is_empty() => {
            match serde_json::from_slice::<LocalKeyStore>(&data) {
                Ok(key_store) => {
                    // Chain to read encrypted shards
                    let shards_path = EncryptedShardStore::storage_path(request.user_id);
                    syscall::debug(&format!(
                        "IdentityService: Identity read for combined flow, reading encrypted shards from {} (derivation_user_id={:032x})",
                        shards_path, key_store.user_id
                    ));
                    service.start_keystore_read(
                        &shards_path,
                        PendingKeystoreOp::ReadEncryptedShardsForMachineEnroll {
                            ctx,
                            request,
                            stored_identity_pubkey: key_store.identity_signing_public_key,
                            derivation_user_id: key_store.user_id,
                        },
                    )
                }
                Err(e) => {
                    syscall::debug(&format!(
                        "IdentityService: Failed to parse LocalKeyStore for combined flow: {}",
                        e
                    ));
                    response::send_create_machine_key_and_enroll_error(
                        ctx.client_pid,
                        &ctx.cap_slots,
                        zos_identity::error::ZidError::NetworkError("Corrupted identity key store".into()),
                    )
                }
            }
        }
        _ => {
            syscall::debug("IdentityService: Identity read for combined flow failed (keystore)");
            response::send_create_machine_key_and_enroll_error(
                ctx.client_pid,
                &ctx.cap_slots,
                zos_identity::error::ZidError::MachineKeyNotFound,
            )
        }
    }
}

fn handle_read_encrypted_shards_for_machine_enroll(
    service: &mut IdentityService,
    ctx: RequestContext,
    request: zos_identity::ipc::CreateMachineKeyAndEnrollRequest,
    stored_identity_pubkey: [u8; 32],
    derivation_user_id: u128,
    result: Result<Vec<u8>, String>,
) -> Result<(), AppError> {
    match result {
        Ok(data) if !data.is_empty() => {
            match serde_json::from_slice::<EncryptedShardStore>(&data) {
                Ok(encrypted_store) => keys::continue_create_machine_enroll_after_shards_read(
                    service,
                    ctx.client_pid,
                    request,
                    stored_identity_pubkey,
                    derivation_user_id,
                    encrypted_store,
                    ctx.cap_slots,
                ),
                Err(e) => {
                    syscall::debug(&format!(
                        "IdentityService: Failed to parse EncryptedShardStore for combined flow: {}",
                        e
                    ));
                    response::send_create_machine_key_and_enroll_error(
                        ctx.client_pid,
                        &ctx.cap_slots,
                        zos_identity::error::ZidError::NetworkError("Corrupted encrypted shard store".into()),
                    )
                }
            }
        }
        _ => {
            syscall::debug("IdentityService: Encrypted shards not found for combined flow (keystore)");
            response::send_create_machine_key_and_enroll_error(
                ctx.client_pid,
                &ctx.cap_slots,
                zos_identity::error::ZidError::AuthenticationFailed,
            )
        }
    }
}

fn handle_read_machine_key(
    service: &mut IdentityService,
    ctx: RequestContext,
    user_id: u128,
    mut remaining_paths: Vec<String>,
    mut records: Vec<MachineKeyRecord>,
    result: Result<Vec<u8>, String>,
) -> Result<(), AppError> {
    // Process this machine key result
    if let Ok(data) = result {
        if let Ok(record) = serde_json::from_slice::<MachineKeyRecord>(&data) {
            records.push(record);
        }
    }

    // Continue reading remaining paths or send response
    if remaining_paths.is_empty() {
        response::send_list_machine_keys(ctx.client_pid, &ctx.cap_slots, records)
    } else {
        let next_path = remaining_paths.remove(0);
        service.start_keystore_read(
            &next_path,
            PendingKeystoreOp::ReadMachineKey {
                ctx: RequestContext::new(ctx.client_pid, ctx.cap_slots),
                user_id,
                remaining_paths,
                records,
            },
        )
    }
}

fn handle_read_single_machine_key(
    ctx: RequestContext,
    result: Result<Vec<u8>, String>,
) -> Result<(), AppError> {
    match result {
        Ok(data) => match serde_json::from_slice::<MachineKeyRecord>(&data) {
            Ok(record) => response::send_get_machine_key_success(
                ctx.client_pid,
                &ctx.cap_slots,
                Some(record),
            ),
            Err(_) => {
                response::send_get_machine_key_success(ctx.client_pid, &ctx.cap_slots, None)
            }
        },
        Err(_) => response::send_get_machine_key_success(ctx.client_pid, &ctx.cap_slots, None),
    }
}

fn handle_read_machine_for_rotate(
    service: &mut IdentityService,
    ctx: RequestContext,
    user_id: u128,
    machine_id: u128,
    result: Result<Vec<u8>, String>,
) -> Result<(), AppError> {
    match result {
        Ok(data) => keys::continue_rotate_after_read(
            service,
            ctx.client_pid,
            user_id,
            machine_id,
            &data,
            ctx.cap_slots,
        ),
        Err(_) => response::send_rotate_machine_key_error(
            ctx.client_pid,
            &ctx.cap_slots,
            KeyError::MachineKeyNotFound,
        ),
    }
}

fn handle_read_machine_key_for_zid_login(
    service: &mut IdentityService,
    ctx: RequestContext,
    user_id: u128,
    zid_endpoint: String,
    result: Result<Vec<u8>, String>,
) -> Result<(), AppError> {
    match result {
        Ok(data) => session::continue_zid_login_after_read(
            service,
            ctx.client_pid,
            user_id,
            zid_endpoint,
            &data,
            ctx.cap_slots,
        ),
        Err(_) => response::send_zid_login_error(
            ctx.client_pid,
            &ctx.cap_slots,
            zos_identity::error::ZidError::MachineKeyNotFound,
        ),
    }
}

fn handle_read_machine_key_for_zid_enroll(
    service: &mut IdentityService,
    ctx: RequestContext,
    user_id: u128,
    zid_endpoint: String,
    result: Result<Vec<u8>, String>,
) -> Result<(), AppError> {
    match result {
        Ok(data) => session::continue_zid_enroll_after_read(
            service,
            ctx.client_pid,
            user_id,
            zid_endpoint,
            &data,
            ctx.cap_slots,
        ),
        Err(_) => response::send_zid_enroll_error(
            ctx.client_pid,
            &ctx.cap_slots,
            zos_identity::error::ZidError::MachineKeyNotFound,
        ),
    }
}
