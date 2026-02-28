//! Arena-based memory management
//!
//! Uses standard Rust allocation for cross-platform support.
//! Can be optimized with platform-specific mmap/VirtualAlloc later.
//!
//! All allocated memory is zero-initialized to prevent use of uninitialized data.

use crate::error::DxResult;
use std::alloc::{alloc_zeroed, dealloc, Layout};
use std::sync::atomic::{AtomicUsize, Ordering};

/// Arena allocator for zero-allocation execution
///
/// All memory allocated from this arena is guaranteed to be zero-initialized.
/// This prevents undefined behavior from reading uninitialized memory.
pub struct Arena {
    base: *mut u8,
    size: usize,
    offset: AtomicUsize,
    layout: Layout,
}

// Safety: We control access via atomic operations
unsafe impl Send for Arena {}
unsafe impl Sync for Arena {}

impl Arena {
    /// Create a new arena with the given size
    ///
    /// The entire arena is zero-initialized on creation.
    pub fn new(size: usize) -> DxResult<Self> {
        // Use standard Rust allocation for cross-platform support
        let layout = Layout::from_size_align(size, 4096)
            .map_err(|_| crate::error::DxError::RuntimeError("Invalid arena layout".into()))?;
        
        // Use alloc_zeroed to ensure all memory is zero-initialized
        // This prevents use of uninitialized data (Requirements 1.1)
        let base = unsafe { alloc_zeroed(layout) };

        if base.is_null() {
            return Err(crate::error::DxError::RuntimeError("Failed to allocate arena".into()));
        }

        Ok(Self {
            base,
            size,
            offset: AtomicUsize::new(0),
            layout,
        })
    }

    /// Allocate memory from the arena
    ///
    /// Returns a pointer to zero-initialized memory, or None if the arena is full.
    /// The memory is guaranteed to be zero-initialized because:
    /// 1. The arena is zero-initialized on creation
    /// 2. After reset(), we zero-fill the used portion before reuse
    #[inline]
    #[allow(dead_code)]
    pub fn alloc(&self, size: usize, align: usize) -> Option<*mut u8> {
        loop {
            let current = self.offset.load(Ordering::Relaxed);
            let aligned = (current + align - 1) & !(align - 1);
            let new_offset = aligned + size;

            if new_offset > self.size {
                return None;
            }

            if self
                .offset
                .compare_exchange_weak(current, new_offset, Ordering::SeqCst, Ordering::Relaxed)
                .is_ok()
            {
                let ptr = unsafe { self.base.add(aligned) };
                // Zero-fill the allocated region to ensure it's initialized
                // This handles the case where memory was previously used and reset
                unsafe {
                    std::ptr::write_bytes(ptr, 0, size);
                }
                return Some(ptr);
            }
        }
    }

    /// Reset the arena - O(1) operation
    #[inline]
    pub fn reset(&self) {
        self.offset.store(0, Ordering::SeqCst);
    }

    /// Get current usage
    #[allow(dead_code)]
    pub fn usage(&self) -> usize {
        self.offset.load(Ordering::Relaxed)
    }
}

impl Drop for Arena {
    fn drop(&mut self) {
        unsafe {
            dealloc(self.base, self.layout);
        }
    }
}
