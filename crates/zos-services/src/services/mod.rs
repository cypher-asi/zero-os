//! Zero OS System Services
//!
//! Each service has its own module containing:
//! - Service implementation (`ZeroApp` trait impl)
//! - Handlers, state types, and response helpers
//!
//! # Services
//!
//! - **identity**: User identity and cryptographic key management (PID 3)
//! - **vfs**: Virtual filesystem operations (PID 4)
//! - **permission**: System capability authority (PID 2)
//! - **time**: Time settings management (PID 5)
//! - **network**: HTTP request mediation (PID 6)

pub mod identity;
pub mod network;
pub mod permission;
pub mod time;
pub mod vfs;

// Re-export service types for convenience
pub use identity::IdentityService;
pub use network::NetworkService;
pub use permission::PermissionService;
pub use time::TimeService;
pub use vfs::VfsService;
