//! Memory Hog Test Process
//!
//! Allocates memory on command and reports real WASM memory usage.
//! Used to test memory isolation between processes.

#![cfg_attr(target_arch = "wasm32", no_std)]
#![cfg_attr(target_arch = "wasm32", no_main)]

// Initialize bump allocator with 16MB heap for memory stress testing
zos_allocator::init!(16 * 1024 * 1024);

#[cfg(target_arch = "wasm32")]
extern crate alloc;

#[cfg(target_arch = "wasm32")]
use alloc::vec;
#[cfg(target_arch = "wasm32")]
use alloc::vec::Vec;

#[cfg(not(target_arch = "wasm32"))]
use std::vec;
#[cfg(not(target_arch = "wasm32"))]
use std::vec::Vec;

use zos_process::{self as syscall, ReceivedMessage};
use zos_system_procs::{
    MemoryStatus, CMD_ALLOC, CMD_EXIT, CMD_FREE, CMD_FREE_ALL, CMD_QUERY, MSG_MEMORY_STATUS,
};

// Command endpoint slot (assigned by supervisor)
const CMD_ENDPOINT: u32 = 0;
// Report endpoint slot
const REPORT_ENDPOINT: u32 = 1;

/// Stored allocations
static mut ALLOCATIONS: Vec<Vec<u8>> = Vec::new();

/// Process entry point
#[no_mangle]
pub extern "C" fn _start() {
    let mut total_allocated: usize = 0;

    // Main loop: wait for commands and execute them
    loop {
        // Wait for command (blocking receive)
        if let Ok(msg) = syscall::receive(CMD_ENDPOINT) {
            total_allocated = handle_command(msg, total_allocated);
        }
        syscall::yield_now();
    }
}

fn handle_command(msg: ReceivedMessage, mut total_allocated: usize) -> usize {
    match msg.tag {
        CMD_ALLOC => {
            // Allocate requested bytes
            if msg.data.len() >= 4 {
                let size = u32::from_le_bytes([msg.data[0], msg.data[1], msg.data[2], msg.data[3]])
                    as usize;
                let chunk = vec![0xABu8; size]; // Fill with pattern
                total_allocated += size;

                unsafe {
                    (*core::ptr::addr_of_mut!(ALLOCATIONS)).push(chunk);
                }

                report_memory_status(total_allocated);
            }
        }

        CMD_FREE => {
            // Free last allocation
            if let Some(chunk) = unsafe { (*core::ptr::addr_of_mut!(ALLOCATIONS)).pop() } {
                total_allocated = total_allocated.saturating_sub(chunk.len());
            }
            report_memory_status(total_allocated);
        }

        CMD_FREE_ALL => {
            unsafe {
                (*core::ptr::addr_of_mut!(ALLOCATIONS)).clear();
            }
            total_allocated = 0;
            report_memory_status(total_allocated);
        }

        CMD_QUERY => {
            report_memory_status(total_allocated);
        }

        CMD_EXIT => {
            syscall::exit(0);
        }

        _ => {}
    }

    total_allocated
}

fn report_memory_status(allocated: usize) {
    // Get real WASM memory stats
    #[cfg(target_arch = "wasm32")]
    let wasm_pages = core::arch::wasm32::memory_size(0);
    #[cfg(not(target_arch = "wasm32"))]
    let wasm_pages = 1; // Default for non-WASM

    let status = MemoryStatus {
        allocated_by_us: allocated as u64,
        allocation_count: unsafe { (*core::ptr::addr_of!(ALLOCATIONS)).len() } as u32,
        _reserved: wasm_pages as u32,
    };

    let _ = syscall::send(REPORT_ENDPOINT, MSG_MEMORY_STATUS, &status.to_bytes());
}

// Native main (for cargo check/test)
#[cfg(not(target_arch = "wasm32"))]
fn main() {
    println!("This binary is meant to run as WASM in Zero OS");
}

// ============================================================================
// Required for no_std WASM
// ============================================================================

#[cfg(all(target_arch = "wasm32", not(test)))]
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    let msg = alloc::format!("PANIC: {}", info.message());
    syscall::debug(&msg);
    syscall::exit(1);
}

