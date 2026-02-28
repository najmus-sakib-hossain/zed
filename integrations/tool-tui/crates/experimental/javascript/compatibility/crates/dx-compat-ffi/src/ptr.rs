//! Safe pointer operations for FFI.
//!
//! Provides utilities for working with raw pointers in a safer manner.

use std::alloc::{alloc, dealloc, Layout};
use std::ptr::NonNull;

/// Read a value from a pointer.
///
/// # Safety
/// - The pointer must be valid and properly aligned for type T
/// - The memory must be initialized
/// - The pointer must not be null
pub unsafe fn read<T: Copy>(ptr: *const T) -> T {
    debug_assert!(!ptr.is_null(), "Attempted to read from null pointer");
    unsafe { std::ptr::read(ptr) }
}

/// Write a value to a pointer.
///
/// # Safety
/// - The pointer must be valid and properly aligned for type T
/// - The pointer must be writable
/// - The pointer must not be null
pub unsafe fn write<T>(ptr: *mut T, value: T) {
    debug_assert!(!ptr.is_null(), "Attempted to write to null pointer");
    unsafe { std::ptr::write(ptr, value) }
}

/// Convert a pointer and length to a byte slice.
///
/// # Safety
/// - The pointer must be valid for `len` bytes
/// - The memory must be initialized
/// - The pointer must not be null (unless len is 0)
pub unsafe fn to_slice<'a>(ptr: *const u8, len: usize) -> &'a [u8] {
    if len == 0 {
        return &[];
    }
    debug_assert!(!ptr.is_null(), "Attempted to create slice from null pointer");
    unsafe { std::slice::from_raw_parts(ptr, len) }
}

/// Convert a pointer and length to a mutable byte slice.
///
/// # Safety
/// - The pointer must be valid for `len` bytes
/// - The memory must be initialized
/// - The pointer must be writable
/// - The pointer must not be null (unless len is 0)
pub unsafe fn to_slice_mut<'a>(ptr: *mut u8, len: usize) -> &'a mut [u8] {
    if len == 0 {
        return &mut [];
    }
    debug_assert!(!ptr.is_null(), "Attempted to create mutable slice from null pointer");
    unsafe { std::slice::from_raw_parts_mut(ptr, len) }
}

/// Convert a pointer and length to a byte vector (copies data).
///
/// # Safety
/// - The pointer must be valid for `len` bytes
/// - The memory must be initialized
pub unsafe fn to_array_buffer(ptr: *const u8, len: usize) -> Vec<u8> {
    unsafe { to_slice(ptr, len).to_vec() }
}

/// Allocate memory for FFI use.
///
/// Returns a pointer to allocated memory of the given size and alignment.
/// The memory is uninitialized.
///
/// # Safety
/// - The caller must ensure the memory is properly freed with `free`
/// - The size must be non-zero
pub fn malloc(size: usize, align: usize) -> Option<NonNull<u8>> {
    if size == 0 {
        return None;
    }

    let layout = Layout::from_size_align(size, align).ok()?;
    let ptr = unsafe { alloc(layout) };
    NonNull::new(ptr)
}

/// Free memory allocated with `malloc`.
///
/// # Safety
/// - The pointer must have been allocated with `malloc`
/// - The size and alignment must match the original allocation
/// - The pointer must not be used after this call
pub unsafe fn free(ptr: NonNull<u8>, size: usize, align: usize) {
    if let Ok(layout) = Layout::from_size_align(size, align) {
        unsafe { dealloc(ptr.as_ptr(), layout) }
    }
}

/// Pointer wrapper with size information for safer FFI.
pub struct FfiBuffer {
    ptr: NonNull<u8>,
    size: usize,
    align: usize,
}

impl FfiBuffer {
    /// Allocate a new buffer.
    pub fn new(size: usize) -> Option<Self> {
        Self::with_alignment(size, 8) // Default to 8-byte alignment
    }

    /// Allocate a new buffer with specific alignment.
    pub fn with_alignment(size: usize, align: usize) -> Option<Self> {
        let ptr = malloc(size, align)?;
        Some(Self { ptr, size, align })
    }

    /// Create from existing data (copies).
    pub fn from_slice(data: &[u8]) -> Option<Self> {
        let mut buffer = Self::new(data.len())?;
        buffer.as_mut_slice().copy_from_slice(data);
        Some(buffer)
    }

    /// Get the raw pointer.
    pub fn as_ptr(&self) -> *const u8 {
        self.ptr.as_ptr()
    }

    /// Get the raw mutable pointer.
    pub fn as_mut_ptr(&mut self) -> *mut u8 {
        self.ptr.as_ptr()
    }

    /// Get the buffer size.
    pub fn len(&self) -> usize {
        self.size
    }

    /// Check if buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.size == 0
    }

    /// Get as slice.
    pub fn as_slice(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.ptr.as_ptr(), self.size) }
    }

    /// Get as mutable slice.
    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        unsafe { std::slice::from_raw_parts_mut(self.ptr.as_ptr(), self.size) }
    }

    /// Convert to Vec (copies data).
    pub fn to_vec(&self) -> Vec<u8> {
        self.as_slice().to_vec()
    }
}

impl Drop for FfiBuffer {
    fn drop(&mut self) {
        unsafe { free(self.ptr, self.size, self.align) }
    }
}

/// Check if a pointer is aligned for a given type.
pub fn is_aligned<T>(ptr: *const T) -> bool {
    (ptr as usize).is_multiple_of(std::mem::align_of::<T>())
}

/// Check if a pointer is null.
pub fn is_null<T>(ptr: *const T) -> bool {
    ptr.is_null()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_malloc_free() {
        let ptr = malloc(1024, 8).expect("allocation failed");
        unsafe { free(ptr, 1024, 8) };
    }

    #[test]
    fn test_ffi_buffer() {
        let mut buffer = FfiBuffer::new(100).expect("allocation failed");
        assert_eq!(buffer.len(), 100);

        buffer.as_mut_slice()[0] = 42;
        assert_eq!(buffer.as_slice()[0], 42);
    }

    #[test]
    fn test_ffi_buffer_from_slice() {
        let data = vec![1, 2, 3, 4, 5];
        let buffer = FfiBuffer::from_slice(&data).expect("allocation failed");
        assert_eq!(buffer.to_vec(), data);
    }

    #[test]
    fn test_is_aligned() {
        let value: u64 = 0;
        assert!(is_aligned(&value as *const u64));
    }

    #[test]
    fn test_read_write() {
        let mut value: i32 = 0;
        unsafe {
            write(&mut value as *mut i32, 42);
            assert_eq!(read(&value as *const i32), 42);
        }
    }
}
