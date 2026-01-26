//! Spawn state machine types.
//!
//! This module defines the state tracking for process spawn operations,
//! enabling proper correlation of async spawn responses and timeout handling.

use std::collections::BTreeMap;

/// State of a pending spawn operation.
///
/// Each spawn goes through multiple stages, and we need to track where
/// each spawn is to properly correlate responses and handle timeouts.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SpawnState {
    /// Initial state: spawn request received, waiting for WASM binary from JS.
    WaitingForBinary,

    /// WASM binary received, process registered, waiting for PID assignment.
    /// (This is used in the Init-driven flow, currently transitional)
    WaitingForPid,

    /// Process has PID, waiting for endpoint creation.
    WaitingForEndpoint {
        /// The assigned process ID
        pid: u64,
    },

    /// Endpoints created, waiting for capability grants.
    WaitingForCaps {
        /// The assigned process ID
        pid: u64,
        /// The endpoint ID for the process's primary endpoint
        endpoint_id: u64,
    },

    /// Spawn completed successfully.
    Ready {
        /// The assigned process ID
        pid: u64,
    },

    /// Spawn failed.
    Failed {
        /// Reason for failure
        reason: String,
    },
}

impl SpawnState {
    /// Check if this state represents a terminal state (ready or failed).
    pub fn is_terminal(&self) -> bool {
        matches!(self, SpawnState::Ready { .. } | SpawnState::Failed { .. })
    }

    /// Check if this state represents a success.
    pub fn is_success(&self) -> bool {
        matches!(self, SpawnState::Ready { .. })
    }

    /// Get the PID if available.
    pub fn pid(&self) -> Option<u64> {
        match self {
            SpawnState::WaitingForEndpoint { pid } => Some(*pid),
            SpawnState::WaitingForCaps { pid, .. } => Some(*pid),
            SpawnState::Ready { pid } => Some(*pid),
            _ => None,
        }
    }
}

/// Tracks a pending spawn operation with full context.
///
/// This struct captures everything needed to:
/// - Correlate async spawn responses (binary fetch, PID assignment, etc.)
/// - Detect and handle spawn timeouts
/// - Clean up on failure
#[derive(Clone, Debug)]
pub struct PendingSpawn {
    /// Unique request ID for this spawn operation
    pub request_id: u64,

    /// Process type (e.g., "app", "system", "pingpong")
    pub proc_type: String,

    /// Process name (e.g., "terminal", "calculator")
    pub proc_name: String,

    /// Current state in the spawn state machine
    pub state: SpawnState,

    /// Timestamp when the spawn was requested (milliseconds since epoch)
    pub started_at: u64,

    /// Expected capabilities to be granted to this process
    pub expected_caps: Vec<CapabilityGrant>,
}

/// A capability grant expected during spawn.
#[derive(Clone, Debug)]
pub struct CapabilityGrant {
    /// Object type (Endpoint, Process, Console, Storage, Network)
    pub object_type: u8,

    /// Target object ID (e.g., endpoint ID)
    pub object_id: u64,

    /// Permissions bitmask
    pub permissions: u32,
}

impl PendingSpawn {
    /// Create a new pending spawn in the initial state.
    pub fn new(
        request_id: u64,
        proc_type: String,
        proc_name: String,
        started_at: u64,
    ) -> Self {
        Self {
            request_id,
            proc_type,
            proc_name,
            state: SpawnState::WaitingForBinary,
            started_at,
            expected_caps: Vec::new(),
        }
    }

    /// Transition to WaitingForPid state (binary received).
    pub fn binary_received(&mut self) {
        if matches!(self.state, SpawnState::WaitingForBinary) {
            self.state = SpawnState::WaitingForPid;
        }
    }

    /// Transition to WaitingForEndpoint state (PID assigned).
    pub fn pid_assigned(&mut self, pid: u64) {
        if matches!(
            self.state,
            SpawnState::WaitingForBinary | SpawnState::WaitingForPid
        ) {
            self.state = SpawnState::WaitingForEndpoint { pid };
        }
    }

