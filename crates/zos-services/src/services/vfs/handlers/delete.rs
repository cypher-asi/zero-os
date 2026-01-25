//! Delete operation handlers for VFS Service
//!
//! Handles: rmdir, unlink operations

use alloc::format;
use alloc::string::String;
use zos_apps::syscall;
use zos_apps::{AppContext, AppError, Message};
use zos_process::storage_result;
use zos_vfs::ipc::{vfs_msg, RmdirRequest, RmdirResponse, UnlinkRequest, UnlinkResponse};
use zos_vfs::service::{check_write, PermissionContext};
use zos_vfs::Inode;
use zos_vfs::VfsError;

use super::super::{
    content_key, derive_permission_context, inode_key, result_type_name, validate_path,
    ClientContext, InodeOpType, PendingOp, VfsService,
};

impl VfsService {
    // =========================================================================
    // Response helpers (reduce boilerplate)
    // =========================================================================

    /// Send an rmdir error response to the client.
    fn send_rmdir_error(&self, client_ctx: &ClientContext, error: VfsError) -> Result<(), AppError> {
        let response = RmdirResponse {
            result: Err(error),
        };
        self.send_response(client_ctx, vfs_msg::MSG_VFS_RMDIR_RESPONSE, &response)
    }

    /// Send an rmdir error response via debug channel (when no ClientContext available).
    fn send_rmdir_error_via_debug(&self, to_pid: u32, error: VfsError) -> Result<(), AppError> {
        let response = RmdirResponse {
            result: Err(error),
        };
        self.send_response_via_debug(to_pid, vfs_msg::MSG_VFS_RMDIR_RESPONSE, &response)
    }

    /// Send an unlink error response to the client.
    fn send_unlink_error(&self, client_ctx: &ClientContext, error: VfsError) -> Result<(), AppError> {
        let response = UnlinkResponse {
            result: Err(error),
        };
        self.send_response(client_ctx, vfs_msg::MSG_VFS_UNLINK_RESPONSE, &response)
    }

    /// Send an unlink error response via debug channel (when no ClientContext available).
    fn send_unlink_error_via_debug(&self, to_pid: u32, error: VfsError) -> Result<(), AppError> {
        let response = UnlinkResponse {
            result: Err(error),
        };
        self.send_response_via_debug(to_pid, vfs_msg::MSG_VFS_UNLINK_RESPONSE, &response)
    }

    // =========================================================================
    // Request handlers (start async operations)
    // =========================================================================

    /// Handle MSG_VFS_RMDIR - remove directory
    pub fn handle_rmdir(&mut self, _ctx: &AppContext, msg: &Message) -> Result<(), AppError> {
        // Parse request
        let request: RmdirRequest = match serde_json::from_slice(&msg.data) {
            Ok(r) => r,
            Err(e) => {
                return self.send_rmdir_error_via_debug(
                    msg.from_pid,
                    VfsError::InvalidRequest(format!("Failed to parse request: {}", e)),
                );
            }
        };

        // Validate path
        if let Err(reason) = validate_path(&request.path) {
            return self.send_rmdir_error_via_debug(
                msg.from_pid,
                VfsError::InvalidPath(String::from(reason)),
            );
        }

        syscall::debug(&format!("VfsService: rmdir {}", request.path));

        // Derive permission context from caller
        let perm_ctx = derive_permission_context(msg.from_pid, &request.path);
        let client_ctx = ClientContext::from_message(msg);

        // Check inode exists and is directory
        self.start_storage_read(
            &inode_key(&request.path),
            PendingOp::GetInode {
                ctx: client_ctx,
                path: request.path,
                op_type: InodeOpType::Rmdir {
                    recursive: request.recursive,
                },
                perm_ctx,
            },
        )
    }

    /// Handle MSG_VFS_UNLINK - delete file
    pub fn handle_unlink(&mut self, _ctx: &AppContext, msg: &Message) -> Result<(), AppError> {
        // Parse request
        let request: UnlinkRequest = match serde_json::from_slice(&msg.data) {
            Ok(r) => r,
            Err(e) => {
                return self.send_unlink_error_via_debug(
                    msg.from_pid,
                    VfsError::InvalidRequest(format!("Failed to parse request: {}", e)),
                );
            }
        };

        // Validate path
        if let Err(reason) = validate_path(&request.path) {
            return self.send_unlink_error_via_debug(
                msg.from_pid,
                VfsError::InvalidPath(String::from(reason)),
            );
        }

        syscall::debug(&format!("VfsService: unlink {}", request.path));

        // Derive permission context from caller
        let perm_ctx = derive_permission_context(msg.from_pid, &request.path);
        let client_ctx = ClientContext::from_message(msg);

        // Check inode exists and is file
        self.start_storage_read(
            &inode_key(&request.path),
            PendingOp::GetInode {
                ctx: client_ctx,
                path: request.path,
                op_type: InodeOpType::Unlink,
                perm_ctx,
            },
        )
    }

    // =========================================================================
    // Result handlers
    // =========================================================================

