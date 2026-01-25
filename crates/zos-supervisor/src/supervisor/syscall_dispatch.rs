//! Syscall Dispatch
//!
//! This module handles syscall routing through the Axiom gateway.
//! All syscalls are routed through `kernel.execute_raw_syscall()` to ensure
//! proper audit logging via AxiomGateway.syscall().
//!
//! ## Special Syscall Handling
//!
//! Some syscalls require supervisor-level handling:
//! - SYS_DEBUG: Supervisor processes debug messages for actions like spawn requests
//! - SYS_EXIT: Supervisor must terminate the worker after kernel state update
//! - SYS_CONSOLE_WRITE: Supervisor delivers output to UI directly

use zos_kernel::ProcessId;

use super::Supervisor;
use crate::constants::{SYS_CONSOLE_WRITE, SYS_DEBUG, SYS_EXIT, SYS_IPC_RECEIVE};
use crate::util::log;

impl Supervisor {
    /// Process a syscall internally through the Axiom gateway.
    ///
    /// This method routes all syscalls through `kernel.execute_raw_syscall()`
    /// which ensures proper audit logging via AxiomGateway.syscall().
    pub(super) fn process_syscall_internal(
        &mut self,
        pid: ProcessId,
        syscall_num: u32,
        args: [u32; 3],
        data: &[u8],
    ) -> i32 {
        // Check if process exists in kernel
        if self.system.get_process(pid).is_none() {
            log(&format!(
                "[supervisor] Syscall from unknown process {}",
                pid.0
            ));
            return -1;
        }

        // Handle SYS_DEBUG specially - supervisor needs to process the message
        // before routing through the gateway
        if syscall_num == SYS_DEBUG {
            return self.handle_sys_debug(pid, data);
        }

        // Handle SYS_EXIT specially - need to kill worker after kernel operation
        if syscall_num == SYS_EXIT {
            return self.handle_sys_exit(pid, args[0]);
        }

        // Handle SYS_CONSOLE_WRITE specially - supervisor delivers to UI directly
        if syscall_num == SYS_CONSOLE_WRITE {
            return self.handle_sys_console_write(pid, data);
        }

        // Route all other syscalls through the Axiom gateway
        let args4 = [args[0], args[1], args[2], 0];
        let (result, _rich_result, response_data) =
            self.system.process_syscall(pid, syscall_num, args4, data);

        // DEBUG: Log response data for SYS_IPC_RECEIVE
        if syscall_num == SYS_IPC_RECEIVE && result == 1 {
            log(&format!(
                "[supervisor] DEBUG: process_syscall returned {} bytes for IPC_RECEIVE (result={})",
                response_data.len(),
                result
            ));
        }

        // Always write response data (even if empty) to clear stale data from previous syscalls.
        // This prevents the process from reading leftover data from a prior syscall
        // (e.g., SYS_DEBUG text being misinterpreted as an IPC message).
        self.system.hal().write_syscall_data(pid.0, &response_data);

        result as i32
    }

    /// Handle SYS_DEBUG syscall.
    ///
    /// Debug messages are used for inter-process communication with the supervisor:
    /// - Spawn requests (INIT:SPAWN:)
    /// - Capability operations (INIT:GRANT:, INIT:REVOKE:)
    /// - Permission responses
    /// - Service IPC responses
    /// - Console output
    pub(super) fn handle_sys_debug(&mut self, pid: ProcessId, data: &[u8]) -> i32 {
        let args4 = [0u32, 0, 0, 0];

        // Route through gateway for audit logging
        let (result, _, _) = self.system.process_syscall(pid, SYS_DEBUG, args4, data);

        // Process the debug message for supervisor-level actions
        if let Ok(s) = std::str::from_utf8(data) {
            self.dispatch_debug_message(pid, s);
        }

        // Clear data buffer to prevent stale debug message text from being
        // misinterpreted as IPC message data by subsequent syscalls
        self.system.hal().write_syscall_data(pid.0, &[]);

        result as i32
    }

    /// Handle SYS_EXIT syscall.
    ///
    /// Process exit requires both kernel state update (via gateway)
    /// and worker termination (via HAL).
    pub(super) fn handle_sys_exit(&mut self, pid: ProcessId, exit_code: u32) -> i32 {
        log(&format!(
            "[supervisor] Process {} exiting with code {}",
            pid.0, exit_code
        ));

        // Route through gateway for kernel state + audit logging
        let args4 = [exit_code, 0, 0, 0];
        let (result, _, _) = self.system.process_syscall(pid, SYS_EXIT, args4, &[]);

        // Route kill request through Init (except for Init itself)
        if pid.0 == 1 {
            // Init cannot kill itself via IPC, use direct kill
            self.kill_process_direct(pid);
        } else {
            // Route through Init for proper auditing
            self.kill_process_via_init(pid);
        }

        result as i32
    }

    /// Handle SYS_CONSOLE_WRITE syscall.
    ///
    /// Console output is delivered directly to the UI by the supervisor.
    /// Per kernel invariant, no buffering in the kernel - supervisor handles
    /// the output directly from the syscall data.
    pub(super) fn handle_sys_console_write(&mut self, pid: ProcessId, data: &[u8]) -> i32 {
        let args4 = [0u32, 0, 0, 0];

        // Route through gateway for audit logging
        let (result, _, _) = self.system.process_syscall(pid, SYS_CONSOLE_WRITE, args4, data);

        // Deliver console output directly to UI
        if let Ok(text) = std::str::from_utf8(data) {
            log(&format!(
                "[supervisor] Console output from PID {}: {} bytes",
                pid.0,
                text.len()
            ));
            // Route to process-specific callback (or fall back to global)
            self.write_console_to_process(pid.0, text);
        }

        result as i32
    }
}
