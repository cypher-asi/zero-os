//! Async VFS Client for Event-Driven Services
//!
//! This module provides non-blocking VFS IPC helpers for services that cannot
//! use the blocking `VfsClient::call()` pattern.
//!
//! # Invariant Compliance
//!
//! This module ensures Invariant 31 ("All disk read/write operations must flow
//! through the storage hierarchy") by routing storage operations through VFS
//! Service (PID 4) via IPC, rather than using direct storage syscalls.
//!
//! # Usage Pattern
//!
//! ```ignore
//! use zos_vfs::client::async_ops::{self, VfsPendingOp};
//!
//! struct MyService {
//!     pending_vfs_ops: BTreeMap<u32, VfsPendingOp>,
//!     next_vfs_request_id: u32,
//! }
//!
//! impl MyService {
//!     fn read_file(&mut self, path: &str) -> Result<(), VfsError> {
//!         let request_id = self.next_vfs_request_id;
//!         self.next_vfs_request_id += 1;
//!         
//!         async_ops::send_read_request(path)?;
//!         self.pending_vfs_ops.insert(request_id, VfsPendingOp::Read { path: path.into() });
//!         Ok(())
//!     }
//!
//!     fn on_message(&mut self, msg: Message) -> Result<(), VfsError> {
//!         if async_ops::is_vfs_response(msg.tag) {
//!             self.handle_vfs_response(msg)
//!         } else {
//!             // Handle other messages
//!         }
//!     }
//! }
//! ```

use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

use crate::core::{DirEntry, Inode, VfsError};
use crate::ipc::{
    vfs_msg, ExistsRequest, ExistsResponse, MkdirRequest, MkdirResponse, ReadFileRequest,
    ReadFileResponse, ReaddirRequest, ReaddirResponse, StatRequest, StatResponse, UnlinkRequest,
    UnlinkResponse, WriteFileRequest, WriteFileResponse,
};

/// Default capability slot for VFS service endpoint (same as VfsClient).
/// This is assigned by init when the process starts.
pub const VFS_ENDPOINT_SLOT: u32 = 3;

// =============================================================================
// VFS Request Senders (Non-blocking)
// =============================================================================

/// Send a VFS read file request (non-blocking).
///
/// The response will arrive as a message with tag `MSG_VFS_READ_RESPONSE`.
pub fn send_read_request(path: &str) -> Result<(), VfsError> {
    let request = ReadFileRequest {
        path: String::from(path),
        offset: None,
        length: None,
    };
    send_vfs_request(vfs_msg::MSG_VFS_READ, &request)
}

/// Send a VFS write file request (non-blocking).
///
/// The response will arrive as a message with tag `MSG_VFS_WRITE_RESPONSE`.
pub fn send_write_request(path: &str, content: &[u8]) -> Result<(), VfsError> {
    let request = WriteFileRequest {
        path: String::from(path),
        content: content.to_vec(),
        encrypt: false,
    };
    send_vfs_request(vfs_msg::MSG_VFS_WRITE, &request)
}

/// Send a VFS exists check request (non-blocking).
///
/// The response will arrive as a message with tag `MSG_VFS_EXISTS_RESPONSE`.
pub fn send_exists_request(path: &str) -> Result<(), VfsError> {
    let request = ExistsRequest {
        path: String::from(path),
    };
    send_vfs_request(vfs_msg::MSG_VFS_EXISTS, &request)
}

/// Send a VFS mkdir request (non-blocking).
///
/// The response will arrive as a message with tag `MSG_VFS_MKDIR_RESPONSE`.
pub fn send_mkdir_request(path: &str, create_parents: bool) -> Result<(), VfsError> {
    let request = MkdirRequest {
        path: String::from(path),
        create_parents,
    };
    send_vfs_request(vfs_msg::MSG_VFS_MKDIR, &request)
}

/// Send a VFS unlink (delete file) request (non-blocking).
///
/// The response will arrive as a message with tag `MSG_VFS_UNLINK_RESPONSE`.
pub fn send_unlink_request(path: &str) -> Result<(), VfsError> {
    let request = UnlinkRequest {
        path: String::from(path),
    };
    send_vfs_request(vfs_msg::MSG_VFS_UNLINK, &request)
}

/// Send a VFS readdir request (non-blocking).
///
/// The response will arrive as a message with tag `MSG_VFS_READDIR_RESPONSE`.
pub fn send_readdir_request(path: &str) -> Result<(), VfsError> {
    let request = ReaddirRequest {
        path: String::from(path),
    };
    send_vfs_request(vfs_msg::MSG_VFS_READDIR, &request)
}

/// Send a VFS stat request (non-blocking).
///
/// The response will arrive as a message with tag `MSG_VFS_STAT_RESPONSE`.
pub fn send_stat_request(path: &str) -> Result<(), VfsError> {
    let request = StatRequest {
        path: String::from(path),
    };
    send_vfs_request(vfs_msg::MSG_VFS_STAT, &request)
}

// =============================================================================
// VFS Response Helpers
// =============================================================================

