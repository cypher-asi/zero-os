//! Debug Message Dispatch
//!
//! This module handles parsing and dispatching of debug messages from processes.
//! Debug messages are used for inter-process communication with the supervisor:
//!
//! - Spawn requests (INIT:SPAWN:)
//! - Capability operations (INIT:GRANT:, INIT:REVOKE:)
//! - Permission responses
//! - Service IPC responses
//! - Console output

use zos_hal::HAL;
use zos_ipc::debug;
use zos_kernel::ProcessId;

use super::Supervisor;
use crate::syscall;
use crate::util::{hex_to_bytes, log};
use crate::worker::WasmProcessHandle;

impl Supervisor {
    /// Dispatch debug message to appropriate handler based on prefix.
    pub(super) fn dispatch_debug_message(&mut self, pid: ProcessId, msg: &str) {
        // Try each handler in order of specificity
        if let Some(service_name) = msg.strip_prefix(debug::INIT_SPAWN) {
            self.handle_debug_spawn(service_name);
        } else if msg.starts_with(debug::INIT_GRANT) {
            syscall::handle_init_grant(&mut self.system, msg);
        } else if msg.starts_with(debug::INIT_REVOKE) {
            syscall::handle_init_revoke(&mut self.system, msg);
        } else if let Some(rest) = msg.strip_prefix(debug::INIT_KILL_OK) {
            self.handle_init_kill_ok(rest);
        } else if let Some(rest) = msg.strip_prefix(debug::INIT_KILL_FAIL) {
            self.handle_init_kill_fail(rest);
        } else if msg.starts_with(debug::INIT_PERM_RESPONSE) {
            log(&format!("[supervisor] Permission response: {}", msg));
        } else if msg.starts_with(debug::INIT_PERM_LIST) {
            log(&format!("[supervisor] Permission list: {}", msg));
        } else if let Some(init_msg) = msg.strip_prefix(debug::INIT_PREFIX) {
            log(&format!("[init] {}", init_msg));
        } else if let Some(rest) = msg.strip_prefix(debug::SERVICE_RESPONSE) {
            self.handle_debug_service_response(rest);
        } else if let Some(rest) = msg.strip_prefix(debug::VFS_RESPONSE) {
            self.handle_debug_vfs_response(rest);
        } else if let Some(rest) = msg.strip_prefix(debug::KEYSTORE_RESPONSE) {
            self.handle_debug_keystore_response(rest);
        // Init-driven spawn protocol responses
        } else if let Some(rest) = msg.strip_prefix(debug::SPAWN_RESPONSE) {
            self.handle_init_spawn_response(rest);
        } else if let Some(rest) = msg.strip_prefix(debug::ENDPOINT_RESPONSE) {
            self.handle_init_endpoint_response(rest);
        } else if let Some(rest) = msg.strip_prefix(debug::CAP_RESPONSE) {
            self.handle_init_cap_response(rest);
        } else if let Some(rest) = msg.strip_prefix("ERROR:IPC_DELIVERY_FAILED:no_capability:") {
            self.handle_ipc_delivery_failed(rest);
        } else if msg.starts_with(debug::AGENT_LOG) {
            // #region agent log - debug mode instrumentation passthrough
            log(&format!("[AGENT_LOG] PID {} | {}", pid.0, msg));
            // #endregion
            self.handle_debug_console_output(pid, msg);
        } else {
            self.handle_debug_console_output(pid, msg);
        }
    }

    /// Handle IPC delivery failure from Init.
    ///
    /// When Init lacks a capability to deliver to a service, the supervisor
    /// re-grants the capability and notifies Init so it can retry.
    ///
    /// Format: "pid={pid}:slot={slot}:tag=0x{tag}"
    fn handle_ipc_delivery_failed(&mut self, rest: &str) {
        log(&format!(
            "[supervisor] IPC delivery failed, attempting capability recovery: {}",
            rest
        ));

        // Parse: pid={pid}:slot={slot}:tag=0x{tag}
        let parts: Vec<&str> = rest.split(':').collect();
        if parts.len() < 3 {
            log("[supervisor] IPC_DELIVERY_FAILED: malformed message");
            return;
        }

        let target_pid: u32 = match parts[0].strip_prefix("pid=").and_then(|s| s.parse().ok()) {
            Some(p) => p,
            None => {
                log("[supervisor] IPC_DELIVERY_FAILED: invalid pid");
                return;
            }
        };

        let _slot: u32 = match parts[1].strip_prefix("slot=").and_then(|s| s.parse().ok()) {
            Some(s) => s,
            None => {
                log("[supervisor] IPC_DELIVERY_FAILED: invalid slot");
                return;
            }
        };

        // Find the service name by PID
        let service_name = match self.system.get_process(ProcessId(target_pid as u64)) {
            Some(proc) => proc.name.clone(),
            None => {
                log(&format!(
                    "[supervisor] IPC_DELIVERY_FAILED: process {} not found",
                    target_pid
                ));
                return;
            }
        };

        log(&format!(
            "[supervisor] Re-granting Init capability to {} (PID {})",
            service_name, target_pid
        ));

        // Re-grant capability and trigger pending delivery retry
        // Uses MSG_SERVICE_CAP_GRANTED instead of PREREGISTER to trigger retry
        self.regrant_init_capability_to_service(&service_name, ProcessId(target_pid as u64));
    }

