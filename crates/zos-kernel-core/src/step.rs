//! Pure step function - the heart of the verification target
//!
//! This module contains the pure `step(state, syscall) -> (state', result)` function.
//! All state transformations happen here - no HAL, no I/O, no side effects.
//!
//! # Design
//!
//! The step function takes:
//! - Current kernel state
//! - A syscall request
//! - Current timestamp
//!
//! And returns:
//! - Updated state (via mutation)
//! - Syscall result
//! - List of commits (state mutations for audit log)
//!
//! This design enables:
//! - Deterministic replay from commit log
//! - Formal verification via Kani
//! - Property-based testing

use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

use crate::capability::{axiom_check, AxiomError, Capability};
use crate::state::KernelState;
use crate::types::{
    CapInfo, CapSlot, Endpoint, EndpointId, Message, ObjectType, Permissions, ProcessId,
    ProcessState, TransferredCap,
};

// ============================================================================
// Syscall definitions
// ============================================================================

/// Syscall variants - all possible kernel operations
#[derive(Clone, Debug)]
pub enum Syscall {
    /// Print debug message
    Debug { msg: String },

    /// Get current time
    GetTime,

    /// Yield CPU
    Yield,

    /// Exit process
    Exit { code: i32 },

    /// Kill another process
    Kill { target_pid: ProcessId },

    /// List all processes
    ListProcesses,

    /// Create an IPC endpoint
    CreateEndpoint,

    /// Send IPC message
    Send {
        endpoint_slot: CapSlot,
        tag: u32,
        data: Vec<u8>,
    },

    /// Receive IPC message
    Receive { endpoint_slot: CapSlot },

    /// Send IPC with capability transfer
    SendWithCaps {
        endpoint_slot: CapSlot,
        tag: u32,
        data: Vec<u8>,
        cap_slots: Vec<CapSlot>,
    },

    /// Call (send + wait for reply)
    Call {
        endpoint_slot: CapSlot,
        tag: u32,
        data: Vec<u8>,
    },

    /// List capabilities
    ListCaps,

    /// Grant capability to another process
    CapGrant {
        from_slot: CapSlot,
        to_pid: ProcessId,
        permissions: Permissions,
    },

    /// Revoke capability
    CapRevoke { slot: CapSlot },

    /// Delete capability
    CapDelete { slot: CapSlot },

    /// Inspect capability
    CapInspect { slot: CapSlot },

    /// Derive capability with reduced permissions
    CapDerive {
        slot: CapSlot,
        new_permissions: Permissions,
    },
}

// ============================================================================
// Syscall results
// ============================================================================

/// Syscall result - what the kernel returns to the caller
#[derive(Clone, Debug)]
pub enum SyscallResult {
    /// Success with value
    Ok(u64),
    /// Error
    Err(KernelError),
    /// Message received
    Message(Message),
    /// Would block (no message available)
    WouldBlock,
    /// Capability list
    CapList(Vec<(CapSlot, Capability)>),
    /// Capability info
    CapInfo(CapInfo),
    /// Process list
    ProcessList(Vec<(ProcessId, String, ProcessState)>),
}

/// Kernel errors
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KernelError {
    /// Process not found
    ProcessNotFound,
    /// Endpoint not found
    EndpointNotFound,
    /// Invalid capability
    InvalidCapability,
    /// Permission denied
    PermissionDenied,
    /// Message too large
    MessageTooLarge,
    /// Too many capabilities in message
    TooManyCaps,
    /// Invalid argument
    InvalidArgument,
    /// Resource exhausted
    ResourceExhausted,
}

impl From<AxiomError> for KernelError {
    fn from(e: AxiomError) -> Self {
        match e {
            AxiomError::InvalidSlot => KernelError::InvalidCapability,
            AxiomError::WrongType => KernelError::InvalidCapability,
            AxiomError::InsufficientRights => KernelError::PermissionDenied,
            AxiomError::Expired => KernelError::PermissionDenied,
            AxiomError::ObjectNotFound => KernelError::InvalidCapability,
        }
    }
}

// ============================================================================
// Commit types for audit log
// ============================================================================

/// Commit types - describe state mutations for audit/replay
#[derive(Clone, Debug)]
pub enum CommitType {
    /// Genesis commit (initial state)
    Genesis,
    /// Process registered
    ProcessRegistered { pid: u64, name: String },
    /// Process exited
    ProcessExited { pid: u64, code: i32 },
    /// Process killed
    ProcessKilled { pid: u64, by: u64 },
    /// Endpoint created
    EndpointCreated { id: u64, owner: u64 },
    /// Endpoint deleted
    EndpointDeleted { id: u64 },
    /// IPC message sent
    IpcSent {
        from: u64,
        endpoint: u64,
        tag: u32,
        size: usize,
    },
    /// Capability granted
    CapGranted {
        from_pid: u64,
        to_pid: u64,
        slot: u32,
        object_id: u64,
    },
    /// Capability revoked
    CapRevoked { pid: u64, slot: u32 },
    /// Capability deleted
    CapDeleted { pid: u64, slot: u32 },
    /// Capability derived
    CapDerived {
        pid: u64,
        from_slot: u32,
        new_slot: u32,
    },
}

/// A commit record
#[derive(Clone, Debug)]
pub struct Commit {
    /// Commit ID (hash)
    pub id: [u8; 32],
    /// Previous commit ID
    pub prev_commit: [u8; 32],
    /// Sequence number
    pub seq: u64,
    /// Timestamp
    pub timestamp: u64,
    /// Type of mutation
    pub commit_type: CommitType,
    /// Optional: syscall that caused this commit
    pub caused_by: Option<u64>,
}

impl Commit {
    /// Create a new commit (ID will be computed by CommitLog)
    pub fn new(commit_type: CommitType, timestamp: u64) -> Self {
        Self {
            id: [0u8; 32],
            prev_commit: [0u8; 32],
            seq: 0,
            timestamp,
            commit_type,
            caused_by: None,
        }
    }
}

/// Result of a step operation
pub struct StepResult {
    /// The syscall result
    pub result: SyscallResult,
    /// Commits generated by this step
    pub commits: Vec<Commit>,
}

// ============================================================================
// The pure step function - THE verification target
// ============================================================================