    /// Handle rmdir inode result
    pub fn handle_rmdir_inode_result(
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
                return self.send_rmdir_error(client_ctx, VfsError::NotFound);
            }
            _ => {
                syscall::debug(&format!(
                    "VfsService: rmdir {} inode read failed: {} ({})",
                    path,
                    result_type,
                    result_type_name(result_type)
                ));
                return self.send_rmdir_error(
                    client_ctx,
                    VfsError::StorageError(format!(
                        "Inode read failed: {} ({})",
                        result_type,
                        result_type_name(result_type)
                    )),
                );
            }
        }

        match serde_json::from_slice::<Inode>(data) {
            Ok(inode) if inode.is_directory() => {
                // Check write permission before deleting
                if !check_write(&inode, perm_ctx) {
                    syscall::debug(&format!(
                        "VfsService: Permission denied for rmdir {} (pid={})",
                        path, client_ctx.pid
                    ));
                    return self.send_rmdir_error(client_ctx, VfsError::PermissionDenied);
                }

                self.start_storage_delete(
                    &inode_key(path),
                    PendingOp::DeleteInode {
                        ctx: Some(client_ctx.clone()),
                        response_tag: vfs_msg::MSG_VFS_RMDIR_RESPONSE,
                    },
                )
            }
            Ok(_) => self.send_rmdir_error(client_ctx, VfsError::NotADirectory),
            Err(e) => {
                syscall::debug(&format!(
                    "VfsService: rmdir {} failed to parse inode (denying): {}",
                    path, e
                ));
                self.send_rmdir_error(
                    client_ctx,
                    VfsError::StorageError(format!("Failed to parse inode: {}", e)),
                )
            }
        }
    }

    /// Handle unlink inode result
    pub fn handle_unlink_inode_result(
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
                return self.send_unlink_error(client_ctx, VfsError::NotFound);
            }
            _ => {
                syscall::debug(&format!(
                    "VfsService: unlink {} inode read failed: {} ({})",
                    path,
                    result_type,
                    result_type_name(result_type)
                ));
                return self.send_unlink_error(
                    client_ctx,
                    VfsError::StorageError(format!(
                        "Inode read failed: {} ({})",
                        result_type,
                        result_type_name(result_type)
                    )),
                );
            }
        }

        match serde_json::from_slice::<Inode>(data) {
            Ok(inode) if inode.is_file() => {
                // Check write permission before deleting
                if !check_write(&inode, perm_ctx) {
                    syscall::debug(&format!(
                        "VfsService: Permission denied for unlink {} (pid={})",
                        path, client_ctx.pid
                    ));
                    return self.send_unlink_error(client_ctx, VfsError::PermissionDenied);
                }

                // Delete content first (intermediate step - no response)
                if let Err(e) = self.start_storage_delete(
                    &content_key(path),
                    PendingOp::DeleteContent {
                        path: path.to_string(),
                    },
                ) {
                    syscall::debug(&format!(
                        "VfsService: Failed to start content delete for {}: {:?}",
                        path, e
                    ));
                    // Continue to delete inode anyway - content may not exist
                }

                // Delete inode (this will send the response)
                self.start_storage_delete(
                    &inode_key(path),
                    PendingOp::DeleteInode {
                        ctx: Some(client_ctx.clone()),
                        response_tag: vfs_msg::MSG_VFS_UNLINK_RESPONSE,
                    },
                )
            }
            Ok(_) => self.send_unlink_error(client_ctx, VfsError::NotAFile),
            Err(e) => {
                syscall::debug(&format!(
                    "VfsService: unlink {} failed to parse inode (denying): {}",
                    path, e
                ));
                self.send_unlink_error(
                    client_ctx,
                    VfsError::StorageError(format!("Failed to parse inode: {}", e)),
                )
            }
        }
    }

    /// Handle delete inode result
    ///
    /// If `client_ctx` is `None`, this is an intermediate step and no response is sent.
    pub fn handle_delete_inode_result(
        &mut self,
        client_ctx: Option<&ClientContext>,
        response_tag: u32,
        result_type: u8,
    ) -> Result<(), AppError> {
        // If no client context, this is an intermediate step - no response needed
        let client_ctx = match client_ctx {
            Some(ctx) => ctx,
            None => return Ok(()),
        };

        let success = result_type == storage_result::WRITE_OK;

        if response_tag == vfs_msg::MSG_VFS_RMDIR_RESPONSE {
            let response = RmdirResponse {
                result: if success {
                    Ok(())
                } else {
                    Err(VfsError::StorageError(format!(
                        "Rmdir inode delete failed: {} ({})",
                        result_type,
                        result_type_name(result_type)
                    )))
                },
            };
            self.send_response(client_ctx, response_tag, &response)
        } else if response_tag == vfs_msg::MSG_VFS_UNLINK_RESPONSE {
            let response = UnlinkResponse {
                result: if success {
                    Ok(())
                } else {
                    Err(VfsError::StorageError(format!(
                        "Unlink inode delete failed: {} ({})",
                        result_type,
                        result_type_name(result_type)
                    )))
                },
            };
            self.send_response(client_ctx, response_tag, &response)
        } else {
            Ok(())
        }
    }

    /// Handle delete content result
    ///
    /// This is always an intermediate step - no response is sent.
    /// The response is sent after the inode delete completes.
    pub fn handle_delete_content_result(
        &mut self,
        _path: &str,
        _result_type: u8,
    ) -> Result<(), AppError> {
        // Content delete is part of unlink - response sent after inode delete
        Ok(())
    }
}
