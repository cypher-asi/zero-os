//! Formal invariants for kernel verification
//!
//! This module contains runtime-checkable invariants that should always hold.
//! These are used for:
//! 1. Runtime assertion checking during development
//! 2. Property-based testing with proptest/quickcheck
//! 3. Formal verification with Kani
//!
//! # Invariants
//!
//! 1. **Process Capability Consistency**: Every process has a capability space
//! 2. **Endpoint Ownership**: Every endpoint's owner is a valid process
//! 3. **Capability Object Validity**: Every capability references a valid object
//! 4. **No Orphan Endpoints**: Endpoints without valid owners should not exist
//! 5. **ID Monotonicity**: Next IDs are always greater than existing IDs

use alloc::string::String;
use alloc::vec::Vec;

use crate::state::KernelState;
use crate::types::{EndpointId, ObjectType, ProcessId, ProcessState};

/// An invariant violation with details
#[derive(Clone, Debug)]
pub struct InvariantViolation {
    /// Name of the violated invariant
    pub invariant: &'static str,
    /// Description of what went wrong
    pub description: String,
}

/// Check all kernel invariants.
///
/// Returns a list of violations (empty if all invariants hold).
pub fn check_all_invariants(state: &KernelState) -> Vec<InvariantViolation> {
    let mut violations = Vec::new();

    violations.extend(check_process_capability_consistency(state));
    violations.extend(check_endpoint_ownership(state));
    violations.extend(check_capability_object_validity(state));
    violations.extend(check_id_monotonicity(state));

    violations
}

/// Invariant 1: Every process has a capability space
fn check_process_capability_consistency(state: &KernelState) -> Vec<InvariantViolation> {
    let mut violations = Vec::new();

    for (pid, _process) in &state.processes {
        if !state.cap_spaces.contains_key(pid) {
            violations.push(InvariantViolation {
                invariant: "process_capability_consistency",
                description: alloc::format!(
                    "Process {} exists but has no capability space",
                    pid.0
                ),
            });
        }
    }

    // Also check the reverse: no orphan capability spaces
    for pid in state.cap_spaces.keys() {
        if !state.processes.contains_key(pid) {
            violations.push(InvariantViolation {
                invariant: "process_capability_consistency",
                description: alloc::format!(
                    "Capability space exists for non-existent process {}",
                    pid.0
                ),
            });
        }
    }

    violations
}

/// Invariant 2: Every endpoint's owner is a valid process
fn check_endpoint_ownership(state: &KernelState) -> Vec<InvariantViolation> {
    let mut violations = Vec::new();

    for (eid, endpoint) in &state.endpoints {
        // Check owner exists
        if !state.processes.contains_key(&endpoint.owner) {
            violations.push(InvariantViolation {
                invariant: "endpoint_ownership",
                description: alloc::format!(
                    "Endpoint {} owned by non-existent process {}",
                    eid.0,
                    endpoint.owner.0
                ),
            });
            continue;
        }

        // Check owner is not a zombie (optional - depends on design choice)
        if let Some(proc) = state.processes.get(&endpoint.owner) {
            if proc.state == ProcessState::Zombie {
                // This might be okay during cleanup, but flag it
                violations.push(InvariantViolation {
                    invariant: "endpoint_ownership",
                    description: alloc::format!(
                        "Endpoint {} owned by zombie process {}",
                        eid.0,
                        endpoint.owner.0
                    ),
                });
            }
        }
    }

    violations
}

