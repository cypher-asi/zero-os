//! Terminal capability grants
//!
//! Handles granting capabilities for terminal processes.

use zos_kernel::ProcessId;

use crate::constants::TERMINAL_INPUT_SLOT;
use crate::supervisor::Supervisor;
use crate::util::log;

impl Supervisor {
    /// Grant capabilities for terminal process
    ///
    /// - Grant Init (PID 1) capability to terminal's input endpoint
    /// - Grant supervisor (PID 0) capability to terminal's input endpoint
    pub(in crate::supervisor) fn grant_terminal_capabilities(
        &mut self,
        terminal_pid: ProcessId,
    ) {
        let init_pid = ProcessId(1);
        let supervisor_pid = ProcessId(0);

        // Get terminal's input endpoint ID
        let endpoint_id = match self.system.get_cap_space(terminal_pid) {
            Some(cspace) => match cspace.get(TERMINAL_INPUT_SLOT) {
                Some(cap) => zos_kernel::EndpointId(cap.object_id),
                None => {
                    log(&format!(
                        "[supervisor] Terminal PID {} has no endpoint at slot {}",
                        terminal_pid.0, TERMINAL_INPUT_SLOT
                    ));
                    return;
                }
            },
            None => {
                log(&format!(
                    "[supervisor] Terminal PID {} has no CSpace",
                    terminal_pid.0
                ));
                return;
            }
        };

        // Grant Init capability to terminal's input endpoint
        match self.system.grant_capability_to_endpoint(
            terminal_pid,
            endpoint_id,
            init_pid,
            zos_kernel::Permissions {
                read: false,
                write: true, // Can send to terminal
                grant: false,
            },
        ) {
            Ok(slot) => {
                log(&format!(
                    "[supervisor] Granted terminal {} input cap to Init at slot {}",
                    terminal_pid.0, slot
                ));
            }
            Err(e) => {
                log(&format!(
                    "[supervisor] Failed to grant terminal cap to Init: {:?}",
                    e
                ));
            }
        }

        // Grant supervisor capability to terminal's input endpoint
        match self.system.grant_capability_to_endpoint(
            terminal_pid,
            endpoint_id,
            supervisor_pid,
            zos_kernel::Permissions {
                read: false,
                write: true, // Can send to terminal
                grant: false,
            },
        ) {
            Ok(slot) => {
                self.terminal_endpoint_slots.insert(terminal_pid.0, slot);
                log(&format!(
                    "[supervisor] Granted terminal {} input cap to supervisor at slot {}",
                    terminal_pid.0, slot
                ));
            }
            Err(e) => {
                log(&format!(
                    "[supervisor] Failed to grant terminal cap to supervisor: {:?}",
                    e
                ));
            }
        }
    }
}
