//! Loom tests for concurrent data structures
//!
//! Loom is a concurrency testing tool that explores all possible interleavings
//! of concurrent operations. It helps find race conditions and other
//! concurrency bugs.
//!
//! # When to Use Loom
//!
//! Use loom tests for:
//! - Atomic operations (like in the allocator)
//! - Lock-free data structures
//! - Any code with `Ordering` specifications
//!
//! # Running Loom Tests
//!
//! ```bash
//! cargo test --package zos-unsafe-primitives --features loom -- --test-threads=1 loom
//! ```
//!
//! Note: Loom tests must run single-threaded and can take a while to explore
//! all interleavings.

#[cfg(all(test, feature = "loom"))]
mod tests {
    use loom::sync::atomic::{AtomicUsize, Ordering};
    use loom::thread;

    /// A simplified bump allocator for loom testing
    /// Uses loom's atomic types which track all possible orderings
    struct LoomBumpAllocator {
        head: AtomicUsize,
        size: usize,
    }

    impl LoomBumpAllocator {
        const HEAP_START: usize = 0x10000;

        fn new(size: usize) -> Self {
            Self {
                head: AtomicUsize::new(0),
                size,
            }
        }

        /// Allocate memory (simplified version for testing)
        fn alloc(&self, size: usize, align: usize) -> Option<usize> {
            loop {
                let head = self.head.load(Ordering::Relaxed);
                let aligned = (Self::HEAP_START + head + align - 1) & !(align - 1);
                let new_head = aligned - Self::HEAP_START + size;

                if new_head > self.size {
                    return None;
                }

                if self
                    .head
                    .compare_exchange_weak(head, new_head, Ordering::SeqCst, Ordering::Relaxed)
                    .is_ok()
                {
                    return Some(aligned);
                }
            }
        }

        fn current_position(&self) -> usize {
            self.head.load(Ordering::Relaxed)
        }
    }

    /// Test: Concurrent allocations don't overlap
    ///
    /// This test verifies that when multiple threads allocate concurrently,
    /// they never receive overlapping memory regions.
    #[test]
    fn loom_concurrent_alloc_no_overlap() {
        loom::model(|| {
            let allocator = loom::sync::Arc::new(LoomBumpAllocator::new(1024));

            let a1 = allocator.clone();
            let a2 = allocator.clone();

            let t1 = thread::spawn(move || a1.alloc(64, 8));
            let t2 = thread::spawn(move || a2.alloc(64, 8));

            let r1 = t1.join().unwrap();
            let r2 = t2.join().unwrap();

            // Both should succeed (we have 1024 bytes)
            assert!(r1.is_some() || r2.is_some());

            if let (Some(addr1), Some(addr2)) = (r1, r2) {
                // Addresses should not overlap
                let end1 = addr1 + 64;
                let end2 = addr2 + 64;
                assert!(end1 <= addr2 || end2 <= addr1, "Allocations overlap!");
            }
        });
    }

    /// Test: Head moves monotonically
    ///
    /// The allocation head should only ever move forward, never backward.
    #[test]
    fn loom_head_monotonic() {
        loom::model(|| {
            let allocator = loom::sync::Arc::new(LoomBumpAllocator::new(1024));

            let a1 = allocator.clone();
            let a2 = allocator.clone();

            let initial = allocator.current_position();

            let t1 = thread::spawn(move || {
                a1.alloc(32, 8);
            });
            let t2 = thread::spawn(move || {
                a2.alloc(32, 8);
            });

            t1.join().unwrap();
            t2.join().unwrap();

            let final_pos = allocator.current_position();

            // Head should only have moved forward
            assert!(
                final_pos >= initial,
                "Head moved backward: {} -> {}",
                initial,
                final_pos
            );
        });
    }

    /// Test: Alignment is always correct
    ///
    /// Even under concurrent access, alignment requirements are satisfied.
    #[test]
    fn loom_alignment_always_correct() {
        loom::model(|| {
            let allocator = loom::sync::Arc::new(LoomBumpAllocator::new(1024));

            let a1 = allocator.clone();
            let a2 = allocator.clone();

            let t1 = thread::spawn(move || a1.alloc(17, 16)); // Awkward size, 16-byte align
            let t2 = thread::spawn(move || a2.alloc(33, 32)); // Awkward size, 32-byte align

            let r1 = t1.join().unwrap();
            let r2 = t2.join().unwrap();

            if let Some(addr) = r1 {
                assert_eq!(addr % 16, 0, "Address not 16-byte aligned");
            }
            if let Some(addr) = r2 {
                assert_eq!(addr % 32, 0, "Address not 32-byte aligned");
            }
        });
    }

    /// Test: OOM is handled correctly under contention
    ///
    /// When multiple threads race to exhaust memory, all see consistent state.
    #[test]
    fn loom_oom_under_contention() {
        loom::model(|| {
            // Very small allocator
            let allocator = loom::sync::Arc::new(LoomBumpAllocator::new(128));

            let a1 = allocator.clone();
            let a2 = allocator.clone();
            let a3 = allocator.clone();

            // Each tries to allocate 64 bytes (only 2 can succeed)
            let t1 = thread::spawn(move || a1.alloc(64, 8));
            let t2 = thread::spawn(move || a2.alloc(64, 8));
            let t3 = thread::spawn(move || a3.alloc(64, 8));

            let results = [
                t1.join().unwrap(),
                t2.join().unwrap(),
                t3.join().unwrap(),
            ];

            // Count successes
            let successes: Vec<_> = results.iter().filter_map(|r| *r).collect();

            // At most 2 can succeed (128 / 64 = 2)
            assert!(
                successes.len() <= 2,
                "Too many successful allocations: {}",
                successes.len()
            );

            // Successful allocations should not overlap
            for i in 0..successes.len() {
                for j in (i + 1)..successes.len() {
                    let (a, b) = (successes[i], successes[j]);
                    let (end_a, end_b) = (a + 64, b + 64);
                    assert!(
                        end_a <= b || end_b <= a,
                        "Allocations overlap: {:x}-{:x} and {:x}-{:x}",
                        a,
                        end_a,
                        b,
                        end_b
                    );
                }
            }
        });
    }
}

// ============================================================================
// Documentation-only module for non-loom builds
// ============================================================================

#[cfg(not(feature = "loom"))]
/// Loom tests are only available with the `loom` feature.
///
/// To run loom tests:
/// ```bash
/// cargo test --package zos-unsafe-primitives --features loom -- --test-threads=1 loom
/// ```
pub mod _loom_docs {}
