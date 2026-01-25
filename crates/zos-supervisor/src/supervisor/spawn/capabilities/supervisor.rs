//! Supervisor capability grants
//!
//! Handles granting capabilities to the supervisor (PID 0) for Init and
//! PermissionService endpoints.

use zos_kernel::ProcessId;

use crate::constants::{INIT_ENDPOINT_SLOT, PS_INPUT_SLOT};
use crate::supervisor::Supervisor;
use crate::util::log;

impl Supervisor {
    /// Grant supervisor (PID 0) capability to Init's endpoint
    ///
    /// This enables the supervisor to send IPC messages to Init for operations
    /// that need capability-checked kernel access.
    pub(in crate::supervisor) fn grant_supervisor_capability_to_init(
        &mut self,
        init_pid: ProcessId,
    ) {
        let supervisor_pid = ProcessId(0);

        // Get Init's endpoint ID from slot 0
        let endpoint_id = match self.system.get_cap_space(init_pid) {
            Some(cspace) => match cspace.get(INIT_ENDPOINT_SLOT) {
                Some(cap) => zos_kernel::EndpointId(cap.object_id),
                None => {
                    log("[supervisor] Init has no endpoint at slot 0");
                    return;
                }
            },
            None => {
                log("[supervisor] Init has no CSpace");
                return;
            }
        };

        // Grant supervisor capability to Init's endpoint
        match self.system.grant_capability_to_endpoint(
            init_pid,
            endpoint_id,
            supervisor_pid,
            zos_kernel::Permissions {
                read: false,
                write: true, // Can send to Init
                grant: false,
            },
        ) {
            Ok(slot) => {
                self.init_endpoint_slot = Some(slot);
                log(&format!(
                    "[supervisor] Granted Init endpoint cap to supervisor at slot {}",
                    slot
                ));
            }
            Err(e) => {
                log(&format!(
                    "[supervisor] Failed to grant Init cap to supervisor: {:?}",
                    e
                ));
            }
        }
    }

    /// Grant supervisor (PID 0) capability to PermissionService's endpoint
    pub(in crate::supervisor) fn grant_supervisor_capability_to_ps(&mut self, ps_pid: ProcessId) {
        let supervisor_pid = ProcessId(0);

        // Get PS's endpoint ID from PS_INPUT_SLOT
        let endpoint_id = match self.system.get_cap_space(ps_pid) {
            Some(cspace) => match cspace.get(PS_INPUT_SLOT) {
                Some(cap) => zos_kernel::EndpointId(cap.object_id),
                None => {
                    log("[supervisor] PS has no endpoint at slot 1");
                    return;
                }
            },
            None => {
                log("[supervisor] PS has no CSpace");
                return;
            }
        };

        // Grant supervisor capability to PS's endpoint
        match self.system.grant_capability_to_endpoint(
            ps_pid,
            endpoint_id,
            supervisor_pid,
            zos_kernel::Permissions {
                read: false,
                write: true, // Can send to PS
                grant: false,
            },
        ) {
            Ok(slot) => {
                self.ps_endpoint_slot = Some(slot);
                log(&format!(
                    "[supervisor] Granted PS endpoint cap to supervisor at slot {}",
                    slot
                ));
            }
            Err(e) => {
                log(&format!(
                    "[supervisor] Failed to grant PS cap to supervisor: {:?}",
                    e
                ));
            }
        }
    }
}
