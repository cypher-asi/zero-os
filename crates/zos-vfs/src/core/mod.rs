//! Core VFS types and utilities

mod error;
mod path;
mod types;

pub use error::{StorageErrorKind, VfsError};
pub use path::{extract_user_id, filename, is_under, join_path, normalize_path, parent_path, validate_path};
pub use types::{DirEntry, FilePermissions, Inode, InodeType, UserId};
