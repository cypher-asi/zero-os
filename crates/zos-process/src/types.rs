//! Type definitions for Zero OS process syscalls

use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// Object Types (for capabilities)
// ============================================================================

// Re-export ObjectType from zos-ipc - the single source of truth for capability types.
// This ensures all crates use consistent values when granting/checking capabilities.
pub use zos_ipc::ObjectType;

// ============================================================================
// Permissions
// ============================================================================

/// Permission flags for capability operations
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Permissions {
    pub read: bool,
    pub write: bool,
    pub grant: bool,
}

impl Permissions {
    /// Full permissions
    pub fn full() -> Self {
        Self {
            read: true,
            write: true,
            grant: true,
        }
    }

    /// Read-only
    pub fn read_only() -> Self {
        Self {
            read: true,
            write: false,
            grant: false,
        }
    }

    /// Write-only
    pub fn write_only() -> Self {
        Self {
            read: false,
            write: true,
            grant: false,
        }
    }

    /// Pack into a single byte
    pub fn to_byte(&self) -> u8 {
        let mut b = 0u8;
        if self.read {
            b |= 0x01;
        }
        if self.write {
            b |= 0x02;
        }
        if self.grant {
            b |= 0x04;
        }
        b
    }

    /// Unpack from a byte
    pub fn from_byte(b: u8) -> Self {
        Self {
            read: (b & 0x01) != 0,
            write: (b & 0x02) != 0,
            grant: (b & 0x04) != 0,
        }
    }
}

// ============================================================================
// IPC Message Types
// ============================================================================

/// A received IPC message
#[derive(Clone, Debug)]
pub struct ReceivedMessage {
    /// Sender's PID
    pub from_pid: u32,
    /// Message tag
    pub tag: u32,
    /// Capability slots containing transferred capabilities
    /// These are slots in the receiver's CSpace where the kernel installed
    /// capabilities that were transferred with this message.
    pub cap_slots: Vec<u32>,
    /// Message data
    pub data: Vec<u8>,
}

// ============================================================================
// Introspection Types
// ============================================================================

/// Capability info returned from list_caps
#[derive(Clone, Debug)]
pub struct CapInfo {
    pub slot: u32,
    pub object_type: u8,
    pub object_id: u64,
    pub can_read: bool,
    pub can_write: bool,
    pub can_grant: bool,
}

/// Process info returned from list_processes
#[derive(Clone, Debug)]
pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
    pub state: u8,
}
