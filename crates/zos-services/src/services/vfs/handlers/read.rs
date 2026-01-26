//! Read operation handlers for VFS Service
//!
//! Handles: stat, exists, read, readdir operations
//!
//! # Safety Properties
//!
//! - **Success**: inode read successfully, permissions checked, data returned
//! - **Acceptable partial failure**: None (read is atomic)
//! - **Forbidden**: Returning data without permission check

use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;
use alloc::string::ToString;
use zos_apps::syscall;
use zos_apps::{AppContext, AppError, Message};
use zos_process::storage_result;
use zos_vfs::ipc::{
    vfs_msg, ExistsRequest, ExistsResponse, ReadFileRequest, ReadFileResponse, ReaddirRequest,
    ReaddirResponse, StatRequest, StatResponse,
};
use zos_vfs::service::{check_read, PermissionContext};
use zos_vfs::{DirEntry, Inode};
use zos_vfs::VfsError;

use super::super::{
    content_key, derive_permission_context, inode_key, result_type_name, validate_path,
    ClientContext, InodeOpType, PendingOp, ReaddirStage, VfsService,
};

impl VfsService {
    // =========================================================================
    // Request handlers (start async operations)
    // =========================================================================

    /// Handle MSG_VFS_STAT - get inode info
    pub fn handle_stat(&mut self, _ctx: &AppContext, msg: &Message) -> Result<(), AppError> {
        let request: StatRequest = match serde_json::from_slice(&msg.data) {
            Ok(r) => r,
            Err(e) => {
                let response = StatResponse {
                    result: Err(VfsError::InvalidRequest(format!("Failed to parse request: {}", e))),
                };
                return self.send_response_via_debug(
                    msg.from_pid,
                    vfs_msg::MSG_VFS_STAT_RESPONSE,
                    &response,
                );
            }
        };

        // Validate path
        if let Err(reason) = validate_path(&request.path) {
            let response = StatResponse {
                result: Err(VfsError::InvalidPath(String::from(reason))),
            };
            return self.send_response_via_debug(
                msg.from_pid,
                vfs_msg::MSG_VFS_STAT_RESPONSE,
                &response,
            );
        }

        syscall::debug(&format!("VfsService: stat {}", request.path));

        // Derive permission context from caller
        let perm_ctx = derive_permission_context(msg.from_pid, &request.path);
        let client_ctx = ClientContext::from_message(msg);

        // Start async inode read
        self.start_storage_read(
            &inode_key(&request.path),
            PendingOp::GetInode {
                ctx: client_ctx,
                path: request.path,
                op_type: InodeOpType::Stat,
                perm_ctx,
            },
        )
    }

    /// Handle MSG_VFS_EXISTS - check if path exists
    pub fn handle_exists(&mut self, _ctx: &AppContext, msg: &Message) -> Result<(), AppError> {
        let request: ExistsRequest = match serde_json::from_slice(&msg.data) {
            Ok(r) => r,
            Err(e) => {
                // Rule 1: Parse errors must return InvalidRequest, not false
                let response = ExistsResponse {
                    result: Err(VfsError::InvalidRequest(format!("Failed to parse request: {}", e))),
                };
                return self.send_response_via_debug(
                    msg.from_pid,
                    vfs_msg::MSG_VFS_EXISTS_RESPONSE,
                    &response,
                );
            }
        };

        // Validate path - invalid paths don't exist
        if let Err(reason) = validate_path(&request.path) {
            let response = ExistsResponse {
                result: Err(VfsError::InvalidPath(String::from(reason))),
            };
            return self.send_response_via_debug(
                msg.from_pid,
                vfs_msg::MSG_VFS_EXISTS_RESPONSE,
                &response,
            );
        }

        syscall::debug(&format!("VfsService: exists {}", request.path));

        let client_ctx = ClientContext::from_message(msg);

        // Start async exists check (no permission check needed for exists)
        self.start_storage_exists(
            &inode_key(&request.path),
            PendingOp::ExistsCheck {
                ctx: client_ctx,
                path: request.path,
            },
        )
    }

