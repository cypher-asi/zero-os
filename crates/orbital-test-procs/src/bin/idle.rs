//! Idle Test Process
//!
//! Does nothing - serves as a baseline for measurements.

// When building for native target (tests), use std
#![cfg_attr(target_arch = "wasm32", no_std)]
#![cfg_attr(target_arch = "wasm32", no_main)]

#[cfg(target_arch = "wasm32")]
extern crate alloc;

use orbital_process::{self as syscall};
use orbital_test_procs::CMD_EXIT;

// Command endpoint slot
const CMD_ENDPOINT: u32 = 0;

/// Process entry point
#[no_mangle]
pub extern "C" fn _start() {
    // Just loop and yield, doing nothing
    // Check for exit command occasionally
    loop {
        if let Some(msg) = syscall::receive(CMD_ENDPOINT) {
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
    println!("This binary is meant to run as WASM in Orbital OS");
}

// ============================================================================
// Required for no_std WASM
// ============================================================================

#[cfg(all(target_arch = "wasm32", not(test)))]
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    let msg = if let Some(s) = info.payload().downcast_ref::<&str>() {
        alloc::format!("PANIC: {}", s)
    } else {
        alloc::string::String::from("PANIC: unknown")
    };
    syscall::debug(&msg);
    syscall::exit(1);
}

#[cfg(target_arch = "wasm32")]
mod allocator {
    use core::alloc::{GlobalAlloc, Layout};

    struct BumpAllocator {
        head: core::sync::atomic::AtomicUsize,
    }

    #[global_allocator]
    static ALLOCATOR: BumpAllocator = BumpAllocator {
        head: core::sync::atomic::AtomicUsize::new(0),
    };

    const HEAP_START: usize = 0x10000;
    const HEAP_SIZE: usize = 64 * 1024; // 64KB heap (minimal)

    unsafe impl GlobalAlloc for BumpAllocator {
        unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
            let size = layout.size();
            let align = layout.align();

            loop {
                let head = self.head.load(core::sync::atomic::Ordering::Relaxed);
                let aligned = (HEAP_START + head + align - 1) & !(align - 1);
                let new_head = aligned - HEAP_START + size;

                if new_head > HEAP_SIZE {
                    return core::ptr::null_mut();
                }

                if self
                    .head
                    .compare_exchange_weak(
                        head,
                        new_head,
                        core::sync::atomic::Ordering::SeqCst,
                        core::sync::atomic::Ordering::Relaxed,
                    )
                    .is_ok()
                {
                    return aligned as *mut u8;
                }
            }
        }

        unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {
            // Bump allocator doesn't deallocate
        }
    }
}