/// Execute a syscall on the kernel state.
///
/// This is the pure state machine function. It:
/// - Takes the current state and a syscall
/// - Updates the state (via mutation)
/// - Returns the result and commits
///
/// # Properties (Verification Targets)
///
/// 1. **Deterministic**: Same state + syscall always produces same result
/// 2. **No side effects**: Only mutates the provided state
/// 3. **Authority checked**: All capability operations go through axiom_check
pub fn step(state: &mut KernelState, from_pid: ProcessId, syscall: Syscall, timestamp: u64) -> StepResult {
    // Update metrics
    state.update_syscall_metrics(from_pid, timestamp);

    match syscall {
        Syscall::Debug { .. } => StepResult {
            result: SyscallResult::Ok(0),
            commits: vec![],
        },

        Syscall::GetTime => StepResult {
            result: SyscallResult::Ok(timestamp),
            commits: vec![],
        },

        Syscall::Yield => StepResult {
            result: SyscallResult::Ok(0),
            commits: vec![],
        },

        Syscall::Exit { code } => step_exit(state, from_pid, code, timestamp),
        Syscall::Kill { target_pid } => step_kill(state, from_pid, target_pid, timestamp),
        Syscall::ListProcesses => step_list_processes(state),
        Syscall::CreateEndpoint => step_create_endpoint(state, from_pid, timestamp),
        Syscall::Send {
            endpoint_slot,
            tag,
            data,
        } => step_send(state, from_pid, endpoint_slot, tag, data, timestamp),
        Syscall::Receive { endpoint_slot } => step_receive(state, from_pid, endpoint_slot, timestamp),
        Syscall::SendWithCaps {
            endpoint_slot,
            tag,
            data,
            cap_slots,
        } => step_send_with_caps(state, from_pid, endpoint_slot, tag, data, cap_slots, timestamp),
        Syscall::Call {
            endpoint_slot,
            tag,
            data,
        } => step_call(state, from_pid, endpoint_slot, tag, data, timestamp),
        Syscall::ListCaps => step_list_caps(state, from_pid),
        Syscall::CapGrant {
            from_slot,
            to_pid,
            permissions,
        } => step_cap_grant(state, from_pid, from_slot, to_pid, permissions, timestamp),
        Syscall::CapRevoke { slot } => step_cap_revoke(state, from_pid, slot, timestamp),
        Syscall::CapDelete { slot } => step_cap_delete(state, from_pid, slot, timestamp),
        Syscall::CapInspect { slot } => step_cap_inspect(state, from_pid, slot),
        Syscall::CapDerive {
            slot,
            new_permissions,
        } => step_cap_derive(state, from_pid, slot, new_permissions, timestamp),
    }
}

// ============================================================================
// Syscall handlers
// ============================================================================

fn step_exit(state: &mut KernelState, from_pid: ProcessId, code: i32, timestamp: u64) -> StepResult {
    if let Some(proc) = state.processes.get_mut(&from_pid) {
        proc.state = ProcessState::Zombie;
    }

    StepResult {
        result: SyscallResult::Ok(code as u64),
        commits: vec![Commit::new(
            CommitType::ProcessExited {
                pid: from_pid.0,
                code,
            },
            timestamp,
        )],
    }
}

fn step_kill(
    state: &mut KernelState,
    from_pid: ProcessId,
    target_pid: ProcessId,
    timestamp: u64,
) -> StepResult {
    // Check if target exists
    if !state.processes.contains_key(&target_pid) {
        return StepResult {
            result: SyscallResult::Err(KernelError::ProcessNotFound),
            commits: vec![],
        };
    }

    // Kill the target
    state.kill_process(target_pid);

    StepResult {
        result: SyscallResult::Ok(0),
        commits: vec![Commit::new(
            CommitType::ProcessKilled {
                pid: target_pid.0,
                by: from_pid.0,
            },
            timestamp,
        )],
    }
}

fn step_list_processes(state: &KernelState) -> StepResult {
    let procs: Vec<_> = state
        .processes
        .iter()
        .map(|(pid, p)| (*pid, p.name.clone(), p.state))
        .collect();

    StepResult {
        result: SyscallResult::ProcessList(procs),
        commits: vec![],
    }
}

fn step_create_endpoint(state: &mut KernelState, from_pid: ProcessId, timestamp: u64) -> StepResult {
    // Verify process exists
    if !state.process_exists(from_pid) {
        return StepResult {
            result: SyscallResult::Err(KernelError::ProcessNotFound),
            commits: vec![],
        };
    }

    // Create endpoint
    let endpoint_id = state.alloc_endpoint_id();
    let endpoint = Endpoint::new(endpoint_id, from_pid);
    state.endpoints.insert(endpoint_id, endpoint);

    // Create capability for the endpoint
    let cap = Capability {
        id: state.alloc_cap_id(),
        object_type: ObjectType::Endpoint,
        object_id: endpoint_id.0,
        permissions: Permissions::full(),
        generation: 0,
        expires_at: 0,
    };

    let slot = state
        .get_cap_space_mut(from_pid)
        .map(|cs| cs.insert(cap))
        .unwrap_or(0);

    // Pack result as (slot << 32) | endpoint_id
    let result = ((slot as u64) << 32) | (endpoint_id.0 & 0xFFFFFFFF);

    StepResult {
        result: SyscallResult::Ok(result),
        commits: vec![Commit::new(
            CommitType::EndpointCreated {
                id: endpoint_id.0,
                owner: from_pid.0,
            },
            timestamp,
        )],
    }
}

fn step_send(
    state: &mut KernelState,
    from_pid: ProcessId,
    endpoint_slot: CapSlot,
    tag: u32,
    data: Vec<u8>,
    timestamp: u64,
) -> StepResult {
    use crate::types::MAX_MESSAGE_SIZE;

    // Check message size
    if data.len() > MAX_MESSAGE_SIZE {
        return StepResult {
            result: SyscallResult::Err(KernelError::MessageTooLarge),
            commits: vec![],
        };
    }

    // Verify capability
    let cspace = match state.get_cap_space(from_pid) {
        Some(cs) => cs,
        None => {
            return StepResult {
                result: SyscallResult::Err(KernelError::ProcessNotFound),
                commits: vec![],
            }
        }
    };

    let cap = match axiom_check(
        cspace,
        endpoint_slot,
        &Permissions::write_only(),
        Some(ObjectType::Endpoint),
        timestamp,
    ) {
        Ok(c) => c,
        Err(e) => {
            return StepResult {
                result: SyscallResult::Err(e.into()),
                commits: vec![],
            }
        }
    };

    let endpoint_id = EndpointId(cap.object_id);
    let data_size = data.len();

    // Enqueue message
    let endpoint = match state.get_endpoint_mut(endpoint_id) {
        Some(e) => e,
        None => {
            return StepResult {
                result: SyscallResult::Err(KernelError::EndpointNotFound),
                commits: vec![],
            }
        }
    };

    let msg = Message {
        sender: from_pid,
        tag,
        data,
        caps: vec![],
    };
    endpoint.enqueue(msg);

    // Update sender metrics
    if let Some(proc) = state.get_process_mut(from_pid) {
        proc.metrics.ipc_sent += 1;
        proc.metrics.ipc_bytes_sent += data_size as u64;
    }

    state.total_ipc_count += 1;

    StepResult {
        result: SyscallResult::Ok(0),
        commits: vec![Commit::new(
            CommitType::IpcSent {
                from: from_pid.0,
                endpoint: endpoint_id.0,
                tag,
                size: data_size,
            },
            timestamp,
        )],
    }
}

