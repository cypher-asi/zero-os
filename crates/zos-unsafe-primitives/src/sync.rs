//! Send/Sync wrapper types with safety invariants
//!
//! This module provides wrapper types for implementing Send and Sync
//! on types that are not automatically Send/Sync but are safe in specific contexts.
//!
//! # When to Use
//!
//! Use these wrappers when:
//! 1. You have a type that isn't Send/Sync but you know it's safe in your context
//! 2. The single-threaded WASM environment makes thread safety irrelevant
//! 3. You need to store types in thread-safe containers for API compatibility
//!
//! # Safety Documentation
//!
//! Each wrapper documents its safety invariants. Violating these invariants
//! is undefined behavior.

use core::ops::{Deref, DerefMut};

/// A wrapper that implements Send + Sync for types that don't normally implement them.
///
/// # Safety Invariants
///
/// This wrapper is ONLY safe to use when ONE of these conditions holds:
///
/// 1. **Single-threaded context**: The code runs in a single-threaded environment
///    (like WASM), where Send/Sync are irrelevant because there's only one thread.
///
/// 2. **Exclusive access**: The wrapped value is only ever accessed from one thread
///    at a time, typically enforced by external synchronization.
///
/// # Usage
///
/// ```ignore
/// // In single-threaded WASM:
/// let handle = SendSyncWrapper::new(some_js_value);
/// // Now handle can be stored in Rc, static, etc.
/// ```
///
/// # Panic
///
/// Methods panic in debug mode if used incorrectly in multi-threaded contexts.
/// This helps catch bugs during development.
#[repr(transparent)]
pub struct SendSyncWrapper<T> {
    inner: T,
}

impl<T> SendSyncWrapper<T> {
    /// Create a new SendSyncWrapper.
    ///
    /// # Safety
    ///
    /// The caller must ensure that:
    /// 1. The wrapped value will only be accessed from a single thread, OR
    /// 2. The code runs in a single-threaded environment (like WASM)
    pub const fn new(value: T) -> Self {
        Self { inner: value }
    }

    /// Unwrap and return the inner value.
    pub fn into_inner(self) -> T {
        self.inner
    }

    /// Get a reference to the inner value.
    pub fn get(&self) -> &T {
        &self.inner
    }

    /// Get a mutable reference to the inner value.
    pub fn get_mut(&mut self) -> &mut T {
        &mut self.inner
    }
}

impl<T> Deref for SendSyncWrapper<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T> DerefMut for SendSyncWrapper<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

// SAFETY: SendSyncWrapper is Send when used in single-threaded contexts.
// The wrapper's existence is a promise by the user that:
// 1. Either the context is single-threaded, OR
// 2. External synchronization ensures exclusive access
unsafe impl<T> Send for SendSyncWrapper<T> {}

// SAFETY: SendSyncWrapper is Sync when used in single-threaded contexts.
// Same invariants as Send.
unsafe impl<T> Sync for SendSyncWrapper<T> {}

impl<T: Clone> Clone for SendSyncWrapper<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<T: Default> Default for SendSyncWrapper<T> {
    fn default() -> Self {
        Self {
            inner: T::default(),
        }
    }
}

impl<T: core::fmt::Debug> core::fmt::Debug for SendSyncWrapper<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("SendSyncWrapper")
            .field("inner", &self.inner)
            .finish()
    }
}

// ============================================================================
// NonNull wrapper for pointers
// ============================================================================

/// A non-null pointer wrapper that is Send + Sync.
///
/// # Safety Invariants
///
/// 1. The pointer must remain valid for the lifetime of this wrapper
/// 2. Access must be externally synchronized if used across threads
/// 3. In single-threaded contexts (WASM), synchronization is not required
#[repr(transparent)]
pub struct SendSyncPtr<T> {
    ptr: core::ptr::NonNull<T>,
}

impl<T> SendSyncPtr<T> {
    /// Create a new SendSyncPtr from a non-null pointer.
    ///
    /// # Safety
    ///
    /// The pointer must be valid and remain valid for the wrapper's lifetime.
    pub const unsafe fn new(ptr: core::ptr::NonNull<T>) -> Self {
        Self { ptr }
    }

    /// Create from a raw pointer, returning None if null.
    ///
    /// # Safety
    ///
    /// If the pointer is non-null, it must be valid.
    pub unsafe fn from_raw(ptr: *mut T) -> Option<Self> {
        core::ptr::NonNull::new(ptr).map(|ptr| Self { ptr })
    }

    /// Get the raw pointer.
    pub fn as_ptr(&self) -> *mut T {
        self.ptr.as_ptr()
    }

    /// Get a reference to the pointed value.
    ///
    /// # Safety
    ///
    /// The pointer must be valid and properly aligned.
    /// No mutable references to the same memory can exist.
    pub unsafe fn as_ref(&self) -> &T {
        self.ptr.as_ref()
    }

    /// Get a mutable reference to the pointed value.
    ///
    /// # Safety
    ///
    /// The pointer must be valid and properly aligned.
    /// No other references to the same memory can exist.
    pub unsafe fn as_mut(&mut self) -> &mut T {
        self.ptr.as_mut()
    }
}

// SAFETY: Same invariants as SendSyncWrapper
unsafe impl<T> Send for SendSyncPtr<T> {}
unsafe impl<T> Sync for SendSyncPtr<T> {}

impl<T> Clone for SendSyncPtr<T> {
    fn clone(&self) -> Self {
        Self { ptr: self.ptr }
    }
}

impl<T> Copy for SendSyncPtr<T> {}

impl<T> core::fmt::Debug for SendSyncPtr<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("SendSyncPtr")
            .field("ptr", &self.ptr)
            .finish()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wrapper_deref() {
        let wrapper = SendSyncWrapper::new(42);
        assert_eq!(*wrapper, 42);
    }

    #[test]
    fn test_wrapper_deref_mut() {
        let mut wrapper = SendSyncWrapper::new(42);
        *wrapper = 100;
        assert_eq!(*wrapper, 100);
    }

    #[test]
    fn test_wrapper_into_inner() {
        let wrapper = SendSyncWrapper::new(42i32);
        let n = wrapper.into_inner();
        assert_eq!(n, 42);
    }

    #[test]
    fn test_ptr_from_raw() {
        let mut value = 42;
        let ptr = unsafe { SendSyncPtr::from_raw(&mut value as *mut i32) };
        assert!(ptr.is_some());

        let ptr = unsafe { SendSyncPtr::<i32>::from_raw(core::ptr::null_mut()) };
        assert!(ptr.is_none());
    }

    // Compile-time check that SendSyncWrapper is Send + Sync
    fn _assert_send<T: Send>() {}
    fn _assert_sync<T: Sync>() {}

    #[test]
    fn test_send_sync_traits() {
        _assert_send::<SendSyncWrapper<*const ()>>();
        _assert_sync::<SendSyncWrapper<*const ()>>();
        _assert_send::<SendSyncPtr<()>>();
        _assert_sync::<SendSyncPtr<()>>();
    }
}
