//! Error types for the VFS layer.

use alloc::string::String;
use serde::{Deserialize, Serialize};

/// Specific kinds of storage errors for better error context.
///
/// Instead of collapsing all storage errors to strings, this enum preserves
/// the specific failure mode for better error handling and debugging.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum StorageErrorKind {
    /// Connection to storage backend failed
    ConnectionFailed,
    /// Storage operation timed out
    Timeout {
        /// What operation was being attempted
        operation: String,
    },
    /// Data corruption detected (e.g., checksum mismatch)
    ChunkCorrupt {
        /// Which chunk was corrupt (if known)
        chunk_index: Option<u32>,
        /// Expected hash (if known)
        expected_hash: Option<String>,
    },
    /// Storage quota exceeded
    QuotaExceeded {
        /// Current usage in bytes
        used: u64,
        /// Quota limit in bytes
        limit: u64,
    },
    /// Permission denied for storage operation
    PermissionDenied {
        /// Path that was denied
        path: String,
        /// Required permission (read, write, etc.)
        required: String,
    },
    /// Key not found in storage
    KeyNotFound,
    /// Storage backend unavailable
    Unavailable,
    /// Unknown error (fallback for string messages)
    Unknown(String),
}

impl StorageErrorKind {
    /// Create a timeout error.
    pub fn timeout(operation: impl Into<String>) -> Self {
        Self::Timeout {
            operation: operation.into(),
        }
    }

    /// Create a permission denied error.
    pub fn permission_denied(path: impl Into<String>, required: impl Into<String>) -> Self {
        Self::PermissionDenied {
            path: path.into(),
            required: required.into(),
        }
    }

    /// Create a quota exceeded error.
    pub fn quota_exceeded(used: u64, limit: u64) -> Self {
        Self::QuotaExceeded { used, limit }
    }

    /// Create an unknown error from a string (for legacy compatibility).
    pub fn unknown(msg: impl Into<String>) -> Self {
        Self::Unknown(msg.into())
    }
}

/// Errors from VFS operations.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum VfsError {
    /// Path not found
    NotFound,

    /// Path already exists
    AlreadyExists,

    /// Not a directory
    NotADirectory,

    /// Not a file
    NotAFile,

    /// Directory not empty
    DirectoryNotEmpty,

    /// Permission denied
    PermissionDenied,

    /// Invalid path format
    InvalidPath(String),

    /// Invalid request (e.g., malformed JSON, missing fields)
    InvalidRequest(String),

    /// Storage backend error with detailed context
    Storage {
        /// The specific error kind
        kind: StorageErrorKind,
        /// Optional additional context
        context: Option<String>,
    },

    /// Storage backend error (legacy - string only)
    /// Deprecated: Prefer `Storage { kind, context }` for new code.
    StorageError(String),

    /// Quota exceeded
    QuotaExceeded,

    /// File too large
    FileTooLarge,

    /// Encryption error
    EncryptionError(String),

    /// Decryption error
    DecryptionError(String),

    /// I/O error
    IoError(String),

    /// Operation not supported
    NotSupported(String),
}

impl VfsError {
    /// Create a storage error with message (legacy compatibility).
    pub fn storage(msg: impl Into<String>) -> Self {
        Self::StorageError(msg.into())
    }

    /// Create a typed storage error.
    pub fn storage_error(kind: StorageErrorKind) -> Self {
        Self::Storage {
            kind,
            context: None,
        }
    }

    /// Create a typed storage error with context.
    pub fn storage_error_with_context(kind: StorageErrorKind, context: impl Into<String>) -> Self {
        Self::Storage {
            kind,
            context: Some(context.into()),
        }
    }

    /// Create an I/O error with message.
    pub fn io(msg: impl Into<String>) -> Self {
        Self::IoError(msg.into())
    }

    /// Create an invalid path error with message.
    pub fn invalid_path(msg: impl Into<String>) -> Self {
        Self::InvalidPath(msg.into())
    }

    /// Check if this is a "not found" error (including storage key not found).
    pub fn is_not_found(&self) -> bool {
        matches!(
            self,
            VfsError::NotFound | VfsError::Storage { kind: StorageErrorKind::KeyNotFound, .. }
        )
    }

    /// Check if this is a permission error.
    pub fn is_permission_denied(&self) -> bool {
        matches!(
            self,
            VfsError::PermissionDenied
                | VfsError::Storage {
                    kind: StorageErrorKind::PermissionDenied { .. },
                    ..
                }
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_construction() {
        let err = VfsError::storage("test error");
        match err {
            VfsError::StorageError(msg) => assert_eq!(msg, "test error"),
            _ => panic!("Expected StorageError"),
        }
    }

    #[test]
    fn test_typed_storage_error() {
        let err = VfsError::storage_error(StorageErrorKind::ConnectionFailed);
        match err {
            VfsError::Storage { kind, context } => {
                assert_eq!(kind, StorageErrorKind::ConnectionFailed);
                assert!(context.is_none());
            }
            _ => panic!("Expected Storage variant"),
        }
    }

    #[test]
    fn test_storage_error_with_context() {
        let err = VfsError::storage_error_with_context(
            StorageErrorKind::timeout("read"),
            "file: /test/data",
        );
        match err {
            VfsError::Storage { kind, context } => {
                assert!(matches!(kind, StorageErrorKind::Timeout { .. }));
                assert_eq!(context, Some(String::from("file: /test/data")));
            }
            _ => panic!("Expected Storage variant"),
        }
    }

    #[test]
    fn test_is_not_found() {
        assert!(VfsError::NotFound.is_not_found());
        assert!(VfsError::storage_error(StorageErrorKind::KeyNotFound).is_not_found());
        assert!(!VfsError::PermissionDenied.is_not_found());
    }

    #[test]
    fn test_is_permission_denied() {
        assert!(VfsError::PermissionDenied.is_permission_denied());
        assert!(VfsError::storage_error(StorageErrorKind::permission_denied("/test", "write"))
            .is_permission_denied());
        assert!(!VfsError::NotFound.is_permission_denied());
    }
}
