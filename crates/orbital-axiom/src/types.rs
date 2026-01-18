//! Common types for the Axiom layer.

use serde::{Deserialize, Serialize};

/// Process identifier (matches orbital-kernel's ProcessId)
pub type ProcessId = u64;

/// Event identifier (monotonic, unique within SysLog)
pub type EventId = u64;

/// Commit identifier (32-byte hash)
pub type CommitId = [u8; 32];

/// Capability slot index
pub type CapSlot = u32;

/// Endpoint identifier
pub type EndpointId = u64;

/// Capability permissions (serializable)
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct Permissions {
    /// Can read/receive
    pub read: bool,
    /// Can write/send
    pub write: bool,
    /// Can grant to others
    pub grant: bool,
}

impl Permissions {
    /// Full permissions (read, write, grant)
    pub fn full() -> Self {
        Self {
            read: true,
            write: true,
            grant: true,
        }
    }

    /// Read-only permission
    pub fn read_only() -> Self {
        Self {
            read: true,
            write: false,
            grant: false,
        }
    }

    /// Write-only permission
    pub fn write_only() -> Self {
        Self {
            read: false,
            write: true,
            grant: false,
        }
    }

    /// Convert to byte representation
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

    /// Create from byte representation
    pub fn from_byte(b: u8) -> Self {
        Self {
            read: (b & 0x01) != 0,
            write: (b & 0x02) != 0,
            grant: (b & 0x04) != 0,
        }
    }
}

/// Object types that capabilities can reference
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum ObjectType {
    /// IPC endpoint
    Endpoint = 1,
    /// Another process
    Process = 2,
    /// Memory region (for VMM)
    Memory = 3,
    /// IRQ handler
    Irq = 4,
    /// I/O port range
    IoPort = 5,
    /// Console/debug output
    Console = 6,
}

impl ObjectType {
    /// Convert from u8
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            1 => Some(ObjectType::Endpoint),
            2 => Some(ObjectType::Process),
            3 => Some(ObjectType::Memory),
            4 => Some(ObjectType::Irq),
            5 => Some(ObjectType::IoPort),
            6 => Some(ObjectType::Console),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_permissions_byte_roundtrip() {
        let perms = Permissions::full();
        let byte = perms.to_byte();
        let recovered = Permissions::from_byte(byte);
        assert_eq!(perms, recovered);

        let perms = Permissions::read_only();
        let byte = perms.to_byte();
        let recovered = Permissions::from_byte(byte);
        assert_eq!(perms, recovered);
    }

    #[test]
    fn test_object_type_from_u8() {
        assert_eq!(ObjectType::from_u8(1), Some(ObjectType::Endpoint));
        assert_eq!(ObjectType::from_u8(2), Some(ObjectType::Process));
        assert_eq!(ObjectType::from_u8(99), None);
    }
}
