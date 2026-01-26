//! Kernel state - pure data structure holding all kernel state
//!
//! This module contains the KernelState struct which holds all mutable kernel
//! state. It has NO HAL dependency - all platform-specific behavior is in
//! the runtime wrapper (`zos-kernel`).

use alloc::collections::BTreeMap;

use crate::capability::CapabilitySpace;
use crate::types::{
    Endpoint, EndpointDetail, EndpointId, EndpointInfo, MessageSummary, Process, ProcessId,
    ProcessMetrics, ProcessState, SystemMetrics,
};
use alloc::string::ToString;
use alloc::vec::Vec;

/// The pure kernel state - no HAL, no I/O, no side effects.
///
/// All state transformations are done via the `step` function.
/// This struct is the verification target.
pub struct KernelState {
    /// Process table
    pub processes: BTreeMap<ProcessId, Process>,
    /// Capability spaces (per-process)
    pub cap_spaces: BTreeMap<ProcessId, CapabilitySpace>,
    /// IPC endpoints
    pub endpoints: BTreeMap<EndpointId, Endpoint>,
    /// Next process ID to allocate
    pub next_pid: u64,
    /// Next endpoint ID to allocate
    pub next_endpoint_id: u64,
    /// Next capability ID to allocate
    pub next_cap_id: u64,
    /// Total IPC messages since boot
    pub total_ipc_count: u64,
}

impl KernelState {
    /// Create a new empty kernel state.
    pub fn new() -> Self {
        Self {
            processes: BTreeMap::new(),
            cap_spaces: BTreeMap::new(),
            endpoints: BTreeMap::new(),
            next_pid: 1,
            next_endpoint_id: 1,
            next_cap_id: 1,
            total_ipc_count: 0,
        }
    }

    /// Generate next process ID
    pub fn alloc_pid(&mut self) -> ProcessId {
        let pid = ProcessId(self.next_pid);
        self.next_pid += 1;
        pid
    }

    /// Generate next endpoint ID
    pub fn alloc_endpoint_id(&mut self) -> EndpointId {
        let id = EndpointId(self.next_endpoint_id);
        self.next_endpoint_id += 1;
        id
    }

    /// Generate next capability ID
    pub fn alloc_cap_id(&mut self) -> u64 {
        let id = self.next_cap_id;
        self.next_cap_id += 1;
        id
    }

    // ========================================================================
    // Read-only accessors
    // ========================================================================

    /// Get process info
    pub fn get_process(&self, pid: ProcessId) -> Option<&Process> {
        self.processes.get(&pid)
    }

    /// Get mutable process info
    pub fn get_process_mut(&mut self, pid: ProcessId) -> Option<&mut Process> {
        self.processes.get_mut(&pid)
    }

    /// Get all processes
    pub fn list_processes(&self) -> Vec<(ProcessId, &Process)> {
        self.processes.iter().map(|(&pid, p)| (pid, p)).collect()
    }

    /// Get capability space for a process
    pub fn get_cap_space(&self, pid: ProcessId) -> Option<&CapabilitySpace> {
        self.cap_spaces.get(&pid)
    }

    /// Get mutable capability space for a process
    pub fn get_cap_space_mut(&mut self, pid: ProcessId) -> Option<&mut CapabilitySpace> {
        self.cap_spaces.get_mut(&pid)
    }

    /// Get endpoint by ID
    pub fn get_endpoint(&self, id: EndpointId) -> Option<&Endpoint> {
        self.endpoints.get(&id)
    }

    /// Get mutable endpoint by ID
    pub fn get_endpoint_mut(&mut self, id: EndpointId) -> Option<&mut Endpoint> {
        self.endpoints.get_mut(&id)
    }

    /// List all endpoints
    pub fn list_endpoints(&self) -> Vec<EndpointInfo> {
        self.endpoints
            .values()
            .map(|e| EndpointInfo {
                id: e.id,
                owner: e.owner,
                queue_depth: e.pending_messages.len(),
            })
            .collect()
    }