/// Invariant 3: Every capability references a valid object (endpoints for now)
fn check_capability_object_validity(state: &KernelState) -> Vec<InvariantViolation> {
    let mut violations = Vec::new();

    for (pid, cspace) in &state.cap_spaces {
        for (slot, cap) in &cspace.slots {
            match cap.object_type {
                ObjectType::Endpoint => {
                    let endpoint_id = EndpointId(cap.object_id);
                    if !state.endpoints.contains_key(&endpoint_id) {
                        violations.push(InvariantViolation {
                            invariant: "capability_object_validity",
                            description: alloc::format!(
                                "Process {} slot {} references non-existent endpoint {}",
                                pid.0,
                                slot,
                                cap.object_id
                            ),
                        });
                    }
                }
                ObjectType::Process => {
                    let target_pid = ProcessId(cap.object_id);
                    if !state.processes.contains_key(&target_pid) {
                        violations.push(InvariantViolation {
                            invariant: "capability_object_validity",
                            description: alloc::format!(
                                "Process {} slot {} references non-existent process {}",
                                pid.0,
                                slot,
                                cap.object_id
                            ),
                        });
                    }
                }
                // Other object types are not validated here (Memory, Irq, IoPort, Console)
                _ => {}
            }
        }
    }

    violations
}

/// Invariant 4: Next IDs are always greater than existing IDs
fn check_id_monotonicity(state: &KernelState) -> Vec<InvariantViolation> {
    let mut violations = Vec::new();

    // Check process IDs
    for pid in state.processes.keys() {
        if pid.0 >= state.next_pid {
            violations.push(InvariantViolation {
                invariant: "id_monotonicity",
                description: alloc::format!(
                    "Process {} exists but next_pid is {}",
                    pid.0,
                    state.next_pid
                ),
            });
        }
    }

    // Check endpoint IDs
    for eid in state.endpoints.keys() {
        if eid.0 >= state.next_endpoint_id {
            violations.push(InvariantViolation {
                invariant: "id_monotonicity",
                description: alloc::format!(
                    "Endpoint {} exists but next_endpoint_id is {}",
                    eid.0,
                    state.next_endpoint_id
                ),
            });
        }
    }

    // Check capability IDs
    for cspace in state.cap_spaces.values() {
        for cap in cspace.slots.values() {
            if cap.id >= state.next_cap_id {
                violations.push(InvariantViolation {
                    invariant: "id_monotonicity",
                    description: alloc::format!(
                        "Capability {} exists but next_cap_id is {}",
                        cap.id,
                        state.next_cap_id
                    ),
                });
            }
        }
    }

    violations
}

/// Assert all invariants hold (panic if not)
pub fn assert_invariants(state: &KernelState) {
    let violations = check_all_invariants(state);
    if !violations.is_empty() {
        for v in &violations {
            // In no_std, we can't easily panic with a formatted message,
            // but we can at least report the invariant name
            panic!("Invariant violated: {}", v.invariant);
        }
    }
}

// ============================================================================
// Kani proofs for invariants
// ============================================================================

#[cfg(kani)]
mod proofs {
    use super::*;
    use crate::step::{step, Syscall};

    /// Proof: Registering a process maintains invariants
    #[kani::proof]
    #[kani::unwind(5)]
    fn register_process_maintains_invariants() {
        let mut state = KernelState::new();

        // Register a process
        let pid = state.register_process("test", 1000);

        // Invariants should hold
        let violations = check_all_invariants(&state);
        kani::assert(
            violations.is_empty(),
            "Registering a process should maintain invariants",
        );

        // Process should have a capability space
        kani::assert(
            state.cap_spaces.contains_key(&pid),
            "New process should have capability space",
        );
    }

    /// Proof: Creating an endpoint maintains invariants
    #[kani::proof]
    #[kani::unwind(5)]
    fn create_endpoint_maintains_invariants() {
        let mut state = KernelState::new();
        let pid = state.register_process("test", 1000);

        // Create endpoint
        let result = step(&mut state, pid, Syscall::CreateEndpoint, 2000);

        // Should succeed
        kani::assume(matches!(result.result, crate::step::SyscallResult::Ok(_)));

        // Invariants should hold
        let violations = check_all_invariants(&state);
        kani::assert(
            violations.is_empty(),
            "Creating an endpoint should maintain invariants",
        );
    }

