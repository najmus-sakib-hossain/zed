//! Shared memory ring buffer for large payloads

use dx_py_core::ProtocolError;
use std::sync::atomic::{AtomicUsize, Ordering};

/// Ring buffer for shared memory communication
pub struct RingBuffer {
    /// Underlying memory
    data: Vec<u8>,
    /// Write position
    write_pos: AtomicUsize,
    /// Read position
    read_pos: AtomicUsize,
    /// Buffer capacity
    capacity: usize,
}

impl RingBuffer {
    /// Create a new ring buffer with the given capacity
    pub fn new(capacity: usize) -> Self {
        Self {
            data: vec![0; capacity],
            write_pos: AtomicUsize::new(0),
            read_pos: AtomicUsize::new(0),
            capacity,
        }
    }

    /// Get the capacity of the buffer
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Check if the buffer is empty
    pub fn is_empty(&self) -> bool {
        self.write_pos.load(Ordering::Acquire) == self.read_pos.load(Ordering::Acquire)
    }

    /// Get the number of bytes available to read
    pub fn available(&self) -> usize {
        let write = self.write_pos.load(Ordering::Acquire);
        let read = self.read_pos.load(Ordering::Acquire);
        if write >= read {
            write - read
        } else {
            self.capacity - read + write
        }
    }

    /// Get the number of bytes available to write
    pub fn free_space(&self) -> usize {
        self.capacity - self.available() - 1
    }

    /// Write data to the buffer
    pub fn write(&mut self, data: &[u8]) -> Result<(), ProtocolError> {
        if data.len() > self.free_space() {
            return Err(ProtocolError::SharedMemoryError("Ring buffer full".to_string()));
        }

        let write_pos = self.write_pos.load(Ordering::Acquire);

        for (i, &byte) in data.iter().enumerate() {
            let pos = (write_pos + i) % self.capacity;
            self.data[pos] = byte;
        }

        self.write_pos
            .store((write_pos + data.len()) % self.capacity, Ordering::Release);

        Ok(())
    }

    /// Read data from the buffer
    pub fn read(&mut self, len: usize) -> Result<Vec<u8>, ProtocolError> {
        if len > self.available() {
            return Err(ProtocolError::SharedMemoryError(
                "Not enough data in ring buffer".to_string(),
            ));
        }

        let read_pos = self.read_pos.load(Ordering::Acquire);
        let mut result = Vec::with_capacity(len);

        for i in 0..len {
            let pos = (read_pos + i) % self.capacity;
            result.push(self.data[pos]);
        }

        self.read_pos.store((read_pos + len) % self.capacity, Ordering::Release);

        Ok(result)
    }
}
