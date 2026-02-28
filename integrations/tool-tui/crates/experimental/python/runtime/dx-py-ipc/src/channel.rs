//! HBTP Channel for inter-process communication
//!
//! Provides high-performance message passing with zero-copy array transfer.

use crossbeam::queue::SegQueue;
use parking_lot::Mutex;
use std::sync::Arc;

use crate::protocol::{ArrayDtype, ArrayMetadata, HbtpFlags, HbtpHeader, MessageType};
use crate::shared_memory::{SharedArrayHandle, SharedMemoryArena, SharedMemoryError};

/// Error types for channel operations
#[derive(Debug, thiserror::Error)]
pub enum ChannelError {
    #[error("Channel is closed")]
    Closed,

    #[error("Shared memory error: {0}")]
    SharedMemory(#[from] SharedMemoryError),

    #[error("Invalid message")]
    InvalidMessage,

    #[error("Serialization error: {0}")]
    Serialization(String),
}

/// A message in the HBTP channel
#[derive(Debug)]
pub struct HbtpMessage {
    /// Message header
    pub header: HbtpHeader,
    /// Message payload
    pub payload: Vec<u8>,
}

impl HbtpMessage {
    /// Create a new message
    pub fn new(msg_type: MessageType, flags: HbtpFlags, payload: Vec<u8>) -> Self {
        Self {
            header: HbtpHeader::new(msg_type, flags, payload.len() as u32),
            payload,
        }
    }

    /// Create an acknowledgment message
    pub fn ack() -> Self {
        Self::new(MessageType::Ack, HbtpFlags::IS_RESPONSE, Vec::new())
    }

    /// Create a ping message
    pub fn ping() -> Self {
        Self::new(MessageType::Ping, HbtpFlags::REQUIRES_ACK, Vec::new())
    }

    /// Create a pong message
    pub fn pong() -> Self {
        Self::new(MessageType::Pong, HbtpFlags::IS_RESPONSE, Vec::new())
    }

    /// Serialize the message to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(8 + self.payload.len());
        bytes.extend_from_slice(&self.header.to_bytes());
        bytes.extend_from_slice(&self.payload);
        bytes
    }

    /// Deserialize a message from bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, ChannelError> {
        if bytes.len() < 8 {
            return Err(ChannelError::InvalidMessage);
        }

        let header_bytes: [u8; 8] = bytes[0..8].try_into().unwrap();
        let header = HbtpHeader::from_bytes(&header_bytes).ok_or(ChannelError::InvalidMessage)?;

        let payload_len = header.payload_len as usize;
        if bytes.len() < 8 + payload_len {
            return Err(ChannelError::InvalidMessage);
        }

        let payload = bytes[8..8 + payload_len].to_vec();

        Ok(Self { header, payload })
    }
}

/// HBTP Channel for bidirectional communication
pub struct HbtpChannel {
    /// Send queue
    send_queue: Arc<SegQueue<HbtpMessage>>,
    /// Receive queue
    recv_queue: Arc<SegQueue<HbtpMessage>>,
    /// Shared memory arena for large transfers
    arena: Option<Mutex<SharedMemoryArena>>,
    /// Threshold for using shared memory (bytes)
    shm_threshold: usize,
    /// Channel is closed
    closed: std::sync::atomic::AtomicBool,
}

impl HbtpChannel {
    /// Create a new HBTP channel
    pub fn new() -> Self {
        Self {
            send_queue: Arc::new(SegQueue::new()),
            recv_queue: Arc::new(SegQueue::new()),
            arena: None,
            shm_threshold: 64 * 1024, // 64KB default threshold
            closed: std::sync::atomic::AtomicBool::new(false),
        }
    }

    /// Create a channel with shared memory support
    pub fn with_shared_memory(arena_name: &str, capacity: usize) -> Result<Self, ChannelError> {
        let arena = SharedMemoryArena::create(arena_name, capacity)?;

        Ok(Self {
            send_queue: Arc::new(SegQueue::new()),
            recv_queue: Arc::new(SegQueue::new()),
            arena: Some(Mutex::new(arena)),
            shm_threshold: 64 * 1024,
            closed: std::sync::atomic::AtomicBool::new(false),
        })
    }

    /// Set the threshold for using shared memory
    pub fn set_shm_threshold(&mut self, threshold: usize) {
        self.shm_threshold = threshold;
    }

    /// Send a message
    pub fn send(&self, message: HbtpMessage) -> Result<(), ChannelError> {
        if self.closed.load(std::sync::atomic::Ordering::SeqCst) {
            return Err(ChannelError::Closed);
        }

        self.send_queue.push(message);
        Ok(())
    }

    /// Receive a message (non-blocking)
    pub fn try_recv(&self) -> Option<HbtpMessage> {
        self.recv_queue.pop()
    }

