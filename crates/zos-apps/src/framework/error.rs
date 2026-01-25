//! Error Types for Zero OS Apps
//!
//! Defines errors that can occur during app execution.

use super::manifest::ObjectType;
use alloc::string::String;

/// Errors that can occur in app execution.
///
/// This enum captures all failure modes for Zero OS applications,
/// from initialization failures to IPC and protocol errors.
#[derive(Clone, Debug, thiserror::Error)]
pub enum AppError {
    /// Initialization failed with the given reason.
    #[error("initialization failed: {0}")]
    InitFailed(String),

    /// A required capability was not granted to this process.
    #[error("missing capability: {0:?}")]
    MissingCapability(ObjectType),

    /// IPC communication failed.
    #[error("IPC error: {0}")]
    IpcError(String),

    /// Protocol parsing or encoding failed.
    #[error("protocol error: {0}")]
    ProtocolError(#[from] ProtocolError),

    /// An internal application error occurred.
    #[error("internal error: {0}")]
    Internal(String),
}

impl From<zos_vfs::VfsError> for AppError {
    fn from(e: zos_vfs::VfsError) -> Self {
        AppError::IpcError(alloc::format!("VFS error: {:?}", e))
    }
}

/// Errors that can occur during protocol parsing.
///
/// These errors indicate malformed or invalid wire-format messages
/// received via IPC.
#[derive(Clone, Debug, thiserror::Error)]
pub enum ProtocolError {
    /// Message is too short to contain required header or fields.
    #[error("message too short")]
    TooShort,

    /// Protocol version byte is not recognized.
    #[error("unknown protocol version: {0}")]
    UnknownVersion(u8),

    /// Payload length in header exceeds available data.
    #[error("payload overflow: declared {declared} bytes, only {available} available")]
    PayloadOverflow { declared: usize, available: usize },

    /// String length prefix exceeds available data.
    #[error("string overflow: declared {declared} bytes, only {available} available")]
    StringOverflow { declared: usize, available: usize },

    /// String data is not valid UTF-8.
    #[error("invalid UTF-8 in string")]
    InvalidUtf8,

    /// Payload is empty when data was expected.
    #[error("empty payload")]
    EmptyPayload,

    /// Type tag in envelope does not match expected type.
    #[error("unexpected type: expected 0x{expected:02x}, got 0x{got:02x}")]
    UnexpectedType { expected: u8, got: u8 },

    /// Message type tag is not recognized.
    #[error("unknown message type: 0x{0:02x}")]
    UnknownMessageType(u8),

    /// Enum discriminant is not a valid variant.
    #[error("invalid enum value for {field}: {value}")]
    InvalidEnumValue { field: &'static str, value: u8 },
}
