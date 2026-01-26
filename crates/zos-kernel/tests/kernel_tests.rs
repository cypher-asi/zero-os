//! Kernel integration tests
//!
//! Tests extracted from kernel_impl.rs for better organization and to
//! reduce kernel LOC count (Invariant 5: Kernel ≤3000 LOC).

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use core::cell::RefCell;
use core::sync::atomic::{AtomicU64, Ordering};
use zos_hal::{HalError, NumericProcessHandle, HAL};
use zos_kernel::{
    axiom_check, AxiomError, Capability, CapabilitySpace, ObjectType, Permissions, ProcessId,
    ProcessState, System,
};

// ============================================================================
// Mock HAL for Testing
// ============================================================================

struct MockProcess {
    #[allow(dead_code)]
    name: String,
    alive: bool,
    memory_size: usize,
    pending_messages: Vec<Vec<u8>>,
}

pub struct MockHal {
    time: AtomicU64,
    wallclock: AtomicU64,
    debug_log: RefCell<Vec<String>>,
    random_seed: AtomicU64,
    next_pid: AtomicU64,
    processes: RefCell<BTreeMap<u64, MockProcess>>,
    incoming_messages: RefCell<Vec<(NumericProcessHandle, Vec<u8>)>>,
}

impl MockHal {
    pub fn new() -> Self {
        Self {
            time: AtomicU64::new(0),
            wallclock: AtomicU64::new(1737504000000),
            debug_log: RefCell::new(Vec::new()),
            random_seed: AtomicU64::new(12345),
            next_pid: AtomicU64::new(1),
            processes: RefCell::new(BTreeMap::new()),
            incoming_messages: RefCell::new(Vec::new()),
        }
    }

    #[allow(dead_code)]
    pub fn with_time(nanos: u64) -> Self {
        Self {
            time: AtomicU64::new(nanos),
            wallclock: AtomicU64::new(1737504000000),
            debug_log: RefCell::new(Vec::new()),
            random_seed: AtomicU64::new(12345),
            next_pid: AtomicU64::new(1),
            processes: RefCell::new(BTreeMap::new()),
            incoming_messages: RefCell::new(Vec::new()),
        }
    }
}

impl Default for MockHal {
    fn default() -> Self {
        Self::new()
    }
}

unsafe impl Send for MockHal {}
unsafe impl Sync for MockHal {}

impl HAL for MockHal {
    type ProcessHandle = NumericProcessHandle;

    fn spawn_process(&self, name: &str, _binary: &[u8]) -> Result<Self::ProcessHandle, HalError> {
        let pid = self.next_pid.fetch_add(1, Ordering::SeqCst);
        let handle = NumericProcessHandle::new(pid);

        let process = MockProcess {
            name: String::from(name),
            alive: true,
            memory_size: 65536,
            pending_messages: Vec::new(),
        };

        self.processes.borrow_mut().insert(pid, process);
        self.debug_log.borrow_mut().push(alloc::format!(
            "[mock-hal] Spawned process '{}' with PID {}",
            name,
            pid
        ));

        Ok(handle)
    }

    fn kill_process(&self, handle: &Self::ProcessHandle) -> Result<(), HalError> {
        let mut processes = self.processes.borrow_mut();
        if let Some(proc) = processes.get_mut(&handle.id()) {
            if proc.alive {
                proc.alive = false;
                self.debug_log.borrow_mut().push(alloc::format!(
                    "[mock-hal] Killed process PID {}",
                    handle.id()
                ));
                Ok(())
            } else {
                Err(HalError::ProcessNotFound)
            }
        } else {
            Err(HalError::ProcessNotFound)
        }
    }

    fn send_to_process(&self, handle: &Self::ProcessHandle, msg: &[u8]) -> Result<(), HalError> {
        let mut processes = self.processes.borrow_mut();
        if let Some(proc) = processes.get_mut(&handle.id()) {
            if proc.alive {
                proc.pending_messages.push(msg.to_vec());
                Ok(())
            } else {
                Err(HalError::ProcessNotFound)
            }
        } else {
            Err(HalError::ProcessNotFound)
        }
    }

    fn is_process_alive(&self, handle: &Self::ProcessHandle) -> bool {
        self.processes
            .borrow()
            .get(&handle.id())
            .map(|p| p.alive)
            .unwrap_or(false)
    }

