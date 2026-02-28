//! Arena allocator for zero-allocation transformations
//! Pre-allocate all memory upfront, no runtime allocations during transforms

use std::cell::UnsafeCell;
use std::sync::atomic::{AtomicUsize, Ordering};

/// Arena allocator with pre-allocated memory for zero-allocation transforms
///
/// # Performance
/// - Lock-free allocation using atomic compare-exchange
/// - Thread-safe with separate thread-local arenas
/// - Instant reset (just set offset to 0)
pub struct BundleArena {
    /// Large pre-allocated buffer
    buffer: UnsafeCell<Vec<u8>>,
    /// Current allocation offset (atomic for thread-safety)
    offset: AtomicUsize,
    /// Buffer capacity
    capacity: usize,
}

// SAFETY: BundleArena is thread-safe because:
// 1. The buffer is only modified through atomic offset updates
// 2. Each allocation gets a unique, non-overlapping slice
// 3. reset() must only be called when no references exist
unsafe impl Send for BundleArena {}
unsafe impl Sync for BundleArena {}

impl BundleArena {
    /// Create arena with specified capacity
    ///
    /// # Arguments
    /// * `capacity` - Size in bytes (e.g., 64MB for large projects)
    ///
    /// # Example
    /// ```
    /// use dx_bundle_core::BundleArena;
    /// let arena = BundleArena::new(64 * 1024 * 1024); // 64MB
    /// ```
    #[allow(clippy::uninit_vec)]
    pub fn new(capacity: usize) -> Self {
        let mut buffer = Vec::with_capacity(capacity);
        // SAFETY: We're setting length to capacity, all memory is allocated
        // The memory will be initialized when written to via alloc()
        unsafe {
            buffer.set_len(capacity);
        }

        Self {
            buffer: UnsafeCell::new(buffer),
            offset: AtomicUsize::new(0),
            capacity,
        }
    }

    /// Allocate bytes from arena (lock-free!)
    ///
    /// # Arguments
    /// * `size` - Number of bytes to allocate
    ///
    /// # Returns
    /// Mutable slice of allocated bytes, or None if arena exhausted
    ///
    /// # Safety
    /// This returns a mutable reference from an immutable self reference.
    /// This is safe because the arena uses atomic operations to ensure
    /// each allocation gets a unique, non-overlapping region.
    #[inline(always)]
    #[allow(clippy::mut_from_ref)]
    pub fn alloc(&self, size: usize) -> Option<&mut [u8]> {
        // Align to 8 bytes for better memory access
        let aligned_size = (size + 7) & !7;

        loop {
            let current = self.offset.load(Ordering::Relaxed);
            let new_offset = current + aligned_size;

            if new_offset > self.capacity {
                return None;
            }

            if self
                .offset
                .compare_exchange_weak(current, new_offset, Ordering::SeqCst, Ordering::Relaxed)
                .is_ok()
            {
                // SAFETY: We have exclusive access to this slice region
                // because we successfully atomically reserved it
                unsafe {
                    let ptr = (*self.buffer.get()).as_mut_ptr().add(current);
                    return Some(std::slice::from_raw_parts_mut(ptr, size));
                }
            }
            // CAS failed, retry
        }
    }

    /// Allocate and copy bytes into arena
    #[inline(always)]
    pub fn alloc_copy(&self, data: &[u8]) -> Option<&[u8]> {
        let buf = self.alloc(data.len())?;
        buf.copy_from_slice(data);
        // SAFETY: We just copied valid data into this buffer
        Some(unsafe { &*(buf as *const [u8]) })
    }

    /// Allocate string into arena
    #[inline(always)]
    pub fn alloc_str(&self, s: &str) -> Option<&str> {
        let buf = self.alloc(s.len())?;
        buf.copy_from_slice(s.as_bytes());
        // SAFETY: We copied valid UTF-8 string bytes
        Some(unsafe { std::str::from_utf8_unchecked(&*(buf as *const [u8])) })
    }

    /// Allocate a typed value
    #[inline(always)]
    #[allow(clippy::mut_from_ref)]
    pub fn alloc_value<T: Copy>(&self, value: T) -> Option<&mut T> {
        let size = std::mem::size_of::<T>();
        let buf = self.alloc(size)?;
        // SAFETY: Buffer is aligned and sized correctly for T
        unsafe {
            let ptr = buf.as_mut_ptr() as *mut T;
            std::ptr::write(ptr, value);
            Some(&mut *ptr)
        }
    }

    /// Reset arena for reuse (instant - just reset offset)
    ///
    /// # Safety
    /// Caller must ensure no references to arena memory exist
    #[inline(always)]
    pub fn reset(&self) {
        self.offset.store(0, Ordering::Relaxed);
    }

    /// Get total allocated bytes
    #[inline(always)]
    pub fn allocated(&self) -> usize {
        self.offset.load(Ordering::Relaxed)
    }

    /// Get remaining capacity
    #[inline(always)]
    pub fn remaining(&self) -> usize {
        self.capacity - self.allocated()
    }