fn step_receive(
    state: &mut KernelState,
    from_pid: ProcessId,
    endpoint_slot: CapSlot,
    timestamp: u64,
) -> StepResult {
    // Verify capability
    let cspace = match state.get_cap_space(from_pid) {
        Some(cs) => cs,
        None => {
            return StepResult {
                result: SyscallResult::Err(KernelError::ProcessNotFound),
                commits: vec![],
            }
        }
    };

    let cap = match axiom_check(
        cspace,
        endpoint_slot,
        &Permissions::read_only(),
        Some(ObjectType::Endpoint),
        timestamp,
    ) {
        Ok(c) => c,
        Err(e) => {
            return StepResult {
                result: SyscallResult::Err(e.into()),
                commits: vec![],
            }
        }
    };

    let endpoint_id = EndpointId(cap.object_id);

    // Dequeue message
    let endpoint = match state.get_endpoint_mut(endpoint_id) {
        Some(e) => e,
        None => {
            return StepResult {
                result: SyscallResult::Err(KernelError::EndpointNotFound),
                commits: vec![],
            }
        }
    };

    match endpoint.dequeue() {
        Some(msg) => {
            // Update receiver metrics
            let data_size = msg.data.len() as u64;
            if let Some(proc) = state.get_process_mut(from_pid) {
                proc.metrics.ipc_received += 1;
                proc.metrics.ipc_bytes_received += data_size;
            }

            StepResult {
                result: SyscallResult::Message(msg),
                commits: vec![],
            }
        }
        None => StepResult {
            result: SyscallResult::WouldBlock,
            commits: vec![],
        },
    }
}

fn step_send_with_caps(
    state: &mut KernelState,
    from_pid: ProcessId,
    endpoint_slot: CapSlot,
    tag: u32,
    data: Vec<u8>,
    cap_slots: Vec<CapSlot>,
    timestamp: u64,
) -> StepResult {
    use crate::types::{MAX_CAPS_PER_MESSAGE, MAX_MESSAGE_SIZE};

    // Check limits
    if data.len() > MAX_MESSAGE_SIZE {
        return StepResult {
            result: SyscallResult::Err(KernelError::MessageTooLarge),
            commits: vec![],
        };
    }
    if cap_slots.len() > MAX_CAPS_PER_MESSAGE {
        return StepResult {
            result: SyscallResult::Err(KernelError::TooManyCaps),
            commits: vec![],
        };
    }

    // Verify endpoint capability
    let cspace = match state.get_cap_space(from_pid) {
        Some(cs) => cs,
        None => {
            return StepResult {
                result: SyscallResult::Err(KernelError::ProcessNotFound),
                commits: vec![],
            }
        }
    };

    let endpoint_cap = match axiom_check(
        cspace,
        endpoint_slot,
        &Permissions::write_only(),
        Some(ObjectType::Endpoint),
        timestamp,
    ) {
        Ok(c) => c.clone(),
        Err(e) => {
            return StepResult {
                result: SyscallResult::Err(e.into()),
                commits: vec![],
            }
        }
    };

    // Collect capabilities to transfer
    let mut transferred_caps = Vec::new();
    for &slot in &cap_slots {
        let cap = match cspace.get(slot) {
            Some(c) => c,
            None => {
                return StepResult {
                    result: SyscallResult::Err(KernelError::InvalidCapability),
                    commits: vec![],
                }
            }
        };
        transferred_caps.push(TransferredCap {
            cap_id: cap.id,
            object_type: cap.object_type,
            object_id: cap.object_id,
            permissions: cap.permissions,
        });
    }

    let endpoint_id = EndpointId(endpoint_cap.object_id);
    let data_size = data.len();

    // Enqueue message
    let endpoint = match state.get_endpoint_mut(endpoint_id) {
        Some(e) => e,
        None => {
            return StepResult {
                result: SyscallResult::Err(KernelError::EndpointNotFound),
                commits: vec![],
            }
        }
    };

    let msg = Message {
        sender: from_pid,
        tag,
        data,
        caps: transferred_caps,
    };
    endpoint.enqueue(msg);

    state.total_ipc_count += 1;

    StepResult {
        result: SyscallResult::Ok(0),
        commits: vec![Commit::new(
            CommitType::IpcSent {
                from: from_pid.0,
                endpoint: endpoint_id.0,
                tag,
                size: data_size,
            },
            timestamp,
        )],
    }
}

fn step_call(
    state: &mut KernelState,
    from_pid: ProcessId,
    endpoint_slot: CapSlot,
    tag: u32,
    data: Vec<u8>,
    timestamp: u64,
) -> StepResult {
    // Send the message first
    let send_result = step_send(state, from_pid, endpoint_slot, tag, data, timestamp);

    match send_result.result {
        SyscallResult::Ok(_) => StepResult {
            result: SyscallResult::WouldBlock, // Block waiting for reply
            commits: send_result.commits,
        },
        err => StepResult {
            result: err,
            commits: send_result.commits,
        },
    }
}

fn step_list_caps(state: &KernelState, from_pid: ProcessId) -> StepResult {
    let caps = state
        .get_cap_space(from_pid)
        .map(|cs| cs.list())
        .unwrap_or_default();

    StepResult {
        result: SyscallResult::CapList(caps),
        commits: vec![],
    }
}

fn step_cap_grant(
    state: &mut KernelState,
    from_pid: ProcessId,
    from_slot: CapSlot,
    to_pid: ProcessId,
    permissions: Permissions,
    timestamp: u64,
) -> StepResult {
    // Verify source capability has grant permission
    let source_cap = {
        let cspace = match state.get_cap_space(from_pid) {
            Some(cs) => cs,
            None => {
                return StepResult {
                    result: SyscallResult::Err(KernelError::ProcessNotFound),
                    commits: vec![],
                }
            }
        };

        let grant_perms = Permissions {
            read: false,
            write: false,
            grant: true,
        };

        match axiom_check(cspace, from_slot, &grant_perms, None, timestamp) {
            Ok(c) => c.clone(),
            Err(e) => {
                return StepResult {
                    result: SyscallResult::Err(e.into()),
                    commits: vec![],
                }
            }
        }
    };

    // Verify target process exists
    if !state.process_exists(to_pid) {
        return StepResult {
            result: SyscallResult::Err(KernelError::ProcessNotFound),
            commits: vec![],
        };
    }

    // Check that requested permissions are subset of source
    if !permissions.is_subset_of(&source_cap.permissions) {
        return StepResult {
            result: SyscallResult::Err(KernelError::PermissionDenied),
            commits: vec![],
        };
    }

    // Create new capability in target's cspace
    let new_cap = Capability {
        id: state.alloc_cap_id(),
        object_type: source_cap.object_type,
        object_id: source_cap.object_id,
        permissions,
        generation: source_cap.generation + 1,
        expires_at: source_cap.expires_at,
    };

    let new_slot = state
        .get_cap_space_mut(to_pid)
        .map(|cs| cs.insert(new_cap))
        .unwrap_or(0);

    StepResult {
        result: SyscallResult::Ok(new_slot as u64),
        commits: vec![Commit::new(
            CommitType::CapGranted {
                from_pid: from_pid.0,
                to_pid: to_pid.0,
                slot: new_slot,
                object_id: source_cap.object_id,
            },
            timestamp,
        )],
    }
}

fn step_cap_revoke(state: &mut KernelState, from_pid: ProcessId, slot: CapSlot, timestamp: u64) -> StepResult {
    // For now, revoke just removes the capability (full revocation tree would be more complex)
    let removed = state
        .get_cap_space_mut(from_pid)
        .and_then(|cs| cs.remove(slot));

    match removed {
        Some(_) => StepResult {
            result: SyscallResult::Ok(0),
            commits: vec![Commit::new(
                CommitType::CapRevoked {
                    pid: from_pid.0,
                    slot,
                },
                timestamp,
            )],
        },
        None => StepResult {
            result: SyscallResult::Err(KernelError::InvalidCapability),
            commits: vec![],
        },
    }
}

