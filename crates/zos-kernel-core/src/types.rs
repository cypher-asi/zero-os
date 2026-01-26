//! Core kernel types
//!
//! This module contains the fundamental types used throughout the kernel core.
//! All types here are pure data - no behavior that depends on HAL.

use alloc::collections::VecDeque;
use alloc::string::String;
use alloc::vec::Vec;
use serde::{Deserialize, Serialize};

/// Capability slot index
pub type CapSlot = u32;

/// Process identifier
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ProcessId(pub u64);

/// IPC endpoint identifier
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct EndpointId(pub u64);

/// Process state
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProcessState {
    /// Process is running
    Running,
    /// Process is blocked waiting for IPC
    Blocked,
    /// Process has exited
    Zombie,
}

/// Process descriptor
pub struct Process {
    /// Process ID
    pub pid: ProcessId,
    /// Process name
    pub name: String,
    /// Current state
    pub state: ProcessState,
    /// Detailed metrics for this process
    pub metrics: ProcessMetrics,
}

/// Per-process resource tracking
#[derive(Clone, Debug, Default)]
pub struct ProcessMetrics {
    /// Memory size (bytes)
    pub memory_size: usize,
    /// Messages sent
    pub ipc_sent: u64,
    /// Messages received
    pub ipc_received: u64,
    /// Bytes sent via IPC
    pub ipc_bytes_sent: u64,
    /// Bytes received via IPC
    pub ipc_bytes_received: u64,
    /// Syscalls made
    pub syscall_count: u64,
    /// Time of last activity (nanos since boot)
    pub last_active_ns: u64,
    /// Process start time (nanos since boot)
    pub start_time_ns: u64,
}

/// Per-endpoint tracking
#[derive(Clone, Debug, Default)]
pub struct EndpointMetrics {
    /// Messages currently queued
    pub queue_depth: usize,
    /// Total messages ever sent to this endpoint
    pub total_messages: u64,
    /// Total bytes received
    pub total_bytes: u64,
    /// High water mark (max queue depth seen)
    pub queue_high_water: usize,
}

/// System-wide metrics
#[derive(Clone, Debug)]
pub struct SystemMetrics {
    /// Process count
    pub process_count: usize,
    /// Total memory across all processes
    pub total_memory: usize,
    /// Endpoint count
    pub endpoint_count: usize,
    /// Total pending messages
    pub total_pending_messages: usize,
    /// Total IPC messages since boot
    pub total_ipc_messages: u64,
    /// Uptime in nanoseconds
    pub uptime_ns: u64,
}