    /// Handle SPAWN:RESPONSE from Init (Init-driven spawn protocol).
    ///
    /// This is called when Init responds to MSG_SUPERVISOR_SPAWN_PROCESS.
    /// Format: hex-encoded [success: u8, pid: u32]
    fn handle_init_spawn_response(&mut self, hex_data: &str) {
        let bytes = match hex_to_bytes(hex_data) {
            Ok(b) => b,
            Err(_) => {
                log("[supervisor] SPAWN:RESPONSE invalid hex");
                return;
            }
        };

        if bytes.len() < 5 {
            log("[supervisor] SPAWN:RESPONSE too short");
            return;
        }

        let success = bytes[0];
        let pid = u32::from_le_bytes([bytes[1], bytes[2], bytes[3], bytes[4]]);

        if success == 1 {
            log(&format!(
                "[supervisor] Init-driven spawn: process registered with PID {}",
                pid
            ));
            // TODO: Continue spawn flow with pending spawn tracking
        } else {
            log("[supervisor] Init-driven spawn: registration failed");
        }
    }

    /// Handle ENDPOINT:RESPONSE from Init (Init-driven spawn protocol).
    ///
    /// This is called when Init responds to MSG_SUPERVISOR_CREATE_ENDPOINT.
    /// Format: hex-encoded [success: u8, endpoint_id: u64, slot: u32]
    fn handle_init_endpoint_response(&mut self, hex_data: &str) {
        let bytes = match hex_to_bytes(hex_data) {
            Ok(b) => b,
            Err(_) => {
                log("[supervisor] ENDPOINT:RESPONSE invalid hex");
                return;
            }
        };

        if bytes.len() < 13 {
            log("[supervisor] ENDPOINT:RESPONSE too short");
            return;
        }

        let success = bytes[0];
        let endpoint_id = u64::from_le_bytes([
            bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7], bytes[8],
        ]);
        let slot = u32::from_le_bytes([bytes[9], bytes[10], bytes[11], bytes[12]]);

        if success == 1 {
            log(&format!(
                "[supervisor] Init-driven spawn: endpoint {} created at slot {}",
                endpoint_id, slot
            ));
            // TODO: Continue spawn flow with pending spawn tracking
        } else {
            log("[supervisor] Init-driven spawn: endpoint creation failed");
        }
    }

    /// Handle CAP:RESPONSE from Init (Init-driven spawn protocol).
    ///
    /// This is called when Init responds to MSG_SUPERVISOR_GRANT_CAP.
    /// Format: hex-encoded [success: u8, new_slot: u32]
    fn handle_init_cap_response(&mut self, hex_data: &str) {
        let bytes = match hex_to_bytes(hex_data) {
            Ok(b) => b,
            Err(_) => {
                log("[supervisor] CAP:RESPONSE invalid hex");
                return;
            }
        };

        if bytes.len() < 5 {
            log("[supervisor] CAP:RESPONSE too short");
            return;
        }

        let success = bytes[0];
        let new_slot = u32::from_le_bytes([bytes[1], bytes[2], bytes[3], bytes[4]]);

        if success == 1 {
            log(&format!(
                "[supervisor] Init-driven spawn: capability granted at slot {}",
                new_slot
            ));
            // TODO: Continue spawn flow with pending spawn tracking
        } else {
            log("[supervisor] Init-driven spawn: capability grant failed");
        }
    }

    /// Handle INIT:KILL_OK from Init.
    ///
    /// This is called when Init successfully kills a process via SYS_KILL.
    /// After kernel-level cleanup is complete, we terminate the HAL worker.
    ///
    /// Format: "INIT:KILL_OK:{pid}"
    pub(super) fn handle_init_kill_ok(&mut self, pid_str: &str) {
        let target_pid: u64 = match pid_str.parse() {
            Ok(p) => p,
            Err(_) => {
                log(&format!(
                    "[supervisor] INIT:KILL_OK: invalid PID '{}'",
                    pid_str
                ));
                return;
            }
        };

        log(&format!(
            "[supervisor] Init confirmed kill of PID {}, terminating HAL worker",
            target_pid
        ));

        // Kernel process is dead, now cleanup the HAL worker
        let handle = WasmProcessHandle::new(target_pid);
        let _ = self.system.hal().kill_process(&handle);

        // Cleanup supervisor state
        self.cleanup_process_state(target_pid);
    }

    /// Handle INIT:KILL_FAIL from Init.
    ///
    /// This is called when Init fails to kill a process.
    ///
    /// Format: "INIT:KILL_FAIL:{pid}:{error_code}"
    pub(super) fn handle_init_kill_fail(&mut self, rest: &str) {
        log(&format!(
            "[supervisor] Init failed to kill process: {}",
            rest
        ));
        // Process might already be dead or invalid PID
        // No HAL cleanup needed since kernel kill failed
    }

    /// Handle INIT:SPAWN: debug message.
    fn handle_debug_spawn(&mut self, service_name: &str) {
        log(&format!(
            "[supervisor] Init requesting spawn of '{}'",
            service_name
        ));
        self.request_spawn(service_name, service_name);
    }

    /// Handle default debug message (console output).
    pub(super) fn handle_debug_console_output(&mut self, pid: ProcessId, msg: &str) {
        log(&format!("[process {}] {}", pid.0, msg));
        self.write_console(&format!("[P{}] {}\n", pid.0, msg));
        if msg.contains("========================================") {
            self.check_pingpong_complete(pid.0);
        }
    }
}