    /// Handle MSG_VFS_READ - read file content
    pub fn handle_read(&mut self, _ctx: &AppContext, msg: &Message) -> Result<(), AppError> {
        let request: ReadFileRequest = match serde_json::from_slice(&msg.data) {
            Ok(r) => r,
            Err(e) => {
                let response = ReadFileResponse {
                    result: Err(VfsError::InvalidRequest(format!("Failed to parse request: {}", e))),
                };
                return self.send_response_via_debug(
                    msg.from_pid,
                    vfs_msg::MSG_VFS_READ_RESPONSE,
                    &response,
                );
            }
        };

        // Validate path
        if let Err(reason) = validate_path(&request.path) {
            let response = ReadFileResponse {
                result: Err(VfsError::InvalidPath(String::from(reason))),
            };
            return self.send_response_via_debug(
                msg.from_pid,
                vfs_msg::MSG_VFS_READ_RESPONSE,
                &response,
            );
        }

        syscall::debug(&format!("VfsService: read {}", request.path));

        // Derive permission context from caller
        let perm_ctx = derive_permission_context(msg.from_pid, &request.path);
        let client_ctx = ClientContext::from_message(msg);

        // First check inode exists and is a file
        self.start_storage_read(
            &inode_key(&request.path),
            PendingOp::GetInode {
                ctx: client_ctx,
                path: request.path,
                op_type: InodeOpType::ReadFile,
                perm_ctx,
            },
        )
    }

    /// Handle MSG_VFS_READDIR - list directory
    pub fn handle_readdir(&mut self, _ctx: &AppContext, msg: &Message) -> Result<(), AppError> {
        let request: ReaddirRequest = match serde_json::from_slice(&msg.data) {
            Ok(r) => r,
            Err(e) => {
                let response = ReaddirResponse {
                    result: Err(VfsError::InvalidRequest(format!("Failed to parse request: {}", e))),
                };
                return self.send_response_via_debug(
                    msg.from_pid,
                    vfs_msg::MSG_VFS_READDIR_RESPONSE,
                    &response,
                );
            }
        };

        // Validate path
        if let Err(reason) = validate_path(&request.path) {
            let response = ReaddirResponse {
                result: Err(VfsError::InvalidPath(String::from(reason))),
            };
            return self.send_response_via_debug(
                msg.from_pid,
                vfs_msg::MSG_VFS_READDIR_RESPONSE,
                &response,
            );
        }

        syscall::debug(&format!("VfsService: readdir {}", request.path));

        // Derive permission context from caller
        let perm_ctx = derive_permission_context(msg.from_pid, &request.path);
        let client_ctx = ClientContext::from_message(msg);

        // First read directory inode to check permissions
        self.start_storage_read(
            &inode_key(&request.path),
            PendingOp::ReaddirOp {
                ctx: client_ctx,
                path: request.path,
                perm_ctx,
                stage: ReaddirStage::ReadingInode,
            },
        )
    }

    // =========================================================================
    // Result handlers
    // =========================================================================

    /// Handle stat operation inode result
    pub fn handle_stat_inode_result(
        &self,
        client_ctx: &ClientContext,
        perm_ctx: &PermissionContext,
        result_type: u8,
        data: &[u8],
    ) -> Result<(), AppError> {
        let response = match result_type {
            storage_result::READ_OK => {
                match serde_json::from_slice::<Inode>(data) {
                    Ok(inode) => {
                        // Check read permission before returning inode info
                        if !check_read(&inode, perm_ctx) {
                            syscall::debug(&format!(
                                "VfsService: Permission denied for stat (pid={})",
                                client_ctx.pid
                            ));
                            StatResponse {
                                result: Err(VfsError::PermissionDenied),
                            }
                        } else {
                            StatResponse { result: Ok(inode) }
                        }
                    }
                    Err(e) => StatResponse {
                        result: Err(VfsError::StorageError(format!(
                            "Failed to parse inode: {}",
                            e
                        ))),
                    },
                }
            }
            storage_result::NOT_FOUND => StatResponse {
                result: Err(VfsError::NotFound),
            },
            _ => {
                syscall::debug(&format!(
                    "VfsService: stat failed with unexpected result: {} ({})",
                    result_type,
                    result_type_name(result_type)
                ));
                StatResponse {
                    result: Err(VfsError::StorageError(format!(
                        "Inode read failed: {} ({})",
                        result_type,
                        result_type_name(result_type)
                    ))),
                }
            }
        };
        self.send_response(client_ctx, vfs_msg::MSG_VFS_STAT_RESPONSE, &response)
    }