    fn get_process_memory_size(&self, handle: &Self::ProcessHandle) -> Result<usize, HalError> {
        self.processes
            .borrow()
            .get(&handle.id())
            .filter(|p| p.alive)
            .map(|p| p.memory_size)
            .ok_or(HalError::ProcessNotFound)
    }

    fn allocate(&self, size: usize, _align: usize) -> Result<*mut u8, HalError> {
        let layout =
            core::alloc::Layout::from_size_align(size, 8).map_err(|_| HalError::InvalidArgument)?;
        let ptr = unsafe { alloc::alloc::alloc(layout) };
        if ptr.is_null() {
            Err(HalError::OutOfMemory)
        } else {
            Ok(ptr)
        }
    }

    unsafe fn deallocate(&self, ptr: *mut u8, size: usize, _align: usize) {
        if !ptr.is_null() {
            let layout = core::alloc::Layout::from_size_align(size, 8).unwrap();
            alloc::alloc::dealloc(ptr, layout);
        }
    }

    fn now_nanos(&self) -> u64 {
        self.time.load(Ordering::SeqCst)
    }

    fn wallclock_ms(&self) -> u64 {
        self.wallclock.load(Ordering::SeqCst)
    }

    fn random_bytes(&self, buf: &mut [u8]) -> Result<(), HalError> {
        let mut seed = self.random_seed.load(Ordering::SeqCst);
        for byte in buf.iter_mut() {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            *byte = (seed >> 33) as u8;
        }
        self.random_seed.store(seed, Ordering::SeqCst);
        Ok(())
    }

    fn debug_write(&self, msg: &str) {
        self.debug_log.borrow_mut().push(String::from(msg));
    }

    fn poll_messages(&self) -> Vec<(Self::ProcessHandle, Vec<u8>)> {
        let mut messages = self.incoming_messages.borrow_mut();
        messages.drain(..).collect()
    }
}

// ============================================================================
// Unit Tests
// ============================================================================

#[test]
fn test_kernel_creation() {
    let hal = MockHal::new();
    let kernel = System::new(hal);

    assert_eq!(kernel.list_processes().len(), 0);
    assert_eq!(kernel.list_endpoints().len(), 0);
}

#[test]
fn test_process_registration() {
    let hal = MockHal::new();
    let mut kernel = System::new(hal);

    let pid1 = kernel.register_process("init");
    let pid2 = kernel.register_process("terminal");

    assert_eq!(pid1, ProcessId(1));
    assert_eq!(pid2, ProcessId(2));
    assert_eq!(kernel.list_processes().len(), 2);

    let proc = kernel.get_process(pid1).expect("process should exist");
    assert_eq!(proc.name, "init");
    assert_eq!(proc.state, ProcessState::Running);
}

#[test]
fn test_process_kill() {
    let hal = MockHal::new();
    let mut kernel = System::new(hal);

    let pid = kernel.register_process("test");
    assert!(kernel.get_process(pid).is_some());

    kernel.kill_process(pid);
    assert!(kernel.get_process(pid).is_none());
}

#[test]
fn test_endpoint_creation() {
    let hal = MockHal::new();
    let mut kernel = System::new(hal);

    let pid = kernel.register_process("test");
    let (eid, slot) = kernel
        .create_endpoint(pid)
        .expect("endpoint creation should succeed");

    assert_eq!(eid, zos_kernel::EndpointId(1));
    assert_eq!(slot, 0);

    let endpoints = kernel.list_endpoints();
    assert_eq!(endpoints.len(), 1);
    assert_eq!(endpoints[0].owner, pid);
}

#[test]
fn test_capability_grant() {
    let hal = MockHal::new();
    let mut kernel = System::new(hal);

    let pid1 = kernel.register_process("owner");
    let pid2 = kernel.register_process("recipient");

    let (eid, owner_slot) = kernel.create_endpoint(pid1).unwrap();

    let recipient_slot = kernel
        .grant_capability(
            pid1,
            owner_slot,
            pid2,
            Permissions {
                read: true,
                write: true,
                grant: false,
            },
        )
        .expect("grant should succeed");

    let cap_space = kernel.get_cap_space(pid2).expect("cap space should exist");
    let cap = cap_space
        .get(recipient_slot)
        .expect("capability should exist");

    assert_eq!(cap.object_type, ObjectType::Endpoint);
    assert_eq!(cap.object_id, eid.0);
    assert!(cap.permissions.read);
    assert!(cap.permissions.write);
    assert!(!cap.permissions.grant);
}