    /// Transition to WaitingForCaps state (endpoint created).
    pub fn endpoint_created(&mut self, endpoint_id: u64) {
        if let SpawnState::WaitingForEndpoint { pid } = self.state {
            self.state = SpawnState::WaitingForCaps { pid, endpoint_id };
        }
    }

    /// Transition to Ready state (all capabilities granted).
    pub fn caps_granted(&mut self) {
        if let SpawnState::WaitingForCaps { pid, .. } = self.state {
            self.state = SpawnState::Ready { pid };
        }
    }

    /// Transition to Failed state.
    pub fn fail(&mut self, reason: String) {
        self.state = SpawnState::Failed { reason };
    }

    /// Check if spawn has timed out.
    ///
    /// # Arguments
    /// * `current_time` - Current timestamp in milliseconds
    /// * `timeout_ms` - Timeout duration in milliseconds
    pub fn is_timed_out(&self, current_time: u64, timeout_ms: u64) -> bool {
        !self.state.is_terminal() && current_time.saturating_sub(self.started_at) > timeout_ms
    }

    /// Get elapsed time since spawn started.
    pub fn elapsed_ms(&self, current_time: u64) -> u64 {
        current_time.saturating_sub(self.started_at)
    }
}

/// Manager for tracking multiple pending spawn operations.
///
/// This provides:
/// - Unique request ID allocation
/// - Timeout detection for stuck spawns
/// - Correlation of spawn responses by request ID
#[derive(Default)]
pub struct SpawnTracker {
    /// Map of request_id -> pending spawn
    pending: BTreeMap<u64, PendingSpawn>,

    /// Next request ID to allocate
    next_request_id: u64,

    /// Default timeout for spawn operations (30 seconds)
    timeout_ms: u64,
}

impl SpawnTracker {
    /// Create a new spawn tracker with default timeout.
    pub fn new() -> Self {
        Self {
            pending: BTreeMap::new(),
            next_request_id: 1,
            timeout_ms: 30_000, // 30 seconds
        }
    }

    /// Create a new spawn tracker with custom timeout.
    pub fn with_timeout(timeout_ms: u64) -> Self {
        Self {
            pending: BTreeMap::new(),
            next_request_id: 1,
            timeout_ms,
        }
    }

    /// Start tracking a new spawn operation.
    pub fn start_spawn(
        &mut self,
        proc_type: &str,
        proc_name: &str,
        current_time: u64,
    ) -> u64 {
        let request_id = self.next_request_id;
        self.next_request_id = self.next_request_id.wrapping_add(1);
        if self.next_request_id == 0 {
            self.next_request_id = 1;
        }

        let spawn = PendingSpawn::new(
            request_id,
            proc_type.to_string(),
            proc_name.to_string(),
            current_time,
        );

        self.pending.insert(request_id, spawn);
        request_id
    }

    /// Get a pending spawn by request ID.
    pub fn get(&self, request_id: u64) -> Option<&PendingSpawn> {
        self.pending.get(&request_id)
    }

    /// Get a mutable pending spawn by request ID.
    pub fn get_mut(&mut self, request_id: u64) -> Option<&mut PendingSpawn> {
        self.pending.get_mut(&request_id)
    }

    /// Find a pending spawn by process name.
    pub fn find_by_name(&self, proc_name: &str) -> Option<&PendingSpawn> {
        self.pending
            .values()
            .find(|s| s.proc_name == proc_name && !s.state.is_terminal())
    }

    /// Find a pending spawn by process name (mutable).
    pub fn find_by_name_mut(&mut self, proc_name: &str) -> Option<&mut PendingSpawn> {
        self.pending
            .values_mut()
            .find(|s| s.proc_name == proc_name && !s.state.is_terminal())
    }

    /// Remove a pending spawn (completed or failed).
    pub fn remove(&mut self, request_id: u64) -> Option<PendingSpawn> {
        self.pending.remove(&request_id)
    }

    /// Check for timed-out spawns and return their request IDs.
    pub fn check_timeouts(&self, current_time: u64) -> Vec<u64> {
        self.pending
            .iter()
            .filter(|(_, spawn)| spawn.is_timed_out(current_time, self.timeout_ms))
            .map(|(id, _)| *id)
            .collect()
    }