    /// Handle exists check inode result
    pub fn handle_exists_inode_result(
        &self,
        client_ctx: &ClientContext,
        result_type: u8,
    ) -> Result<(), AppError> {
        let exists = result_type == storage_result::READ_OK;
        let response = ExistsResponse { result: Ok(exists) };
        self.send_response(client_ctx, vfs_msg::MSG_VFS_EXISTS_RESPONSE, &response)
    }

    /// Handle read file inode result
    pub fn handle_read_file_inode_result(
        &mut self,
        client_ctx: &ClientContext,
        path: &str,
        perm_ctx: &PermissionContext,
        result_type: u8,
        data: &[u8],
    ) -> Result<(), AppError> {
        if result_type == storage_result::READ_OK {
            match serde_json::from_slice::<Inode>(data) {
                Ok(inode) if inode.is_file() => {
                    // Check read permission before fetching content
                    if !check_read(&inode, perm_ctx) {
                        syscall::debug(&format!(
                            "VfsService: Permission denied for read {} (pid={})",
                            path, client_ctx.pid
                        ));
                        let response = ReadFileResponse {
                            result: Err(VfsError::PermissionDenied),
                        };
                        return self.send_response(
                            client_ctx,
                            vfs_msg::MSG_VFS_READ_RESPONSE,
                            &response,
                        );
                    }

                    self.start_storage_read(
                        &content_key(path),
                        PendingOp::GetContent {
                            ctx: client_ctx.clone(),
                            path: path.to_string(),
                            perm_ctx: perm_ctx.clone(),
                        },
                    )
                }
                Ok(_) => {
                    let response = ReadFileResponse {
                        result: Err(VfsError::NotAFile),
                    };
                    self.send_response(
                        client_ctx,
                        vfs_msg::MSG_VFS_READ_RESPONSE,
                        &response,
                    )
                }
                Err(e) => {
                    let response = ReadFileResponse {
                        result: Err(VfsError::StorageError(e.to_string())),
                    };
                    self.send_response(
                        client_ctx,
                        vfs_msg::MSG_VFS_READ_RESPONSE,
                        &response,
                    )
                }
            }
        } else if result_type == storage_result::NOT_FOUND {
            let response = ReadFileResponse {
                result: Err(VfsError::NotFound),
            };
            self.send_response(client_ctx, vfs_msg::MSG_VFS_READ_RESPONSE, &response)
        } else {
            let response = ReadFileResponse {
                result: Err(VfsError::StorageError(
                    String::from_utf8_lossy(data).to_string(),
                )),
            };
            self.send_response(client_ctx, vfs_msg::MSG_VFS_READ_RESPONSE, &response)
        }
    }

    /// Handle content read result
    pub fn handle_content_result(
        &self,
        client_ctx: &ClientContext,
        path: &str,
        result_type: u8,
        data: &[u8],
    ) -> Result<(), AppError> {
        let response = match result_type {
            storage_result::READ_OK => {
                ReadFileResponse {
                    result: Ok(data.to_vec()),
                }
            }
            storage_result::NOT_FOUND => {
                // Rule 5: If inode exists but content is missing, this is a storage inconsistency
                // not an empty file. Return an error to surface the corruption.
                syscall::debug(&format!(
                    "VfsService: CORRUPTION: Content missing for existing inode {}",
                    path
                ));
                ReadFileResponse {
                    result: Err(VfsError::StorageError(
                        "Content missing for existing inode".into(),
                    )),
                }
            }
            _ => {
                syscall::debug(&format!(
                    "VfsService: read {} content fetch failed: {} ({})",
                    path,
                    result_type,
                    result_type_name(result_type)
                ));
                ReadFileResponse {
                    result: Err(VfsError::StorageError(format!(
                        "Content read failed: {} ({})",
                        result_type,
                        result_type_name(result_type)
                    ))),
                }
            }
        };
        self.send_response(client_ctx, vfs_msg::MSG_VFS_READ_RESPONSE, &response)
    }

