//! Zero OS Kernel Core - Pure State Machine for Formal Verification
//!
//! This crate contains the **pure, HAL-free** kernel state machine that serves
//! as the primary verification target for Zero OS.
//!
//! # Design Principles
//!
//! 1. **No HAL dependency**: All platform-specific code lives in `zos-kernel`
//! 2. **No I/O or side effects**: Pure state transformations only
//! 3. **Deterministic**: Same input always produces same output
//! 4. **Verifiable**: Small TCB (~1500 LOC) suitable for Kani/TLA+ proofs
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                    zos-kernel-core                          │
//! │                 (Pure State Machine)                        │
//! │                                                             │
//! │   ┌───────────────┐    ┌───────────────┐                   │
//! │   │  KernelState  │    │    step()     │                   │
//! │   │  - processes  │───▶│  Pure state   │                   │
//! │   │  - cap_spaces │    │  transformer  │                   │
//! │   │  - endpoints  │    └───────────────┘                   │
//! │   └───────────────┘                                         │
//! │                                                             │
//! │   ┌───────────────┐    ┌───────────────┐                   │
//! │   │  Capability   │    │  Invariants   │                   │
//! │   │  axiom_check  │    │  Assertions   │                   │
//! │   └───────────────┘    └───────────────┘                   │
//! └─────────────────────────────────────────────────────────────┘
//!                              │
//!                              │ used by
//!                              ▼
//! ┌─────────────────────────────────────────────────────────────┐
//! │                      zos-kernel                             │
//! │                  (Runtime Wrapper)                          │
//! │                                                             │
//! │   - HAL integration (debug output, timing)                  │
//! │   - CommitLog recording                                     │
//! │   - SysLog audit trail                                      │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Verification Strategy
//!
//! - **Kani proofs**: Capability soundness, no rights escalation
//! - **TLA+ specs**: IPC protocol correctness, no deadlock
//! - **Loom tests**: Concurrent data structure correctness
//!
//! # Module Organization
//!
//! - `types` - Core kernel types (ProcessId, EndpointId, etc.)
//! - `capability` - Capability tokens and `axiom_check` verification
//! - `state` - KernelState struct with all kernel data
//! - `step` - Pure `step(state, syscall) -> (state', result)` function
//! - `invariants` - Formal invariant assertions for verification

#![no_std]
extern crate alloc;

pub mod capability;
pub mod invariants;
pub mod state;
pub mod step;
pub mod types;

// Re-export all public types for convenient access
pub use capability::{axiom_check, AxiomError, Capability, CapabilitySpace};
pub use invariants::{check_all_invariants, InvariantViolation};
pub use state::KernelState;
pub use step::{step, Commit, CommitType, StepResult, Syscall, SyscallResult};
pub use types::{
    CapSlot, Endpoint, EndpointId, EndpointMetrics, Message, ObjectType, Permissions, Process,
    ProcessId, ProcessMetrics, ProcessState, SystemMetrics, TransferredCap, MAX_CAPS_PER_MESSAGE,
    MAX_MESSAGE_SIZE,
};
