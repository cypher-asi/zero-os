//! Calculator Application
//!
//! Basic arithmetic calculator. Demonstrates:
//! - Bidirectional IPC
//! - State management
//! - User input handling

mod state;

pub use state::CalculatorState;

use alloc::format;
use alloc::string::{String, ToString};
use crate::protocol::{tags, InputEvent};
use crate::framework::{
    AppContext, AppError, AppManifest, ControlFlow, Message, ZeroApp,
    CALCULATOR_MANIFEST,
};
use crate::syscall;

/// Calculator application state
#[derive(Default)]
pub struct CalculatorApp {
    /// Current display value
    display: String,

    /// Accumulator for calculations
    accumulator: f64,

    /// Pending operation
    pending_op: Option<char>,

    /// Whether we just completed an operation (next digit clears display)
    just_computed: bool,

    /// Error state
    has_error: bool,

    /// Memory register
    memory: f64,
}

impl CalculatorApp {
    fn handle_digit(&mut self, digit: char) {
        if self.has_error {
            return;
        }

        if self.just_computed || self.display == "0" {
            self.display.clear();
            self.just_computed = false;
        }

        // Limit display length
        if self.display.len() < 15 {
            self.display.push(digit);
        }
    }

    fn handle_operation(&mut self, op: char) {
        if self.has_error {
            return;
        }

        // Complete pending operation first
        if self.pending_op.is_some() {
            self.handle_equals();
        }

        // Parse current display as accumulator
        if let Ok(value) = self.display.parse::<f64>() {
            self.accumulator = value;
            self.pending_op = Some(op);
            self.just_computed = true;
        }
    }

    fn handle_equals(&mut self) {
        if self.has_error || self.pending_op.is_none() {
            return;
        }

        let current = match self.display.parse::<f64>() {
            Ok(v) => v,
            Err(_) => {
                self.has_error = true;
                self.display = String::from("Error");
                return;
            }
        };

        let result = match self.pending_op {
            Some('+') => self.accumulator + current,
            Some('-') => self.accumulator - current,
            Some('×') | Some('*') => self.accumulator * current,
            Some('÷') | Some('/') => {
                if current == 0.0 {
                    self.has_error = true;
                    self.display = String::from("Error");
                    self.pending_op = None;
                    return;
                }
                self.accumulator / current
            }
            _ => current,
        };

        // Format result
        self.display = if result.fract() == 0.0 && result.abs() < 1e15 {
            format!("{}", result as i64)
        } else {
            let s = format!("{:.8}", result);
            s.trim_end_matches('0').trim_end_matches('.').to_string()
        };

        self.accumulator = result;
        self.pending_op = None;
        self.just_computed = true;
    }

    fn handle_clear(&mut self) {
        self.display = String::from("0");
        self.accumulator = 0.0;
        self.pending_op = None;
        self.just_computed = false;
        self.has_error = false;
    }

    fn handle_button(&mut self, name: &str) {
        match name {
            // Digits
            "digit_0" => self.handle_digit('0'),
            "digit_1" => self.handle_digit('1'),
            "digit_2" => self.handle_digit('2'),
            "digit_3" => self.handle_digit('3'),
            "digit_4" => self.handle_digit('4'),
            "digit_5" => self.handle_digit('5'),
            "digit_6" => self.handle_digit('6'),
            "digit_7" => self.handle_digit('7'),
            "digit_8" => self.handle_digit('8'),
            "digit_9" => self.handle_digit('9'),
            "decimal" => {
                if !self.display.contains('.') {
                    self.handle_digit('.');
                }
            }

            // Operations
            "op_add" => self.handle_operation('+'),
            "op_sub" => self.handle_operation('-'),
            "op_mul" => self.handle_operation('×'),
            "op_div" => self.handle_operation('÷'),
            "op_equals" => self.handle_equals(),

            // Control
            "clear" => self.handle_clear(),
            "clear_entry" => {
                self.display = String::from("0");
                self.has_error = false;
            }
            "backspace" => {
                if !self.has_error && !self.just_computed && self.display.len() > 1 {
                    self.display.pop();
                } else {
                    self.display = String::from("0");
                }
            }
            "negate" => {
                if !self.has_error {
                    if self.display.starts_with('-') {
                        self.display = self.display[1..].to_string();
                    } else if self.display != "0" {
                        self.display = format!("-{}", self.display);
                    }
                }
            }

            _ => {}
        }
    }

    fn send_state(&self, ctx: &AppContext) -> Result<(), AppError> {
        let state = CalculatorState::new(
            self.display.clone(),
            self.pending_op,
            self.has_error,
            self.memory != 0.0,
        );

        let bytes = state.to_bytes();

        if let Some(slot) = ctx.ui_endpoint {
            syscall::send(slot, tags::MSG_APP_STATE, &bytes)
                .map_err(|e| AppError::IpcError(format!("Send failed: {}", e)))?;
        }

        Ok(())
    }
}

impl ZeroApp for CalculatorApp {
    fn manifest() -> &'static AppManifest {
        &CALCULATOR_MANIFEST
    }

    fn init(&mut self, ctx: &AppContext) -> Result<(), AppError> {
        self.display = String::from("0");
        self.send_state(ctx)
    }

    fn update(&mut self, _ctx: &AppContext) -> ControlFlow {
        ControlFlow::Yield
    }

    fn on_message(&mut self, ctx: &AppContext, msg: Message) -> Result<(), AppError> {
        if msg.tag == tags::MSG_APP_INPUT {
            // Decode input event
            let event = InputEvent::from_bytes(&msg.data)?;

            // Handle button press
            if let Some(name) = event.button_name() {
                self.handle_button(name);
                self.send_state(ctx)?;
            }
        }

        Ok(())
    }

    fn shutdown(&mut self, _ctx: &AppContext) {
        syscall::debug("Calculator: shutting down");
    }
}
