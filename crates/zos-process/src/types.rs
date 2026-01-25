//! Type definitions for Zero OS process syscalls

use alloc::string::String;
use alloc::vec::Vec;

// ============================================================================
// Object Types (for capabilities)
// ============================================================================

/// Types of kernel objects that can be accessed via capabilities
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum ObjectType {
    /// IPC endpoint
    Endpoint = 1,
    /// Console I/O
    Console = 2,
    /// Persistent storage (namespaced per-app)
    Storage = 3,
    /// Network access
    Network = 4,
    /// Process management (spawn, kill)
    Process = 5,
    /// Memory region
    Memory = 6,
}

impl ObjectType {
    /// Convert from u8
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            1 => Some(ObjectType::Endpoint),
            2 => Some(ObjectType::Console),
            3 => Some(ObjectType::Storage),
            4 => Some(ObjectType::Network),
            5 => Some(ObjectType::Process),
            6 => Some(ObjectType::Memory),
            _ => None,
        }
    }

    /// Get display name
    pub fn name(&self) -> &'static str {
        match self {
            ObjectType::Endpoint => "Endpoint",
            ObjectType::Console => "Console",
            ObjectType::Storage => "Storage",
            ObjectType::Network => "Network",
            ObjectType::Process => "Process",
            ObjectType::Memory => "Memory",
        }
    }
}

// ============================================================================
// Permissions
// ============================================================================

/// Permission flags for capability operations
#[derive(Clone, Copy, Debug, Default)]
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
