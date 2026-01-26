//! Deterministic replay implementation for the System.
//!
//! This module implements the `Replayable` trait, allowing system state to be
//! reconstructed from a commit log for auditing and verification purposes.

use alloc::collections::VecDeque;
use alloc::string::String;

use crate::ipc::Endpoint;
use crate::system::System;
use crate::types::{
    EndpointId, EndpointMetrics, ObjectType, Process, ProcessId, ProcessMetrics, ProcessState,
};
use crate::{Capability, CapabilitySpace, Permissions};
use zos_axiom::{ReplayError, ReplayResult, Replayable, StateHasher};
use zos_hal::HAL;

impl<H: HAL> Replayable for System<H> {
    fn replay_genesis(&mut self) -> ReplayResult<()> {
        Ok(())
    }

    fn replay_create_process(&mut self, pid: u64, _parent: u64, name: String) -> ReplayResult<()> {
        let process = Process {
            pid: ProcessId(pid),
            name,
            state: ProcessState::Running,
            metrics: ProcessMetrics::default(),
        };
        self.kernel.processes.insert(ProcessId(pid), process);
        self.kernel
            .cap_spaces
            .insert(ProcessId(pid), CapabilitySpace::new());

        // Update next_pid to avoid collisions
        if pid >= self.kernel.next_pid {
            self.kernel.next_pid = pid + 1;
        }

        Ok(())
    }

    fn replay_exit_process(&mut self, pid: u64, _code: i32) -> ReplayResult<()> {
        let process = self
            .kernel
            .processes
            .get_mut(&ProcessId(pid))
            .ok_or(ReplayError::ProcessNotFound(pid))?;
        process.state = ProcessState::Zombie;
        Ok(())
    }

    fn replay_process_faulted(
        &mut self,
        pid: u64,
        _reason: u32,
        _description: String,
    ) -> ReplayResult<()> {
        let process = self
            .kernel
            .processes
            .get_mut(&ProcessId(pid))
            .ok_or(ReplayError::ProcessNotFound(pid))?;
        process.state = ProcessState::Zombie;
        Ok(())
    }

    fn replay_insert_capability(
        &mut self,
        pid: u64,
        slot: u32,
        cap_id: u64,
        object_type: u8,
        object_id: u64,
        perms: u8,
    ) -> ReplayResult<()> {
        let obj_type = map_object_type(object_type)?;

        let cap = Capability {
            id: cap_id,
            object_type: obj_type,
            object_id,
            permissions: Permissions::from_byte(perms),
            generation: 0,
            expires_at: 0,
        };

        let cspace = self
            .kernel
            .cap_spaces
            .get_mut(&ProcessId(pid))
            .ok_or(ReplayError::ProcessNotFound(pid))?;

        cspace.slots.insert(slot, cap);

        // Update next_slot to avoid collisions
        if slot >= cspace.next_slot {
            cspace.next_slot = slot + 1;
        }

        // Update next_cap_id to avoid collisions
        if cap_id >= self.kernel.next_cap_id {
            self.kernel.next_cap_id = cap_id + 1;
        }

        Ok(())
    }

    fn replay_remove_capability(&mut self, pid: u64, slot: u32) -> ReplayResult<()> {
        let cspace = self
            .kernel
            .cap_spaces
            .get_mut(&ProcessId(pid))
            .ok_or(ReplayError::ProcessNotFound(pid))?;
        cspace.slots.remove(&slot);
        Ok(())
    }

    fn replay_cap_granted(
        &mut self,
        _from_pid: u64,
        _to_pid: u64,
        _from_slot: u32,
        _to_slot: u32,
        new_cap_id: u64,
        _perms: zos_axiom::Permissions,
    ) -> ReplayResult<()> {
        // Just update next_cap_id (actual insertion handled by CapInserted)
        if new_cap_id >= self.kernel.next_cap_id {
            self.kernel.next_cap_id = new_cap_id + 1;
        }
        Ok(())
    }

    fn replay_create_endpoint(&mut self, id: u64, owner: u64) -> ReplayResult<()> {
        if !self.kernel.processes.contains_key(&ProcessId(owner)) {
            return Err(ReplayError::ProcessNotFound(owner));
        }

        let endpoint = Endpoint {
            id: EndpointId(id),
            owner: ProcessId(owner),
            pending_messages: VecDeque::new(),
            metrics: EndpointMetrics::default(),
        };
        self.kernel.endpoints.insert(EndpointId(id), endpoint);

        // Update next_endpoint_id to avoid collisions
        if id >= self.kernel.next_endpoint_id {
            self.kernel.next_endpoint_id = id + 1;
        }

        Ok(())
    }

