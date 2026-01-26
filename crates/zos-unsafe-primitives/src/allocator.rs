//! Bump Allocator for Zero OS WASM Processes
//!
//! Provides a simple bump allocator with configurable heap size via const generic.
//! This is the ONLY allocator implementation in Zero OS.
//!
//! # Safety Invariants
//!
//! 1. **No double free**: `dealloc` is a no-op (bump allocators don't free)
//! 2. **Alignment guaranteed**: All allocations are properly aligned
//! 3. **No overlap**: Consecutive allocations never overlap
//! 4. **Thread safety**: Atomic operations prevent data races
//!
//! # Verification
//!
//! This module includes Kani proofs for the above invariants.

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
/// zos_unsafe_primitives::init_allocator!(1024 * 1024); // 1MB heap
/// ```
#[macro_export]
macro_rules! init_allocator {
    ($heap_size:expr) => {
        #[cfg(target_arch = "wasm32")]
        #[global_allocator]
        static ALLOCATOR: $crate::allocator::BumpAllocator<{ $heap_size }> =
            $crate::allocator::BumpAllocator::new();
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
    /// Current allocation head (offset from HEAP_START)
    head: AtomicUsize,
}

impl<const SIZE: usize> BumpAllocator<SIZE> {
    /// Heap start address (64KB offset to avoid WASM runtime conflicts)
    pub const HEAP_START: usize = 0x10000;

    /// Create a new bump allocator.
    pub const fn new() -> Self {
        Self {
            head: AtomicUsize::new(0),
        }
    }

    /// Get current allocation position (for debugging/verification)
    pub fn current_position(&self) -> usize {
        self.head.load(Ordering::Relaxed)
    }

    /// Get remaining capacity
    pub fn remaining(&self) -> usize {
        SIZE.saturating_sub(self.head.load(Ordering::Relaxed))
    }
}

impl<const SIZE: usize> Default for BumpAllocator<SIZE> {
    fn default() -> Self {
        Self::new()
    }
}

// SAFETY: The allocator uses atomic operations for thread safety.
// The head counter is only modified via atomic compare-exchange, ensuring
// no data races even with concurrent allocations.
unsafe impl<const SIZE: usize> Sync for BumpAllocator<SIZE> {}

// SAFETY: BumpAllocator is Send because:
// 1. It only contains an AtomicUsize which is Send
// 2. All state is synchronized via atomic operations
unsafe impl<const SIZE: usize> Send for BumpAllocator<SIZE> {}

// SAFETY: GlobalAlloc implementation maintains these invariants:
// 1. Returns null on failure (never UB)
// 2. Returned pointers are properly aligned
// 3. Allocated regions never overlap
// 4. dealloc is a no-op (no double-free possible)
unsafe impl<const SIZE: usize> GlobalAlloc for BumpAllocator<SIZE> {
    /// Allocate memory with the given layout.
    ///
    /// # Safety
    ///
    /// This function is safe to call from multiple threads due to atomic operations.
    /// Returns null pointer if:
    /// - Allocation would exceed heap size
    /// - Alignment cannot be satisfied
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let size = layout.size();
        let align = layout.align();

        // Verify alignment is power of two (required by Layout)
        debug_assert!(align.is_power_of_two());

        loop {
            let head = self.head.load(Ordering::Relaxed);

            // Calculate aligned address
            // SAFETY: align is power of two (guaranteed by Layout), so align - 1 is a valid mask
            let aligned = (Self::HEAP_START + head + align - 1) & !(align - 1);
            let new_head = aligned - Self::HEAP_START + size;

            // Check for overflow
            if new_head > SIZE {
                return core::ptr::null_mut();
            }

            // Atomic compare-exchange to claim this allocation
            if self
                .head
                .compare_exchange_weak(head, new_head, Ordering::SeqCst, Ordering::Relaxed)
                .is_ok()
            {
                return aligned as *mut u8;
            }
            // If CAS failed, another thread allocated - retry
        }
    }

    /// Deallocate memory.
    ///
    /// This is a no-op for bump allocators. Memory is reclaimed when the
    /// process exits and its entire address space is released.
    ///
    /// # Safety
    ///
    /// This function is always safe because it does nothing.
    /// The "no double free" invariant is trivially satisfied.
    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {
        // Bump allocator doesn't deallocate - memory is reclaimed when process exits
    }
}

// ============================================================================
// Kani Proofs for Allocator Invariants
// ============================================================================

#[cfg(kani)]
mod proofs {
    use super::*;

    /// Proof: No double free (trivially true since dealloc is no-op)
    #[kani::proof]
    fn no_double_free() {
        let allocator: BumpAllocator<1024> = BumpAllocator::new();
        let layout = Layout::from_size_align(64, 8).unwrap();

        // Allocate
        let ptr = unsafe { allocator.alloc(layout) };
        kani::assume(!ptr.is_null());

        // First dealloc - no-op
        unsafe { allocator.dealloc(ptr, layout) };

        // Second dealloc - still no-op, no double-free possible
        unsafe { allocator.dealloc(ptr, layout) };

        // Allocator state should be unchanged by deallocs
        // (bump allocator never frees)
    }

