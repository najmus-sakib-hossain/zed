//! Lock-free ring buffer for streaming support.
//!
//! Implements a single-producer single-consumer (SPSC) ring buffer
//! with atomic operations for thread-safe streaming.

use crate::DCPError;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};

/// Backpressure signal for flow control
#[derive(Debug)]
pub struct Backpressure {
    /// Available capacity in bytes
    available: AtomicU32,
}

impl Backpressure {
    /// Create a new backpressure signal with initial capacity
    pub fn new(capacity: u32) -> Self {
        Self {
            available: AtomicU32::new(capacity),
        }
    }

    /// Get available capacity
    #[inline]
    pub fn available(&self) -> u32 {
        self.available.load(Ordering::Acquire)
    }

    /// Try to reserve capacity, returns true if successful
    #[inline]
    pub fn try_reserve(&self, amount: u32) -> bool {
        let mut current = self.available.load(Ordering::Acquire);
        loop {
            if current < amount {
                return false;
            }
            match self.available.compare_exchange_weak(
                current,
                current - amount,
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(_) => return true,
                Err(new_current) => current = new_current,
            }
        }
    }

    /// Release capacity back
    #[inline]
    pub fn release(&self, amount: u32) {
        self.available.fetch_add(amount, Ordering::Release);
    }

    /// Check if backpressure is active (no capacity available)
    #[inline]
    pub fn is_full(&self) -> bool {
        self.available() == 0
    }
}

/// Lock-free ring buffer for streaming
///
/// Uses atomic operations for single-producer single-consumer (SPSC) pattern.
/// The buffer is power-of-two sized for efficient modulo operations.
pub struct StreamRingBuffer {
    /// Ring buffer storage
    buffer: Box<[u8]>,
    /// Write position (producer) - monotonically increasing
    write_pos: AtomicU64,
    /// Read position (consumer) - monotonically increasing
    read_pos: AtomicU64,
    /// Capacity (must be power of 2)
    capacity: usize,
    /// Mask for efficient modulo (capacity - 1)
    mask: usize,
    /// Backpressure signal
    backpressure: Backpressure,
}

impl StreamRingBuffer {
    /// Create a new ring buffer with the given capacity.
    /// Capacity will be rounded up to the next power of 2.
    pub fn new(capacity: usize) -> Self {
        let capacity = capacity.next_power_of_two().max(16);
        Self {
            buffer: vec![0u8; capacity].into_boxed_slice(),
            write_pos: AtomicU64::new(0),
            read_pos: AtomicU64::new(0),
            capacity,
            mask: capacity - 1,
            backpressure: Backpressure::new(capacity as u32),
        }
    }

    /// Get the capacity of the buffer
    #[inline]
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Get the number of bytes available to read
    #[inline]
    pub fn len(&self) -> usize {
        let write = self.write_pos.load(Ordering::Acquire);
        let read = self.read_pos.load(Ordering::Acquire);
        (write - read) as usize
    }

    /// Check if the buffer is empty
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Get available space for writing
    #[inline]
    pub fn available_space(&self) -> usize {
        self.capacity - self.len()
    }

    /// Get the backpressure signal
    pub fn backpressure(&self) -> &Backpressure {
        &self.backpressure
    }

    /// Push data into the buffer.
    /// Returns Err(Backpressure) if there's not enough space.
    pub fn push(&self, data: &[u8]) -> Result<(), DCPError> {
        let len = data.len();
        if len > self.capacity {
            return Err(DCPError::OutOfBounds);
        }

        // Check if we have space
        if !self.backpressure.try_reserve(len as u32) {
            return Err(DCPError::Backpressure);
        }

        let write = self.write_pos.load(Ordering::Acquire);
        let start = (write as usize) & self.mask;

        // Get mutable access to buffer (safe because we're single producer)
        let buffer = unsafe {
            std::slice::from_raw_parts_mut(self.buffer.as_ptr() as *mut u8, self.capacity)
        };

        // Handle wrap-around
        if start + len <= self.capacity {
            buffer[start..start + len].copy_from_slice(data);
        } else {
            let first_part = self.capacity - start;
            buffer[start..].copy_from_slice(&data[..first_part]);
            buffer[..len - first_part].copy_from_slice(&data[first_part..]);
        }

        // Update write position with release ordering
        self.write_pos.store(write + len as u64, Ordering::Release);
        Ok(())
    }