    /// Handle list children result
    ///
    /// DEPRECATED: This is the old handler. New code should use ReaddirOp state machine
    /// which properly checks permissions before listing.
    pub fn handle_list_children_result(
        &self,
        client_ctx: &ClientContext,
        result_type: u8,
        data: &[u8],
    ) -> Result<(), AppError> {
        let response = match result_type {
            storage_result::LIST_OK => {
                // data is JSON array of keys
                match serde_json::from_slice::<Vec<String>>(data) {
                    Ok(keys) => {
                        // Convert keys to DirEntry (simplified - would need to fetch each inode)
                        let entries: Vec<DirEntry> = keys
                            .iter()
                            .map(|path| {
                                let name = path.rsplit('/').next().unwrap_or(path).to_string();
                                DirEntry {
                                    name,
                                    path: path.clone(),
                                    is_directory: false, // Would need inode to know
                                    is_symlink: false,
                                    size: 0,
                                    modified_at: 0,
                                }
                            })
                            .collect();
                        ReaddirResponse {
                            result: Ok(entries),
                        }
                    }
                    Err(e) => ReaddirResponse {
                        result: Err(VfsError::StorageError(e.to_string())),
                    },
                }
            }
            storage_result::NOT_FOUND => {
                // Rule 5: NOT_FOUND on list means directory doesn't exist or has no children
                // Since we should have checked existence first via ReaddirOp, this is unexpected
                ReaddirResponse {
                    result: Err(VfsError::NotFound),
                }
            }
            _ => {
                // Rule 5: Return proper error for unexpected result types
                ReaddirResponse {
                    result: Err(VfsError::StorageError(format!(
                        "List failed: {} ({})",
                        result_type,
                        result_type_name(result_type)
                    ))),
                }
            }
        };
        self.send_response(client_ctx, vfs_msg::MSG_VFS_READDIR_RESPONSE, &response)
    }

    /// Handle exists check result
    pub fn handle_exists_result(
        &self,
        client_ctx: &ClientContext,
        result_type: u8,
        data: &[u8],
    ) -> Result<(), AppError> {
        let response = match result_type {
            storage_result::EXISTS_OK => {
                let exists = !data.is_empty() && data[0] == 1;
                ExistsResponse { result: Ok(exists) }
            }
            storage_result::NOT_FOUND => {
                ExistsResponse { result: Ok(false) }
            }
            _ => {
                ExistsResponse {
                    result: Err(VfsError::StorageError(format!(
                        "Exists check failed: unexpected result type {}",
                        result_type
                    ))),
                }
            }
        };
        self.send_response(client_ctx, vfs_msg::MSG_VFS_EXISTS_RESPONSE, &response)
    }

    // =========================================================================
    // Readdir State Machine
    // =========================================================================

    /// Handle readdir operation result (state machine)
    ///
    /// This handler implements the readdir state machine:
    /// 1. ReadingInode: Read directory inode to check it exists and is a directory
    /// 2. ListingChildren: Permission checked, list children
    pub fn handle_readdir_op_result(
        &mut self,
        client_ctx: &ClientContext,
        path: &str,
        perm_ctx: &PermissionContext,
        stage: ReaddirStage,
        result_type: u8,
        data: &[u8],
    ) -> Result<(), AppError> {
        match stage {
            ReaddirStage::ReadingInode => {
                self.handle_readdir_reading_inode(client_ctx, path, perm_ctx, result_type, data)
            }
            ReaddirStage::ListingChildren => {
                self.handle_readdir_listing_children(client_ctx, result_type, data)
            }
        }
    }

