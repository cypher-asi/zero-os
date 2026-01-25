//! Bump Allocator for Zero OS WASM Processes
//!
//! Provides a simple bump allocator with configurable heap size via const generic.
//! This eliminates code duplication across all WASM binaries that need an allocator.
//!
//! # Usage
//!
//! ```ignore
//! // At the crate root level:
//! zos_allocator::init!(1024 * 1024); // 1MB heap
//! ```
//!
//! # Heap Sizes by Binary
//!
//! | Binary | Heap Size | Rationale |
//! |--------|-----------|-----------|
//! | init | 1MB | Service registry, message handling |
//! | idle | 64KB | Minimal - does nothing |
//! | pingpong | 1MB | Latency measurement with vectors |
//! | sender | 1MB | Message burst handling |
//! | receiver | 1MB | Message counting |
//! | memhog | 16MB | Memory stress testing |

#![no_std]

use core::alloc::{GlobalAlloc, Layout};
use core::sync::atomic::{AtomicUsize, Ordering};

/// Initialize the global allocator with the specified heap size in bytes.
///
/// This macro must be called exactly once at the crate root level.
/// It only activates on wasm32 targets.
///
/// # Example
///
/// ```ignore
/// zos_allocator::init!(1024 * 1024); // 1MB heap
/// ```
#[macro_export]
macro_rules! init {
    ($heap_size:expr) => {
        #[cfg(target_arch = "wasm32")]
        #[global_allocator]
        static ALLOCATOR: $crate::BumpAllocator<{ $heap_size }> = $crate::BumpAllocator::new();
    };
}

/// Bump allocator with configurable heap size.
///
/// The heap starts at 0x10000 (64KB offset) to avoid conflicts with
/// WASM linear memory's initial pages used by the runtime.
///
/// This is a simple "bump pointer" allocator that:
/// - Allocates by incrementing a pointer
/// - Never deallocates (suitable for short-lived WASM processes)
/// - Is thread-safe via atomic operations
pub struct BumpAllocator<const SIZE: usize> {
    head: AtomicUsize,
}

impl<const SIZE: usize> BumpAllocator<SIZE> {
    /// Heap start address (64KB offset to avoid WASM runtime conflicts)
    const HEAP_START: usize = 0x10000;

    /// Create a new bump allocator.
    pub const fn new() -> Self {
        Self {
            head: AtomicUsize::new(0),
        }
    }
}

// SAFETY: The allocator uses atomic operations for thread safety
unsafe impl<const SIZE: usize> Sync for BumpAllocator<SIZE> {}

unsafe impl<const SIZE: usize> GlobalAlloc for BumpAllocator<SIZE> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let size = layout.size();
        let align = layout.align();

        loop {
            let head = self.head.load(Ordering::Relaxed);
            let aligned = (Self::HEAP_START + head + align - 1) & !(align - 1);
            let new_head = aligned - Self::HEAP_START + size;

            if new_head > SIZE {
                return core::ptr::null_mut();
            }

            if self
                .head
                .compare_exchange_weak(head, new_head, Ordering::SeqCst, Ordering::Relaxed)
                .is_ok()
            {
                return aligned as *mut u8;
            }
        }
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {
        // Bump allocator doesn't deallocate - memory is reclaimed when process exits
    }
}