    /// Get total capacity
    #[inline(always)]
    pub fn capacity(&self) -> usize {
        self.capacity
    }
}

/// Arena-backed output buffer for zero-copy emission
///
/// This buffer maintains contiguous memory by using unaligned allocation
/// to ensure sequential writes produce a continuous byte sequence.
pub struct ArenaOutput<'a> {
    arena: &'a BundleArena,
    start: usize,
    len: usize,
}

impl<'a> ArenaOutput<'a> {
    /// Create new output buffer backed by arena
    pub fn new(arena: &'a BundleArena) -> Self {
        let start = arena.offset.load(Ordering::Relaxed);
        Self {
            arena,
            start,
            len: 0,
        }
    }

    /// Allocate bytes without alignment (for contiguous output)
    #[inline(always)]
    #[allow(clippy::mut_from_ref)]
    fn alloc_unaligned(&self, size: usize) -> Option<&mut [u8]> {
        loop {
            let current = self.arena.offset.load(Ordering::Relaxed);
            let new_offset = current + size;

            if new_offset > self.arena.capacity {
                return None;
            }

            if self
                .arena
                .offset
                .compare_exchange_weak(current, new_offset, Ordering::SeqCst, Ordering::Relaxed)
                .is_ok()
            {
                // SAFETY: We have exclusive access to this slice region
                unsafe {
                    let ptr = (*self.arena.buffer.get()).as_mut_ptr().add(current);
                    return Some(std::slice::from_raw_parts_mut(ptr, size));
                }
            }
        }
    }

    /// Push single byte
    #[inline(always)]
    pub fn push(&mut self, byte: u8) {
        if let Some(buf) = self.alloc_unaligned(1) {
            buf[0] = byte;
            self.len += 1;
        }
    }

    /// Extend with bytes
    #[inline(always)]
    pub fn extend(&mut self, bytes: &[u8]) {
        if let Some(buf) = self.alloc_unaligned(bytes.len()) {
            buf.copy_from_slice(bytes);
            self.len += bytes.len();
        }
    }

    /// Extend with string
    #[inline(always)]
    pub fn extend_str(&mut self, s: &str) {
        self.extend(s.as_bytes());
    }

    /// Get output as slice
    pub fn as_slice(&self) -> &[u8] {
        // SAFETY: We track our own region within the arena
        unsafe {
            let ptr = (*self.arena.buffer.get()).as_ptr().add(self.start);
            std::slice::from_raw_parts(ptr, self.len)
        }
    }

    /// Get output length
    #[inline(always)]
    pub fn len(&self) -> usize {
        self.len
    }

    /// Check if empty
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Convert to owned Vec (for final output)
    pub fn to_vec(&self) -> Vec<u8> {
        self.as_slice().to_vec()
    }
}

impl std::io::Write for ArenaOutput<'_> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.extend(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// Default arena size per thread (16MB)
const THREAD_ARENA_SIZE: usize = 16 * 1024 * 1024;

thread_local! {
    /// Thread-local arena for parallel processing
    static THREAD_ARENA: BundleArena = BundleArena::new(THREAD_ARENA_SIZE);
}

/// Execute function with thread-local arena
///
/// Arena is automatically reset before use for clean state
///
/// # Example
/// ```
/// use dx_bundle_core::with_arena;
///
/// let result = with_arena(|arena| {
///     let buf = arena.alloc(100).unwrap();
///     buf.len()
/// });
/// assert_eq!(result, 100);
/// ```
pub fn with_arena<F, R>(f: F) -> R
where
    F: FnOnce(&BundleArena) -> R,
{
    THREAD_ARENA.with(|arena| {
        arena.reset();
        f(arena)
    })
}

/// Get reference to thread-local arena without reset
pub fn thread_arena() -> &'static BundleArena {
    // SAFETY: thread_local guarantees unique access per thread
    THREAD_ARENA.with(|arena| unsafe { &*(arena as *const BundleArena) })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_arena_alloc() {
        let arena = BundleArena::new(1024);

        let buf1 = arena.alloc(100).unwrap();
        assert_eq!(buf1.len(), 100);

        let buf2 = arena.alloc(200).unwrap();
        assert_eq!(buf2.len(), 200);

        // Check they don't overlap
        let ptr1 = buf1.as_ptr() as usize;
        let ptr2 = buf2.as_ptr() as usize;
        assert!(ptr2 >= ptr1 + 100);
    }

    #[test]
    fn test_arena_reset() {
        let arena = BundleArena::new(1024);

        arena.alloc(500).unwrap();
        assert!(arena.allocated() >= 500);

        arena.reset();
        assert_eq!(arena.allocated(), 0);
    }

    #[test]
    fn test_arena_output() {
        let arena = BundleArena::new(1024);
        let mut output = ArenaOutput::new(&arena);

        output.extend(b"Hello, ");
        output.extend(b"World!");

        assert_eq!(output.as_slice(), b"Hello, World!");
    }

    #[test]
    fn test_with_arena() {
        let result = with_arena(|arena| {
            let s = arena.alloc_str("test").unwrap();
            s.len()
        });
        assert_eq!(result, 4);
    }
}
