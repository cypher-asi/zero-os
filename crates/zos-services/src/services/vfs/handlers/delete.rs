//! Delete operation handlers for VFS Service
//!
//! Handles: rmdir, unlink operations
//!
//! # Safety Properties
//!
//! - **Success**: content deleted (if file), inode deleted
//! - **Acceptable partial failure**: orphan content (content exists without inode)
//! - **Forbidden**: inode deleted while content still referenced elsewhere

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
    ClientContext, InodeOpType, PendingOp, UnlinkStage, VfsService,
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

        // Start the unlink state machine: first read inode to verify it's a file
        self.start_storage_read(
            &inode_key(&request.path),
            PendingOp::UnlinkOp {
                ctx: client_ctx,
                path: request.path,
                perm_ctx,
                stage: UnlinkStage::ReadingInode,
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
    /// DEPRECATED: This is the old handler for concurrent deletes.
    /// New code uses UnlinkOp state machine for sequential delete.
    pub fn handle_delete_content_result(
        &mut self,
        _path: &str,
        _result_type: u8,
    ) -> Result<(), AppError> {
        // Content delete is part of unlink - response sent after inode delete
        Ok(())
    }

    // =========================================================================
    // Unlink State Machine (Rule 5, 7: Sequential delete)
    // =========================================================================

    /// Handle unlink operation result (state machine)
    ///
    /// This handler implements the sequential unlink state machine:
    /// 1. ReadingInode: Verify it's a file and check permissions
    /// 2. DeletingContent: Delete content (must complete before inode)
    /// 3. DeletingInode: Delete inode (only after content delete succeeds)
    pub fn handle_unlink_op_result(
        &mut self,
        client_ctx: &ClientContext,
        path: &str,
        perm_ctx: &PermissionContext,
        stage: UnlinkStage,
        result_type: u8,
        data: &[u8],
    ) -> Result<(), AppError> {
        match stage {
            UnlinkStage::ReadingInode => {
                self.handle_unlink_reading_inode(client_ctx, path, perm_ctx, result_type, data)
            }
            UnlinkStage::DeletingContent => {
                self.handle_unlink_deleting_content(client_ctx, path, perm_ctx, result_type)
            }
            UnlinkStage::DeletingInode => {
                self.handle_unlink_deleting_inode(client_ctx, path, result_type)
            }
        }
    }

    /// Stage 1: Read inode to verify it's a file and check permissions
    fn handle_unlink_reading_inode(
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

        // Parse inode - FAIL CLOSED on parse error
        let inode = match serde_json::from_slice::<Inode>(data) {
            Ok(inode) => inode,
            Err(e) => {
                syscall::debug(&format!(
                    "VfsService: unlink {} failed to parse inode (denying): {}",
                    path, e
                ));
                return self.send_unlink_error(
                    client_ctx,
                    VfsError::StorageError(format!("Failed to parse inode: {}", e)),
                );
            }
        };

        // Verify it's a file
        if !inode.is_file() {
            return self.send_unlink_error(client_ctx, VfsError::NotAFile);
        }

        // Check write permission before deleting
        if !check_write(&inode, perm_ctx) {
            syscall::debug(&format!(
                "VfsService: Permission denied for unlink {} (pid={})",
                path, client_ctx.pid
            ));
            return self.send_unlink_error(client_ctx, VfsError::PermissionDenied);
        }

        // Permission granted - delete content FIRST (sequential, not concurrent)
        // This ensures we don't have a dangling inode reference
        self.start_storage_delete(
            &content_key(path),
            PendingOp::UnlinkOp {
                ctx: client_ctx.clone(),
                path: path.to_string(),
                perm_ctx: perm_ctx.clone(),
                stage: UnlinkStage::DeletingContent,
            },
        )
    }

    /// Stage 2: Content delete completed - now delete inode
    fn handle_unlink_deleting_content(
        &mut self,
        client_ctx: &ClientContext,
        path: &str,
        perm_ctx: &PermissionContext,
        result_type: u8,
    ) -> Result<(), AppError> {
        // Rule 5: Handle content delete result properly
        match result_type {
            storage_result::WRITE_OK => {
                // Content deleted successfully - proceed to delete inode
            }
            storage_result::NOT_FOUND => {
                // Content was already missing - this is acceptable, proceed to delete inode
                // (orphaned inode scenario)
                syscall::debug(&format!(
                    "VfsService: unlink {} content was already missing, proceeding to delete inode",
                    path
                ));
            }
            _ => {
                // Rule 5: Content delete failed - abort the operation
                syscall::debug(&format!(
                    "VfsService: unlink {} content delete failed: {} ({}) - aborting",
                    path,
                    result_type,
                    result_type_name(result_type)
                ));
                return self.send_unlink_error(
                    client_ctx,
                    VfsError::StorageError(format!(
                        "Content delete failed: {} ({})",
                        result_type,
                        result_type_name(result_type)
                    )),
                );
            }
        }

        // Content handled - now delete inode
        self.start_storage_delete(
            &inode_key(path),
            PendingOp::UnlinkOp {
                ctx: client_ctx.clone(),
                path: path.to_string(),
                perm_ctx: perm_ctx.clone(),
                stage: UnlinkStage::DeletingInode,
            },
        )
    }

    /// Stage 3: Inode delete completed - send response
    fn handle_unlink_deleting_inode(
        &mut self,
        client_ctx: &ClientContext,
        path: &str,
        result_type: u8,
    ) -> Result<(), AppError> {
        if result_type != storage_result::WRITE_OK {
            // Inode delete failed - content is now orphaned but that's acceptable
            syscall::debug(&format!(
                "VfsService: unlink {} inode delete failed: {} ({}) - content is orphaned",
                path,
                result_type,
                result_type_name(result_type)
            ));
            return self.send_unlink_error(
                client_ctx,
                VfsError::StorageError(format!(
                    "Inode delete failed: {} ({})",
                    result_type,
                    result_type_name(result_type)
                )),
            );
        }

        // Both content and inode deleted successfully
        syscall::debug(&format!("VfsService: unlink {} completed successfully", path));
        let response = UnlinkResponse { result: Ok(()) };
        self.send_response(client_ctx, vfs_msg::MSG_VFS_UNLINK_RESPONSE, &response)
    }
}