    /// Send an array with automatic shared memory optimization
    pub fn send_array(&self, data: &[u8], metadata: ArrayMetadata) -> Result<(), ChannelError> {
        if self.closed.load(std::sync::atomic::Ordering::SeqCst) {
            return Err(ChannelError::Closed);
        }

        // Use shared memory for large arrays
        if data.len() >= self.shm_threshold {
            if let Some(ref arena_mutex) = self.arena {
                let mut arena = arena_mutex.lock();
                let handle = SharedArrayHandle::from_array(&mut arena, data, metadata)?;

                let message = HbtpMessage::new(
                    MessageType::TransferArray,
                    HbtpFlags::SHARED_MEMORY,
                    handle.to_bytes(),
                );

                self.send_queue.push(message);
                return Ok(());
            }
        }

        // Fall back to copying for small arrays or no shared memory
        let mut payload = Vec::with_capacity(data.len() + 128);

        // Serialize metadata
        payload.push(metadata.dtype as u8);
        payload.push(metadata.ndim);
        for i in 0..8 {
            payload.extend_from_slice(&metadata.shape[i].to_le_bytes());
        }
        for i in 0..8 {
            payload.extend_from_slice(&metadata.strides[i].to_le_bytes());
        }
        payload.extend_from_slice(&metadata.size.to_le_bytes());

        // Append data
        payload.extend_from_slice(data);

        let message = HbtpMessage::new(MessageType::TransferArray, HbtpFlags::empty(), payload);

        self.send_queue.push(message);
        Ok(())
    }

    /// Receive an array
    pub fn recv_array(&self) -> Result<Option<(Vec<u8>, ArrayMetadata)>, ChannelError> {
        let message = match self.recv_queue.pop() {
            Some(m) => m,
            None => return Ok(None),
        };

        if message.header.msg_type != MessageType::TransferArray {
            // Put it back and return None
            self.recv_queue.push(message);
            return Ok(None);
        }

        if message.header.flags.contains(HbtpFlags::SHARED_MEMORY) {
            // Shared memory transfer
            let handle = SharedArrayHandle::from_bytes(&message.payload)?;

            if let Some(ref arena_mutex) = self.arena {
                let arena = arena_mutex.lock();
                let data = handle.as_slice(&arena)?.to_vec();
                return Ok(Some((data, handle.metadata)));
            } else {
                return Err(ChannelError::InvalidMessage);
            }
        }

        // Direct transfer - parse metadata and data
        let payload = &message.payload;
        if payload.len() < 2 + 64 + 64 + 8 {
            return Err(ChannelError::InvalidMessage);
        }

        let mut pos = 0;
        let dtype = ArrayDtype::from_u8(payload[pos]).ok_or(ChannelError::InvalidMessage)?;
        pos += 1;
        let ndim = payload[pos];
        pos += 1;

        let mut shape = [0u64; 8];
        for item in &mut shape {
            *item = u64::from_le_bytes([
                payload[pos],
                payload[pos + 1],
                payload[pos + 2],
                payload[pos + 3],
                payload[pos + 4],
                payload[pos + 5],
                payload[pos + 6],
                payload[pos + 7],
            ]);
            pos += 8;
        }

        let mut strides = [0i64; 8];
        for item in &mut strides {
            *item = i64::from_le_bytes([
                payload[pos],
                payload[pos + 1],
                payload[pos + 2],
                payload[pos + 3],
                payload[pos + 4],
                payload[pos + 5],
                payload[pos + 6],
                payload[pos + 7],
            ]);
            pos += 8;
        }

        let size = u64::from_le_bytes([
            payload[pos],
            payload[pos + 1],
            payload[pos + 2],
            payload[pos + 3],
            payload[pos + 4],
            payload[pos + 5],
            payload[pos + 6],
            payload[pos + 7],
        ]);
        pos += 8;

        let metadata = ArrayMetadata {
            dtype,
            ndim,
            shape,
            strides,
            size,
        };

        let data = payload[pos..].to_vec();

        Ok(Some((data, metadata)))
    }

    /// Close the channel
    pub fn close(&self) {
        self.closed.store(true, std::sync::atomic::Ordering::SeqCst);
    }

    /// Check if the channel is closed
    pub fn is_closed(&self) -> bool {
        self.closed.load(std::sync::atomic::Ordering::SeqCst)
    }

    /// Get the send queue for testing/integration
    pub fn send_queue(&self) -> &Arc<SegQueue<HbtpMessage>> {
        &self.send_queue
    }

    /// Get the receive queue for testing/integration
    pub fn recv_queue(&self) -> &Arc<SegQueue<HbtpMessage>> {
        &self.recv_queue
    }
}

impl Default for HbtpChannel {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_roundtrip() {
        let message =
            HbtpMessage::new(MessageType::CallFunction, HbtpFlags::REQUIRES_ACK, vec![1, 2, 3, 4]);

        let bytes = message.to_bytes();
        let restored = HbtpMessage::from_bytes(&bytes).unwrap();

        assert_eq!(restored.header.msg_type, MessageType::CallFunction);
        assert!(restored.header.flags.contains(HbtpFlags::REQUIRES_ACK));
        assert_eq!(restored.payload, vec![1, 2, 3, 4]);
    }

    #[test]
    fn test_channel_send_recv() {
        let channel = HbtpChannel::new();

        // Simulate receiving by pushing to recv_queue
        channel.recv_queue.push(HbtpMessage::ping());

        let msg = channel.try_recv().unwrap();
        assert_eq!(msg.header.msg_type, MessageType::Ping);
    }

    #[test]
    fn test_array_transfer() {
        let channel = HbtpChannel::new();

        let data: Vec<u8> = (0..100).collect();
        let metadata = ArrayMetadata::new(ArrayDtype::UInt8, &[100]);

        channel.send_array(&data, metadata.clone()).unwrap();

        // Move from send to recv for testing
        while let Some(msg) = channel.send_queue.pop() {
            channel.recv_queue.push(msg);
        }

        let (recv_data, recv_meta) = channel.recv_array().unwrap().unwrap();
        assert_eq!(recv_data, data);
        assert_eq!(recv_meta.dtype, ArrayDtype::UInt8);
    }
}
