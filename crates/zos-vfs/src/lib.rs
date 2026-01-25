//! Zero OS Virtual Filesystem Layer
//!
//! The VFS layer provides a hierarchical filesystem abstraction for Zero OS:
//!
//! - **Types**: Inode, FilePermissions, DirEntry for filesystem metadata
//! - **Path**: Path validation, normalization, and resolution
//! - **Service**: VfsService trait for filesystem operations
//! - **Storage**: Content storage, encryption, and quota management
//! - **Bootstrap**: Filesystem initialization on first boot
//! - **IPC**: Inter-process communication protocol for VFS operations
//!
//! # Design Principles
//!
//! 1. **Hierarchical paths**: Unix-like `/path/to/file` semantics
//! 2. **User-centric**: Each user has an isolated home directory
//! 3. **Permission-aware**: File access controlled by ownership and permissions
//! 4. **Encryption-ready**: Support for encrypted file storage
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────────┐
//! │                            VFS Layer                                         │
//! │                                                                              │
//! │  ┌──────────────────────────────────────────────────────────────────────┐   │
//! │  │                         VFS Service                                    │   │
//! │  │  • Path resolution      • Permission checking                         │   │
//! │  │  • Inode management     • Directory operations                        │   │
//! │  │  • File read/write      • Metadata operations                         │   │
//! │  └────────────────────────────────┬──────────────────────────────────────┘   │
//! │                                   │                                          │
//! │                                   ▼                                          │
//! │  ┌──────────────────────────────────────────────────────────────────────┐   │
//! │  │                       Storage Backend                                  │   │
//! │  │  ┌─────────────────────────┐  ┌─────────────────────────┐            │   │
//! │  │  │   zos-userspace DB      │  │   Content Store         │            │   │
//! │  │  │  • Inodes (metadata)    │  │  • File content blobs   │            │   │
//! │  │  │  • Directory entries    │  │  • Encrypted content    │            │   │
//! │  │  └─────────────────────────┘  └─────────────────────────┘            │   │
//! │  └──────────────────────────────────────────────────────────────────────┘   │
//! └─────────────────────────────────────────────────────────────────────────────┘
//! ```

#![no_std]
extern crate alloc;

pub mod client;
pub mod core;
pub mod ipc;
pub mod service;
pub mod testing;

pub mod bootstrap;
pub mod storage;

// Convenient re-exports at crate root
pub use client::{VfsClient, VFS_ENDPOINT_SLOT, VFS_RESPONSE_SLOT};
pub use core::{normalize_path, parent_path, validate_path};
pub use core::{DirEntry, FilePermissions, Inode, InodeType, StorageErrorKind, UserId, VfsError};
pub use ipc::vfs_msg;
pub use service::{check_execute, check_read, check_write, PermissionContext, ProcessClass, VfsService};
pub use storage::{StorageQuota, StorageUsage};
pub use testing::MemoryVfs;

// Re-export async_client module for backward compatibility
pub use client::async_ops as async_client;