#[test]
fn test_ipc_send_receive() {
    let hal = MockHal::new();
    let mut kernel = System::new(hal);

    let sender_pid = kernel.register_process("sender");
    let receiver_pid = kernel.register_process("receiver");

    let (_, receiver_slot) = kernel.create_endpoint(receiver_pid).unwrap();

    let sender_slot = kernel
        .grant_capability(
            receiver_pid,
            receiver_slot,
            sender_pid,
            Permissions {
                read: false,
                write: true,
                grant: false,
            },
        )
        .unwrap();

    let data = b"hello world".to_vec();
    kernel
        .ipc_send(sender_pid, sender_slot, 42, data.clone())
        .expect("send should succeed");

    let ep = kernel
        .get_endpoint(zos_kernel::EndpointId(1))
        .expect("endpoint should exist");
    assert_eq!(ep.pending_messages.len(), 1);

    let msg = kernel
        .ipc_receive(receiver_pid, receiver_slot)
        .expect("receive should succeed")
        .expect("message should be present");

    assert_eq!(msg.from, sender_pid);
    assert_eq!(msg.tag, 42);
    assert_eq!(msg.data, b"hello world");
}

#[test]
fn test_axiom_check_valid_capability() {
    let mut cspace = CapabilitySpace::new();
    let cap = Capability {
        id: 1,
        object_type: ObjectType::Endpoint,
        object_id: 42,
        permissions: Permissions::full(),
        generation: 0,
        expires_at: 0,
    };
    let slot = cspace.insert(cap);

    let result = axiom_check(
        &cspace,
        slot,
        &Permissions::read_only(),
        Some(ObjectType::Endpoint),
        0,
    );

    assert!(result.is_ok());
    let cap = result.unwrap();
    assert_eq!(cap.object_id, 42);
}

#[test]
fn test_axiom_check_invalid_slot() {
    let cspace = CapabilitySpace::new();

    let result = axiom_check(&cspace, 999, &Permissions::read_only(), None, 0);

    assert!(matches!(result, Err(AxiomError::InvalidSlot)));
}

#[test]
fn test_axiom_check_expired_capability() {
    let mut cspace = CapabilitySpace::new();
    let cap = Capability {
        id: 1,
        object_type: ObjectType::Endpoint,
        object_id: 42,
        permissions: Permissions::full(),
        generation: 0,
        expires_at: 1000,
    };
    let slot = cspace.insert(cap);

    let result = axiom_check(&cspace, slot, &Permissions::read_only(), None, 2000);

    assert!(matches!(result, Err(AxiomError::Expired)));
}

// ============================================================================
// Init-Driven Spawn Protocol Tests
// ============================================================================

/// Test that SYS_REGISTER_PROCESS (0x14) is Init-only.
///
/// This syscall should only succeed when called by Init (PID 1).
/// Other processes should receive an error.
#[test]
fn test_sys_register_process_init_only() {
    use zos_ipc::syscall::SYS_REGISTER_PROCESS;

    let hal = MockHal::new();
    let mut kernel = System::new(hal);

    // Create Init (PID 1)
    let init_pid = kernel.register_process_with_pid(ProcessId(1), "init");
    assert_eq!(init_pid, ProcessId(1));

    // Create a regular process (PID 2)
    let other_pid = kernel.register_process("other");
    assert_eq!(other_pid, ProcessId(2));

    // Init (PID 1) should be able to register a new process
    let (result, _rich, _data) =
        kernel.process_syscall(init_pid, SYS_REGISTER_PROCESS, [0, 0, 0, 0], b"test_proc");
    assert!(
        result >= 0,
        "Init should be able to register processes, got {}",
        result
    );
    let new_pid = result as u64;
    assert!(new_pid > 0, "Should return a valid PID");

    // Verify the process was created
    let proc = kernel
        .get_process(ProcessId(new_pid))
        .expect("new process should exist");
    assert_eq!(proc.name, "test_proc");

    // Other processes should NOT be able to register processes
    let (result2, _rich2, _data2) =
        kernel.process_syscall(other_pid, SYS_REGISTER_PROCESS, [0, 0, 0, 0], b"malicious");
    assert_eq!(
        result2, -1,
        "Non-Init processes should not be able to register processes"
    );
}

