//! Idle Test Process
//!
//! Does nothing - serves as a baseline for measurements.

// When building for native target (tests), use std
#![cfg_attr(target_arch = "wasm32", no_std)]
#![cfg_attr(target_arch = "wasm32", no_main)]

// Initialize bump allocator with 64KB heap (minimal)
zos_allocator::init!(64 * 1024);

#[cfg(target_arch = "wasm32")]
extern crate alloc;

use zos_process::{self as syscall};
use zos_system_procs::CMD_EXIT;

// Command endpoint slot
const CMD_ENDPOINT: u32 = 0;

/// Process entry point
#[no_mangle]
pub extern "C" fn _start() {
    // Just loop and yield, doing nothing
    // Check for exit command occasionally
    loop {
        if let Ok(msg) = syscall::receive(CMD_ENDPOINT) {
            if msg.tag == CMD_EXIT {
                syscall::exit(0);
            }
        }
        syscall::yield_now();
    }
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

