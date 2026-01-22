//! Process spawning logic
//!
//! Handles spawning new processes and capability setup.

use orbital_kernel::ProcessId;
use wasm_bindgen::prelude::*;

use super::{log, Supervisor};
use crate::pingpong;

#[wasm_bindgen]
impl Supervisor {
    /// Complete spawning a process with the WASM binary
    /// Called by JS after fetching the WASM file
    #[wasm_bindgen]
    pub fn complete_spawn(&mut self, name: &str, wasm_binary: &[u8]) -> u64 {
        log(&format!(
            "[supervisor] complete_spawn called for '{}', {} bytes",
            name,
            wasm_binary.len()
        ));

        // First register in kernel to get a PID
        let kernel_pid = self.kernel.register_process(name);
        log(&format!(
            "[supervisor] Kernel assigned PID {} for '{}'",
            kernel_pid.0, name
        ));

        // Create endpoints for the process based on its role
        self.setup_process_endpoints(kernel_pid, name);

        // Spawn via HAL with the kernel PID
        match self
            .kernel
            .hal()
            .spawn_with_pid(kernel_pid.0, name, wasm_binary)
        {
            Ok(handle) => {
                log(&format!(
                    "[supervisor] Spawned Worker '{}' with PID {}",
                    name, handle.id
                ));

                // Track init spawn and grant capabilities
                self.setup_process_capabilities(kernel_pid, name);

                // Check if this is part of an automated pingpong test
                let pid = kernel_pid.0;
                self.on_process_spawned(name, pid);

                pid
            }
            Err(e) => {
                self.write_console(&format!("Error spawning {}: {:?}\n", name, e));
                log(&format!("[supervisor] Failed to spawn {}: {:?}", name, e));
                // Clean up kernel registration
                self.kernel.kill_process(kernel_pid);
                0
            }
        }
    }

    /// Set up endpoints for a process based on its role
    fn setup_process_endpoints(&mut self, kernel_pid: ProcessId, name: &str) {
        if name == "init" {
            // Init gets: slot 0 = init endpoint, slot 1 = console output
            if let Ok((eid, slot)) = self.kernel.create_endpoint(kernel_pid) {
                log(&format!(
                    "[supervisor] Created init endpoint {} at slot {} for init",
                    eid.0, slot
                ));
            }
            if let Ok((eid, slot)) = self.kernel.create_endpoint(kernel_pid) {
                log(&format!(
                    "[supervisor] Created console output endpoint {} at slot {} for init",
                    eid.0, slot
                ));
            }
        } else if name == "terminal" {
            self.setup_terminal_endpoints(kernel_pid);
        } else {
            // Other processes get two endpoints: output (slot 0) and input (slot 1)
            // This matches the app_main! macro which expects:
            // - Slot 0: UI output endpoint
            // - Slot 1: Input endpoint (for receiving messages)
            if let Ok((eid, slot)) = self.kernel.create_endpoint(kernel_pid) {
                log(&format!(
                    "[supervisor] Created output endpoint {} at slot {} for {}",
                    eid.0, slot, name
                ));
            }
            if let Ok((eid, slot)) = self.kernel.create_endpoint(kernel_pid) {
                log(&format!(
                    "[supervisor] Created input endpoint {} at slot {} for {}",
                    eid.0, slot, name
                ));
            }
        }
    }

    /// Set up terminal endpoints
    ///
    /// Terminal only needs its own input endpoint for receiving console input
    /// from the supervisor. Console output goes through SYS_CONSOLE_WRITE syscall.
    fn setup_terminal_endpoints(&mut self, kernel_pid: ProcessId) {
        // Terminal endpoint setup:
        // - Slot 0: Terminal's own endpoint (for general IPC if needed)
        // - Slot 1: Terminal's input endpoint (receives console input from supervisor)
        //
        // Note: Terminal does NOT need a capability to supervisor's endpoint.
        // Console output uses SYS_CONSOLE_WRITE syscall, which the kernel buffers
        // and the supervisor drains via drain_console_output().

        // Create terminal's primary endpoint at slot 0
        if let Ok((eid, slot)) = self.kernel.create_endpoint(kernel_pid) {
            log(&format!(
                "[supervisor] Created terminal endpoint {} at slot {} for terminal",
                eid.0, slot
            ));
        }

        // Create terminal's input endpoint at slot 1
        // Supervisor will deliver console input here via deliver_console_input()
        if let Ok((input_eid, slot)) = self.kernel.create_endpoint(kernel_pid) {
            log(&format!(
                "[supervisor] Created terminal input endpoint {} at slot {} for terminal",
                input_eid.0, slot
            ));
            
            // Note: Supervisor does NOT need a capability to this endpoint.
            // It uses the privileged deliver_console_input() API instead.
        }
    }

    /// Set up capabilities for a spawned process
    fn setup_process_capabilities(&mut self, kernel_pid: ProcessId, name: &str) {
        if name == "init" {
            self.init_spawned = true;
            log("[supervisor] Init process spawned (PID 1)");
        } else if self.init_spawned {
            // Grant this process a capability to init's endpoint (slot 0 of PID 1)
            let init_pid = ProcessId(1);
            match self.kernel.grant_capability(
                init_pid,
                0, // init's endpoint at slot 0
                kernel_pid,
                orbital_kernel::Permissions {
                    read: false,
                    write: true,
                    grant: false,
                },
            ) {
                Ok(slot) => {
                    log(&format!(
                        "[supervisor] Granted init endpoint cap to {} at slot {}",
                        name, slot
                    ));
                }
                Err(e) => {
                    log(&format!(
                        "[supervisor] Failed to grant init cap to {}: {:?}",
                        name, e
                    ));
                }
            }
        }
    }

    /// Called when a process is successfully spawned
    pub(crate) fn on_process_spawned(&mut self, name: &str, pid: u64) {
        // Check if we're waiting for this spawn as part of the pingpong test
        let (new_state, should_spawn_ponger) =
            pingpong::on_process_spawned(&self.pingpong_test, name, pid);

        if should_spawn_ponger {
            self.write_console(&format!("  Pinger spawned as PID {}\n", pid));
            self.pingpong_test = new_state;
            self.request_spawn("pingpong", "pp_ponger");
        } else if matches!(new_state, pingpong::PingPongTestState::SettingUpCaps { .. }) {
            self.write_console(&format!("  Ponger spawned as PID {}\n", pid));
            self.pingpong_test = new_state;
            self.progress_pingpong_test();
        } else {
            // Normal spawn, just report
            self.write_console(&format!("Spawned Worker '{}' as PID {}\n", name, pid));

            // NOTE: Terminal is now spawned per-window by the Desktop component
            // (no longer auto-spawned after init to enable process isolation)
            if name == "init" {
                log("[supervisor] Init started - terminal will be spawned per-window");
            }
        }
    }
}
