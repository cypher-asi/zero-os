//! Supervisor boot and initialization
//!
//! Handles kernel boot sequence and supervisor process initialization.

use orbital_kernel::ProcessId;
use wasm_bindgen::prelude::*;

use super::{log, Supervisor};

#[wasm_bindgen]
impl Supervisor {
    /// Boot the kernel
    #[wasm_bindgen]
    pub fn boot(&mut self) {
        log("[supervisor] Booting Orbital OS kernel...");

        self.write_console("Orbital OS Kernel Bootstrap\n");
        self.write_console("===========================\n\n");

        // Initialize supervisor as a kernel process (PID 0)
        self.initialize_supervisor_process();

        log("[supervisor] Boot complete - call spawn_init() to start init process");
    }

    /// Spawn the init process (PID 1)
    /// Call this after boot() and after setting the spawn callback
    #[wasm_bindgen]
    pub fn spawn_init(&mut self) {
        if self.init_spawned {
            log("[supervisor] Init already spawned");
            return;
        }

        log("[supervisor] Requesting init spawn...");
        self.write_console("Starting init process...\n");
        self.request_spawn("init", "init");
    }

    /// Initialize the supervisor as a kernel process (PID 0).
    ///
    /// The supervisor is registered in the process table for auditing purposes,
    /// but does NOT have endpoints or capabilities. It uses privileged kernel
    /// APIs instead of IPC for console I/O.
    pub(crate) fn initialize_supervisor_process(&mut self) {
        if self.supervisor_initialized {
            log("[supervisor] Already initialized");
            return;
        }

        // Register supervisor as PID 0 in the kernel (for auditing)
        self.supervisor_pid = self
            .kernel
            .register_process_with_pid(ProcessId(0), "supervisor");
        log(&format!(
            "[supervisor] Registered supervisor process as PID {}",
            self.supervisor_pid.0
        ));

        // Note: Supervisor does NOT create endpoints - it uses privileged kernel APIs:
        // - drain_console_output(): Get console output from processes
        // - deliver_console_input(): Send keyboard input to terminal

        self.supervisor_initialized = true;
        log("[supervisor] Supervisor initialized - uses privileged kernel APIs (no endpoints)");
    }
}
