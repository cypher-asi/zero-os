//! Zero OS Applications
//!
//! Each application has its own module containing:
//! - App implementation (`ZeroApp` trait impl)
//! - State types for UI communication

pub mod calculator;
pub mod clock;
pub mod settings;
pub mod terminal;

// Re-export app types for convenience
pub use calculator::CalculatorApp;
pub use clock::ClockApp;
pub use settings::SettingsApp;
pub use terminal::TerminalApp;

// Re-export state types (for UI/frontend consumption)
pub use calculator::CalculatorState;
pub use clock::ClockState;
pub use settings::{SettingsArea, SettingsState, SettingsStateBuilder};
pub use terminal::{InputAction, TerminalInput, TerminalState, MSG_CONSOLE_INPUT, TYPE_TERMINAL_INPUT, TYPE_TERMINAL_STATE};