    /// Get detailed endpoint info
    pub fn get_endpoint_detail(&self, id: EndpointId) -> Option<EndpointDetail> {
        self.endpoints.get(&id).map(|e| EndpointDetail {
            id: e.id,
            owner: e.owner,
            pending_messages: e
                .pending_messages
                .iter()
                .map(|m| MessageSummary {
                    sender: m.sender,
                    tag: m.tag,
                    data_size: m.data.len(),
                    cap_count: m.caps.len(),
                })
                .collect(),
            metrics: e.metrics.clone(),
        })
    }

    /// Get total system memory usage
    pub fn total_memory(&self) -> usize {
        self.processes.values().map(|p| p.metrics.memory_size).sum()
    }

    /// Get total message count in all endpoint queues
    pub fn total_pending_messages(&self) -> usize {
        self.endpoints
            .values()
            .map(|e| e.pending_messages.len())
            .sum()
    }

    /// Get system-wide metrics
    pub fn get_system_metrics(&self, uptime_ns: u64) -> SystemMetrics {
        SystemMetrics {
            process_count: self.processes.len(),
            total_memory: self.total_memory(),
            endpoint_count: self.endpoints.len(),
            total_pending_messages: self.total_pending_messages(),
            total_ipc_messages: self.total_ipc_count,
            uptime_ns,
        }
    }

    // ========================================================================
    // State mutation helpers (pure - no side effects)
    // ========================================================================

    /// Register a new process, returns the PID
    pub fn register_process(&mut self, name: &str, timestamp: u64) -> ProcessId {
        let pid = self.alloc_pid();
        let process = Process {
            pid,
            name: name.to_string(),
            state: ProcessState::Running,
            metrics: ProcessMetrics {
                start_time_ns: timestamp,
                ..Default::default()
            },
        };
        self.processes.insert(pid, process);
        self.cap_spaces.insert(pid, CapabilitySpace::new());
        pid
    }

    /// Register a process with a specific PID
    pub fn register_process_with_pid(
        &mut self,
        pid: ProcessId,
        name: &str,
        timestamp: u64,
    ) -> ProcessId {
        // Update next_pid if necessary
        if pid.0 >= self.next_pid {
            self.next_pid = pid.0 + 1;
        }

        let process = Process {
            pid,
            name: name.to_string(),
            state: ProcessState::Running,
            metrics: ProcessMetrics {
                start_time_ns: timestamp,
                ..Default::default()
            },
        };
        self.processes.insert(pid, process);
        self.cap_spaces.insert(pid, CapabilitySpace::new());
        pid
    }

    /// Kill a process (set state to Zombie)
    pub fn kill_process(&mut self, pid: ProcessId) -> bool {
        if let Some(proc) = self.processes.get_mut(&pid) {
            proc.state = ProcessState::Zombie;
            true
        } else {
            false
        }
    }

    /// Remove a process completely
    pub fn remove_process(&mut self, pid: ProcessId) -> bool {
        self.processes.remove(&pid).is_some() && self.cap_spaces.remove(&pid).is_some()
    }

    /// Create an endpoint
    pub fn create_endpoint(&mut self, owner: ProcessId) -> EndpointId {
        let id = self.alloc_endpoint_id();
        let endpoint = Endpoint::new(id, owner);
        self.endpoints.insert(id, endpoint);
        id
    }

    /// Remove an endpoint
    pub fn remove_endpoint(&mut self, id: EndpointId) -> bool {
        self.endpoints.remove(&id).is_some()
    }

    /// Check if a process exists and is alive
    pub fn process_exists(&self, pid: ProcessId) -> bool {
        self.processes
            .get(&pid)
            .map(|p| p.state != ProcessState::Zombie)
            .unwrap_or(false)
    }

    /// Update syscall metrics for a process
    pub fn update_syscall_metrics(&mut self, pid: ProcessId, timestamp: u64) {
        if let Some(proc) = self.processes.get_mut(&pid) {
            proc.metrics.syscall_count += 1;
            proc.metrics.last_active_ns = timestamp;
        }
    }
}