    fn replay_destroy_endpoint(&mut self, id: u64) -> ReplayResult<()> {
        self.kernel.endpoints.remove(&EndpointId(id));
        Ok(())
    }

    fn replay_message_sent(
        &mut self,
        _from_pid: u64,
        _to_endpoint: u64,
        _tag: u32,
        _size: usize,
    ) -> ReplayResult<()> {
        // Messages are transient and not replayed into state
        Ok(())
    }

    fn state_hash(&self) -> [u8; 32] {
        let mut hasher = StateHasher::new();

        // Hash process table
        hasher.write_u64(self.kernel.processes.len() as u64);
        for (pid, proc) in &self.kernel.processes {
            hasher.write_u64(pid.0);
            hasher.write_str(&proc.name);
            hasher.write_u8(process_state_to_u8(proc.state));
        }

        // Hash capability spaces
        hasher.write_u64(self.kernel.cap_spaces.len() as u64);
        for (pid, cspace) in &self.kernel.cap_spaces {
            hasher.write_u64(pid.0);
            hasher.write_u64(cspace.slots.len() as u64);
            for (slot, cap) in &cspace.slots {
                hasher.write_u32(*slot);
                hasher.write_u64(cap.id);
                hasher.write_u8(cap.object_type as u8);
                hasher.write_u64(cap.object_id);
                hasher.write_u8(cap.permissions.to_byte());
                hasher.write_u32(cap.generation);
                hasher.write_u64(cap.expires_at);
            }
        }

        // Hash endpoints
        hasher.write_u64(self.kernel.endpoints.len() as u64);
        for (id, ep) in &self.kernel.endpoints {
            hasher.write_u64(id.0);
            hasher.write_u64(ep.owner.0);
        }

        hasher.finalize()
    }
}

/// Map object type byte to ObjectType enum
fn map_object_type(object_type: u8) -> ReplayResult<ObjectType> {
    match object_type {
        1 => Ok(ObjectType::Endpoint),
        2 => Ok(ObjectType::Process),
        3 => Ok(ObjectType::Memory),
        4 => Ok(ObjectType::Irq),
        5 => Ok(ObjectType::IoPort),
        6 => Ok(ObjectType::Console),
        _ => Err(ReplayError::UnknownObjectType(object_type)),
    }
}