/// Capability permissions
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

    /// Check if this permission set is a subset of another
    pub fn is_subset_of(&self, other: &Self) -> bool {
        (!self.read || other.read) && (!self.write || other.write) && (!self.grant || other.grant)
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

// ============================================================================
// IPC Types
// ============================================================================

/// Maximum IPC message size in bytes
pub const MAX_MESSAGE_SIZE: usize = 4096;

/// Maximum capabilities that can be transferred in one message
pub const MAX_CAPS_PER_MESSAGE: usize = 4;

/// IPC endpoint
pub struct Endpoint {
    /// Endpoint ID
    pub id: EndpointId,
    /// Owner process
    pub owner: ProcessId,
    /// Queue of pending messages
    pub pending_messages: VecDeque<Message>,
    /// Metrics
    pub metrics: EndpointMetrics,
}

impl Endpoint {
    /// Create a new endpoint
    pub fn new(id: EndpointId, owner: ProcessId) -> Self {
        Self {
            id,
            owner,
            pending_messages: VecDeque::new(),
            metrics: EndpointMetrics::default(),
        }
    }

    /// Enqueue a message
    pub fn enqueue(&mut self, msg: Message) {
        let data_len = msg.data.len() as u64;
        self.pending_messages.push_back(msg);
        self.metrics.queue_depth = self.pending_messages.len();
        self.metrics.total_messages += 1;
        self.metrics.total_bytes += data_len;
        if self.metrics.queue_depth > self.metrics.queue_high_water {
            self.metrics.queue_high_water = self.metrics.queue_depth;
        }
    }

    /// Dequeue a message
    pub fn dequeue(&mut self) -> Option<Message> {
        let msg = self.pending_messages.pop_front();
        self.metrics.queue_depth = self.pending_messages.len();
        msg
    }
}

/// IPC message
#[derive(Clone, Debug)]
pub struct Message {
    /// Sender process ID
    pub sender: ProcessId,
    /// Message tag (application-defined)
    pub tag: u32,
    /// Message payload
    pub data: Vec<u8>,
    /// Transferred capabilities (if any)
    pub caps: Vec<TransferredCap>,
}

/// A capability being transferred via IPC
#[derive(Clone, Debug)]
pub struct TransferredCap {
    /// Original capability ID
    pub cap_id: u64,
    /// Object type
    pub object_type: ObjectType,
    /// Object ID
    pub object_id: u64,
    /// Permissions being transferred
    pub permissions: Permissions,
}

// ============================================================================
// Info types for syscall responses
// ============================================================================

/// Capability info returned by inspect syscall
#[derive(Clone, Debug)]
pub struct CapInfo {
    /// Capability ID
    pub id: u64,
    /// Object type
    pub object_type: u8,
    /// Object ID
    pub object_id: u64,
    /// Permissions byte
    pub permissions: u8,
    /// Generation
    pub generation: u32,
    /// Expiration timestamp
    pub expires_at: u64,
}

/// Endpoint info for listing
#[derive(Clone, Debug)]
pub struct EndpointInfo {
    /// Endpoint ID
    pub id: EndpointId,
    /// Owner PID
    pub owner: ProcessId,
    /// Queue depth
    pub queue_depth: usize,
}

/// Detailed endpoint info
#[derive(Clone, Debug)]
pub struct EndpointDetail {
    /// Endpoint ID
    pub id: EndpointId,
    /// Owner PID
    pub owner: ProcessId,
    /// Pending messages (summaries)
    pub pending_messages: Vec<MessageSummary>,
    /// Metrics
    pub metrics: EndpointMetrics,
}

/// Message summary for listing
#[derive(Clone, Debug)]
pub struct MessageSummary {
    /// Sender PID
    pub sender: ProcessId,
    /// Message tag
    pub tag: u32,
    /// Data size
    pub data_size: usize,
    /// Number of capabilities
    pub cap_count: usize,
}

/// Revoke notification data
#[derive(Clone, Debug)]
pub struct RevokeNotification {
    /// Process whose cap was revoked
    pub pid: ProcessId,
    /// Slot that was revoked
    pub slot: CapSlot,
    /// Object type
    pub object_type: u8,
    /// Object ID
    pub object_id: u64,
    /// Reason code
    pub reason: u8,
}

impl RevokeNotification {
    /// Create an empty notification
    pub fn empty() -> Self {
        Self {
            pid: ProcessId(0),
            slot: 0,
            object_type: 0,
            object_id: 0,
            reason: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    // ========================================================================
    // Permissions tests
    // ========================================================================

    #[test]
    fn test_permissions_to_byte_from_byte_roundtrip() {
        // Test all 8 combinations of permissions
        let combinations = [
            Permissions { read: false, write: false, grant: false },
            Permissions { read: true, write: false, grant: false },
            Permissions { read: false, write: true, grant: false },
            Permissions { read: true, write: true, grant: false },
            Permissions { read: false, write: false, grant: true },
            Permissions { read: true, write: false, grant: true },
            Permissions { read: false, write: true, grant: true },
            Permissions { read: true, write: true, grant: true },
        ];

        for perms in &combinations {
            let byte = perms.to_byte();
            let restored = Permissions::from_byte(byte);
            assert_eq!(perms.read, restored.read, "read mismatch for {:?}", perms);
            assert_eq!(perms.write, restored.write, "write mismatch for {:?}", perms);
            assert_eq!(perms.grant, restored.grant, "grant mismatch for {:?}", perms);
        }
    }

    #[test]
    fn test_permissions_to_byte_values() {
        assert_eq!(Permissions { read: false, write: false, grant: false }.to_byte(), 0x00);
        assert_eq!(Permissions { read: true, write: false, grant: false }.to_byte(), 0x01);
        assert_eq!(Permissions { read: false, write: true, grant: false }.to_byte(), 0x02);
        assert_eq!(Permissions { read: true, write: true, grant: false }.to_byte(), 0x03);
        assert_eq!(Permissions { read: false, write: false, grant: true }.to_byte(), 0x04);
        assert_eq!(Permissions { read: true, write: false, grant: true }.to_byte(), 0x05);
        assert_eq!(Permissions { read: false, write: true, grant: true }.to_byte(), 0x06);
        assert_eq!(Permissions { read: true, write: true, grant: true }.to_byte(), 0x07);
    }

    #[test]
    fn test_permissions_from_byte_ignores_high_bits() {
        // Only bits 0-2 should matter
        let p1 = Permissions::from_byte(0x07); // All permissions
        let p2 = Permissions::from_byte(0xFF); // All bits set

        // Both should have the same permissions
        assert_eq!(p1.read, p2.read);
        assert_eq!(p1.write, p2.write);
        assert_eq!(p1.grant, p2.grant);
    }

    #[test]
    fn test_permissions_full() {
        let full = Permissions::full();
        assert!(full.read);
        assert!(full.write);
        assert!(full.grant);
        assert_eq!(full.to_byte(), 0x07);
    }

    #[test]
    fn test_permissions_read_only() {
        let read = Permissions::read_only();
        assert!(read.read);
        assert!(!read.write);
        assert!(!read.grant);
        assert_eq!(read.to_byte(), 0x01);
    }

    #[test]
    fn test_permissions_write_only() {
        let write = Permissions::write_only();
        assert!(!write.read);
        assert!(write.write);
        assert!(!write.grant);
        assert_eq!(write.to_byte(), 0x02);
    }

    #[test]
    fn test_permissions_default() {
        let default = Permissions::default();
        assert!(!default.read);
        assert!(!default.write);
        assert!(!default.grant);
        assert_eq!(default.to_byte(), 0x00);
    }

    #[test]
    fn test_permissions_is_subset_of() {
        let full = Permissions::full();
        let read = Permissions::read_only();
        let write = Permissions::write_only();
        let none = Permissions::default();

        // Everything is subset of full
        assert!(full.is_subset_of(&full));
        assert!(read.is_subset_of(&full));
        assert!(write.is_subset_of(&full));
        assert!(none.is_subset_of(&full));

        // None is subset of everything
        assert!(none.is_subset_of(&read));
        assert!(none.is_subset_of(&write));
        assert!(none.is_subset_of(&none));

        // Full is not subset of partial
        assert!(!full.is_subset_of(&read));
        assert!(!full.is_subset_of(&write));
        assert!(!full.is_subset_of(&none));

        // Read is not subset of write (no overlap)
        assert!(!read.is_subset_of(&write));
        assert!(!write.is_subset_of(&read));
    }

    // ========================================================================
    // ObjectType tests
    // ========================================================================

    #[test]
    fn test_object_type_from_u8_all_variants() {
        assert_eq!(ObjectType::from_u8(1), Some(ObjectType::Endpoint));
        assert_eq!(ObjectType::from_u8(2), Some(ObjectType::Process));
        assert_eq!(ObjectType::from_u8(3), Some(ObjectType::Memory));
        assert_eq!(ObjectType::from_u8(4), Some(ObjectType::Irq));
        assert_eq!(ObjectType::from_u8(5), Some(ObjectType::IoPort));
        assert_eq!(ObjectType::from_u8(6), Some(ObjectType::Console));
    }

    #[test]
    fn test_object_type_from_u8_invalid_values() {
        assert_eq!(ObjectType::from_u8(0), None);
        assert_eq!(ObjectType::from_u8(7), None);
        assert_eq!(ObjectType::from_u8(100), None);
        assert_eq!(ObjectType::from_u8(255), None);
    }

    #[test]
    fn test_object_type_repr_values() {
        // Verify repr(u8) values match from_u8 expectations
        assert_eq!(ObjectType::Endpoint as u8, 1);
        assert_eq!(ObjectType::Process as u8, 2);
        assert_eq!(ObjectType::Memory as u8, 3);
        assert_eq!(ObjectType::Irq as u8, 4);
        assert_eq!(ObjectType::IoPort as u8, 5);
        assert_eq!(ObjectType::Console as u8, 6);
    }

    // ========================================================================
    // Endpoint metrics tests
    // ========================================================================

    #[test]
    fn test_endpoint_queue_high_water() {
        let mut endpoint = Endpoint::new(EndpointId(1), ProcessId(1));

        // Initially 0
        assert_eq!(endpoint.metrics.queue_high_water, 0);

        // Enqueue a message
        endpoint.enqueue(Message {
            sender: ProcessId(1),
            tag: 0,
            data: vec![],
            caps: vec![],
        });
        assert_eq!(endpoint.metrics.queue_depth, 1);
        assert_eq!(endpoint.metrics.queue_high_water, 1);

        // Enqueue another
        endpoint.enqueue(Message {
            sender: ProcessId(1),
            tag: 0,
            data: vec![],
            caps: vec![],
        });
        assert_eq!(endpoint.metrics.queue_depth, 2);
        assert_eq!(endpoint.metrics.queue_high_water, 2);

        // Dequeue one
        endpoint.dequeue();
        assert_eq!(endpoint.metrics.queue_depth, 1);
        assert_eq!(endpoint.metrics.queue_high_water, 2); // High water unchanged

        // Dequeue all
        endpoint.dequeue();
        assert_eq!(endpoint.metrics.queue_depth, 0);
        assert_eq!(endpoint.metrics.queue_high_water, 2); // Still 2
    }

    #[test]
    fn test_endpoint_metrics_total_messages_and_bytes() {
        let mut endpoint = Endpoint::new(EndpointId(1), ProcessId(1));

        assert_eq!(endpoint.metrics.total_messages, 0);
        assert_eq!(endpoint.metrics.total_bytes, 0);

        // Enqueue message with 10 bytes
        endpoint.enqueue(Message {
            sender: ProcessId(1),
            tag: 0,
            data: vec![0u8; 10],
            caps: vec![],
        });
        assert_eq!(endpoint.metrics.total_messages, 1);
        assert_eq!(endpoint.metrics.total_bytes, 10);

        // Enqueue message with 5 bytes
        endpoint.enqueue(Message {
            sender: ProcessId(1),
            tag: 0,
            data: vec![0u8; 5],
            caps: vec![],
        });
        assert_eq!(endpoint.metrics.total_messages, 2);
        assert_eq!(endpoint.metrics.total_bytes, 15);

        // Dequeue doesn't change total counts
        endpoint.dequeue();
        assert_eq!(endpoint.metrics.total_messages, 2);
        assert_eq!(endpoint.metrics.total_bytes, 15);
    }

    #[test]
    fn test_endpoint_enqueue_dequeue() {
        let mut endpoint = Endpoint::new(EndpointId(1), ProcessId(1));

        assert!(endpoint.pending_messages.is_empty());

        let msg1 = Message {
            sender: ProcessId(2),
            tag: 100,
            data: vec![1, 2, 3],
            caps: vec![],
        };
        let msg2 = Message {
            sender: ProcessId(3),
            tag: 200,
            data: vec![4, 5],
            caps: vec![],
        };

        endpoint.enqueue(msg1);
        endpoint.enqueue(msg2);

        assert_eq!(endpoint.pending_messages.len(), 2);

        // FIFO order
        let deq1 = endpoint.dequeue().unwrap();
        assert_eq!(deq1.tag, 100);
        assert_eq!(deq1.sender, ProcessId(2));

        let deq2 = endpoint.dequeue().unwrap();
        assert_eq!(deq2.tag, 200);
        assert_eq!(deq2.sender, ProcessId(3));

        // Empty
        assert!(endpoint.dequeue().is_none());
    }

    // ========================================================================
    // RevokeNotification tests
    // ========================================================================

    #[test]
    fn test_revoke_notification_empty() {
        let notification = RevokeNotification::empty();

        assert_eq!(notification.pid, ProcessId(0));
        assert_eq!(notification.slot, 0);
        assert_eq!(notification.object_type, 0);
        assert_eq!(notification.object_id, 0);
        assert_eq!(notification.reason, 0);
    }

    // ========================================================================
    // ProcessState tests
    // ========================================================================

    #[test]
    fn test_process_state_equality() {
        assert_eq!(ProcessState::Running, ProcessState::Running);
        assert_eq!(ProcessState::Blocked, ProcessState::Blocked);
        assert_eq!(ProcessState::Zombie, ProcessState::Zombie);

        assert_ne!(ProcessState::Running, ProcessState::Blocked);
        assert_ne!(ProcessState::Running, ProcessState::Zombie);
        assert_ne!(ProcessState::Blocked, ProcessState::Zombie);
    }

    // ========================================================================
    // ProcessId and EndpointId tests
    // ========================================================================

    #[test]
    fn test_process_id_ordering() {
        let p1 = ProcessId(1);
        let p2 = ProcessId(2);
        let p3 = ProcessId(2);

        assert!(p1 < p2);
        assert!(p2 > p1);
        assert_eq!(p2, p3);
    }

    #[test]
    fn test_endpoint_id_ordering() {
        let e1 = EndpointId(1);
        let e2 = EndpointId(2);
        let e3 = EndpointId(2);

        assert!(e1 < e2);
        assert!(e2 > e1);
        assert_eq!(e2, e3);
    }

    // ========================================================================
    // Constants tests
    // ========================================================================

    #[test]
    fn test_constants() {
        // These are sanity checks - the values are documented and depended upon
        assert!(MAX_MESSAGE_SIZE >= 1024, "MAX_MESSAGE_SIZE should be at least 1KB");
        assert!(MAX_CAPS_PER_MESSAGE >= 1, "Should allow at least 1 cap per message");
        assert!(MAX_CAPS_PER_MESSAGE <= 16, "Cap limit should be reasonable");
    }
}