impl Default for KernelState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    #[test]
    fn test_state_creation() {
        let state = KernelState::new();
        assert_eq!(state.processes.len(), 0);
        assert_eq!(state.endpoints.len(), 0);
        assert_eq!(state.next_pid, 1);
    }

    #[test]
    fn test_register_process() {
        let mut state = KernelState::new();
        let pid = state.register_process("test", 1000);

        assert_eq!(pid.0, 1);
        assert!(state.get_process(pid).is_some());
        assert!(state.get_cap_space(pid).is_some());
        assert_eq!(state.next_pid, 2);
    }

    #[test]
    fn test_create_endpoint() {
        let mut state = KernelState::new();
        let pid = state.register_process("test", 1000);
        let eid = state.create_endpoint(pid);

        assert_eq!(eid.0, 1);
        assert!(state.get_endpoint(eid).is_some());
        assert_eq!(state.get_endpoint(eid).unwrap().owner, pid);
    }

    #[test]
    fn test_kill_process() {
        let mut state = KernelState::new();
        let pid = state.register_process("test", 1000);

        assert!(state.kill_process(pid));
        assert_eq!(
            state.get_process(pid).unwrap().state,
            ProcessState::Zombie
        );
    }

    #[test]
    fn test_system_metrics() {
        let mut state = KernelState::new();
        state.register_process("proc1", 1000);
        state.register_process("proc2", 2000);

        let metrics = state.get_system_metrics(3000);
        assert_eq!(metrics.process_count, 2);
        assert_eq!(metrics.uptime_ns, 3000);
    }

    // ========================================================================
    // register_process_with_pid tests
    // ========================================================================

    #[test]
    fn test_register_process_with_pid_specific_pid() {
        let mut state = KernelState::new();

        // Register with specific PID
        let pid = state.register_process_with_pid(ProcessId(100), "special", 1000);
        assert_eq!(pid.0, 100);

        // Verify process was created correctly
        let proc = state.get_process(pid).unwrap();
        assert_eq!(proc.name, "special");
        assert_eq!(proc.state, ProcessState::Running);

        // Verify cap space was created
        assert!(state.get_cap_space(pid).is_some());

        // next_pid should be updated to avoid collision
        assert_eq!(state.next_pid, 101);
    }

    #[test]
    fn test_register_process_with_pid_updates_next_pid() {
        let mut state = KernelState::new();
        assert_eq!(state.next_pid, 1);

        // Register with PID 50, next_pid should update
        state.register_process_with_pid(ProcessId(50), "proc1", 1000);
        assert_eq!(state.next_pid, 51);

        // Register with PID lower than next_pid, next_pid should NOT update
        state.register_process_with_pid(ProcessId(10), "proc2", 2000);
        assert_eq!(state.next_pid, 51); // Unchanged

        // Register with PID exactly at next_pid
        state.register_process_with_pid(ProcessId(51), "proc3", 3000);
        assert_eq!(state.next_pid, 52);
    }

    #[test]
    fn test_register_process_with_pid_then_normal_register() {
        let mut state = KernelState::new();

        // Register with specific high PID
        state.register_process_with_pid(ProcessId(100), "special", 1000);
        assert_eq!(state.next_pid, 101);

        // Normal register should use next_pid
        let pid = state.register_process("normal", 2000);
        assert_eq!(pid.0, 101);
        assert_eq!(state.next_pid, 102);
    }

    // ========================================================================
    // remove_process tests
    // ========================================================================

    #[test]
    fn test_remove_process_complete_removal() {
        let mut state = KernelState::new();
        let pid = state.register_process("test", 1000);

        assert!(state.get_process(pid).is_some());
        assert!(state.get_cap_space(pid).is_some());

        let removed = state.remove_process(pid);
        assert!(removed);

        // Both process and cap space should be gone
        assert!(state.get_process(pid).is_none());
        assert!(state.get_cap_space(pid).is_none());
    }

    #[test]
    fn test_remove_process_nonexistent() {
        let mut state = KernelState::new();

        // Removing non-existent process returns false
        let removed = state.remove_process(ProcessId(999));
        assert!(!removed);
    }

    #[test]
    fn test_remove_process_cleans_cap_space() {
        let mut state = KernelState::new();
        let pid = state.register_process("test", 1000);

        // Add a capability
        let cspace = state.get_cap_space_mut(pid).unwrap();
        let cap = crate::capability::Capability {
            id: 1,
            object_type: crate::types::ObjectType::Endpoint,
            object_id: 42,
            permissions: crate::types::Permissions::full(),
            generation: 0,
            expires_at: 0,
        };
        cspace.insert(cap);
        assert_eq!(state.get_cap_space(pid).unwrap().len(), 1);

        // Remove process
        state.remove_process(pid);

        // Cap space should be gone
        assert!(state.get_cap_space(pid).is_none());
    }

    // ========================================================================
    // process_exists tests
    // ========================================================================

    #[test]
    fn test_process_exists_for_running_process() {
        let mut state = KernelState::new();
        let pid = state.register_process("test", 1000);

        assert!(state.process_exists(pid));
    }

    #[test]
    fn test_process_exists_returns_false_for_zombies() {
        let mut state = KernelState::new();
        let pid = state.register_process("test", 1000);

        assert!(state.process_exists(pid));

        // Kill process (becomes zombie)
        state.kill_process(pid);

        // process_exists should return false for zombies
        assert!(!state.process_exists(pid));

        // But the process is still in the table
        assert!(state.get_process(pid).is_some());
        assert_eq!(state.get_process(pid).unwrap().state, ProcessState::Zombie);
    }

    #[test]
    fn test_process_exists_nonexistent() {
        let state = KernelState::new();
        assert!(!state.process_exists(ProcessId(999)));
    }

    // ========================================================================
    // update_syscall_metrics tests
    // ========================================================================

    #[test]
    fn test_update_syscall_metrics() {
        let mut state = KernelState::new();
        let pid = state.register_process("test", 1000);

        // Initial metrics
        assert_eq!(state.get_process(pid).unwrap().metrics.syscall_count, 0);
        assert_eq!(state.get_process(pid).unwrap().metrics.last_active_ns, 0);

        // Update metrics
        state.update_syscall_metrics(pid, 2000);
        assert_eq!(state.get_process(pid).unwrap().metrics.syscall_count, 1);
        assert_eq!(state.get_process(pid).unwrap().metrics.last_active_ns, 2000);

        // Update again
        state.update_syscall_metrics(pid, 3000);
        assert_eq!(state.get_process(pid).unwrap().metrics.syscall_count, 2);
        assert_eq!(state.get_process(pid).unwrap().metrics.last_active_ns, 3000);
    }

    #[test]
    fn test_update_syscall_metrics_nonexistent_process() {
        let mut state = KernelState::new();

        // Should not panic for non-existent process
        state.update_syscall_metrics(ProcessId(999), 1000);
    }

    // ========================================================================
    // list_endpoints and get_endpoint_detail tests
    // ========================================================================

    #[test]
    fn test_list_endpoints_with_messages() {
        let mut state = KernelState::new();
        let pid = state.register_process("test", 1000);
        let eid = state.create_endpoint(pid);

        // Add a message
        let endpoint = state.get_endpoint_mut(eid).unwrap();
        let msg = crate::types::Message {
            sender: pid,
            tag: 42,
            data: vec![1, 2, 3],
            caps: vec![],
        };
        endpoint.enqueue(msg);

        // List endpoints
        let endpoints = state.list_endpoints();
        assert_eq!(endpoints.len(), 1);
        assert_eq!(endpoints[0].id, eid);
        assert_eq!(endpoints[0].owner, pid);
        assert_eq!(endpoints[0].queue_depth, 1);
    }

    #[test]
    fn test_get_endpoint_detail_with_messages() {
        let mut state = KernelState::new();
        let pid1 = state.register_process("sender", 1000);
        let pid2 = state.register_process("receiver", 1000);
        let eid = state.create_endpoint(pid2);

        // Add messages
        let endpoint = state.get_endpoint_mut(eid).unwrap();
        endpoint.enqueue(crate::types::Message {
            sender: pid1,
            tag: 100,
            data: vec![1, 2, 3, 4, 5],
            caps: vec![],
        });
        endpoint.enqueue(crate::types::Message {
            sender: pid1,
            tag: 200,
            data: vec![],
            caps: vec![],
        });

        // Get detail
        let detail = state.get_endpoint_detail(eid).unwrap();
        assert_eq!(detail.id, eid);
        assert_eq!(detail.owner, pid2);
        assert_eq!(detail.pending_messages.len(), 2);

        let msg0 = &detail.pending_messages[0];
        assert_eq!(msg0.sender, pid1);
        assert_eq!(msg0.tag, 100);
        assert_eq!(msg0.data_size, 5);
        assert_eq!(msg0.cap_count, 0);

        let msg1 = &detail.pending_messages[1];
        assert_eq!(msg1.tag, 200);
        assert_eq!(msg1.data_size, 0);
    }

    #[test]
    fn test_get_endpoint_detail_nonexistent() {
        let state = KernelState::new();
        assert!(state.get_endpoint_detail(EndpointId(999)).is_none());
    }

    // ========================================================================
    // total_memory and total_pending_messages tests
    // ========================================================================

    #[test]
    fn test_total_memory_aggregation() {
        let mut state = KernelState::new();
        let pid1 = state.register_process("proc1", 1000);
        let pid2 = state.register_process("proc2", 1000);

        // Set memory sizes
        state.get_process_mut(pid1).unwrap().metrics.memory_size = 1000;
        state.get_process_mut(pid2).unwrap().metrics.memory_size = 2000;

        assert_eq!(state.total_memory(), 3000);
    }

    #[test]
    fn test_total_pending_messages_aggregation() {
        let mut state = KernelState::new();
        let pid = state.register_process("test", 1000);
        let eid1 = state.create_endpoint(pid);
        let eid2 = state.create_endpoint(pid);

        // Add messages to endpoints
        let ep1 = state.get_endpoint_mut(eid1).unwrap();
        ep1.enqueue(crate::types::Message { sender: pid, tag: 0, data: vec![], caps: vec![] });
        ep1.enqueue(crate::types::Message { sender: pid, tag: 0, data: vec![], caps: vec![] });

        let ep2 = state.get_endpoint_mut(eid2).unwrap();
        ep2.enqueue(crate::types::Message { sender: pid, tag: 0, data: vec![], caps: vec![] });

        assert_eq!(state.total_pending_messages(), 3);
    }

    // ========================================================================
    // Alloc ID tests
    // ========================================================================

    #[test]
    fn test_alloc_endpoint_id() {
        let mut state = KernelState::new();
        assert_eq!(state.next_endpoint_id, 1);

        let eid1 = state.alloc_endpoint_id();
        assert_eq!(eid1.0, 1);
        assert_eq!(state.next_endpoint_id, 2);

        let eid2 = state.alloc_endpoint_id();
        assert_eq!(eid2.0, 2);
        assert_eq!(state.next_endpoint_id, 3);
    }

    #[test]
    fn test_alloc_cap_id() {
        let mut state = KernelState::new();
        assert_eq!(state.next_cap_id, 1);

        let id1 = state.alloc_cap_id();
        assert_eq!(id1, 1);
        assert_eq!(state.next_cap_id, 2);

        let id2 = state.alloc_cap_id();
        assert_eq!(id2, 2);
        assert_eq!(state.next_cap_id, 3);
    }

    // ========================================================================
    // remove_endpoint tests
    // ========================================================================

    #[test]
    fn test_remove_endpoint() {
        let mut state = KernelState::new();
        let pid = state.register_process("test", 1000);
        let eid = state.create_endpoint(pid);

        assert!(state.get_endpoint(eid).is_some());

        let removed = state.remove_endpoint(eid);
        assert!(removed);
        assert!(state.get_endpoint(eid).is_none());

        // Removing again returns false
        let removed_again = state.remove_endpoint(eid);
        assert!(!removed_again);
    }

    #[test]
    fn test_kill_process_nonexistent() {
        let mut state = KernelState::new();
        let killed = state.kill_process(ProcessId(999));
        assert!(!killed);
    }
}