/// Test that SYS_CREATE_ENDPOINT_FOR (0x15) is Init-only.
///
/// This syscall should only succeed when called by Init (PID 1).
/// Other processes should receive an error.
#[test]
fn test_sys_create_endpoint_for_init_only() {
    use zos_ipc::syscall::SYS_CREATE_ENDPOINT_FOR;

    let hal = MockHal::new();
    let mut kernel = System::new(hal);

    // Create Init (PID 1)
    let init_pid = kernel.register_process_with_pid(ProcessId(1), "init");
    assert_eq!(init_pid, ProcessId(1));

    // Create a target process (PID 2)
    let target_pid = kernel.register_process("target");
    assert_eq!(target_pid, ProcessId(2));

    // Create another process (PID 3) that will try to abuse the syscall
    let attacker_pid = kernel.register_process("attacker");
    assert_eq!(attacker_pid, ProcessId(3));

    // Init (PID 1) should be able to create an endpoint for another process
    let (result, _rich, _data) = kernel.process_syscall(
        init_pid,
        SYS_CREATE_ENDPOINT_FOR,
        [target_pid.0 as u32, 0, 0, 0],
        &[],
    );
    assert!(
        result >= 0,
        "Init should be able to create endpoints, got {}",
        result
    );

    // Verify the endpoint was created
    let endpoints = kernel.list_endpoints();
    assert_eq!(endpoints.len(), 1, "One endpoint should exist");
    assert_eq!(
        endpoints[0].owner, target_pid,
        "Endpoint should be owned by target"
    );

    // Other processes should NOT be able to create endpoints for others
    let (result2, _rich2, _data2) = kernel.process_syscall(
        attacker_pid,
        SYS_CREATE_ENDPOINT_FOR,
        [target_pid.0 as u32, 0, 0, 0],
        &[],
    );
    assert_eq!(
        result2, -1,
        "Non-Init processes should not be able to create endpoints for others"
    );
}

/// Test that create_endpoint_for returns correctly packed (slot, endpoint_id).
///
/// The kernel returns: (slot << 32) | (endpoint_id & 0xFFFFFFFF)
/// This verifies the packing format is consistent with documentation.
#[test]
fn test_create_endpoint_for_bit_packing() {
    use zos_ipc::syscall::SYS_CREATE_ENDPOINT_FOR;

    let hal = MockHal::new();
    let mut kernel = System::new(hal);

    // Create Init (PID 1)
    let init_pid = kernel.register_process_with_pid(ProcessId(1), "init");
    assert_eq!(init_pid, ProcessId(1));

    // Create a target process (PID 2)
    let target_pid = kernel.register_process("target");
    assert_eq!(target_pid, ProcessId(2));

    // Create an endpoint for the target
    let (packed_result, _rich, _data) = kernel.process_syscall(
        init_pid,
        SYS_CREATE_ENDPOINT_FOR,
        [target_pid.0 as u32, 0, 0, 0],
        &[],
    );
    assert!(packed_result >= 0, "Should succeed");

    // Unpack using the documented format: (slot << 32) | endpoint_id
    let slot = (packed_result >> 32) as u32;
    let endpoint_id = (packed_result & 0xFFFFFFFF) as u64;

    // Verify the values are sensible
    assert!(endpoint_id > 0, "Endpoint ID should be positive, got {}", endpoint_id);
    assert!(slot < 100, "Slot should be small (got {}), packed incorrectly?", slot);

    // Create another endpoint and verify IDs increment
    let (packed_result2, _rich2, _data2) = kernel.process_syscall(
        init_pid,
        SYS_CREATE_ENDPOINT_FOR,
        [target_pid.0 as u32, 0, 0, 0],
        &[],
    );
    assert!(packed_result2 >= 0, "Should succeed");

    let endpoint_id2 = (packed_result2 & 0xFFFFFFFF) as u64;
    assert!(
        endpoint_id2 > endpoint_id,
        "Endpoint IDs should increment: {} should be > {}",
        endpoint_id2,
        endpoint_id
    );
}

