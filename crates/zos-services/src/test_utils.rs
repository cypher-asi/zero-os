//! Test utilities for service unit tests.
//!
//! These helpers create mock IPC messages without requiring
//! a running supervisor or syscall environment.

extern crate alloc;

use alloc::vec::Vec;
use zos_apps::Message;

/// Create a mock IPC message with empty capability slots.
pub fn mock_message(tag: u32, from_pid: u32, data: Vec<u8>) -> Message {
    Message {
        tag,
        from_pid,
        cap_slots: Vec::new(),
        data,
    }
}

/// Create a mock IPC message with provided capability slots.
pub fn mock_message_with_caps(
    tag: u32,
    from_pid: u32,
    cap_slots: Vec<u32>,
    data: Vec<u8>,
) -> Message {
    Message {
        tag,
        from_pid,
        cap_slots,
        data,
    }
}
