//! IPC protocol definitions for the Identity layer.
//!
//! Defines request/response types for inter-process communication.
//! Message constants are defined in `zos-ipc` (the single source of truth).

mod user;
mod session;
mod credentials;
mod keys;
mod zid;

// Re-export all types for backward compatibility
pub use user::*;
pub use session::*;
pub use credentials::*;
pub use keys::*;
pub use zid::*;