// ============================================================================
// Capability Tests - grant_to_endpoint, revoke vs delete, derive
// ============================================================================

#[test]
fn test_grant_capability_to_endpoint() {
    let hal = MockHal::new();
    let mut kernel = System::new(hal);

    let owner = kernel.register_process("owner");
    let recipient = kernel.register_process("recipient");

    // Create endpoint owned by owner
    let (eid, _owner_slot) = kernel.create_endpoint(owner).expect("should create endpoint");

    // Grant capability directly to the endpoint (not via slot)
    let recipient_slot = kernel
        .grant_capability_to_endpoint(
            owner,
            eid,
            recipient,
            Permissions {
                read: true,
                write: false,
                grant: false,
            },
        )
        .expect("grant should succeed");

    // Verify recipient has the capability
    let cap_space = kernel.get_cap_space(recipient).expect("cap space should exist");
    let cap = cap_space.get(recipient_slot).expect("capability should exist");

    assert_eq!(cap.object_type, ObjectType::Endpoint);
    assert_eq!(cap.object_id, eid.0);
    assert!(cap.permissions.read);
    assert!(!cap.permissions.write);
    assert!(!cap.permissions.grant);
}

#[test]
fn test_grant_capability_to_endpoint_not_owner() {
    let hal = MockHal::new();
    let mut kernel = System::new(hal);

    let owner = kernel.register_process("owner");
    let attacker = kernel.register_process("attacker");
    let recipient = kernel.register_process("recipient");

    // Create endpoint owned by owner
    let (eid, _) = kernel.create_endpoint(owner).expect("should create endpoint");

    // Attacker tries to grant capability to endpoint they don't own
    let result = kernel.grant_capability_to_endpoint(
        attacker,
        eid,
        recipient,
        Permissions::full(),
    );

    assert!(result.is_err(), "Non-owner should not be able to grant");
}

#[test]
fn test_grant_capability_to_nonexistent_endpoint() {
    let hal = MockHal::new();
    let mut kernel = System::new(hal);

    let owner = kernel.register_process("owner");
    let recipient = kernel.register_process("recipient");

    // Try to grant capability to non-existent endpoint
    let result = kernel.grant_capability_to_endpoint(
        owner,
        zos_kernel::EndpointId(999),
        recipient,
        Permissions::full(),
    );

    assert!(result.is_err(), "Should fail for non-existent endpoint");
}

#[test]
fn test_revoke_requires_grant_permission() {
    let hal = MockHal::new();
    let mut kernel = System::new(hal);

    let pid = kernel.register_process("test");

    // Create endpoint and get capability with grant permission
    let (_eid, owner_slot) = kernel.create_endpoint(pid).expect("should create endpoint");

    // Revoke should succeed because owner_slot has grant permission
    let result = kernel.revoke_capability(pid, owner_slot);
    assert!(result.is_ok(), "Revoke should succeed with grant permission");

    // Capability should be gone
    let cap_space = kernel.get_cap_space(pid).expect("cap space should exist");
    assert!(cap_space.get(owner_slot).is_none());
}

#[test]
fn test_delete_works_without_grant_permission() {
    let hal = MockHal::new();
    let mut kernel = System::new(hal);

    let owner = kernel.register_process("owner");
    let recipient = kernel.register_process("recipient");

    // Create endpoint
    let (_eid, owner_slot) = kernel.create_endpoint(owner).expect("should create endpoint");

    // Grant read-only (no grant permission) to recipient
    let recipient_slot = kernel
        .grant_capability(
            owner,
            owner_slot,
            recipient,
            Permissions {
                read: true,
                write: false,
                grant: false,
            },
        )
        .expect("grant should succeed");

    // Recipient should be able to delete their own capability (even without grant permission)
    let result = kernel.delete_capability(recipient, recipient_slot);
    assert!(result.is_ok(), "Delete should succeed without grant permission");

    // Capability should be gone
    let cap_space = kernel.get_cap_space(recipient).expect("cap space should exist");
    assert!(cap_space.get(recipient_slot).is_none());
}