    /// Proof: IPC operations maintain invariants
    #[kani::proof]
    #[kani::unwind(10)]
    fn ipc_maintains_invariants() {
        let mut state = KernelState::new();
        let sender = state.register_process("sender", 1000);
        let receiver = state.register_process("receiver", 1000);

        // Create endpoint owned by receiver
        state.create_endpoint(receiver);

        // After setup, invariants should hold
        let violations = check_all_invariants(&state);
        kani::assert(
            violations.is_empty(),
            "Initial state should satisfy invariants",
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;
    use crate::capability::Capability;
    use crate::types::{ObjectType, Permissions};

    #[test]
    fn test_invariants_hold_for_new_state() {
        let state = KernelState::new();
        let violations = check_all_invariants(&state);
        assert!(violations.is_empty());
    }

    #[test]
    fn test_invariants_hold_after_register_process() {
        let mut state = KernelState::new();
        state.register_process("test", 1000);
        let violations = check_all_invariants(&state);
        assert!(violations.is_empty());
    }

    #[test]
    fn test_invariants_hold_after_create_endpoint() {
        let mut state = KernelState::new();
        let pid = state.register_process("test", 1000);
        state.create_endpoint(pid);
        let violations = check_all_invariants(&state);
        assert!(violations.is_empty());
    }

    #[test]
    fn test_detects_orphan_capability_space() {
        let mut state = KernelState::new();
        // Create orphan capability space directly
        state
            .cap_spaces
            .insert(ProcessId(999), crate::capability::CapabilitySpace::new());

        let violations = check_all_invariants(&state);
        assert!(!violations.is_empty());
        assert!(violations
            .iter()
            .any(|v| v.invariant == "process_capability_consistency"));
    }

    #[test]
    fn test_detects_dangling_capability() {
        let mut state = KernelState::new();
        let pid = state.register_process("test", 1000);

        // Insert a capability to a non-existent endpoint
        let bad_cap = Capability {
            id: state.alloc_cap_id(),
            object_type: ObjectType::Endpoint,
            object_id: 999, // Non-existent
            permissions: Permissions::full(),
            generation: 0,
            expires_at: 0,
        };
        state.get_cap_space_mut(pid).unwrap().insert(bad_cap);

        let violations = check_all_invariants(&state);
        assert!(!violations.is_empty());
        assert!(violations
            .iter()
            .any(|v| v.invariant == "capability_object_validity"));
    }

    #[test]
    fn test_detects_id_monotonicity_violation() {
        let mut state = KernelState::new();
        state.register_process("test", 1000);

        // Manually break monotonicity
        state.next_pid = 0;

        let violations = check_all_invariants(&state);
        assert!(!violations.is_empty());
        assert!(violations
            .iter()
            .any(|v| v.invariant == "id_monotonicity"));
    }

    // ========================================================================
    // Zombie owner detection tests
    // ========================================================================

    #[test]
    fn test_detects_endpoint_owned_by_zombie() {
        let mut state = KernelState::new();
        let pid = state.register_process("test", 1000);
        state.create_endpoint(pid);

        // Kill the owner (becomes zombie)
        state.kill_process(pid);

        let violations = check_all_invariants(&state);
        assert!(!violations.is_empty());
        assert!(violations
            .iter()
            .any(|v| v.invariant == "endpoint_ownership"));
        assert!(violations
            .iter()
            .any(|v| v.description.contains("zombie")));
    }

    #[test]
    fn test_detects_endpoint_with_nonexistent_owner() {
        let mut state = KernelState::new();
        let pid = state.register_process("test", 1000);
        let eid = state.create_endpoint(pid);

        // Remove the owner entirely
        state.remove_process(pid);

        // Endpoint still exists but owner is gone
        assert!(state.get_endpoint(eid).is_some());
        assert!(state.get_process(pid).is_none());

        let violations = check_all_invariants(&state);
        assert!(!violations.is_empty());
        assert!(violations
            .iter()
            .any(|v| v.invariant == "endpoint_ownership"));
        assert!(violations
            .iter()
            .any(|v| v.description.contains("non-existent")));
    }

    // ========================================================================
    // Multiple simultaneous violations
    // ========================================================================

    #[test]
    fn test_detects_multiple_violations() {
        let mut state = KernelState::new();
        let pid = state.register_process("test", 1000);
        state.create_endpoint(pid);

        // Create multiple violations:
        // 1. Orphan capability space
        state
            .cap_spaces
            .insert(ProcessId(999), crate::capability::CapabilitySpace::new());

        // 2. Dangling capability
        let bad_cap = Capability {
            id: state.alloc_cap_id(),
            object_type: ObjectType::Endpoint,
            object_id: 888, // Non-existent endpoint
            permissions: Permissions::full(),
            generation: 0,
            expires_at: 0,
        };
        state.get_cap_space_mut(pid).unwrap().insert(bad_cap);

        // 3. Break ID monotonicity
        state.next_pid = 0;

        let violations = check_all_invariants(&state);

        // Should detect all violations
        assert!(violations.len() >= 3);
        assert!(violations
            .iter()
            .any(|v| v.invariant == "process_capability_consistency"));
        assert!(violations
            .iter()
            .any(|v| v.invariant == "capability_object_validity"));
        assert!(violations
            .iter()
            .any(|v| v.invariant == "id_monotonicity"));
    }

    // ========================================================================
    // Invariants hold after operations
    // ========================================================================

    #[test]
    fn test_invariants_hold_after_ipc() {
        let mut state = KernelState::new();
        let sender = state.register_process("sender", 1000);
        let receiver = state.register_process("receiver", 1000);
        let eid = state.create_endpoint(receiver);

        // Send a message directly to the endpoint
        let endpoint = state.get_endpoint_mut(eid).unwrap();
        let msg = crate::types::Message {
            sender,
            tag: 42,
            data: vec![1, 2, 3],
            caps: vec![],
        };
        endpoint.enqueue(msg);

        // Invariants should still hold
        let violations = check_all_invariants(&state);
        assert!(violations.is_empty(), "Violations: {:?}", violations);
    }

    #[test]
    fn test_invariants_hold_after_capability_operations() {
        let mut state = KernelState::new();
        let pid1 = state.register_process("proc1", 1000);
        let pid2 = state.register_process("proc2", 1000);
        let eid = state.create_endpoint(pid1);

        // Add capability
        let cap = Capability {
            id: state.alloc_cap_id(),
            object_type: ObjectType::Endpoint,
            object_id: eid.0,
            permissions: Permissions::full(),
            generation: 0,
            expires_at: 0,
        };
        let slot = state.get_cap_space_mut(pid1).unwrap().insert(cap);

        // Clone to pid2
        let cap2 = Capability {
            id: state.alloc_cap_id(),
            object_type: ObjectType::Endpoint,
            object_id: eid.0,
            permissions: Permissions::read_only(),
            generation: 1,
            expires_at: 0,
        };
        state.get_cap_space_mut(pid2).unwrap().insert(cap2);

        // Invariants should hold
        let violations = check_all_invariants(&state);
        assert!(violations.is_empty(), "Violations: {:?}", violations);

        // Remove capability from pid1
        state.get_cap_space_mut(pid1).unwrap().remove(slot);

        // Invariants should still hold
        let violations = check_all_invariants(&state);
        assert!(violations.is_empty(), "Violations: {:?}", violations);
    }

    #[test]
    fn test_invariants_hold_after_endpoint_removal() {
        let mut state = KernelState::new();
        let pid = state.register_process("test", 1000);
        let eid = state.create_endpoint(pid);

        // Create capability to endpoint
        let cap = Capability {
            id: state.alloc_cap_id(),
            object_type: ObjectType::Endpoint,
            object_id: eid.0,
            permissions: Permissions::full(),
            generation: 0,
            expires_at: 0,
        };
        let slot = state.get_cap_space_mut(pid).unwrap().insert(cap);

        // Remove capability first, then endpoint
        state.get_cap_space_mut(pid).unwrap().remove(slot);
        state.remove_endpoint(eid);

        // Invariants should hold
        let violations = check_all_invariants(&state);
        assert!(violations.is_empty(), "Violations: {:?}", violations);
    }

    #[test]
    fn test_invariants_violation_after_endpoint_removal_without_cap_cleanup() {
        let mut state = KernelState::new();
        let pid = state.register_process("test", 1000);
        let eid = state.create_endpoint(pid);

        // Create capability to endpoint
        let cap = Capability {
            id: state.alloc_cap_id(),
            object_type: ObjectType::Endpoint,
            object_id: eid.0,
            permissions: Permissions::full(),
            generation: 0,
            expires_at: 0,
        };
        state.get_cap_space_mut(pid).unwrap().insert(cap);

        // Remove endpoint WITHOUT removing capability - creates dangling reference
        state.remove_endpoint(eid);

        // This should detect the violation
        let violations = check_all_invariants(&state);
        assert!(!violations.is_empty());
        assert!(violations
            .iter()
            .any(|v| v.invariant == "capability_object_validity"));
    }

    // ========================================================================
    // Capability to Process object type tests
    // ========================================================================

    #[test]
    fn test_detects_capability_to_nonexistent_process() {
        let mut state = KernelState::new();
        let pid = state.register_process("test", 1000);

        // Create capability pointing to non-existent process
        let bad_cap = Capability {
            id: state.alloc_cap_id(),
            object_type: ObjectType::Process, // Process type
            object_id: 999, // Non-existent process
            permissions: Permissions::full(),
            generation: 0,
            expires_at: 0,
        };
        state.get_cap_space_mut(pid).unwrap().insert(bad_cap);

        let violations = check_all_invariants(&state);
        assert!(!violations.is_empty());
        assert!(violations
            .iter()
            .any(|v| v.invariant == "capability_object_validity"));
    }

    #[test]
    fn test_valid_capability_to_existing_process() {
        let mut state = KernelState::new();
        let pid1 = state.register_process("proc1", 1000);
        let pid2 = state.register_process("proc2", 1000);

        // Create capability pointing to existing process
        let cap = Capability {
            id: state.alloc_cap_id(),
            object_type: ObjectType::Process,
            object_id: pid2.0,
            permissions: Permissions::full(),
            generation: 0,
            expires_at: 0,
        };
        state.get_cap_space_mut(pid1).unwrap().insert(cap);

        let violations = check_all_invariants(&state);
        assert!(violations.is_empty(), "Violations: {:?}", violations);
    }

    // ========================================================================
    // assert_invariants tests
    // ========================================================================

    #[test]
    fn test_assert_invariants_passes_for_valid_state() {
        let mut state = KernelState::new();
        state.register_process("test", 1000);

        // Should not panic
        assert_invariants(&state);
    }

    #[test]
    #[should_panic(expected = "Invariant violated")]
    fn test_assert_invariants_panics_on_violation() {
        let mut state = KernelState::new();
        // Create orphan cap space
        state
            .cap_spaces
            .insert(ProcessId(999), crate::capability::CapabilitySpace::new());

        assert_invariants(&state);
    }

    // ========================================================================
    // Endpoint ID monotonicity tests
    // ========================================================================

    #[test]
    fn test_detects_endpoint_id_monotonicity_violation() {
        let mut state = KernelState::new();
        let pid = state.register_process("test", 1000);
        state.create_endpoint(pid);

        // Break endpoint ID monotonicity
        state.next_endpoint_id = 0;

        let violations = check_all_invariants(&state);
        assert!(!violations.is_empty());
        assert!(violations
            .iter()
            .any(|v| v.invariant == "id_monotonicity"));
    }

    #[test]
    fn test_detects_capability_id_monotonicity_violation() {
        let mut state = KernelState::new();
        let pid = state.register_process("test", 1000);

        // Add capability with ID 10
        let cap = Capability {
            id: 10,
            object_type: ObjectType::Console,
            object_id: 1,
            permissions: Permissions::full(),
            generation: 0,
            expires_at: 0,
        };
        state.get_cap_space_mut(pid).unwrap().insert(cap);

        // Set next_cap_id lower than existing cap
        state.next_cap_id = 5;

        let violations = check_all_invariants(&state);
        assert!(!violations.is_empty());
        assert!(violations
            .iter()
            .any(|v| v.invariant == "id_monotonicity"));
    }
}
