//! Syscall handling for Zero OS WASM processes
//!
//! This module provides the syscall dispatch and handling for processes
//! running in Web Workers.
//!
//! Syscalls use the SharedArrayBuffer polling path (poll_syscalls) which
//! provides better performance than the legacy postMessage approach.

use zos_kernel::{ProcessId, System};

use crate::hal::WasmHal;
use crate::util::log;

/// Handle capability grant request from Init process
/// Format: INIT:GRANT:{target_pid}:{from_slot}:{perms_byte}
pub(crate) fn handle_init_grant(system: &mut System<WasmHal>, msg: &str) {
    // Parse: INIT:GRANT:{target_pid}:{from_slot}:{perms_byte}
    let parts: Vec<&str> = msg[11..].split(':').collect();
    if parts.len() >= 3 {
        let target_pid = parts[0].parse::<u64>().ok();
        let from_slot = parts[1].parse::<u32>().ok();
        let perms_byte = parts[2].parse::<u8>().ok();

        if let (Some(pid), Some(slot), Some(p)) = (target_pid, from_slot, perms_byte) {
            // Decode permissions from byte
            let permissions = zos_kernel::Permissions {
                read: (p & 0x01) != 0,
                write: (p & 0x02) != 0,
                grant: (p & 0x04) != 0,
            };

            // Grant from init's capability to target process
            let init_pid = ProcessId(1);
            match system.grant_capability(init_pid, slot, ProcessId(pid), permissions) {
                Ok(new_slot) => {
                    log(&format!(
                        "[supervisor] Granted capability to PID {} at slot {} (from init slot {}, perms {:?})",
                        pid, new_slot, slot, permissions
                    ));
                }
                Err(e) => {
                    log(&format!(
                        "[supervisor] Grant failed for PID {} from slot {}: {:?}",
                        pid, slot, e
                    ));
                }
            }
        } else {
            log(&format!("[supervisor] Invalid INIT:GRANT format: {}", msg));
        }
    } else {
        log(&format!("[supervisor] Invalid INIT:GRANT format: {}", msg));
    }
}

/// Handle capability revoke request from Init process
/// Format: INIT:REVOKE:{target_pid}:{slot}
pub(crate) fn handle_init_revoke(system: &mut System<WasmHal>, msg: &str) {
    // Parse: INIT:REVOKE:{target_pid}:{slot}
    let parts: Vec<&str> = msg[12..].split(':').collect();
    if parts.len() >= 2 {
        let target_pid = parts[0].parse::<u64>().ok();
        let slot = parts[1].parse::<u32>().ok();

        if let (Some(pid), Some(s)) = (target_pid, slot) {
            // Use delete_capability for forceful removal (supervisor privilege)
            match system.delete_capability(ProcessId(pid), s) {
                Ok(()) => {
                    log(&format!(
                        "[supervisor] Revoked capability from PID {} slot {}",
                        pid, s
                    ));
                }
                Err(e) => {
                    log(&format!(
                        "[supervisor] Revoke failed for PID {} slot {}: {:?}",
                        pid, s, e
                    ));
                }
            }
        } else {
            log(&format!("[supervisor] Invalid INIT:REVOKE format: {}", msg));
        }
    } else {
        log(&format!("[supervisor] Invalid INIT:REVOKE format: {}", msg));
    }
}