/// Check if a message tag is a VFS response.
pub fn is_vfs_response(tag: u32) -> bool {
    matches!(
        tag,
        vfs_msg::MSG_VFS_MKDIR_RESPONSE
            | vfs_msg::MSG_VFS_RMDIR_RESPONSE
            | vfs_msg::MSG_VFS_READDIR_RESPONSE
            | vfs_msg::MSG_VFS_WRITE_RESPONSE
            | vfs_msg::MSG_VFS_READ_RESPONSE
            | vfs_msg::MSG_VFS_UNLINK_RESPONSE
            | vfs_msg::MSG_VFS_RENAME_RESPONSE
            | vfs_msg::MSG_VFS_COPY_RESPONSE
            | vfs_msg::MSG_VFS_STAT_RESPONSE
            | vfs_msg::MSG_VFS_EXISTS_RESPONSE
            | vfs_msg::MSG_VFS_CHMOD_RESPONSE
            | vfs_msg::MSG_VFS_CHOWN_RESPONSE
            | vfs_msg::MSG_VFS_GET_USAGE_RESPONSE
            | vfs_msg::MSG_VFS_GET_QUOTA_RESPONSE
    )
}

/// Parse a VFS read response.
///
/// Returns `Ok(data)` on success, `Err(error_message)` on failure.
pub fn parse_read_response(data: &[u8]) -> Result<Vec<u8>, String> {
    match serde_json::from_slice::<ReadFileResponse>(data) {
        Ok(response) => response.result.map_err(|e| format!("{:?}", e)),
        Err(e) => Err(format!("Parse error: {}", e)),
    }
}

/// Parse a VFS write response.
///
/// Returns `Ok(())` on success, `Err(error_message)` on failure.
pub fn parse_write_response(data: &[u8]) -> Result<(), String> {
    match serde_json::from_slice::<WriteFileResponse>(data) {
        Ok(response) => response.result.map_err(|e| format!("{:?}", e)),
        Err(e) => Err(format!("Parse error: {}", e)),
    }
}

/// Parse a VFS exists response.
///
/// Returns `Ok(exists)` where `exists` is true if path exists.
pub fn parse_exists_response(data: &[u8]) -> Result<bool, String> {
    match serde_json::from_slice::<ExistsResponse>(data) {
        Ok(response) => response.result.map_err(|e| format!("{:?}", e)),
        Err(e) => Err(format!("Parse error: {}", e)),
    }
}

/// Parse a VFS mkdir response.
///
/// Returns `Ok(())` on success, `Err(error_message)` on failure.
pub fn parse_mkdir_response(data: &[u8]) -> Result<(), String> {
    match serde_json::from_slice::<MkdirResponse>(data) {
        Ok(response) => response.result.map_err(|e| format!("{:?}", e)),
        Err(e) => Err(format!("Parse error: {}", e)),
    }
}

/// Parse a VFS unlink response.
///
/// Returns `Ok(())` on success, `Err(error_message)` on failure.
pub fn parse_unlink_response(data: &[u8]) -> Result<(), String> {
    match serde_json::from_slice::<UnlinkResponse>(data) {
        Ok(response) => response.result.map_err(|e| format!("{:?}", e)),
        Err(e) => Err(format!("Parse error: {}", e)),
    }
}

/// Parse a VFS readdir response.
///
/// Returns `Ok(entries)` on success, `Err(error_message)` on failure.
pub fn parse_readdir_response(data: &[u8]) -> Result<Vec<DirEntry>, String> {
    match serde_json::from_slice::<ReaddirResponse>(data) {
        Ok(response) => response.result.map_err(|e| format!("{:?}", e)),
        Err(e) => Err(format!("Parse error: {}", e)),
    }
}

/// Parse a VFS stat response.
///
/// Returns `Ok(inode)` on success, `Err(error_message)` on failure.
pub fn parse_stat_response(data: &[u8]) -> Result<Inode, String> {
    match serde_json::from_slice::<StatResponse>(data) {
        Ok(response) => response.result.map_err(|e| format!("{:?}", e)),
        Err(e) => Err(format!("Parse error: {}", e)),
    }
}

// =============================================================================
// Internal Helpers
// =============================================================================

/// Send a VFS request via IPC (non-blocking).
#[cfg(target_arch = "wasm32")]
fn send_vfs_request<T: serde::Serialize>(tag: u32, request: &T) -> Result<(), VfsError> {
    let data = serde_json::to_vec(request)
        .map_err(|e| VfsError::StorageError(format!("Serialize error: {}", e)))?;

    zos_process::send(VFS_ENDPOINT_SLOT, tag, &data)
        .map_err(|e| VfsError::StorageError(format!("Send error: {}", e)))
}

#[cfg(not(target_arch = "wasm32"))]
fn send_vfs_request<T: serde::Serialize>(_tag: u32, _request: &T) -> Result<(), VfsError> {
    // No-op outside WASM - allows tests to compile
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_vfs_response() {
        assert!(is_vfs_response(vfs_msg::MSG_VFS_READ_RESPONSE));
        assert!(is_vfs_response(vfs_msg::MSG_VFS_WRITE_RESPONSE));
        assert!(is_vfs_response(vfs_msg::MSG_VFS_EXISTS_RESPONSE));

        // Not a VFS response
        assert!(!is_vfs_response(vfs_msg::MSG_VFS_READ)); // Request, not response
        assert!(!is_vfs_response(0x1000)); // Init message
    }
}
