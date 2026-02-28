//! I/O buffer types for zero-copy operations

use std::ops::{Deref, DerefMut};

/// I/O buffer that can be registered for zero-copy operations.
///
/// When registered with the reactor, the kernel can directly access
/// this buffer without copying data through the kernel/user boundary.
#[derive(Debug)]
pub struct IoBuffer {
    /// Pointer to the buffer data
    ptr: *mut u8,
    /// Length of the buffer in bytes
    len: usize,
    /// Buffer group ID (for registered buffer pools)
    buf_group: Option<u16>,
    /// Buffer index within the group
    buf_index: Option<u16>,
    /// Whether this buffer owns its memory
    owned: bool,
}

// Safety: IoBuffer is Send because it's just a pointer to memory
// that we ensure is valid for the lifetime of the buffer
unsafe impl Send for IoBuffer {}

// Safety: IoBuffer is Sync because we don't provide interior mutability
// without proper synchronization
unsafe impl Sync for IoBuffer {}

impl IoBuffer {
    /// Create a new owned buffer with the specified capacity.
    pub fn new(capacity: usize) -> Self {
        let mut vec = vec![0; capacity];
        let ptr = vec.as_mut_ptr();
        let len = vec.len();
        std::mem::forget(vec); // We'll manage this memory ourselves

        Self {
            ptr,
            len,
            buf_group: None,
            buf_index: None,
            owned: true,
        }
    }

    /// Create a buffer from an existing slice (borrowed, not owned).
    ///
    /// # Safety
    /// The caller must ensure the slice remains valid for the lifetime
    /// of this IoBuffer and any I/O operations using it.
    pub unsafe fn from_slice(slice: &[u8]) -> Self {
        Self {
            ptr: slice.as_ptr() as *mut u8,
            len: slice.len(),
            buf_group: None,
            buf_index: None,
            owned: false,
        }
    }

    /// Create a buffer from a mutable slice (borrowed, not owned).
    ///
    /// # Safety
    /// The caller must ensure the slice remains valid for the lifetime
    /// of this IoBuffer and any I/O operations using it.
    pub unsafe fn from_slice_mut(slice: &mut [u8]) -> Self {
        Self {
            ptr: slice.as_mut_ptr(),
            len: slice.len(),
            buf_group: None,
            buf_index: None,
            owned: false,
        }
    }

    /// Create a buffer from a Vec, taking ownership.
    pub fn from_vec(mut vec: Vec<u8>) -> Self {
        let ptr = vec.as_mut_ptr();
        let len = vec.len();
        std::mem::forget(vec);

        Self {
            ptr,
            len,
            buf_group: None,
            buf_index: None,
            owned: true,
        }
    }

    /// Get the raw pointer to the buffer data.
    #[inline]
    pub fn as_ptr(&self) -> *const u8 {
        self.ptr
    }

    /// Get a mutable raw pointer to the buffer data.
    #[inline]
    pub fn as_mut_ptr(&mut self) -> *mut u8 {
        self.ptr
    }

    /// Get the length of the buffer.
    #[inline]
    pub fn len(&self) -> usize {
        self.len
    }

    /// Check if the buffer is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Get the buffer group ID (if registered).
    #[inline]
    pub fn buf_group(&self) -> Option<u16> {
        self.buf_group
    }

    /// Get the buffer index within its group (if registered).
    #[inline]
    pub fn buf_index(&self) -> Option<u16> {
        self.buf_index
    }

    /// Set the buffer group and index (used during registration).
    pub fn set_registration(&mut self, group: u16, index: u16) {
        self.buf_group = Some(group);
        self.buf_index = Some(index);
    }

    /// Check if this buffer is registered with the reactor.
    #[inline]
    pub fn is_registered(&self) -> bool {
        self.buf_group.is_some()
    }

