//! Zero OS Application Framework
//!
//! This crate provides the platform-agnostic application model for Zero OS:
//!
//! - **framework**: Core types and traits (ZeroApp, AppContext, AppRuntime, AppManifest)
//! - **protocol**: Wire format for Backend ↔ UI communication
//! - **apps**: Built-in applications (Calculator, Clock, Settings, Terminal)
//!
//! # Example
//!
//! ```ignore
//! use zos_apps::{app_main, ZeroApp, AppContext, ControlFlow, AppError, Message};
//!
//! #[derive(Default)]
//! struct MyApp { /* state */ }
//!
//! impl ZeroApp for MyApp {
//!     fn manifest() -> &'static AppManifest { &MY_APP_MANIFEST }
//!     fn init(&mut self, _ctx: &AppContext) -> Result<(), AppError> { Ok(()) }
//!     fn update(&mut self, _ctx: &AppContext) -> ControlFlow { ControlFlow::Yield }
//!     fn on_message(&mut self, _ctx: &AppContext, _msg: Message) -> Result<(), AppError> { Ok(()) }
//!     fn shutdown(&mut self, _ctx: &AppContext) {}
//! }
//!
//! app_main!(MyApp);
//! ```

// Note: Library uses std, but the app_main! macro sets up no_std for binaries
extern crate alloc;

pub mod apps;
pub mod framework;
pub mod protocol;

// Re-export core types at crate root for convenience
pub use framework::{
    AppContext, AppError, AppManifest, AppRuntime, CapabilityRequest, ControlFlow, Message,
    ObjectType, Permissions, ProtocolError, SessionId, UserContext, UserId, ZeroApp,
    // Factory manifests
    CALCULATOR_MANIFEST, CLOCK_MANIFEST, SETTINGS_MANIFEST, TERMINAL_MANIFEST,
    // Debug helpers
    debug_log, debug_log_with_pid,
};

// Re-export protocol types for convenience
pub use protocol::{tags, type_tags, InputEvent, WireSerializable};

// Re-export app state types (for UI/frontend consumption)
pub use apps::{
    CalculatorState, ClockState, InputAction, SettingsArea, SettingsState,
    TerminalInput, TerminalState, MSG_CONSOLE_INPUT, TYPE_TERMINAL_INPUT, TYPE_TERMINAL_STATE,
};

// Re-export syscall interface from zos-process
pub use zos_process as syscall;

// Re-export IPC protocol modules from zos-process (which re-exports from zos-ipc)
// This allows apps to use consistent message constants.
pub use zos_process::{init, kernel, permission, pm, storage, supervisor};

// Re-export canonical slot constants from zos-ipc (single source of truth)
pub use zos_ipc::slots::{INPUT_ENDPOINT_SLOT, OUTPUT_ENDPOINT_SLOT, VFS_RESPONSE_SLOT};

/// Generate the entry point and runtime setup for a Zero app.
///
/// This macro eliminates boilerplate by generating:
/// - The `_start` entry point
/// - Panic handler
/// - Global allocator
///
/// # Usage
///
/// ```ignore
/// use zos_apps::{app_main, ZeroApp};
///
/// #[derive(Default)]
/// struct MyApp;
///
/// impl ZeroApp for MyApp {
///     // ... trait implementation
/// }
///
/// app_main!(MyApp);
/// ```
#[macro_export]
macro_rules! app_main {
    ($app_type:ty) => {
        /// Entry point called by the WASM runtime
        #[no_mangle]
        pub extern "C" fn _start() {
            use alloc::format;
            
            // Create app instance
            let app = <$app_type>::default();

            // Create and run runtime
            let mut runtime = $crate::AppRuntime::new();

            // Set app ID from manifest
            let manifest = <$app_type as $crate::ZeroApp>::manifest();
            runtime.set_app_id(manifest.id);

            // Setup endpoints from capability slots (using canonical constants from zos-ipc).
            // The supervisor creates two endpoints for each process:
            // - Slot 0: UI output endpoint (for sending state updates)
            // - Slot 1: Input endpoint (for receiving messages)
            runtime.set_ui_endpoint($crate::OUTPUT_ENDPOINT_SLOT);
            runtime.set_input_endpoint($crate::INPUT_ENDPOINT_SLOT);
            
            $crate::syscall::debug(&format!(
                "[{}] starting (PID={})",
                manifest.id,
                runtime.pid()
            ));

            // Run forever (exits via syscall::exit)
            runtime.run(app);
        }
    };
}
