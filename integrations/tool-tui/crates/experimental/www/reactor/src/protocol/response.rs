//! Pre-allocated response buffer for zero-allocation responses.

use super::{HbtpFlags, HbtpHeader, HbtpOpcode};

/// Pre-allocated response buffer for HBTP messages.
///
/// This buffer is designed to be reused across multiple responses,
/// avoiding allocations in the hot path.
pub struct ResponseBuffer {
    /// Internal buffer.
    buffer: Vec<u8>,
    /// Current write position.
    position: usize,
}

impl ResponseBuffer {
    /// Default buffer capacity.
    pub const DEFAULT_CAPACITY: usize = 4096;

    /// Create a new response buffer with default capacity.
    pub fn new() -> Self {
        Self::with_capacity(Self::DEFAULT_CAPACITY)
    }

    /// Create a new response buffer with specified capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        let buffer = vec![0; capacity];
        Self {
            buffer,
            position: 0,
        }
    }

    /// Reset the buffer for reuse.
    ///
    /// This does not deallocate memory, just resets the write position.
    pub fn reset(&mut self) {
        self.position = 0;
    }

    /// Get the current buffer contents as a byte slice.
    pub fn as_bytes(&self) -> &[u8] {
        &self.buffer[..self.position]
    }

    /// Get the current length of written data.
    pub fn len(&self) -> usize {
        self.position
    }

    /// Check if the buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.position == 0
    }

    /// Write a header to the buffer.
    fn write_header(&mut self, opcode: HbtpOpcode, flags: HbtpFlags, sequence: u16, length: u32) {
        let header = HbtpHeader::new(opcode, flags, sequence, length);
        let bytes = header.to_bytes();
        self.buffer[self.position..self.position + HbtpHeader::SIZE].copy_from_slice(&bytes);
        self.position += HbtpHeader::SIZE;
    }

    /// Write raw bytes to the buffer.
    fn write_bytes(&mut self, data: &[u8]) {
        let end = self.position + data.len();
        if end > self.buffer.len() {
            self.buffer.resize(end, 0);
        }
        self.buffer[self.position..end].copy_from_slice(data);
        self.position = end;
    }

    /// Write a Pong response.
    pub fn write_pong(&mut self, sequence: u16) {
        self.write_header(HbtpOpcode::Pong, HbtpFlags::empty(), sequence, 0);
    }

    /// Write an RPC response.
    pub fn write_rpc_response(&mut self, sequence: u16, payload: &[u8]) {
        self.write_header(
            HbtpOpcode::RpcResponse,
            HbtpFlags::empty(),
            sequence,
            payload.len() as u32,
        );
        self.write_bytes(payload);
    }

    /// Write an RPC error response.
    pub fn write_rpc_error(&mut self, sequence: u16, error: &[u8]) {
        self.write_header(HbtpOpcode::RpcError, HbtpFlags::empty(), sequence, error.len() as u32);
        self.write_bytes(error);
    }

    /// Write a state sync response.
    pub fn write_state_sync(&mut self, sequence: u16, state: &[u8], compressed: bool) {
        let flags = if compressed {
            HbtpFlags::COMPRESSED
        } else {
            HbtpFlags::empty()
        };
        self.write_header(HbtpOpcode::StateSync, flags, sequence, state.len() as u32);
        self.write_bytes(state);
    }

    /// Write a close message.
    pub fn write_close(&mut self, sequence: u16) {
        self.write_header(HbtpOpcode::Close, HbtpFlags::FINAL, sequence, 0);
    }
}

impl Default for ResponseBuffer {
    fn default() -> Self {
        Self::new()
    }
}