    /// Stage 1: Read directory inode and check permissions
    fn handle_readdir_reading_inode(
        &mut self,
        client_ctx: &ClientContext,
        path: &str,
        perm_ctx: &PermissionContext,
        result_type: u8,
        data: &[u8],
    ) -> Result<(), AppError> {
        // Handle result type strictly
        match result_type {
            storage_result::READ_OK => {
                // Good - parse and validate
            }
            storage_result::NOT_FOUND => {
                let response = ReaddirResponse {
                    result: Err(VfsError::NotFound),
                };
                return self.send_response(client_ctx, vfs_msg::MSG_VFS_READDIR_RESPONSE, &response);
            }
            _ => {
                syscall::debug(&format!(
                    "VfsService: readdir {} inode read failed: {} ({})",
                    path,
                    result_type,
                    result_type_name(result_type)
                ));
                let response = ReaddirResponse {
                    result: Err(VfsError::StorageError(format!(
                        "Inode read failed: {} ({})",
                        result_type,
                        result_type_name(result_type)
                    ))),
                };
                return self.send_response(client_ctx, vfs_msg::MSG_VFS_READDIR_RESPONSE, &response);
            }
        }

        // Parse inode - FAIL CLOSED on parse error
        let inode = match serde_json::from_slice::<Inode>(data) {
            Ok(inode) => inode,
            Err(e) => {
                syscall::debug(&format!(
                    "VfsService: SECURITY: Failed to parse inode for readdir {}: {} (denying)",
                    path, e
                ));
                let response = ReaddirResponse {
                    result: Err(VfsError::StorageError(format!(
                        "Inode corrupt or invalid: {}",
                        e
                    ))),
                };
                return self.send_response(client_ctx, vfs_msg::MSG_VFS_READDIR_RESPONSE, &response);
            }
        };

        // SECURITY: Verify path is a directory
        if !inode.is_directory() {
            syscall::debug(&format!(
                "VfsService: readdir {} failed - not a directory (type: {:?})",
                path, inode.inode_type
            ));
            let response = ReaddirResponse {
                result: Err(VfsError::NotADirectory),
            };
            return self.send_response(client_ctx, vfs_msg::MSG_VFS_READDIR_RESPONSE, &response);
        }

        // Check read permission on directory
        if !check_read(&inode, perm_ctx) {
            syscall::debug(&format!(
                "VfsService: Permission denied for readdir {} (pid={})",
                path, client_ctx.pid
            ));
            let response = ReaddirResponse {
                result: Err(VfsError::PermissionDenied),
            };
            return self.send_response(client_ctx, vfs_msg::MSG_VFS_READDIR_RESPONSE, &response);
        }

        // Permission granted - list children
        self.start_storage_list(
            &inode_key(path),
            PendingOp::ReaddirOp {
                ctx: client_ctx.clone(),
                path: path.to_string(),
                perm_ctx: perm_ctx.clone(),
                stage: ReaddirStage::ListingChildren,
            },
        )
    }

    /// Stage 2: Handle list children result
    fn handle_readdir_listing_children(
        &self,
        client_ctx: &ClientContext,
        result_type: u8,
        data: &[u8],
    ) -> Result<(), AppError> {
        let response = match result_type {
            storage_result::LIST_OK => {
                // data is JSON array of keys
                match serde_json::from_slice::<Vec<String>>(data) {
                    Ok(keys) => {
                        // Convert keys to DirEntry
                        let entries: Vec<DirEntry> = keys
                            .iter()
                            .map(|path| {
                                let name = path.rsplit('/').next().unwrap_or(path).to_string();
                                DirEntry {
                                    name,
                                    path: path.clone(),
                                    is_directory: false, // Would need inode to know
                                    is_symlink: false,
                                    size: 0,
                                    modified_at: 0,
                                }
                            })
                            .collect();
                        ReaddirResponse {
                            result: Ok(entries),
                        }
                    }
                    Err(e) => ReaddirResponse {
                        result: Err(VfsError::StorageError(e.to_string())),
                    },
                }
            }
            storage_result::NOT_FOUND => {
                // No children (empty directory)
                ReaddirResponse {
                    result: Ok(Vec::new()),
                }
            }
            _ => {
                // Unexpected result type - return error
                ReaddirResponse {
                    result: Err(VfsError::StorageError(format!(
                        "List failed: {} ({})",
                        result_type,
                        result_type_name(result_type)
                    ))),
                }
            }
        };
        self.send_response(client_ctx, vfs_msg::MSG_VFS_READDIR_RESPONSE, &response)
    }
}
