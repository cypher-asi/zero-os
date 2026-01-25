//! Permission checking utilities for the VFS layer.

use crate::core::{Inode, UserId};

/// Process classification for permission checking.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProcessClass {
    /// System processes (init, terminal, etc.)
    System,
    /// Runtime services (storage, network, identity, etc.)
    Runtime,
    /// User applications
    Application,
}

/// Permission check context.
#[derive(Clone, Debug)]
pub struct PermissionContext {
    /// Calling user ID (if authenticated)
    pub user_id: Option<UserId>,
    /// Process classification
    pub process_class: ProcessClass,
}

impl PermissionContext {
    /// Create a system context.
    pub fn system() -> Self {
        Self {
            user_id: None,
            process_class: ProcessClass::System,
        }
    }

    /// Create a user context.
    pub fn user(user_id: UserId) -> Self {
        Self {
            user_id: Some(user_id),
            process_class: ProcessClass::Application,
        }
    }
}

/// Check if a context has read permission on an inode.
pub fn check_read(inode: &Inode, ctx: &PermissionContext) -> bool {
    // System processes check system_read
    if ctx.process_class == ProcessClass::System || ctx.process_class == ProcessClass::Runtime {
        return inode.permissions.system_read;
    }

    // Owner check
    if let Some(user_id) = ctx.user_id {
        if inode.owner_id == Some(user_id) {
            return inode.permissions.owner_read;
        }
    }

    // World check
    inode.permissions.world_read
}

/// Check if a context has write permission on an inode.
pub fn check_write(inode: &Inode, ctx: &PermissionContext) -> bool {
    // System processes check system_write
    if ctx.process_class == ProcessClass::System || ctx.process_class == ProcessClass::Runtime {
        return inode.permissions.system_write;
    }

    // Owner check
    if let Some(user_id) = ctx.user_id {
        if inode.owner_id == Some(user_id) {
            return inode.permissions.owner_write;
        }
    }

    // World check
    inode.permissions.world_write
}

/// Check if a context has execute (traverse) permission on a directory.
pub fn check_execute(inode: &Inode, ctx: &PermissionContext) -> bool {
    if !inode.is_directory() {
        return false;
    }

    // System processes always have traverse
    if ctx.process_class == ProcessClass::System || ctx.process_class == ProcessClass::Runtime {
        return true;
    }

    // Owner check
    if let Some(user_id) = ctx.user_id {
        if inode.owner_id == Some(user_id) {
            return inode.permissions.owner_execute;
        }
    }

    // For directories, world_read implies traverse
    inode.permissions.world_read
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::FilePermissions;
    use alloc::string::String;

    #[test]
    fn test_permission_check_read() {
        let inode = Inode::new_file(
            String::from("/test"),
            String::from("/"),
            String::from("test"),
            Some(1),
            100,
            None,
            1000,
        );

        // Owner can read
        let owner_ctx = PermissionContext::user(1);
        assert!(check_read(&inode, &owner_ctx));

        // Non-owner cannot read (no world read)
        let other_ctx = PermissionContext::user(2);
        assert!(!check_read(&inode, &other_ctx));

        // System can read
        let system_ctx = PermissionContext::system();
        assert!(check_read(&inode, &system_ctx));
    }

    #[test]
    fn test_permission_check_write() {
        let mut inode = Inode::new_file(
            String::from("/tmp/test"),
            String::from("/tmp"),
            String::from("test"),
            Some(1),
            100,
            None,
            1000,
        );
        inode.permissions = FilePermissions::world_rw();

        // Owner can write
        let owner_ctx = PermissionContext::user(1);
        assert!(check_write(&inode, &owner_ctx));

        // World can write (world_rw permissions)
        let other_ctx = PermissionContext::user(2);
        assert!(check_write(&inode, &other_ctx));
    }
}
