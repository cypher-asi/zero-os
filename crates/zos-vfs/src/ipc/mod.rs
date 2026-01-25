//! VFS IPC protocol types
//!
//! Note: VFS message constants are defined in `zos-ipc` as the single source of truth.
//! This module re-exports them for backward compatibility and provides request/response types.

mod types;

pub use types::*;

/// VFS service IPC message types - re-exported from zos-ipc.
///
/// This module re-exports the VFS constants from zos-ipc which is the single
/// source of truth for all IPC protocol constants. This ensures Invariant 32:
/// "Single Source of Truth for All Constants".
pub mod vfs_msg {
    // Re-export all VFS constants from zos-ipc
    pub use zos_ipc::vfs_dir::*;
    pub use zos_ipc::vfs_file::*;
    pub use zos_ipc::vfs_meta::*;
    pub use zos_ipc::vfs_quota::*;
}
