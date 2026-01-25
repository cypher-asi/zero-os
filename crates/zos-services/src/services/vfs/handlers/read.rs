//! Read operation handlers for VFS Service
//!
//! Handles: stat, exists, read, readdir operations

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
    ClientContext, InodeOpType, PendingOp, VfsService,
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
            Err(_) => {
                // For exists, invalid request means path doesn't exist
                let response = ExistsResponse { exists: false };
                return self.send_response_via_debug(
                    msg.from_pid,
                    vfs_msg::MSG_VFS_EXISTS_RESPONSE,
                    &response,
                );
            }
        };

        // Validate path - invalid paths don't exist
        if validate_path(&request.path).is_err() {
            let response = ExistsResponse { exists: false };
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

        // List children
        self.start_storage_list(
            &inode_key(&request.path),
            PendingOp::ListChildren {
                ctx: client_ctx,
                path: request.path,
                perm_ctx,
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
        let response = ExistsResponse { exists };
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
        _path: &str,
        result_type: u8,
        data: &[u8],
    ) -> Result<(), AppError> {
        let response = if result_type == storage_result::READ_OK {
            ReadFileResponse {
                result: Ok(data.to_vec()),
            }
        } else if result_type == storage_result::NOT_FOUND {
            // File exists but content is empty
            ReadFileResponse {
                result: Ok(Vec::new()),
            }
        } else {
            ReadFileResponse {
                result: Err(VfsError::StorageError(
                    String::from_utf8_lossy(data).to_string(),
                )),
            }
        };
        self.send_response(client_ctx, vfs_msg::MSG_VFS_READ_RESPONSE, &response)
    }

    /// Handle list children result
    pub fn handle_list_children_result(
        &self,
        client_ctx: &ClientContext,
        result_type: u8,
        data: &[u8],
    ) -> Result<(), AppError> {
        let response = if result_type == storage_result::LIST_OK {
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
        } else {
            ReaddirResponse {
                result: Ok(Vec::new()), // Empty for errors/not found
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
        let exists = if result_type == storage_result::EXISTS_OK {
            !data.is_empty() && data[0] == 1
        } else {
            false
        };
        let response = ExistsResponse { exists };
        self.send_response(client_ctx, vfs_msg::MSG_VFS_EXISTS_RESPONSE, &response)
    }
}