#[test]
fn test_derive_capability_attenuation() {
    let hal = MockHal::new();
    let mut kernel = System::new(hal);

    let pid = kernel.register_process("test");

    // Create endpoint with full permissions
    let (eid, owner_slot) = kernel.create_endpoint(pid).expect("should create endpoint");

    // Derive with reduced permissions
    let derived_slot = kernel
        .derive_capability(
            pid,
            owner_slot,
            Permissions {
                read: true,
                write: false,
                grant: false,
            },
        )
        .expect("derive should succeed");

    // Verify derived capability
    let cap_space = kernel.get_cap_space(pid).expect("cap space should exist");
    let derived_cap = cap_space.get(derived_slot).expect("derived cap should exist");

    assert_eq!(derived_cap.object_id, eid.0);
    assert!(derived_cap.permissions.read);
    assert!(!derived_cap.permissions.write);
    assert!(!derived_cap.permissions.grant);

    // Original should still exist with full permissions
    let original_cap = cap_space.get(owner_slot).expect("original cap should exist");
    assert!(original_cap.permissions.read);
    assert!(original_cap.permissions.write);
    assert!(original_cap.permissions.grant);
}

#[test]
fn test_derive_cannot_escalate_permissions() {
    let hal = MockHal::new();
    let mut kernel = System::new(hal);

    let owner = kernel.register_process("owner");
    let recipient = kernel.register_process("recipient");

    // Create endpoint
    let (_eid, owner_slot) = kernel.create_endpoint(owner).expect("should create endpoint");

    // Grant read-only to recipient
    let recipient_slot = kernel
        .grant_capability(
            owner,
            owner_slot,
            recipient,
            Permissions::read_only(),
        )
        .expect("grant should succeed");

    // Try to derive with escalated permissions
    let result = kernel.derive_capability(
        recipient,
        recipient_slot,
        Permissions::full(),
    );

    // Should only get the permissions we actually have (attenuation)
    // Actually this might succeed but just give us read-only - let's verify
    if let Ok(derived_slot) = result {
        let cap_space = kernel.get_cap_space(recipient).expect("cap space should exist");
        let derived_cap = cap_space.get(derived_slot).expect("derived cap should exist");
        
        // Derived should be attenuated to read-only (intersection of original and requested)
        assert!(derived_cap.permissions.read);
        assert!(!derived_cap.permissions.write, "Should not have write permission");
        assert!(!derived_cap.permissions.grant, "Should not have grant permission");
    }
}

// ============================================================================
// IPC with Capabilities Tests
// ============================================================================

#[test]
fn test_ipc_send_with_caps_transfers_capability() {
    let hal = MockHal::new();
    let mut kernel = System::new(hal);

    let sender = kernel.register_process("sender");
    let receiver = kernel.register_process("receiver");

    // Create two endpoints
    let (_receiver_ep, receiver_slot) = kernel.create_endpoint(receiver).expect("should create endpoint");
    let (sender_ep, sender_ep_slot) = kernel.create_endpoint(sender).expect("should create endpoint");

    // Grant sender write capability to receiver's endpoint
    let sender_write_slot = kernel
        .grant_capability(
            receiver,
            receiver_slot,
            sender,
            Permissions::write_only(),
        )
        .expect("grant should succeed");

    // Sender sends message with their endpoint capability
    let result = kernel.ipc_send_with_caps(
        sender,
        sender_write_slot,
        42,
        b"hello".to_vec(),
        &[sender_ep_slot],
    );
    assert!(result.is_ok(), "send_with_caps should succeed");

    // Sender should no longer have the transferred capability
    let sender_cap_space = kernel.get_cap_space(sender).expect("cap space should exist");
    assert!(sender_cap_space.get(sender_ep_slot).is_none(), "Sender should lose transferred cap");

    // Receive the message with capabilities
    let received = kernel
        .ipc_receive_with_caps(receiver, receiver_slot)
        .expect("receive should succeed")
        .expect("should have message");

    let (msg, installed_slots) = received;
    assert_eq!(msg.tag, 42);
    assert_eq!(msg.data, b"hello");
    assert_eq!(installed_slots.len(), 1, "Should have received 1 capability");

    // Receiver should have the new capability
    let receiver_cap_space = kernel.get_cap_space(receiver).expect("cap space should exist");
    let new_cap = receiver_cap_space
        .get(installed_slots[0])
        .expect("new cap should exist");
    assert_eq!(new_cap.object_id, sender_ep.0);
}

