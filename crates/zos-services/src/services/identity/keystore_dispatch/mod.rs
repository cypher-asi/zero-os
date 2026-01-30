//! Keystore Result Dispatch
//!
//! Handles keystore IPC responses and dispatches to appropriate handlers.
//! This is the keystore equivalent of vfs_dispatch.rs.
//!
//! # Invariant 32 Compliance
//!
//! All `/keys/` paths are handled via Keystore IPC, keeping cryptographic key
//! material separate from the general filesystem. This module processes responses
//! from KeystoreService (PID 7) and routes them to continuation handlers.

mod read;
mod write;
mod delete;
mod list;
mod exists;

use alloc::format;

use super::pending::PendingKeystoreOp;
use super::IdentityService;
use zos_apps::syscall;
use zos_apps::{AppError, Message};
use zos_ipc::keystore_svc;
use zos_vfs::client::keystore_async;

extern crate alloc;

impl IdentityService {
    /// Handle keystore IPC response messages
    ///
    /// This dispatches keystore responses to the appropriate continuation handlers
    /// based on the pending operation type.
    pub fn handle_keystore_result(&mut self, msg: &Message) -> Result<(), AppError> {
        syscall::debug(&format!(
            "IdentityService: Received keystore result tag=0x{:x} (pending_ops={})",
            msg.tag,
            self.pending_keystore_ops.len()
        ));

        match msg.tag {
            keystore_svc::MSG_KEYSTORE_READ_RESPONSE => self.handle_keystore_read_response(msg),
            keystore_svc::MSG_KEYSTORE_WRITE_RESPONSE => self.handle_keystore_write_response(msg),
            keystore_svc::MSG_KEYSTORE_DELETE_RESPONSE => self.handle_keystore_delete_response(msg),
            keystore_svc::MSG_KEYSTORE_EXISTS_RESPONSE => self.handle_keystore_exists_response(msg),
            keystore_svc::MSG_KEYSTORE_LIST_RESPONSE => self.handle_keystore_list_response(msg),
            _ => {
                syscall::debug(&format!(
                    "IdentityService: Unexpected keystore response tag 0x{:x}",
                    msg.tag
                ));
                Ok(())
            }
        }
    }

    /// Take the next pending keystore operation (FIFO order).
    /// Returns None if no operations are pending.
    pub(super) fn take_next_pending_keystore_op(&mut self) -> Option<PendingKeystoreOp> {
        // Get the smallest key (oldest operation)
        let key = *self.pending_keystore_ops.keys().next()?;
        self.pending_keystore_ops.remove(&key)
    }

    /// Handle keystore read response
    fn handle_keystore_read_response(&mut self, msg: &Message) -> Result<(), AppError> {
        let pending_op = match self.take_next_pending_keystore_op() {
            Some(op) => op,
            None => {
                syscall::debug("IdentityService: Keystore read response but no pending operation");
                return Ok(());
            }
        };

        let result = keystore_async::parse_read_response(&msg.data);
        read::dispatch_keystore_read_result(self, pending_op, result)
    }

    /// Handle keystore write response
    fn handle_keystore_write_response(&mut self, msg: &Message) -> Result<(), AppError> {
        let pending_op = match self.take_next_pending_keystore_op() {
            Some(op) => op,
            None => {
                syscall::debug("IdentityService: Keystore write response but no pending operation");
                return Ok(());
            }
        };

        let result = keystore_async::parse_write_response(&msg.data);
        write::dispatch_keystore_write_result(self, pending_op, result)
    }

    /// Handle keystore delete response
    fn handle_keystore_delete_response(&mut self, msg: &Message) -> Result<(), AppError> {
        let pending_op = match self.take_next_pending_keystore_op() {
            Some(op) => op,
            None => {
                syscall::debug("IdentityService: Keystore delete response but no pending operation");
                return Ok(());
            }
        };

        let result = keystore_async::parse_delete_response(&msg.data);
        delete::dispatch_keystore_delete_result(self, pending_op, result)
    }

    /// Handle keystore exists response
    fn handle_keystore_exists_response(&mut self, msg: &Message) -> Result<(), AppError> {
        let pending_op = match self.take_next_pending_keystore_op() {
            Some(op) => op,
            None => {
                syscall::debug("IdentityService: Keystore exists response but no pending operation");
                return Ok(());
            }
        };

        let result = keystore_async::parse_exists_response(&msg.data);
        exists::dispatch_keystore_exists_result(self, pending_op, result)
    }

    /// Handle keystore list response
    fn handle_keystore_list_response(&mut self, msg: &Message) -> Result<(), AppError> {
        let pending_op = match self.take_next_pending_keystore_op() {
            Some(op) => op,
            None => {
                syscall::debug("IdentityService: Keystore list response but no pending operation");
                return Ok(());
            }
        };

        let result = keystore_async::parse_list_response(&msg.data);
        list::dispatch_keystore_list_result(self, pending_op, result)
    }
}
