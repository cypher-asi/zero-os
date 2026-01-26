//! Zero OS Unsafe Primitives - Consolidated Unsafe Code TCB
//!
//! This crate contains ALL unsafe code in the Zero OS kernel, consolidated
//! into a single auditable location. All other crates should use
//! `#![deny(unsafe_code)]` (except where platform-specific FFI is required).
//!
//! # Design Principles
//!
//! 1. **Minimal unsafe surface**: Only truly necessary unsafe operations
//! 2. **Safe wrappers**: All unsafe is wrapped in safe interfaces
//! 3. **Auditable**: Small, focused modules for security review
//! 4. **Verified**: Kani proofs where applicable
//!
//! # Module Organization
//!
//! - `allocator` - Bump allocator implementation (GlobalAlloc unsafe impl)
//! - `ffi` - Safe wrappers for WASM host function FFI
//! - `sync` - Send/Sync wrapper types with safety invariants
//! - `loom_tests` - Concurrency tests using loom (with `loom` feature)
//!
//! # Verification
//!
//! This crate uses multiple verification approaches:
//!
//! 1. **Kani proofs** (`cargo kani`): Bounded model checking for safety properties
//! 2. **Loom tests** (`cargo test --features loom`): Concurrency testing
//! 3. **Unit tests**: Traditional testing for basic functionality

#![no_std]

pub mod allocator;
pub mod ffi;
pub mod sync;

#[cfg(any(test, feature = "loom"))]
mod loom_tests;

// Re-export commonly used items
pub use allocator::BumpAllocator;
pub use sync::SendSyncWrapper;