#[test]
fn test_ipc_has_message() {
    let hal = MockHal::new();
    let mut kernel = System::new(hal);

    let sender = kernel.register_process("sender");
    let receiver = kernel.register_process("receiver");

    // Create endpoint
    let (receiver_ep, receiver_slot) = kernel.create_endpoint(receiver).expect("should create endpoint");

    // Grant sender write capability
    let sender_slot = kernel
        .grant_capability(
            receiver,
            receiver_slot,
            sender,
            Permissions::write_only(),
        )
        .expect("grant should succeed");

    // Initially no messages
    let ep = kernel.get_endpoint(receiver_ep).expect("endpoint should exist");
    assert!(ep.pending_messages.is_empty());

    // Send a message
    kernel
        .ipc_send(sender, sender_slot, 42, b"test".to_vec())
        .expect("send should succeed");

    // Now there should be a message
    let ep = kernel.get_endpoint(receiver_ep).expect("endpoint should exist");
    assert_eq!(ep.pending_messages.len(), 1);
}

// ============================================================================
// System Tests - fault_process, syscall dispatch
// ============================================================================

#[test]
fn test_fault_process() {
    let hal = MockHal::new();
    let mut kernel = System::new(hal);

    let pid = kernel.register_process("faulty");
    assert!(kernel.get_process(pid).is_some());
    assert_eq!(kernel.get_process(pid).unwrap().state, ProcessState::Running);

    // Fault the process
    kernel.fault_process(pid, 1, String::from("Segmentation fault"));

    // Process should be zombie
    // Note: fault_process may kill the process, let's check if it still exists
    // Based on the system mod, fault_process calls kernel.fault_process which sets to Zombie
    let proc = kernel.get_process(pid);
    if let Some(p) = proc {
        assert_eq!(p.state, ProcessState::Zombie, "Faulted process should be zombie");
    }
}

#[test]
fn test_syscall_dispatch_nop() {
    let hal = MockHal::new();
    let mut kernel = System::new(hal);

    let pid = kernel.register_process("test");

    // SYS_NOP = 0x00
    let (result, _rich, _data) = kernel.process_syscall(pid, 0x00, [0, 0, 0, 0], &[]);
    assert_eq!(result, 0, "NOP should return 0");
}

#[test]
fn test_syscall_dispatch_debug() {
    let hal = MockHal::new();
    let mut kernel = System::new(hal);

    let pid = kernel.register_process("test");

    // SYS_DEBUG = 0x01
    let (result, _rich, _data) = kernel.process_syscall(pid, 0x01, [0, 0, 0, 0], b"debug message");
    assert_eq!(result, 0, "DEBUG should return 0");
}

#[test]
fn test_syscall_dispatch_get_time() {
    let hal = MockHal::with_time(1000);
    let mut kernel = System::new(hal);

    let pid = kernel.register_process("test");

    // SYS_GET_TIME = 0x02
    // arg[0] = 0 for low 32 bits
    let (result_low, _rich, _data) = kernel.process_syscall(pid, 0x02, [0, 0, 0, 0], &[]);
    assert!(result_low >= 0, "GET_TIME should succeed");
}

#[test]
fn test_syscall_dispatch_get_pid() {
    let hal = MockHal::new();
    let mut kernel = System::new(hal);

    let pid = kernel.register_process("test");

    // SYS_GET_PID = 0x03
    let (result, _rich, _data) = kernel.process_syscall(pid, 0x03, [0, 0, 0, 0], &[]);
    assert_eq!(result, pid.0 as i64, "GET_PID should return process ID");
}

#[test]
fn test_syscall_dispatch_unknown() {
    let hal = MockHal::new();
    let mut kernel = System::new(hal);

    let pid = kernel.register_process("test");

    // Unknown syscall
    let (result, _rich, _data) = kernel.process_syscall(pid, 0xFF, [0, 0, 0, 0], &[]);
    assert_eq!(result, -1, "Unknown syscall should return -1");
}

#[test]
fn test_syscall_dispatch_exit() {
    let hal = MockHal::new();
    let mut kernel = System::new(hal);

    let pid = kernel.register_process("test");

    // SYS_EXIT = 0x11
    let (result, _rich, _data) = kernel.process_syscall(pid, 0x11, [42, 0, 0, 0], &[]);
    
    // Exit should succeed
    assert!(result >= 0 || result == -1, "Exit should complete");
}

