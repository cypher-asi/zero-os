//! VfsService trait definition.

use alloc::string::String;
use alloc::vec::Vec;

use crate::core::{DirEntry, FilePermissions, Inode, UserId, VfsError};
use crate::storage::{StorageQuota, StorageUsage};

/// Virtual filesystem service interface.
pub trait VfsService {
    // ========== Directory Operations ==========

    /// Create a directory.
    fn mkdir(&self, path: &str) -> Result<(), VfsError>;

    /// Create a directory and all parent directories.
    fn mkdir_p(&self, path: &str) -> Result<(), VfsError>;

    /// Remove an empty directory.
    fn rmdir(&self, path: &str) -> Result<(), VfsError>;

    /// Remove a directory and all contents recursively.
    fn rmdir_recursive(&self, path: &str) -> Result<(), VfsError>;

    /// List directory contents.
    fn readdir(&self, path: &str) -> Result<Vec<DirEntry>, VfsError>;

    // ========== File Operations ==========

    /// Write a file (create or overwrite).
    fn write_file(&self, path: &str, content: &[u8]) -> Result<(), VfsError>;

    /// Write an encrypted file.
    fn write_file_encrypted(
        &self,
        path: &str,
        content: &[u8],
        key: &[u8; 32],
    ) -> Result<(), VfsError>;

    /// Read a file.
    fn read_file(&self, path: &str) -> Result<Vec<u8>, VfsError>;

    /// Read an encrypted file.
    fn read_file_encrypted(&self, path: &str, key: &[u8; 32]) -> Result<Vec<u8>, VfsError>;

    /// Delete a file.
    fn unlink(&self, path: &str) -> Result<(), VfsError>;

    /// Rename/move a file or directory.
    fn rename(&self, from: &str, to: &str) -> Result<(), VfsError>;

    /// Copy a file.
    fn copy(&self, from: &str, to: &str) -> Result<(), VfsError>;

    // ========== Metadata Operations ==========

    /// Get file/directory metadata.
    fn stat(&self, path: &str) -> Result<Inode, VfsError>;

    /// Check if a path exists.
    fn exists(&self, path: &str) -> Result<bool, VfsError>;

    /// Change permissions.
    fn chmod(&self, path: &str, perms: FilePermissions) -> Result<(), VfsError>;

    /// Change ownership.
    fn chown(&self, path: &str, owner_id: Option<UserId>) -> Result<(), VfsError>;

    // ========== Symlink Operations ==========

    /// Create a symbolic link.
    fn symlink(&self, target: &str, link_path: &str) -> Result<(), VfsError>;

    /// Read a symbolic link target.
    fn readlink(&self, path: &str) -> Result<String, VfsError>;

    // ========== Path Utilities ==========

    /// Get user home directory path.
    fn get_home_dir(&self, user_id: UserId) -> String {
        alloc::format!("/home/{}", user_id)
    }

    /// Get user's .zos directory path.
    fn get_zos_dir(&self, user_id: UserId) -> String {
        alloc::format!("/home/{}/.zos", user_id)
    }

    /// Resolve a path (follow symlinks, normalize).
    fn resolve_path(&self, path: &str) -> Result<String, VfsError>;

    // ========== Quota Operations ==========

    /// Get storage usage for a path subtree.
    fn get_usage(&self, path: &str) -> Result<StorageUsage, VfsError>;

    /// Get quota for a user.
    fn get_quota(&self, user_id: UserId) -> Result<StorageQuota, VfsError>;

    /// Set quota for a user.
    fn set_quota(&self, user_id: UserId, max_bytes: u64) -> Result<(), VfsError>;
}