    /// Pop data from the buffer into the provided slice.
    /// Returns the number of bytes read.
    pub fn pop(&self, buf: &mut [u8]) -> usize {
        let available = self.len();
        let to_read = buf.len().min(available);

        if to_read == 0 {
            return 0;
        }

        let read = self.read_pos.load(Ordering::Acquire);
        let start = (read as usize) & self.mask;

        // Handle wrap-around
        if start + to_read <= self.capacity {
            buf[..to_read].copy_from_slice(&self.buffer[start..start + to_read]);
        } else {
            let first_part = self.capacity - start;
            buf[..first_part].copy_from_slice(&self.buffer[start..]);
            buf[first_part..to_read].copy_from_slice(&self.buffer[..to_read - first_part]);
        }

        // Update read position and release backpressure
        self.read_pos.store(read + to_read as u64, Ordering::Release);
        self.backpressure.release(to_read as u32);

        to_read
    }

    /// Peek at data without consuming it.
    /// Returns the number of bytes peeked.
    pub fn peek(&self, buf: &mut [u8]) -> usize {
        let available = self.len();
        let to_read = buf.len().min(available);

        if to_read == 0 {
            return 0;
        }

        let read = self.read_pos.load(Ordering::Acquire);
        let start = (read as usize) & self.mask;

        // Handle wrap-around
        if start + to_read <= self.capacity {
            buf[..to_read].copy_from_slice(&self.buffer[start..start + to_read]);
        } else {
            let first_part = self.capacity - start;
            buf[..first_part].copy_from_slice(&self.buffer[start..]);
            buf[first_part..to_read].copy_from_slice(&self.buffer[..to_read - first_part]);
        }

        to_read
    }

    /// Clear the buffer
    pub fn clear(&self) {
        let write = self.write_pos.load(Ordering::Acquire);
        let read = self.read_pos.load(Ordering::Acquire);
        let consumed = (write - read) as u32;

        self.read_pos.store(write, Ordering::Release);
        self.backpressure.release(consumed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backpressure() {
        let bp = Backpressure::new(100);
        assert_eq!(bp.available(), 100);
        assert!(!bp.is_full());

        assert!(bp.try_reserve(50));
        assert_eq!(bp.available(), 50);

        assert!(!bp.try_reserve(60));
        assert_eq!(bp.available(), 50);

        bp.release(30);
        assert_eq!(bp.available(), 80);
    }

    #[test]
    fn test_ring_buffer_basic() {
        let rb = StreamRingBuffer::new(64);
        assert!(rb.is_empty());
        assert_eq!(rb.capacity(), 64);

        rb.push(b"hello").unwrap();
        assert_eq!(rb.len(), 5);

        let mut buf = [0u8; 10];
        let read = rb.pop(&mut buf);
        assert_eq!(read, 5);
        assert_eq!(&buf[..5], b"hello");
        assert!(rb.is_empty());
    }

    #[test]
    fn test_ring_buffer_wrap_around() {
        let rb = StreamRingBuffer::new(16);

        // Fill most of the buffer
        rb.push(&[1u8; 12]).unwrap();

        // Read some
        let mut buf = [0u8; 8];
        rb.pop(&mut buf);

        // Write more (will wrap around)
        rb.push(&[2u8; 10]).unwrap();

        // Read all
        let mut buf = [0u8; 14];
        let read = rb.pop(&mut buf);
        assert_eq!(read, 14);
        assert_eq!(&buf[..4], &[1u8; 4]);
        assert_eq!(&buf[4..14], &[2u8; 10]);
    }

    #[test]
    fn test_ring_buffer_backpressure() {
        let rb = StreamRingBuffer::new(16);

        // Fill the buffer
        rb.push(&[0u8; 16]).unwrap();

        // Should fail with backpressure
        assert_eq!(rb.push(&[0u8; 1]), Err(DCPError::Backpressure));

        // Read some to make space
        let mut buf = [0u8; 8];
        rb.pop(&mut buf);

        // Now should succeed
        rb.push(&[0u8; 8]).unwrap();
    }

    #[test]
    fn test_ring_buffer_peek() {
        let rb = StreamRingBuffer::new(32);
        rb.push(b"test data").unwrap();

        let mut buf = [0u8; 4];
        let peeked = rb.peek(&mut buf);
        assert_eq!(peeked, 4);
        assert_eq!(&buf, b"test");

        // Data should still be there
        assert_eq!(rb.len(), 9);
    }
}