fn step_cap_delete(state: &mut KernelState, from_pid: ProcessId, slot: CapSlot, timestamp: u64) -> StepResult {
    let removed = state
        .get_cap_space_mut(from_pid)
        .and_then(|cs| cs.remove(slot));

    match removed {
        Some(_) => StepResult {
            result: SyscallResult::Ok(0),
            commits: vec![Commit::new(
                CommitType::CapDeleted {
                    pid: from_pid.0,
                    slot,
                },
                timestamp,
            )],
        },
        None => StepResult {
            result: SyscallResult::Err(KernelError::InvalidCapability),
            commits: vec![],
        },
    }
}

fn step_cap_inspect(state: &KernelState, from_pid: ProcessId, slot: CapSlot) -> StepResult {
    let cspace = match state.get_cap_space(from_pid) {
        Some(cs) => cs,
        None => {
            return StepResult {
                result: SyscallResult::Err(KernelError::ProcessNotFound),
                commits: vec![],
            }
        }
    };

    match cspace.get(slot) {
        Some(cap) => StepResult {
            result: SyscallResult::CapInfo(CapInfo {
                id: cap.id,
                object_type: cap.object_type as u8,
                object_id: cap.object_id,
                permissions: cap.permissions.to_byte(),
                generation: cap.generation,
                expires_at: cap.expires_at,
            }),
            commits: vec![],
        },
        None => StepResult {
            result: SyscallResult::Err(KernelError::InvalidCapability),
            commits: vec![],
        },
    }
}