    /// Mark timed-out spawns as failed and return them.
    pub fn timeout_spawns(&mut self, current_time: u64) -> Vec<PendingSpawn> {
        let timed_out_ids = self.check_timeouts(current_time);
        let mut timed_out = Vec::new();

        for id in timed_out_ids {
            if let Some(mut spawn) = self.pending.remove(&id) {
                spawn.fail(format!(
                    "Spawn timed out after {}ms",
                    spawn.elapsed_ms(current_time)
                ));
                timed_out.push(spawn);
            }
        }

        timed_out
    }

    /// Get count of pending (non-terminal) spawns.
    pub fn pending_count(&self) -> usize {
        self.pending
            .values()
            .filter(|s| !s.state.is_terminal())
            .count()
    }

    /// Clean up completed spawns (terminal states).
    pub fn cleanup_completed(&mut self) -> Vec<PendingSpawn> {
        let completed_ids: Vec<u64> = self
            .pending
            .iter()
            .filter(|(_, s)| s.state.is_terminal())
            .map(|(id, _)| *id)
            .collect();

        completed_ids
            .into_iter()
            .filter_map(|id| self.pending.remove(&id))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spawn_state_transitions() {
        let mut spawn = PendingSpawn::new(1, "app".into(), "calculator".into(), 1000);
        
        assert!(matches!(spawn.state, SpawnState::WaitingForBinary));
        assert!(!spawn.state.is_terminal());
        assert!(spawn.state.pid().is_none());

        spawn.binary_received();
        assert!(matches!(spawn.state, SpawnState::WaitingForPid));

        spawn.pid_assigned(42);
        assert!(matches!(spawn.state, SpawnState::WaitingForEndpoint { pid: 42 }));
        assert_eq!(spawn.state.pid(), Some(42));

        spawn.endpoint_created(100);
        assert!(matches!(spawn.state, SpawnState::WaitingForCaps { pid: 42, endpoint_id: 100 }));

        spawn.caps_granted();
        assert!(matches!(spawn.state, SpawnState::Ready { pid: 42 }));
        assert!(spawn.state.is_terminal());
        assert!(spawn.state.is_success());
    }

    #[test]
    fn test_spawn_timeout() {
        let spawn = PendingSpawn::new(1, "app".into(), "test".into(), 1000);
        
        // Not timed out yet
        assert!(!spawn.is_timed_out(2000, 5000));
        
        // Timed out
        assert!(spawn.is_timed_out(7000, 5000));
        
        // Elapsed time
        assert_eq!(spawn.elapsed_ms(3000), 2000);
    }

    #[test]
    fn test_spawn_tracker() {
        let mut tracker = SpawnTracker::with_timeout(5000);
        
        let id1 = tracker.start_spawn("app", "calc", 1000);
        let id2 = tracker.start_spawn("app", "clock", 2000);
        
        assert_eq!(tracker.pending_count(), 2);
        
        // Simulate calc completing
        if let Some(spawn) = tracker.get_mut(id1) {
            spawn.pid_assigned(10);
            spawn.endpoint_created(100);
            spawn.caps_granted();
        }
        
        assert!(tracker.get(id1).unwrap().state.is_terminal());
        
        // Timeout clock
        let timed_out = tracker.timeout_spawns(10000);
        assert_eq!(timed_out.len(), 1);
        assert_eq!(timed_out[0].proc_name, "clock");
        
        // Cleanup completed
        let completed = tracker.cleanup_completed();
        assert_eq!(completed.len(), 1);
        assert_eq!(completed[0].proc_name, "calc");
    }

    #[test]
    fn test_spawn_failure() {
        let mut spawn = PendingSpawn::new(1, "app".into(), "failing_app".into(), 1000);
        
        spawn.binary_received();
        spawn.pid_assigned(99);
        spawn.fail("Worker spawn error".into());
        
        assert!(spawn.state.is_terminal());
        assert!(!spawn.state.is_success());
        assert!(matches!(spawn.state, SpawnState::Failed { .. }));
        
        // PID should not be available after failure
        assert!(spawn.state.pid().is_none());
    }

    #[test]
    fn test_spawn_tracker_find_by_name() {
        let mut tracker = SpawnTracker::new();
        
        tracker.start_spawn("app", "terminal", 1000);
        tracker.start_spawn("app", "calculator", 2000);
        
        // Find by name
        let terminal = tracker.find_by_name("terminal");
        assert!(terminal.is_some());
        assert_eq!(terminal.unwrap().proc_name, "terminal");
        
        // Non-existent
        let missing = tracker.find_by_name("nonexistent");
        assert!(missing.is_none());
        
        // Complete terminal and verify it's not found by name anymore
        if let Some(spawn) = tracker.find_by_name_mut("terminal") {
            spawn.pid_assigned(10);
            spawn.endpoint_created(100);
            spawn.caps_granted();
        }
        
        // Terminal is now complete, should not be found (find_by_name excludes terminal states)
        let terminal_after = tracker.find_by_name("terminal");
        assert!(terminal_after.is_none());
    }

    #[test]
    fn test_spawn_tracker_remove() {
        let mut tracker = SpawnTracker::new();
        
        let id = tracker.start_spawn("app", "test", 1000);
        assert_eq!(tracker.pending_count(), 1);
        
        let removed = tracker.remove(id);
        assert!(removed.is_some());
        assert_eq!(removed.unwrap().proc_name, "test");
        assert_eq!(tracker.pending_count(), 0);
        
        // Double remove returns None
        assert!(tracker.remove(id).is_none());
    }

    #[test]
    fn test_spawn_tracker_request_id_uniqueness() {
        let mut tracker = SpawnTracker::new();
        
        let id1 = tracker.start_spawn("app", "a", 1000);
        let id2 = tracker.start_spawn("app", "b", 1000);
        let id3 = tracker.start_spawn("app", "c", 1000);
        
        // All IDs should be unique
        assert_ne!(id1, id2);
        assert_ne!(id2, id3);
        assert_ne!(id1, id3);
    }

    #[test]
    fn test_terminal_state_timeout_immunity() {
        let mut spawn = PendingSpawn::new(1, "app".into(), "test".into(), 1000);
        
        // Terminal state should not timeout
        spawn.pid_assigned(42);
        spawn.endpoint_created(100);
        spawn.caps_granted();
        
        assert!(spawn.state.is_terminal());
        assert!(!spawn.is_timed_out(100000, 5000)); // Even with huge time diff
    }

    #[test]
    fn test_spawn_state_skip_transitions() {
        let mut spawn = PendingSpawn::new(1, "app".into(), "fast".into(), 1000);
        
        // Skip directly from WaitingForBinary to WaitingForEndpoint (pid_assigned handles this)
        spawn.pid_assigned(50);
        assert!(matches!(spawn.state, SpawnState::WaitingForEndpoint { pid: 50 }));
        
        // binary_received() should be ignored after pid_assigned
        spawn.binary_received();
        assert!(matches!(spawn.state, SpawnState::WaitingForEndpoint { pid: 50 }));
    }

    #[test]
    fn test_capability_grant() {
        let grant = CapabilityGrant {
            object_type: 1, // Endpoint
            object_id: 42,
            permissions: 0b111, // Read/Write/Execute
        };
        
        let mut spawn = PendingSpawn::new(1, "app".into(), "test".into(), 1000);
        spawn.expected_caps.push(grant.clone());
        
        assert_eq!(spawn.expected_caps.len(), 1);
        assert_eq!(spawn.expected_caps[0].object_id, 42);
    }

    #[test]
    fn test_check_timeouts_returns_only_timed_out() {
        let mut tracker = SpawnTracker::with_timeout(5000);
        
        tracker.start_spawn("app", "fast", 1000);
        tracker.start_spawn("app", "slow", 100); // Started earlier
        
        // At time 4000, neither should be timed out (5000ms timeout)
        let timed_out = tracker.check_timeouts(4000);
        assert!(timed_out.is_empty());
        
        // At time 6000, only 'slow' (started at 100) should be timed out
        let timed_out = tracker.check_timeouts(6000);
        assert_eq!(timed_out.len(), 1);
        
        // At time 7000, both should be timed out
        let timed_out = tracker.check_timeouts(7000);
        assert_eq!(timed_out.len(), 2);
    }
}
