//! VFS service trait and permission checking

mod permissions;
mod trait_def;

pub use permissions::{check_execute, check_read, check_write, PermissionContext, ProcessClass};
pub use trait_def::VfsService;