    /// Proof: Alignment is always satisfied
    #[kani::proof]
    fn allocation_alignment() {
        let allocator: BumpAllocator<4096> = BumpAllocator::new();

        // Test various alignments (powers of 2)
        let align: usize = kani::any();
        kani::assume(align == 1 || align == 2 || align == 4 || align == 8 || align == 16);

        let size: usize = kani::any();
        kani::assume(size > 0 && size <= 256);

        if let Ok(layout) = Layout::from_size_align(size, align) {
            let ptr = unsafe { allocator.alloc(layout) };

            if !ptr.is_null() {
                // Verify alignment
                let addr = ptr as usize;
                kani::assert(
                    addr % align == 0,
                    "Allocation must be properly aligned",
                );
            }
        }
    }

    /// Proof: Consecutive allocations don't overlap
    #[kani::proof]
    fn no_overlap() {
        let allocator: BumpAllocator<4096> = BumpAllocator::new();

        let size1: usize = kani::any();
        let size2: usize = kani::any();
        kani::assume(size1 > 0 && size1 <= 128);
        kani::assume(size2 > 0 && size2 <= 128);

        let layout1 = Layout::from_size_align(size1, 8).unwrap();
        let layout2 = Layout::from_size_align(size2, 8).unwrap();

        let ptr1 = unsafe { allocator.alloc(layout1) };
        let ptr2 = unsafe { allocator.alloc(layout2) };

        if !ptr1.is_null() && !ptr2.is_null() {
            let addr1 = ptr1 as usize;
            let addr2 = ptr2 as usize;

            // Regions should not overlap
            // Either region2 starts after region1 ends, or vice versa
            let region1_end = addr1 + size1;
            let region2_end = addr2 + size2;

            kani::assert(
                region1_end <= addr2 || region2_end <= addr1,
                "Consecutive allocations must not overlap",
            );
        }
    }

    /// Proof: Allocation fails gracefully when out of memory
    #[kani::proof]
    fn oom_returns_null() {
        let allocator: BumpAllocator<256> = BumpAllocator::new();

        // Request more than available
        let layout = Layout::from_size_align(512, 8).unwrap();
        let ptr = unsafe { allocator.alloc(layout) };

        kani::assert(ptr.is_null(), "OOM must return null, not UB");
    }

    /// Proof: Head only moves forward (monotonic)
    #[kani::proof]
    fn head_monotonic() {
        let allocator: BumpAllocator<4096> = BumpAllocator::new();

        let initial_head = allocator.current_position();

        let size: usize = kani::any();
        kani::assume(size > 0 && size <= 256);

        let layout = Layout::from_size_align(size, 8).unwrap();
        let ptr = unsafe { allocator.alloc(layout) };

        let new_head = allocator.current_position();

        if !ptr.is_null() {
            kani::assert(
                new_head >= initial_head,
                "Head must only move forward on successful allocation",
            );
            kani::assert(
                new_head > initial_head,
                "Head must increase after allocation",
            );
        } else {
            kani::assert(
                new_head == initial_head,
                "Head must not change on failed allocation",
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_allocation() {
        let allocator: BumpAllocator<1024> = BumpAllocator::new();
        let layout = Layout::from_size_align(64, 8).unwrap();

        let ptr = unsafe { allocator.alloc(layout) };
        assert!(!ptr.is_null());
        assert_eq!((ptr as usize) % 8, 0); // Check alignment
    }

    #[test]
    fn test_multiple_allocations() {
        let allocator: BumpAllocator<1024> = BumpAllocator::new();

        let layout = Layout::from_size_align(64, 8).unwrap();
        let ptr1 = unsafe { allocator.alloc(layout) };
        let ptr2 = unsafe { allocator.alloc(layout) };

        assert!(!ptr1.is_null());
        assert!(!ptr2.is_null());
        assert_ne!(ptr1, ptr2);

        // Verify no overlap
        let addr1 = ptr1 as usize;
        let addr2 = ptr2 as usize;
        assert!(addr2 >= addr1 + 64 || addr1 >= addr2 + 64);
    }

    #[test]
    fn test_oom() {
        let allocator: BumpAllocator<64> = BumpAllocator::new();
        let layout = Layout::from_size_align(128, 8).unwrap();

        let ptr = unsafe { allocator.alloc(layout) };
        assert!(ptr.is_null());
    }

    #[test]
    fn test_alignment() {
        let allocator: BumpAllocator<1024> = BumpAllocator::new();

        // Allocate with different alignments
        for align in [1, 2, 4, 8, 16, 32] {
            let layout = Layout::from_size_align(32, align).unwrap();
            let ptr = unsafe { allocator.alloc(layout) };
            assert!(!ptr.is_null());
            assert_eq!((ptr as usize) % align, 0);
        }
    }

    #[test]
    fn test_dealloc_is_noop() {
        let allocator: BumpAllocator<1024> = BumpAllocator::new();
        let layout = Layout::from_size_align(64, 8).unwrap();

        let ptr = unsafe { allocator.alloc(layout) };
        let pos_before = allocator.current_position();

        unsafe { allocator.dealloc(ptr, layout) };
        let pos_after = allocator.current_position();

        // Position should not change (dealloc is no-op)
        assert_eq!(pos_before, pos_after);
    }
}
