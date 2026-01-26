//! Safe wrappers for WASM host function FFI
//!
//! This module provides safe Rust wrappers around the unsafe FFI calls
//! to WASM host functions (JavaScript runtime).
//!
//! # Safety Design
//!
//! The unsafe FFI declarations are isolated here. The public API provides
//! safe wrappers that:
//! 1. Validate inputs before calling FFI
//! 2. Handle null/error returns gracefully
//! 3. Document all invariants and assumptions
//!
//! # Usage
//!
//! Instead of calling unsafe FFI directly, use the safe wrappers:
//! ```ignore
//! // Instead of unsafe { zos_syscall(...) }
//! ffi::syscall(num, arg1, arg2, arg3);
//! ```

// ============================================================================
// FFI Declarations (unsafe)
// ============================================================================

#[cfg(target_arch = "wasm32")]
extern "C" {
    /// Make a syscall to the kernel.
    /// Returns a handle or error code.
    ///
    /// # Safety
    /// - Arguments must be valid syscall parameters
    /// - Caller must handle all possible return values
    fn zos_syscall(syscall_num: u32, arg1: u32, arg2: u32, arg3: u32) -> u32;

    /// Send bytes to the kernel (for syscall data).
    ///
    /// # Safety
    /// - ptr must be a valid pointer to len bytes
    /// - Memory must be readable for the duration of the call
    fn zos_send_bytes(ptr: *const u8, len: u32);

    /// Get bytes from the kernel (for syscall results).
    /// Returns the number of bytes written.
    ///
    /// # Safety
    /// - ptr must be a valid pointer with at least max_len bytes of writable memory
    fn zos_recv_bytes(ptr: *mut u8, max_len: u32) -> u32;

    /// Yield to allow other processes to run.
    ///
    /// # Safety
    /// - Always safe to call (no parameters, no memory access)
    fn zos_yield();

    /// Get the process's assigned PID.
    ///
    /// # Safety
    /// - Always safe to call
    fn zos_get_pid() -> u32;
}

// ============================================================================
// Safe Wrappers
// ============================================================================

/// Make a syscall to the kernel (safe wrapper).
///
/// This is the primary entry point for process-to-kernel communication.
#[cfg(target_arch = "wasm32")]
pub fn syscall(syscall_num: u32, arg1: u32, arg2: u32, arg3: u32) -> u32 {
    // SAFETY: All arguments are plain u32 values with no memory access requirements.
    // The syscall mechanism is designed to be safe for any combination of arguments.
    unsafe { zos_syscall(syscall_num, arg1, arg2, arg3) }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn syscall(_syscall_num: u32, _arg1: u32, _arg2: u32, _arg3: u32) -> u32 {
    0 // Mock for non-WASM
}

/// Send bytes to the kernel (safe wrapper).
///
/// Sends a byte slice as syscall data.
#[cfg(target_arch = "wasm32")]
pub fn send_bytes(data: &[u8]) {
    if data.is_empty() {
        return;
    }
    // SAFETY: We pass a valid slice pointer and its exact length.
    // The slice is borrowed for the duration of this call, ensuring the memory is valid.
    unsafe {
        zos_send_bytes(data.as_ptr(), data.len() as u32);
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn send_bytes(_data: &[u8]) {
    // No-op for non-WASM
}

/// Receive bytes from the kernel (safe wrapper).
///
/// Fills the provided buffer with syscall response data.
/// Returns the number of bytes actually received.
#[cfg(target_arch = "wasm32")]
pub fn recv_bytes(buffer: &mut [u8]) -> usize {
    if buffer.is_empty() {
        return 0;
    }
    // SAFETY: We pass a valid mutable slice pointer and its length.
    // The slice is exclusively borrowed, ensuring no aliasing.
    let received = unsafe { zos_recv_bytes(buffer.as_mut_ptr(), buffer.len() as u32) };
    received as usize
}

#[cfg(not(target_arch = "wasm32"))]
pub fn recv_bytes(_buffer: &mut [u8]) -> usize {
    0 // Mock for non-WASM
}

/// Yield CPU to other processes (safe wrapper).
pub fn yield_now() {
    #[cfg(target_arch = "wasm32")]
    // SAFETY: This function has no parameters and performs no memory access.
    // It's always safe to yield.
    unsafe {
        zos_yield();
    }
}

/// Get current process ID (safe wrapper).
pub fn get_pid() -> u32 {
    #[cfg(target_arch = "wasm32")]
    // SAFETY: This function has no parameters and performs no memory access.
    // It simply returns a u32 value.
    unsafe {
        zos_get_pid()
    }
    
    #[cfg(not(target_arch = "wasm32"))]
    0 // Mock for non-WASM
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_syscall_mock() {
        // On non-WASM, should return 0
        let result = syscall(0, 0, 0, 0);
        assert_eq!(result, 0);
    }

    #[test]
    fn test_get_pid_mock() {
        // On non-WASM, should return 0
        let pid = get_pid();
        assert_eq!(pid, 0);
    }

    #[test]
    fn test_send_empty_bytes() {
        // Should not panic on empty slice
        send_bytes(&[]);
    }

    #[test]
    fn test_recv_empty_buffer() {
        // Should return 0 for empty buffer
        let mut buf: [u8; 0] = [];
        let received = recv_bytes(&mut buf);
        assert_eq!(received, 0);
    }
}