/// Convert ProcessState to u8 for hashing
fn process_state_to_u8(state: ProcessState) -> u8 {
    match state {
        ProcessState::Running => 0,
        ProcessState::Blocked => 1,
        ProcessState::Zombie => 2,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use zos_axiom::Replayable;
    use zos_hal::TestHal;

    // ========================================================================
    // replay_genesis tests
    // ========================================================================

    #[test]
    fn test_replay_genesis() {
        let mut system: System<TestHal> = System::new_for_replay();
        
        let result = system.replay_genesis();
        assert!(result.is_ok(), "Genesis should succeed");
    }

    // ========================================================================
    // replay_create_process tests
    // ========================================================================

    #[test]
    fn test_replay_create_process() {
        let mut system: System<TestHal> = System::new_for_replay();

        let result = system.replay_create_process(1, 0, String::from("init"));
        assert!(result.is_ok(), "Create process should succeed");

        // Verify process was created
        let proc = system.kernel.processes.get(&ProcessId(1));
        assert!(proc.is_some());
        assert_eq!(proc.unwrap().name, "init");
        assert_eq!(proc.unwrap().state, ProcessState::Running);

        // Verify cap space was created
        assert!(system.kernel.cap_spaces.contains_key(&ProcessId(1)));

        // Verify next_pid was updated
        assert_eq!(system.kernel.next_pid, 2);
    }

    #[test]
    fn test_replay_create_process_updates_next_pid() {
        let mut system: System<TestHal> = System::new_for_replay();

        // Create process with high PID
        system.replay_create_process(100, 0, String::from("high_pid")).unwrap();

        // next_pid should be updated to avoid collision
        assert_eq!(system.kernel.next_pid, 101);

        // Create process with lower PID
        system.replay_create_process(50, 0, String::from("low_pid")).unwrap();

        // next_pid should NOT decrease
        assert_eq!(system.kernel.next_pid, 101);
    }

    // ========================================================================
    // replay_exit_process tests
    // ========================================================================

    #[test]
    fn test_replay_exit_process() {
        let mut system: System<TestHal> = System::new_for_replay();

        system.replay_create_process(1, 0, String::from("test")).unwrap();

        let result = system.replay_exit_process(1, 0);
        assert!(result.is_ok());

        let proc = system.kernel.processes.get(&ProcessId(1)).unwrap();
        assert_eq!(proc.state, ProcessState::Zombie);
    }

    #[test]
    fn test_replay_exit_process_not_found() {
        let mut system: System<TestHal> = System::new_for_replay();

        let result = system.replay_exit_process(999, 0);
        assert!(result.is_err());
        assert!(matches!(result, Err(ReplayError::ProcessNotFound(999))));
    }

    // ========================================================================
    // replay_process_faulted tests
    // ========================================================================

    #[test]
    fn test_replay_process_faulted() {
        let mut system: System<TestHal> = System::new_for_replay();

        system.replay_create_process(1, 0, String::from("test")).unwrap();

        let result = system.replay_process_faulted(1, 1, String::from("segfault"));
        assert!(result.is_ok());

        let proc = system.kernel.processes.get(&ProcessId(1)).unwrap();
        assert_eq!(proc.state, ProcessState::Zombie);
    }

    #[test]
    fn test_replay_process_faulted_not_found() {
        let mut system: System<TestHal> = System::new_for_replay();

        let result = system.replay_process_faulted(999, 1, String::from("error"));
        assert!(result.is_err());
        assert!(matches!(result, Err(ReplayError::ProcessNotFound(999))));
    }

    // ========================================================================
    // replay_insert_capability tests
    // ========================================================================

    #[test]
    fn test_replay_insert_capability() {
        let mut system: System<TestHal> = System::new_for_replay();

        system.replay_create_process(1, 0, String::from("test")).unwrap();

        // Insert capability: endpoint type (1), read permission (1)
        let result = system.replay_insert_capability(1, 0, 100, 1, 42, 0x01);
        assert!(result.is_ok());

        let cspace = system.kernel.cap_spaces.get(&ProcessId(1)).unwrap();
        let cap = cspace.slots.get(&0).unwrap();

        assert_eq!(cap.id, 100);
        assert_eq!(cap.object_type, ObjectType::Endpoint);
        assert_eq!(cap.object_id, 42);
        assert!(cap.permissions.read);
        assert!(!cap.permissions.write);
        assert!(!cap.permissions.grant);
    }

    #[test]
    fn test_replay_insert_capability_updates_next_slot() {
        let mut system: System<TestHal> = System::new_for_replay();

        system.replay_create_process(1, 0, String::from("test")).unwrap();

        // Insert at slot 10
        system.replay_insert_capability(1, 10, 100, 1, 42, 0x01).unwrap();

        let cspace = system.kernel.cap_spaces.get(&ProcessId(1)).unwrap();
        assert_eq!(cspace.next_slot, 11);
    }

    #[test]
    fn test_replay_insert_capability_updates_next_cap_id() {
        let mut system: System<TestHal> = System::new_for_replay();

        system.replay_create_process(1, 0, String::from("test")).unwrap();

        // Insert cap with ID 50
        system.replay_insert_capability(1, 0, 50, 1, 42, 0x01).unwrap();

        assert_eq!(system.kernel.next_cap_id, 51);
    }

    #[test]
    fn test_replay_insert_capability_process_not_found() {
        let mut system: System<TestHal> = System::new_for_replay();

        let result = system.replay_insert_capability(999, 0, 100, 1, 42, 0x01);
        assert!(result.is_err());
        assert!(matches!(result, Err(ReplayError::ProcessNotFound(999))));
    }

    #[test]
    fn test_replay_insert_capability_unknown_object_type() {
        let mut system: System<TestHal> = System::new_for_replay();

        system.replay_create_process(1, 0, String::from("test")).unwrap();

        // Unknown object type (99)
        let result = system.replay_insert_capability(1, 0, 100, 99, 42, 0x01);
        assert!(result.is_err());
        assert!(matches!(result, Err(ReplayError::UnknownObjectType(99))));
    }

    // ========================================================================
    // replay_remove_capability tests
    // ========================================================================

    #[test]
    fn test_replay_remove_capability() {
        let mut system: System<TestHal> = System::new_for_replay();

        system.replay_create_process(1, 0, String::from("test")).unwrap();
        system.replay_insert_capability(1, 0, 100, 1, 42, 0x01).unwrap();

        let result = system.replay_remove_capability(1, 0);
        assert!(result.is_ok());

        let cspace = system.kernel.cap_spaces.get(&ProcessId(1)).unwrap();
        assert!(!cspace.slots.contains_key(&0));
    }

    #[test]
    fn test_replay_remove_capability_process_not_found() {
        let mut system: System<TestHal> = System::new_for_replay();

        let result = system.replay_remove_capability(999, 0);
        assert!(result.is_err());
        assert!(matches!(result, Err(ReplayError::ProcessNotFound(999))));
    }

    // ========================================================================
    // replay_cap_granted tests
    // ========================================================================

    #[test]
    fn test_replay_cap_granted() {
        let mut system: System<TestHal> = System::new_for_replay();

        system.replay_create_process(1, 0, String::from("from")).unwrap();
        system.replay_create_process(2, 0, String::from("to")).unwrap();

        let perms = zos_axiom::Permissions {
            read: true,
            write: false,
            grant: false,
        };

        // replay_cap_granted just updates next_cap_id
        let result = system.replay_cap_granted(1, 2, 0, 0, 100, perms);
        assert!(result.is_ok());

        assert_eq!(system.kernel.next_cap_id, 101);
    }

    // ========================================================================
    // replay_create_endpoint tests
    // ========================================================================

    #[test]
    fn test_replay_create_endpoint() {
        let mut system: System<TestHal> = System::new_for_replay();

        system.replay_create_process(1, 0, String::from("test")).unwrap();

        let result = system.replay_create_endpoint(1, 1);
        assert!(result.is_ok());

        let ep = system.kernel.endpoints.get(&EndpointId(1));
        assert!(ep.is_some());
        assert_eq!(ep.unwrap().owner, ProcessId(1));
    }

    #[test]
    fn test_replay_create_endpoint_updates_next_endpoint_id() {
        let mut system: System<TestHal> = System::new_for_replay();

        system.replay_create_process(1, 0, String::from("test")).unwrap();

        system.replay_create_endpoint(50, 1).unwrap();

        assert_eq!(system.kernel.next_endpoint_id, 51);
    }

    #[test]
    fn test_replay_create_endpoint_process_not_found() {
        let mut system: System<TestHal> = System::new_for_replay();

        let result = system.replay_create_endpoint(1, 999);
        assert!(result.is_err());
        assert!(matches!(result, Err(ReplayError::ProcessNotFound(999))));
    }

    // ========================================================================
    // replay_destroy_endpoint tests
    // ========================================================================

    #[test]
    fn test_replay_destroy_endpoint() {
        let mut system: System<TestHal> = System::new_for_replay();

        system.replay_create_process(1, 0, String::from("test")).unwrap();
        system.replay_create_endpoint(1, 1).unwrap();

        assert!(system.kernel.endpoints.contains_key(&EndpointId(1)));

        let result = system.replay_destroy_endpoint(1);
        assert!(result.is_ok());

        assert!(!system.kernel.endpoints.contains_key(&EndpointId(1)));
    }

    // ========================================================================
    // replay_message_sent tests
    // ========================================================================

    #[test]
    fn test_replay_message_sent() {
        let mut system: System<TestHal> = System::new_for_replay();

        // Messages are transient and not replayed - should just return Ok
        let result = system.replay_message_sent(1, 1, 42, 100);
        assert!(result.is_ok());
    }

    // ========================================================================
    // state_hash tests
    // ========================================================================

    #[test]
    fn test_state_hash_determinism() {
        // Create two identical systems
        let mut system1: System<TestHal> = System::new_for_replay();
        let mut system2: System<TestHal> = System::new_for_replay();

        // Apply same operations to both
        system1.replay_create_process(1, 0, String::from("proc1")).unwrap();
        system1.replay_create_process(2, 0, String::from("proc2")).unwrap();
        system1.replay_create_endpoint(1, 1).unwrap();
        system1.replay_insert_capability(1, 0, 100, 1, 1, 0x07).unwrap();

        system2.replay_create_process(1, 0, String::from("proc1")).unwrap();
        system2.replay_create_process(2, 0, String::from("proc2")).unwrap();
        system2.replay_create_endpoint(1, 1).unwrap();
        system2.replay_insert_capability(1, 0, 100, 1, 1, 0x07).unwrap();

        // Hashes should be identical
        let hash1 = system1.state_hash();
        let hash2 = system2.state_hash();

        assert_eq!(hash1, hash2, "Same state should produce same hash");
    }

    #[test]
    fn test_state_hash_different_states() {
        let mut system1: System<TestHal> = System::new_for_replay();
        let mut system2: System<TestHal> = System::new_for_replay();

        system1.replay_create_process(1, 0, String::from("proc1")).unwrap();
        system2.replay_create_process(1, 0, String::from("different_name")).unwrap();

        let hash1 = system1.state_hash();
        let hash2 = system2.state_hash();

        assert_ne!(hash1, hash2, "Different states should produce different hashes");
    }

    #[test]
    fn test_state_hash_empty_state() {
        let system: System<TestHal> = System::new_for_replay();
        let hash = system.state_hash();

        // Should produce a valid hash (non-zero typically)
        // The exact value doesn't matter, just that it's deterministic
        let system2: System<TestHal> = System::new_for_replay();
        let hash2 = system2.state_hash();

        assert_eq!(hash, hash2, "Empty states should produce same hash");
    }

    #[test]
    fn test_state_hash_includes_process_state() {
        let mut system1: System<TestHal> = System::new_for_replay();
        let mut system2: System<TestHal> = System::new_for_replay();

        system1.replay_create_process(1, 0, String::from("test")).unwrap();
        system2.replay_create_process(1, 0, String::from("test")).unwrap();

        // Exit process in system1
        system1.replay_exit_process(1, 0).unwrap();

        let hash1 = system1.state_hash();
        let hash2 = system2.state_hash();

        assert_ne!(hash1, hash2, "Process state change should affect hash");
    }

    #[test]
    fn test_state_hash_includes_capabilities() {
        let mut system1: System<TestHal> = System::new_for_replay();
        let mut system2: System<TestHal> = System::new_for_replay();

        system1.replay_create_process(1, 0, String::from("test")).unwrap();
        system2.replay_create_process(1, 0, String::from("test")).unwrap();

        // Add capability only to system1
        system1.replay_insert_capability(1, 0, 100, 1, 42, 0x07).unwrap();

        let hash1 = system1.state_hash();
        let hash2 = system2.state_hash();

        assert_ne!(hash1, hash2, "Capability changes should affect hash");
    }

    #[test]
    fn test_state_hash_includes_endpoints() {
        let mut system1: System<TestHal> = System::new_for_replay();
        let mut system2: System<TestHal> = System::new_for_replay();

        system1.replay_create_process(1, 0, String::from("test")).unwrap();
        system2.replay_create_process(1, 0, String::from("test")).unwrap();

        // Add endpoint only to system1
        system1.replay_create_endpoint(1, 1).unwrap();

        let hash1 = system1.state_hash();
        let hash2 = system2.state_hash();

        assert_ne!(hash1, hash2, "Endpoint changes should affect hash");
    }

    // ========================================================================
    // map_object_type tests
    // ========================================================================

    #[test]
    fn test_map_object_type_all_types() {
        assert_eq!(map_object_type(1).unwrap(), ObjectType::Endpoint);
        assert_eq!(map_object_type(2).unwrap(), ObjectType::Process);
        assert_eq!(map_object_type(3).unwrap(), ObjectType::Memory);
        assert_eq!(map_object_type(4).unwrap(), ObjectType::Irq);
        assert_eq!(map_object_type(5).unwrap(), ObjectType::IoPort);
        assert_eq!(map_object_type(6).unwrap(), ObjectType::Console);
    }

    #[test]
    fn test_map_object_type_invalid() {
        assert!(matches!(map_object_type(0), Err(ReplayError::UnknownObjectType(0))));
        assert!(matches!(map_object_type(7), Err(ReplayError::UnknownObjectType(7))));
        assert!(matches!(map_object_type(255), Err(ReplayError::UnknownObjectType(255))));
    }
}
