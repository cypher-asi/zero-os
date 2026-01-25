//! App Framework
//!
//! Core types and traits for building Zero OS applications:
//!
//! - **ZeroApp**: The trait all apps implement
//! - **AppContext**: Execution context provided to app methods
//! - **AppRuntime**: Event loop that drives apps
//! - **AppManifest**: Declarative capability requirements

mod app;
mod error;
mod manifest;
mod runtime;

pub use app::{AppContext, ControlFlow, Message, SessionId, UserContext, UserId, ZeroApp};
pub use error::{AppError, ProtocolError};
pub use manifest::{
    AppManifest, CapabilityRequest, ObjectType, Permissions,
    // Factory manifests
    CALCULATOR_MANIFEST, CLOCK_MANIFEST, SETTINGS_MANIFEST, TERMINAL_MANIFEST,
};
pub use runtime::AppRuntime;

use zos_process as syscall;

/// Log a debug message with component prefix.
///
/// Outputs in format: `[component] message`
#[inline]
pub fn debug_log(component: &str, message: &str) {
    syscall::debug(&alloc::format!("[{}] {}", component, message));
}

/// Log a debug message with component and PID context.
///
/// Outputs in format: `[component PID=pid] message`
#[inline]
pub fn debug_log_with_pid(component: &str, pid: u32, message: &str) {
    syscall::debug(&alloc::format!("[{} PID={}] {}", component, pid, message));
}
