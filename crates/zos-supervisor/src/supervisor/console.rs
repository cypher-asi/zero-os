//! Console Callback Management
//!
//! This module handles per-process console callbacks for routing console output
//! to the appropriate terminal UI component.
//!
//! Each terminal window registers its own callback with its process PID.
//! Console output from that process is routed only to its registered callback.

use wasm_bindgen::prelude::*;

use super::Supervisor;
use crate::util::log;

/// wasm_bindgen methods for console callback registration (exposed to JS)
#[wasm_bindgen]
impl Supervisor {
    /// Register a console callback for a specific process.
    ///
    /// Each terminal window should register its own callback with its process PID.
    /// Console output from that process will be routed only to its registered callback.
    pub fn register_console_callback(&mut self, pid: u64, callback: js_sys::Function) {
        log(&format!(
            "[supervisor] Registered console callback for PID {}",
            pid
        ));
        self.console_callbacks.insert(pid, callback);
    }

    /// Unregister the console callback for a specific process.
    ///
    /// Called when a terminal window is unmounted to clean up.
    pub fn unregister_console_callback(&mut self, pid: u64) {
        if self.console_callbacks.remove(&pid).is_some() {
            log(&format!(
                "[supervisor] Unregistered console callback for PID {}",
                pid
            ));
        }
    }
}

/// Internal console methods (not exposed to JS)
impl Supervisor {
    /// Write system messages to console buffer.
    ///
    /// System messages are buffered until a console callback is registered.
    /// For process-specific output, use write_console_to_process().
    pub(crate) fn write_console(&mut self, text: &str) {
        // Buffer system messages (pingpong test, spawn notifications, etc.)
        self.console_buffer.push(text.to_string());
    }

    /// Write console output to a specific process's callback.
    ///
    /// Each terminal window registers its own callback with its process PID.
    /// Console output from that process is routed only to its registered callback.
    pub(crate) fn write_console_to_process(&mut self, pid: u64, text: &str) {
        if let Some(callback) = self.console_callbacks.get(&pid) {
            let this = JsValue::null();
            let arg = JsValue::from_str(text);
            let _ = callback.call1(&this, &arg);
        } else {
            // Buffer if no callback available for this process
            self.console_buffer.push(text.to_string());
        }
    }
}
