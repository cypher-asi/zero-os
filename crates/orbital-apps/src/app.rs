//! OrbitalApp Trait and Core Types
//!
//! Defines the interface that all Orbital applications implement.

use crate::error::AppError;
use crate::manifest::AppManifest;
use alloc::vec::Vec;

/// Execution context provided to app methods
#[derive(Clone, Debug)]
pub struct AppContext {
    /// This process's ID
    pub pid: u32,

    /// Monotonic uptime in nanoseconds (via SYS_GET_TIME)
    /// Suitable for measuring durations, scheduling
    pub uptime_ns: u64,

    /// Wall-clock time in milliseconds since Unix epoch (via SYS_GET_WALLCLOCK)
    /// Suitable for displaying time-of-day to users
    pub wallclock_ms: u64,

    /// Capability slot for communicating with UI (if connected)
    pub ui_endpoint: Option<u32>,

    /// Capability slot for receiving input
    pub input_endpoint: Option<u32>,
}

impl AppContext {
    /// Create a new context with the given values
    pub fn new(
        pid: u32,
        uptime_ns: u64,
        wallclock_ms: u64,
        ui_endpoint: Option<u32>,
        input_endpoint: Option<u32>,
    ) -> Self {
        Self {
            pid,
            uptime_ns,
            wallclock_ms,
            ui_endpoint,
            input_endpoint,
        }
    }
}

/// Control flow returned by update()
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ControlFlow {
    /// Continue to next update cycle
    Continue,

    /// Exit with the given code
    Exit(i32),

    /// Yield CPU, wait for next scheduling quantum
    Yield,
}

/// An IPC message received by the app
#[derive(Clone, Debug)]
pub struct Message {
    /// Message tag (identifies message type)
    pub tag: u32,

    /// Sender's process ID
    pub from_pid: u32,

    /// Message payload data
    pub data: Vec<u8>,
}

impl Message {
    /// Create a new message
    pub fn new(tag: u32, from_pid: u32, data: Vec<u8>) -> Self {
        Self { tag, from_pid, data }
    }
}

/// The Program Interface that all Orbital apps implement.
///
/// # Lifecycle
///
/// 1. **init()**: Called once when the app starts. Initialize state, set up IPC endpoints.
/// 2. **update()**: Called repeatedly in the event loop. Perform periodic work, update state.
/// 3. **on_message()**: Called when a message is received via IPC.
/// 4. **shutdown()**: Called before the app exits. Clean up resources.
///
/// # Example
///
/// ```ignore
/// use orbital_apps::*;
///
/// #[derive(Default)]
/// struct MyApp {
///     counter: u32,
/// }
///
/// impl OrbitalApp for MyApp {
///     fn manifest() -> &'static AppManifest {
///         &MY_MANIFEST
///     }
///
///     fn init(&mut self, _ctx: &AppContext) -> Result<(), AppError> {
///         Ok(())
///     }
///
///     fn update(&mut self, ctx: &AppContext) -> ControlFlow {
///         self.counter += 1;
///         ControlFlow::Yield
///     }
///
///     fn on_message(&mut self, _ctx: &AppContext, msg: Message) -> Result<(), AppError> {
///         // Handle incoming messages
///         Ok(())
///     }
///
///     fn shutdown(&mut self, _ctx: &AppContext) {
///         // Cleanup
///     }
/// }
/// ```
pub trait OrbitalApp {
    /// Returns the static application manifest.
    fn manifest() -> &'static AppManifest
    where
        Self: Sized;

    /// Called once when the app starts.
    /// Initialize state, set up IPC endpoints.
    fn init(&mut self, ctx: &AppContext) -> Result<(), AppError>;

    /// Called repeatedly in the event loop.
    /// Perform periodic work, update state.
    fn update(&mut self, ctx: &AppContext) -> ControlFlow;

    /// Called when a message is received via IPC.
    fn on_message(&mut self, ctx: &AppContext, msg: Message) -> Result<(), AppError>;

    /// Called before the app exits.
    /// Clean up resources.
    fn shutdown(&mut self, ctx: &AppContext);
}