    /// Get the buffer as a slice.
    #[inline]
    pub fn as_slice(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.ptr, self.len) }
    }

    /// Get the buffer as a mutable slice.
    #[inline]
    pub fn as_slice_mut(&mut self) -> &mut [u8] {
        unsafe { std::slice::from_raw_parts_mut(self.ptr, self.len) }
    }

    /// Convert to a Vec, taking ownership of the memory.
    /// Only works for owned buffers.
    pub fn into_vec(self) -> Option<Vec<u8>> {
        if self.owned {
            let vec = unsafe { Vec::from_raw_parts(self.ptr, self.len, self.len) };
            std::mem::forget(self); // Don't run Drop
            Some(vec)
        } else {
            None
        }
    }
}

impl Drop for IoBuffer {
    fn drop(&mut self) {
        if self.owned && !self.ptr.is_null() {
            // Reconstruct the Vec and let it drop
            unsafe {
                let _ = Vec::from_raw_parts(self.ptr, self.len, self.len);
            }
        }
    }
}

impl Deref for IoBuffer {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}

impl DerefMut for IoBuffer {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_slice_mut()
    }
}

impl Clone for IoBuffer {
    fn clone(&self) -> Self {
        // Always create an owned copy
        let mut new_buf = IoBuffer::new(self.len);
        new_buf.as_slice_mut().copy_from_slice(self.as_slice());
        new_buf
    }
}

/// A pool of pre-allocated buffers for efficient I/O operations.
pub struct IoBufferPool {
    /// Available buffers
    buffers: Vec<IoBuffer>,
    /// Buffer size
    buffer_size: usize,
    /// Group ID for registration
    group_id: u16,
}

impl IoBufferPool {
    /// Create a new buffer pool with the specified number of buffers.
    pub fn new(count: usize, buffer_size: usize, group_id: u16) -> Self {
        let buffers = (0..count)
            .map(|i| {
                let mut buf = IoBuffer::new(buffer_size);
                buf.set_registration(group_id, i as u16);
                buf
            })
            .collect();

        Self {
            buffers,
            buffer_size,
            group_id,
        }
    }

    /// Get a buffer from the pool.
    pub fn get(&mut self) -> Option<IoBuffer> {
        self.buffers.pop()
    }

    /// Return a buffer to the pool.
    pub fn put(&mut self, buf: IoBuffer) {
        if buf.len() == self.buffer_size {
            self.buffers.push(buf);
        }
    }

    /// Get the number of available buffers.
    pub fn available(&self) -> usize {
        self.buffers.len()
    }

    /// Get the buffer size.
    pub fn buffer_size(&self) -> usize {
        self.buffer_size
    }

    /// Get the group ID.
    pub fn group_id(&self) -> u16 {
        self.group_id
    }

    /// Get all buffers as iovecs for registration.
    #[cfg(unix)]
    pub fn as_iovecs(&self) -> Vec<libc::iovec> {
        self.buffers
            .iter()
            .map(|b| libc::iovec {
                iov_base: b.ptr as *mut libc::c_void,
                iov_len: b.len,
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_io_buffer_new() {
        let buf = IoBuffer::new(1024);
        assert_eq!(buf.len(), 1024);
        assert!(!buf.is_empty());
        assert!(!buf.is_registered());
    }

    #[test]
    fn test_io_buffer_from_vec() {
        let data = vec![1u8, 2, 3, 4, 5];
        let buf = IoBuffer::from_vec(data);
        assert_eq!(buf.len(), 5);
        assert_eq!(buf.as_slice(), &[1, 2, 3, 4, 5]);
    }

    #[test]
    fn test_io_buffer_clone() {
        let mut buf1 = IoBuffer::new(10);
        buf1.as_slice_mut()[0] = 42;

        let buf2 = buf1.clone();
        assert_eq!(buf2.as_slice()[0], 42);
    }

    #[test]
    fn test_io_buffer_into_vec() {
        let buf = IoBuffer::from_vec(vec![1, 2, 3]);
        let vec = buf.into_vec().unwrap();
        assert_eq!(vec, vec![1, 2, 3]);
    }

    #[test]
    fn test_io_buffer_pool() {
        let mut pool = IoBufferPool::new(10, 4096, 0);
        assert_eq!(pool.available(), 10);

        let buf = pool.get().unwrap();
        assert_eq!(pool.available(), 9);
        assert_eq!(buf.len(), 4096);

        pool.put(buf);
        assert_eq!(pool.available(), 10);
    }
}
