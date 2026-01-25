//! Worker Event Handling
//!
//! This module handles non-syscall events from Worker processes:
//! - Ready (worker initialized)
//! - MemoryUpdate (memory growth)
//! - Error (worker error)
//! - Terminated (worker exit)
//! - Yield (cooperative yield)
//!
//! Syscalls are handled separately via SharedArrayBuffer polling (poll_syscalls).

use zos_kernel::ProcessId;

use crate::util::log;
use crate::worker::{WorkerMessage, WorkerMessageType};

impl super::Supervisor {
    /// Process pending worker events (non-syscall messages).
    ///
    /// This handles worker lifecycle events that arrive via postMessage:
    /// - Ready: Worker initialized and reports memory size
    /// - MemoryUpdate: Worker memory grew
    /// - Error: Worker encountered an error
    /// - Terminated: Worker exited
    /// - Yield: Worker yielded (no-op, just acknowledgement)
    ///
    /// Syscalls use the SharedArrayBuffer polling path (poll_syscalls) instead.
    pub(super) fn process_worker_messages_internal(&mut self) -> usize {
        const MAX_MESSAGES_PER_BATCH: usize = 5000;

        let incoming = self.system.hal().incoming_messages();
        let messages: Vec<WorkerMessage> = {
            if let Ok(mut queue) = incoming.lock() {
                let take_count = queue.len().min(MAX_MESSAGES_PER_BATCH);
                queue.drain(..take_count).collect()
            } else {
                return 0;
            }
        };

        let count = messages.len();

        for msg in messages {
            match msg.msg_type {
                WorkerMessageType::Ready { memory_size } => {
                    self.system
                        .hal()
                        .update_process_memory(msg.pid, memory_size);
                    log(&format!(
                        "[supervisor] Worker {} ready, memory: {} bytes",
                        msg.pid, memory_size
                    ));
                }
                WorkerMessageType::Error { ref message } => {
                    log(&format!(
                        "[supervisor] Worker {} error: {}",
                        msg.pid, message
                    ));
                }
                WorkerMessageType::Terminated => {
                    log(&format!("[supervisor] Worker {} terminated", msg.pid));
                    let pid = ProcessId(msg.pid);
                    if self.system.get_process(pid).is_some() {
                        // Route through Init for proper auditing
                        self.cleanup_process_state(msg.pid);
                        if msg.pid == 1 {
                            self.kill_process_direct(pid);
                        } else {
                            self.kill_process_via_init(pid);
                        }
                    }
                }
                WorkerMessageType::MemoryUpdate { memory_size } => {
                    self.system
                        .hal()
                        .update_process_memory(msg.pid, memory_size);
                    let pid = ProcessId(msg.pid);
                    self.system.update_process_memory(pid, memory_size);
                }
                WorkerMessageType::Yield => {
                    // Worker yielded - nothing to do
                }
            }
        }

        count
    }
}