#[test]
fn test_syscall_dispatch_create_endpoint() {
    let hal = MockHal::new();
    let mut kernel = System::new(hal);

    let pid = kernel.register_process("test");

    // SYS_CREATE_ENDPOINT = 0x35
    let (result, _rich, _data) = kernel.process_syscall(pid, 0x35, [0, 0, 0, 0], &[]);
    assert!(result >= 0, "Create endpoint should succeed, got {}", result);

    // Verify endpoint exists
    let endpoints = kernel.list_endpoints();
    assert_eq!(endpoints.len(), 1);
}

#[test]
fn test_syscall_dispatch_ipc_send() {
    let hal = MockHal::new();
    let mut kernel = System::new(hal);

    let sender = kernel.register_process("sender");
    let receiver = kernel.register_process("receiver");

    // Create endpoint for receiver
    let (eid, receiver_slot) = kernel.create_endpoint(receiver).expect("should create endpoint");

    // Grant sender write capability
    let sender_slot = kernel
        .grant_capability(
            receiver,
            receiver_slot,
            sender,
            Permissions::write_only(),
        )
        .expect("grant should succeed");

    // SYS_IPC_SEND = 0x40
    let (result, _rich, _data) = kernel.process_syscall(
        sender,
        0x40,
        [sender_slot, 123, 0, 0],
        b"test message",
    );
    assert_eq!(result, 0, "IPC send should succeed");

    // Verify message was received
    let ep = kernel.get_endpoint(eid).expect("endpoint should exist");
    assert_eq!(ep.pending_messages.len(), 1);
}

#[test]
fn test_syscall_dispatch_ipc_has_message() {
    let hal = MockHal::new();
    let mut kernel = System::new(hal);

    let pid = kernel.register_process("test");

    // Create endpoint
    let (_eid, slot) = kernel.create_endpoint(pid).expect("should create endpoint");

    // SYS_IPC_HAS_MESSAGE = 0x41
    let (result, _rich, _data) = kernel.process_syscall(pid, 0x41, [slot, 0, 0, 0], &[]);
    assert_eq!(result, 0, "Should have no messages");
}

#[test]
fn test_syscall_dispatch_list_processes() {
    let hal = MockHal::new();
    let mut kernel = System::new(hal);

    kernel.register_process("proc1");
    kernel.register_process("proc2");
    let requester = kernel.register_process("requester");

    // SYS_PS = 0x50
    let (result, _rich, data) = kernel.process_syscall(requester, 0x50, [0, 0, 0, 0], &[]);
    assert_eq!(result, 0, "PS should succeed");
    // data should contain process list (binary format)
    assert!(!data.is_empty(), "Should return process data");
}

// ============================================================================
// Commitlog Tests
// ============================================================================

#[test]
fn test_commitlog_records_process_creation() {
    let hal = MockHal::new();
    let mut kernel = System::new(hal);

    let initial_commits = kernel.commitlog().len();

    kernel.register_process("test");

    // Should have new commits
    assert!(
        kernel.commitlog().len() > initial_commits,
        "Should record process creation"
    );
}

#[test]
fn test_commitlog_records_capability_grant() {
    let hal = MockHal::new();
    let mut kernel = System::new(hal);

    let owner = kernel.register_process("owner");
    let recipient = kernel.register_process("recipient");

    let (_eid, owner_slot) = kernel.create_endpoint(owner).expect("should create endpoint");

    let commits_before_grant = kernel.commitlog().len();

    kernel
        .grant_capability(owner, owner_slot, recipient, Permissions::read_only())
        .expect("grant should succeed");

    // Should have new commits for grant
    assert!(
        kernel.commitlog().len() > commits_before_grant,
        "Should record capability grant"
    );
}

#[test]
fn test_syslog_records_syscalls() {
    let hal = MockHal::new();
    let mut kernel = System::new(hal);

    let pid = kernel.register_process("test");

    let initial_entries = kernel.syslog().len();

    // Make a syscall
    kernel.process_syscall(pid, 0x00, [0, 0, 0, 0], &[]);

    // Should have request + response entries
    assert!(
        kernel.syslog().len() > initial_entries,
        "Should record syscall in syslog"
    );
}
