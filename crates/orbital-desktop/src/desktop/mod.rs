//! Desktop management module
//!
//! Provides desktop (workspace) management with multiple infinite canvases.

mod desktop;
mod manager;
mod void;

pub use desktop::{Desktop, PersistedDesktop};
pub use manager::DesktopManager;
pub use void::VoidState;

/// Unique desktop identifier
pub type DesktopId = u32;