fn step_cap_derive(
    state: &mut KernelState,
    from_pid: ProcessId,
    slot: CapSlot,
    new_permissions: Permissions,
    timestamp: u64,
) -> StepResult {
    // Get source capability
    let source_cap = {
        let cspace = match state.get_cap_space(from_pid) {
            Some(cs) => cs,
            None => {
                return StepResult {
                    result: SyscallResult::Err(KernelError::ProcessNotFound),
                    commits: vec![],
                }
            }
        };

        match cspace.get(slot) {
            Some(c) => c.clone(),
            None => {
                return StepResult {
                    result: SyscallResult::Err(KernelError::InvalidCapability),
                    commits: vec![],
                }
            }
        }
    };

    // Verify new permissions are subset of source
    if !new_permissions.is_subset_of(&source_cap.permissions) {
        return StepResult {
            result: SyscallResult::Err(KernelError::PermissionDenied),
            commits: vec![],
        };
    }

    // Create derived capability
    let new_cap = Capability {
        id: state.alloc_cap_id(),
        object_type: source_cap.object_type,
        object_id: source_cap.object_id,
        permissions: new_permissions,
        generation: source_cap.generation + 1,
        expires_at: source_cap.expires_at,
    };

    let new_slot = state
        .get_cap_space_mut(from_pid)
        .map(|cs| cs.insert(new_cap))
        .unwrap_or(0);

    StepResult {
        result: SyscallResult::Ok(new_slot as u64),
        commits: vec![Commit::new(
            CommitType::CapDerived {
                pid: from_pid.0,
                from_slot: slot,
                new_slot,
            },
            timestamp,
        )],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_step_get_time() {
        let mut state = KernelState::new();
        let pid = state.register_process("test", 1000);

        let result = step(&mut state, pid, Syscall::GetTime, 5000);

        match result.result {
            SyscallResult::Ok(t) => assert_eq!(t, 5000),
            _ => panic!("Expected Ok"),
        }
    }

    #[test]
    fn test_step_create_endpoint() {
        let mut state = KernelState::new();
        let pid = state.register_process("test", 1000);

        let result = step(&mut state, pid, Syscall::CreateEndpoint, 2000);

        match result.result {
            SyscallResult::Ok(packed) => {
                let endpoint_id = (packed & 0xFFFFFFFF) as u64;
                assert_eq!(endpoint_id, 1);
            }
            _ => panic!("Expected Ok"),
        }

        assert_eq!(result.commits.len(), 1);
        assert!(state.get_endpoint(EndpointId(1)).is_some());
    }

    #[test]
    fn test_step_send_receive() {
        let mut state = KernelState::new();
        let sender = state.register_process("sender", 1000);
        let receiver = state.register_process("receiver", 1000);

        // Create endpoint owned by receiver
        state.create_endpoint(receiver);
        let endpoint_id = EndpointId(1);

        // Give sender a capability to the endpoint
        let cap = Capability {
            id: state.alloc_cap_id(),
            object_type: ObjectType::Endpoint,
            object_id: endpoint_id.0,
            permissions: Permissions::full(),
            generation: 0,
            expires_at: 0,
        };
        let slot = state.get_cap_space_mut(sender).unwrap().insert(cap);

        // Give receiver a capability too
        let recv_cap = Capability {
            id: state.alloc_cap_id(),
            object_type: ObjectType::Endpoint,
            object_id: endpoint_id.0,
            permissions: Permissions::full(),
            generation: 0,
            expires_at: 0,
        };
        let recv_slot = state.get_cap_space_mut(receiver).unwrap().insert(recv_cap);

        // Send message
        let send_result = step(
            &mut state,
            sender,
            Syscall::Send {
                endpoint_slot: slot,
                tag: 42,
                data: vec![1, 2, 3],
            },
            2000,
        );
        assert!(matches!(send_result.result, SyscallResult::Ok(0)));

        // Receive message
        let recv_result = step(
            &mut state,
            receiver,
            Syscall::Receive {
                endpoint_slot: recv_slot,
            },
            3000,
        );
        match recv_result.result {
            SyscallResult::Message(msg) => {
                assert_eq!(msg.sender, sender);
                assert_eq!(msg.tag, 42);
                assert_eq!(msg.data, vec![1, 2, 3]);
            }
            _ => panic!("Expected Message"),
        }
    }

    #[test]
    fn test_step_capability_grant() {
        let mut state = KernelState::new();
        let granter = state.register_process("granter", 1000);
        let grantee = state.register_process("grantee", 1000);

        // Create endpoint
        state.create_endpoint(granter);
        let endpoint_id = EndpointId(1);

        // Give granter a capability with grant permission
        let cap = Capability {
            id: state.alloc_cap_id(),
            object_type: ObjectType::Endpoint,
            object_id: endpoint_id.0,
            permissions: Permissions::full(),
            generation: 0,
            expires_at: 0,
        };
        let slot = state.get_cap_space_mut(granter).unwrap().insert(cap);

        // Grant to grantee with reduced permissions
        let result = step(
            &mut state,
            granter,
            Syscall::CapGrant {
                from_slot: slot,
                to_pid: grantee,
                permissions: Permissions::read_only(),
            },
            2000,
        );

        match result.result {
            SyscallResult::Ok(new_slot) => {
                // Verify grantee has the capability
                let grantee_cap = state
                    .get_cap_space(grantee)
                    .unwrap()
                    .get(new_slot as u32)
                    .unwrap();
                assert_eq!(grantee_cap.object_id, endpoint_id.0);
                assert!(grantee_cap.permissions.read);
                assert!(!grantee_cap.permissions.write);
            }
            _ => panic!("Expected Ok"),
        }
    }

    // ========================================================================
    // Simple syscalls: Debug, Yield, Exit
    // ========================================================================

    #[test]
    fn test_step_debug() {
        let mut state = KernelState::new();
        let pid = state.register_process("test", 1000);

        let result = step(
            &mut state,
            pid,
            Syscall::Debug {
                msg: alloc::string::String::from("Hello debug!"),
            },
            2000,
        );

        assert!(matches!(result.result, SyscallResult::Ok(0)));
        assert!(result.commits.is_empty()); // Debug doesn't generate commits
    }

    #[test]
    fn test_step_yield() {
        let mut state = KernelState::new();
        let pid = state.register_process("test", 1000);

        let result = step(&mut state, pid, Syscall::Yield, 2000);

        assert!(matches!(result.result, SyscallResult::Ok(0)));
        assert!(result.commits.is_empty()); // Yield doesn't generate commits
    }

    #[test]
    fn test_step_exit() {
        let mut state = KernelState::new();
        let pid = state.register_process("test", 1000);

        // Process should be running
        assert_eq!(state.get_process(pid).unwrap().state, ProcessState::Running);

        let result = step(&mut state, pid, Syscall::Exit { code: 42 }, 2000);

        // Should return exit code
        match result.result {
            SyscallResult::Ok(code) => assert_eq!(code, 42),
            _ => panic!("Expected Ok with exit code"),
        }

        // Process should be zombie
        assert_eq!(state.get_process(pid).unwrap().state, ProcessState::Zombie);

        // Should generate commit
        assert_eq!(result.commits.len(), 1);
        match &result.commits[0].commit_type {
            CommitType::ProcessExited { pid: exit_pid, code } => {
                assert_eq!(*exit_pid, pid.0);
                assert_eq!(*code, 42);
            }
            _ => panic!("Expected ProcessExited commit"),
        }
    }

    #[test]
    fn test_step_exit_negative_code() {
        let mut state = KernelState::new();
        let pid = state.register_process("test", 1000);

        let result = step(&mut state, pid, Syscall::Exit { code: -1 }, 2000);

        // Should handle negative exit codes
        match result.result {
            SyscallResult::Ok(code) => assert_eq!(code as i64, -1i32 as i64),
            _ => panic!("Expected Ok with exit code"),
        }
    }

    // ========================================================================
    // Kill syscall tests
    // ========================================================================

    #[test]
    fn test_step_kill_success() {
        let mut state = KernelState::new();
        let killer = state.register_process("killer", 1000);
        let victim = state.register_process("victim", 1000);

        let result = step(
            &mut state,
            killer,
            Syscall::Kill { target_pid: victim },
            2000,
        );

        assert!(matches!(result.result, SyscallResult::Ok(0)));
        assert_eq!(state.get_process(victim).unwrap().state, ProcessState::Zombie);

        // Should generate commit
        assert_eq!(result.commits.len(), 1);
        match &result.commits[0].commit_type {
            CommitType::ProcessKilled { pid, by } => {
                assert_eq!(*pid, victim.0);
                assert_eq!(*by, killer.0);
            }
            _ => panic!("Expected ProcessKilled commit"),
        }
    }

    #[test]
    fn test_step_kill_target_not_found() {
        let mut state = KernelState::new();
        let pid = state.register_process("test", 1000);

        let result = step(
            &mut state,
            pid,
            Syscall::Kill {
                target_pid: ProcessId(999),
            },
            2000,
        );

        assert!(matches!(
            result.result,
            SyscallResult::Err(KernelError::ProcessNotFound)
        ));
        assert!(result.commits.is_empty());
    }

    #[test]
    fn test_step_kill_self() {
        let mut state = KernelState::new();
        let pid = state.register_process("test", 1000);

        // Process can kill itself
        let result = step(&mut state, pid, Syscall::Kill { target_pid: pid }, 2000);

        assert!(matches!(result.result, SyscallResult::Ok(0)));
        assert_eq!(state.get_process(pid).unwrap().state, ProcessState::Zombie);
    }

    // ========================================================================
    // ListProcesses tests
    // ========================================================================

    #[test]
    fn test_step_list_processes() {
        let mut state = KernelState::new();
        let pid1 = state.register_process("proc1", 1000);
        let pid2 = state.register_process("proc2", 1000);
        state.kill_process(pid2); // Make one a zombie

        let result = step(&mut state, pid1, Syscall::ListProcesses, 2000);

        match result.result {
            SyscallResult::ProcessList(procs) => {
                assert_eq!(procs.len(), 2);

                let proc1 = procs.iter().find(|(p, _, _)| *p == pid1).unwrap();
                assert_eq!(proc1.1, "proc1");
                assert_eq!(proc1.2, ProcessState::Running);

                let proc2 = procs.iter().find(|(p, _, _)| *p == pid2).unwrap();
                assert_eq!(proc2.1, "proc2");
                assert_eq!(proc2.2, ProcessState::Zombie);
            }
            _ => panic!("Expected ProcessList"),
        }
    }

    // ========================================================================
    // SendWithCaps tests
    // ========================================================================

    #[test]
    fn test_step_send_with_caps() {
        let mut state = KernelState::new();
        let sender = state.register_process("sender", 1000);
        let receiver = state.register_process("receiver", 1000);

        // Create endpoint
        state.create_endpoint(receiver);
        let endpoint_id = EndpointId(1);

        // Give sender capability to endpoint (with write permission)
        let endpoint_cap = Capability {
            id: state.alloc_cap_id(),
            object_type: ObjectType::Endpoint,
            object_id: endpoint_id.0,
            permissions: Permissions::full(),
            generation: 0,
            expires_at: 0,
        };
        let endpoint_slot = state.get_cap_space_mut(sender).unwrap().insert(endpoint_cap);

        // Create a capability to transfer
        state.create_endpoint(sender);
        let transfer_endpoint_id = EndpointId(2);
        let transfer_cap = Capability {
            id: state.alloc_cap_id(),
            object_type: ObjectType::Endpoint,
            object_id: transfer_endpoint_id.0,
            permissions: Permissions::read_only(),
            generation: 0,
            expires_at: 0,
        };
        let transfer_slot = state.get_cap_space_mut(sender).unwrap().insert(transfer_cap);

        // Sender should have the transfer cap
        assert!(state.get_cap_space(sender).unwrap().contains(transfer_slot));

        // Send message with capability
        let result = step(
            &mut state,
            sender,
            Syscall::SendWithCaps {
                endpoint_slot,
                tag: 42,
                data: vec![1, 2, 3],
                cap_slots: vec![transfer_slot],
            },
            2000,
        );

        assert!(matches!(result.result, SyscallResult::Ok(0)));

        // Message should be in endpoint queue with transferred caps
        let endpoint = state.get_endpoint(endpoint_id).unwrap();
        assert_eq!(endpoint.pending_messages.len(), 1);
        let msg = &endpoint.pending_messages[0];
        assert_eq!(msg.sender, sender);
        assert_eq!(msg.tag, 42);
        assert_eq!(msg.caps.len(), 1);
        assert_eq!(msg.caps[0].object_id, transfer_endpoint_id.0);
    }

    #[test]
    fn test_step_send_with_caps_too_many_caps() {
        use crate::types::MAX_CAPS_PER_MESSAGE;

        let mut state = KernelState::new();
        let sender = state.register_process("sender", 1000);
        let receiver = state.register_process("receiver", 1000);

        // Create endpoint
        state.create_endpoint(receiver);
        let endpoint_id = EndpointId(1);

        let endpoint_cap = Capability {
            id: state.alloc_cap_id(),
            object_type: ObjectType::Endpoint,
            object_id: endpoint_id.0,
            permissions: Permissions::full(),
            generation: 0,
            expires_at: 0,
        };
        let endpoint_slot = state.get_cap_space_mut(sender).unwrap().insert(endpoint_cap);

        // Create too many capabilities
        let mut cap_slots = Vec::new();
        for _ in 0..=MAX_CAPS_PER_MESSAGE {
            let cap = Capability {
                id: state.alloc_cap_id(),
                object_type: ObjectType::Endpoint,
                object_id: 100,
                permissions: Permissions::full(),
                generation: 0,
                expires_at: 0,
            };
            let slot = state.get_cap_space_mut(sender).unwrap().insert(cap);
            cap_slots.push(slot);
        }

        let result = step(
            &mut state,
            sender,
            Syscall::SendWithCaps {
                endpoint_slot,
                tag: 42,
                data: vec![],
                cap_slots,
            },
            2000,
        );

        assert!(matches!(
            result.result,
            SyscallResult::Err(KernelError::TooManyCaps)
        ));
    }

    #[test]
    fn test_step_send_with_caps_invalid_cap_slot() {
        let mut state = KernelState::new();
        let sender = state.register_process("sender", 1000);
        let receiver = state.register_process("receiver", 1000);

        // Create endpoint
        state.create_endpoint(receiver);
        let endpoint_id = EndpointId(1);

        let endpoint_cap = Capability {
            id: state.alloc_cap_id(),
            object_type: ObjectType::Endpoint,
            object_id: endpoint_id.0,
            permissions: Permissions::full(),
            generation: 0,
            expires_at: 0,
        };
        let endpoint_slot = state.get_cap_space_mut(sender).unwrap().insert(endpoint_cap);

        // Try to send with invalid cap slot
        let result = step(
            &mut state,
            sender,
            Syscall::SendWithCaps {
                endpoint_slot,
                tag: 42,
                data: vec![],
                cap_slots: vec![999], // Invalid slot
            },
            2000,
        );

        assert!(matches!(
            result.result,
            SyscallResult::Err(KernelError::InvalidCapability)
        ));
    }

    // ========================================================================
    // Message size limits tests
    // ========================================================================

    #[test]
    fn test_step_send_message_too_large() {
        use crate::types::MAX_MESSAGE_SIZE;

        let mut state = KernelState::new();
        let sender = state.register_process("sender", 1000);
        let receiver = state.register_process("receiver", 1000);

        // Create endpoint
        state.create_endpoint(receiver);
        let endpoint_id = EndpointId(1);

        let cap = Capability {
            id: state.alloc_cap_id(),
            object_type: ObjectType::Endpoint,
            object_id: endpoint_id.0,
            permissions: Permissions::full(),
            generation: 0,
            expires_at: 0,
        };
        let slot = state.get_cap_space_mut(sender).unwrap().insert(cap);

        // Try to send oversized message
        let result = step(
            &mut state,
            sender,
            Syscall::Send {
                endpoint_slot: slot,
                tag: 42,
                data: vec![0u8; MAX_MESSAGE_SIZE + 1],
            },
            2000,
        );

        assert!(matches!(
            result.result,
            SyscallResult::Err(KernelError::MessageTooLarge)
        ));
    }

    #[test]
    fn test_step_send_with_caps_message_too_large() {
        use crate::types::MAX_MESSAGE_SIZE;

        let mut state = KernelState::new();
        let sender = state.register_process("sender", 1000);
        let receiver = state.register_process("receiver", 1000);

        // Create endpoint
        state.create_endpoint(receiver);
        let endpoint_id = EndpointId(1);

        let cap = Capability {
            id: state.alloc_cap_id(),
            object_type: ObjectType::Endpoint,
            object_id: endpoint_id.0,
            permissions: Permissions::full(),
            generation: 0,
            expires_at: 0,
        };
        let slot = state.get_cap_space_mut(sender).unwrap().insert(cap);

        // Try to send oversized message
        let result = step(
            &mut state,
            sender,
            Syscall::SendWithCaps {
                endpoint_slot: slot,
                tag: 42,
                data: vec![0u8; MAX_MESSAGE_SIZE + 1],
                cap_slots: vec![],
            },
            2000,
        );

        assert!(matches!(
            result.result,
            SyscallResult::Err(KernelError::MessageTooLarge)
        ));
    }

    // ========================================================================
    // Call syscall tests
    // ========================================================================

    #[test]
    fn test_step_call_sends_and_blocks() {
        let mut state = KernelState::new();
        let caller = state.register_process("caller", 1000);
        let receiver = state.register_process("receiver", 1000);

        // Create endpoint
        state.create_endpoint(receiver);
        let endpoint_id = EndpointId(1);

        let cap = Capability {
            id: state.alloc_cap_id(),
            object_type: ObjectType::Endpoint,
            object_id: endpoint_id.0,
            permissions: Permissions::full(),
            generation: 0,
            expires_at: 0,
        };
        let slot = state.get_cap_space_mut(caller).unwrap().insert(cap);

        let result = step(
            &mut state,
            caller,
            Syscall::Call {
                endpoint_slot: slot,
                tag: 42,
                data: vec![1, 2, 3],
            },
            2000,
        );

        // Call should return WouldBlock
        assert!(matches!(result.result, SyscallResult::WouldBlock));

        // But message should be sent
        let endpoint = state.get_endpoint(endpoint_id).unwrap();
        assert_eq!(endpoint.pending_messages.len(), 1);

        // Should generate commit for the send
        assert!(!result.commits.is_empty());
    }

    // ========================================================================
    // CapRevoke tests
    // ========================================================================

    #[test]
    fn test_step_cap_revoke() {
        let mut state = KernelState::new();
        let pid = state.register_process("test", 1000);

        // Create endpoint and capability
        state.create_endpoint(pid);
        let endpoint_id = EndpointId(1);

        let cap = Capability {
            id: state.alloc_cap_id(),
            object_type: ObjectType::Endpoint,
            object_id: endpoint_id.0,
            permissions: Permissions::full(),
            generation: 0,
            expires_at: 0,
        };
        let slot = state.get_cap_space_mut(pid).unwrap().insert(cap);

        assert!(state.get_cap_space(pid).unwrap().contains(slot));

        let result = step(&mut state, pid, Syscall::CapRevoke { slot }, 2000);

        assert!(matches!(result.result, SyscallResult::Ok(0)));
        assert!(!state.get_cap_space(pid).unwrap().contains(slot));

        // Should generate commit
        assert_eq!(result.commits.len(), 1);
        match &result.commits[0].commit_type {
            CommitType::CapRevoked { pid: revoke_pid, slot: revoke_slot } => {
                assert_eq!(*revoke_pid, pid.0);
                assert_eq!(*revoke_slot, slot);
            }
            _ => panic!("Expected CapRevoked commit"),
        }
    }

    #[test]
    fn test_step_cap_revoke_invalid_slot() {
        let mut state = KernelState::new();
        let pid = state.register_process("test", 1000);

        let result = step(&mut state, pid, Syscall::CapRevoke { slot: 999 }, 2000);

        assert!(matches!(
            result.result,
            SyscallResult::Err(KernelError::InvalidCapability)
        ));
    }

    // ========================================================================
    // CapDelete tests
    // ========================================================================

    #[test]
    fn test_step_cap_delete() {
        let mut state = KernelState::new();
        let pid = state.register_process("test", 1000);

        // Create capability with no grant permission (to show delete doesn't require it)
        state.create_endpoint(pid);
        let endpoint_id = EndpointId(1);

        let cap = Capability {
            id: state.alloc_cap_id(),
            object_type: ObjectType::Endpoint,
            object_id: endpoint_id.0,
            permissions: Permissions::read_only(), // No grant permission
            generation: 0,
            expires_at: 0,
        };
        let slot = state.get_cap_space_mut(pid).unwrap().insert(cap);

        let result = step(&mut state, pid, Syscall::CapDelete { slot }, 2000);

        assert!(matches!(result.result, SyscallResult::Ok(0)));
        assert!(!state.get_cap_space(pid).unwrap().contains(slot));

        // Should generate commit
        assert_eq!(result.commits.len(), 1);
        match &result.commits[0].commit_type {
            CommitType::CapDeleted { pid: del_pid, slot: del_slot } => {
                assert_eq!(*del_pid, pid.0);
                assert_eq!(*del_slot, slot);
            }
            _ => panic!("Expected CapDeleted commit"),
        }
    }

    #[test]
    fn test_step_cap_delete_invalid_slot() {
        let mut state = KernelState::new();
        let pid = state.register_process("test", 1000);

        let result = step(&mut state, pid, Syscall::CapDelete { slot: 999 }, 2000);

        assert!(matches!(
            result.result,
            SyscallResult::Err(KernelError::InvalidCapability)
        ));
    }

    // ========================================================================
    // CapInspect tests
    // ========================================================================

    #[test]
    fn test_step_cap_inspect() {
        let mut state = KernelState::new();
        let pid = state.register_process("test", 1000);

        // Create capability
        let cap = Capability {
            id: 42,
            object_type: ObjectType::Endpoint,
            object_id: 100,
            permissions: Permissions { read: true, write: false, grant: true },
            generation: 5,
            expires_at: 9999,
        };
        let slot = state.get_cap_space_mut(pid).unwrap().insert(cap);

        let result = step(&mut state, pid, Syscall::CapInspect { slot }, 2000);

        match result.result {
            SyscallResult::CapInfo(info) => {
                assert_eq!(info.id, 42);
                assert_eq!(info.object_type, ObjectType::Endpoint as u8);
                assert_eq!(info.object_id, 100);
                assert_eq!(info.generation, 5);
                assert_eq!(info.expires_at, 9999);
                // Permissions: read=1, write=0, grant=4 => 0x05
                assert_eq!(info.permissions, 0x05);
            }
            _ => panic!("Expected CapInfo"),
        }

        // No commits for inspect (read-only)
        assert!(result.commits.is_empty());
    }

    #[test]
    fn test_step_cap_inspect_invalid_slot() {
        let mut state = KernelState::new();
        let pid = state.register_process("test", 1000);

        let result = step(&mut state, pid, Syscall::CapInspect { slot: 999 }, 2000);

        assert!(matches!(
            result.result,
            SyscallResult::Err(KernelError::InvalidCapability)
        ));
    }

    // ========================================================================
    // CapDerive tests
    // ========================================================================

    #[test]
    fn test_step_cap_derive() {
        let mut state = KernelState::new();
        let pid = state.register_process("test", 1000);

        // Create capability with full permissions
        state.create_endpoint(pid);
        let endpoint_id = EndpointId(1);

        let cap = Capability {
            id: state.alloc_cap_id(),
            object_type: ObjectType::Endpoint,
            object_id: endpoint_id.0,
            permissions: Permissions::full(),
            generation: 0,
            expires_at: 0,
        };
        let slot = state.get_cap_space_mut(pid).unwrap().insert(cap);

        // Derive with reduced permissions
        let result = step(
            &mut state,
            pid,
            Syscall::CapDerive {
                slot,
                new_permissions: Permissions::read_only(),
            },
            2000,
        );

        match result.result {
            SyscallResult::Ok(new_slot) => {
                // New capability should exist
                let derived = state.get_cap_space(pid).unwrap().get(new_slot as u32).unwrap();
                assert_eq!(derived.object_id, endpoint_id.0);
                assert!(derived.permissions.read);
                assert!(!derived.permissions.write);
                assert!(!derived.permissions.grant);
                // Original should still exist
                assert!(state.get_cap_space(pid).unwrap().contains(slot));
            }
            _ => panic!("Expected Ok with new slot"),
        }

        // Should generate commit
        assert_eq!(result.commits.len(), 1);
        match &result.commits[0].commit_type {
            CommitType::CapDerived { pid: derive_pid, from_slot, new_slot } => {
                assert_eq!(*derive_pid, pid.0);
                assert_eq!(*from_slot, slot);
                assert!(*new_slot != slot);
            }
            _ => panic!("Expected CapDerived commit"),
        }
    }

    #[test]
    fn test_step_cap_derive_cannot_escalate() {
        let mut state = KernelState::new();
        let pid = state.register_process("test", 1000);

        // Create capability with read-only permissions
        state.create_endpoint(pid);
        let endpoint_id = EndpointId(1);

        let cap = Capability {
            id: state.alloc_cap_id(),
            object_type: ObjectType::Endpoint,
            object_id: endpoint_id.0,
            permissions: Permissions::read_only(),
            generation: 0,
            expires_at: 0,
        };
        let slot = state.get_cap_space_mut(pid).unwrap().insert(cap);

        // Try to derive with more permissions (should fail)
        let result = step(
            &mut state,
            pid,
            Syscall::CapDerive {
                slot,
                new_permissions: Permissions::full(),
            },
            2000,
        );

        assert!(matches!(
            result.result,
            SyscallResult::Err(KernelError::PermissionDenied)
        ));
    }

    #[test]
    fn test_step_cap_derive_invalid_slot() {
        let mut state = KernelState::new();
        let pid = state.register_process("test", 1000);

        let result = step(
            &mut state,
            pid,
            Syscall::CapDerive {
                slot: 999,
                new_permissions: Permissions::read_only(),
            },
            2000,
        );

        assert!(matches!(
            result.result,
            SyscallResult::Err(KernelError::InvalidCapability)
        ));
    }

    // ========================================================================
    // ListCaps tests
    // ========================================================================

    #[test]
    fn test_step_list_caps() {
        let mut state = KernelState::new();
        let pid = state.register_process("test", 1000);

        // Create some capabilities
        let cap1 = Capability {
            id: 1,
            object_type: ObjectType::Endpoint,
            object_id: 100,
            permissions: Permissions::full(),
            generation: 0,
            expires_at: 0,
        };
        let cap2 = Capability {
            id: 2,
            object_type: ObjectType::Process,
            object_id: 200,
            permissions: Permissions::read_only(),
            generation: 0,
            expires_at: 0,
        };
        state.get_cap_space_mut(pid).unwrap().insert(cap1);
        state.get_cap_space_mut(pid).unwrap().insert(cap2);

        let result = step(&mut state, pid, Syscall::ListCaps, 2000);

        match result.result {
            SyscallResult::CapList(caps) => {
                assert_eq!(caps.len(), 2);
                assert!(caps.iter().any(|(_, c)| c.id == 1));
                assert!(caps.iter().any(|(_, c)| c.id == 2));
            }
            _ => panic!("Expected CapList"),
        }
    }

    // ========================================================================
    // Receive tests
    // ========================================================================

    #[test]
    fn test_step_receive_would_block() {
        let mut state = KernelState::new();
        let pid = state.register_process("test", 1000);

        // Create endpoint
        state.create_endpoint(pid);
        let endpoint_id = EndpointId(1);

        let cap = Capability {
            id: state.alloc_cap_id(),
            object_type: ObjectType::Endpoint,
            object_id: endpoint_id.0,
            permissions: Permissions::full(),
            generation: 0,
            expires_at: 0,
        };
        let slot = state.get_cap_space_mut(pid).unwrap().insert(cap);

        // No messages, should return WouldBlock
        let result = step(&mut state, pid, Syscall::Receive { endpoint_slot: slot }, 2000);

        assert!(matches!(result.result, SyscallResult::WouldBlock));
    }

    #[test]
    fn test_step_receive_process_not_found() {
        let mut state = KernelState::new();
        // Don't register a process

        let result = step(
            &mut state,
            ProcessId(999),
            Syscall::Receive { endpoint_slot: 0 },
            2000,
        );

        assert!(matches!(
            result.result,
            SyscallResult::Err(KernelError::ProcessNotFound)
        ));
    }

    #[test]
    fn test_step_receive_updates_metrics() {
        let mut state = KernelState::new();
        let sender = state.register_process("sender", 1000);
        let receiver = state.register_process("receiver", 1000);

        // Create endpoint
        state.create_endpoint(receiver);
        let endpoint_id = EndpointId(1);

        // Give sender and receiver capabilities
        let sender_cap = Capability {
            id: state.alloc_cap_id(),
            object_type: ObjectType::Endpoint,
            object_id: endpoint_id.0,
            permissions: Permissions::write_only(),
            generation: 0,
            expires_at: 0,
        };
        let sender_slot = state.get_cap_space_mut(sender).unwrap().insert(sender_cap);

        let receiver_cap = Capability {
            id: state.alloc_cap_id(),
            object_type: ObjectType::Endpoint,
            object_id: endpoint_id.0,
            permissions: Permissions::read_only(),
            generation: 0,
            expires_at: 0,
        };
        let receiver_slot = state.get_cap_space_mut(receiver).unwrap().insert(receiver_cap);

        // Send message
        step(
            &mut state,
            sender,
            Syscall::Send {
                endpoint_slot: sender_slot,
                tag: 42,
                data: vec![1, 2, 3, 4, 5],
            },
            2000,
        );

        // Initial metrics
        assert_eq!(state.get_process(receiver).unwrap().metrics.ipc_received, 0);

        // Receive message
        step(
            &mut state,
            receiver,
            Syscall::Receive { endpoint_slot: receiver_slot },
            3000,
        );

        // Metrics should be updated
        assert_eq!(state.get_process(receiver).unwrap().metrics.ipc_received, 1);
        assert_eq!(state.get_process(receiver).unwrap().metrics.ipc_bytes_received, 5);
    }

    // ========================================================================
    // Grant without grant permission tests
    // ========================================================================

    #[test]
    fn test_step_cap_grant_requires_grant_permission() {
        let mut state = KernelState::new();
        let granter = state.register_process("granter", 1000);
        let grantee = state.register_process("grantee", 1000);

        // Create capability WITHOUT grant permission
        state.create_endpoint(granter);
        let endpoint_id = EndpointId(1);

        let cap = Capability {
            id: state.alloc_cap_id(),
            object_type: ObjectType::Endpoint,
            object_id: endpoint_id.0,
            permissions: Permissions { read: true, write: true, grant: false },
            generation: 0,
            expires_at: 0,
        };
        let slot = state.get_cap_space_mut(granter).unwrap().insert(cap);

        let result = step(
            &mut state,
            granter,
            Syscall::CapGrant {
                from_slot: slot,
                to_pid: grantee,
                permissions: Permissions::read_only(),
            },
            2000,
        );

        assert!(matches!(
            result.result,
            SyscallResult::Err(KernelError::PermissionDenied)
        ));
    }

    #[test]
    fn test_step_cap_grant_to_nonexistent_process() {
        let mut state = KernelState::new();
        let granter = state.register_process("granter", 1000);

        // Create capability with grant permission
        state.create_endpoint(granter);
        let endpoint_id = EndpointId(1);

        let cap = Capability {
            id: state.alloc_cap_id(),
            object_type: ObjectType::Endpoint,
            object_id: endpoint_id.0,
            permissions: Permissions::full(),
            generation: 0,
            expires_at: 0,
        };
        let slot = state.get_cap_space_mut(granter).unwrap().insert(cap);

        let result = step(
            &mut state,
            granter,
            Syscall::CapGrant {
                from_slot: slot,
                to_pid: ProcessId(999),
                permissions: Permissions::read_only(),
            },
            2000,
        );

        assert!(matches!(
            result.result,
            SyscallResult::Err(KernelError::ProcessNotFound)
        ));
    }

    // ========================================================================
    // Create endpoint tests
    // ========================================================================

    #[test]
    fn test_step_create_endpoint_process_not_found() {
        let mut state = KernelState::new();

        let result = step(&mut state, ProcessId(999), Syscall::CreateEndpoint, 2000);

        assert!(matches!(
            result.result,
            SyscallResult::Err(KernelError::ProcessNotFound)
        ));
    }

    #[test]
    fn test_step_create_endpoint_for_zombie() {
        let mut state = KernelState::new();
        let pid = state.register_process("test", 1000);
        state.kill_process(pid); // Become zombie

        let result = step(&mut state, pid, Syscall::CreateEndpoint, 2000);

        // Zombie processes should not be able to create endpoints
        assert!(matches!(
            result.result,
            SyscallResult::Err(KernelError::ProcessNotFound)
        ));
    }

    // ========================================================================
    // Expired capability during send
    // ========================================================================

    #[test]
    fn test_step_send_with_expired_capability() {
        let mut state = KernelState::new();
        let sender = state.register_process("sender", 1000);
        let receiver = state.register_process("receiver", 1000);

        // Create endpoint
        state.create_endpoint(receiver);
        let endpoint_id = EndpointId(1);

        // Create capability that expires at 1000
        let cap = Capability {
            id: state.alloc_cap_id(),
            object_type: ObjectType::Endpoint,
            object_id: endpoint_id.0,
            permissions: Permissions::full(),
            generation: 0,
            expires_at: 1000,
        };
        let slot = state.get_cap_space_mut(sender).unwrap().insert(cap);

        // Try to send after expiry
        let result = step(
            &mut state,
            sender,
            Syscall::Send {
                endpoint_slot: slot,
                tag: 42,
                data: vec![1, 2, 3],
            },
            2000, // After expiry
        );

        assert!(matches!(
            result.result,
            SyscallResult::Err(KernelError::PermissionDenied)
        ));
    }
}
