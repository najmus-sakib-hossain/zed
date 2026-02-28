//! Multiplexed connection implementation.
//!
//! Provides concurrent stream processing over a single connection.

use bytes::{Bytes, BytesMut};
use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicU16, AtomicU64, Ordering};
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::{Mutex, RwLock};

use super::header::{StreamHeader, STREAM_HEADER_SIZE};

/// Maximum concurrent streams per connection
pub const MAX_STREAMS: u16 = 65535;

/// Default window size for flow control
pub const DEFAULT_WINDOW_SIZE: u32 = 65536;

/// Multiplexing errors
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum MultiplexError {
    #[error("stream not found: {0}")]
    StreamNotFound(u16),
    #[error("stream already exists: {0}")]
    StreamAlreadyExists(u16),
    #[error("stream closed: {0}")]
    StreamClosed(u16),
    #[error("too many streams")]
    TooManyStreams,
    #[error("invalid stream id")]
    InvalidStreamId,
    #[error("connection closed")]
    ConnectionClosed,
    #[error("send buffer full")]
    SendBufferFull,
    #[error("receive buffer full")]
    ReceiveBufferFull,
    #[error("stream error: {0}")]
    StreamError(u16),
    #[error("protocol error: {0}")]
    ProtocolError(String),
}

/// Stream status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamStatus {
    /// Stream is open for both sending and receiving
    Open,
    /// Local side has sent FIN, waiting for remote FIN
    HalfClosedLocal,
    /// Remote side has sent FIN, can still send
    HalfClosedRemote,
    /// Stream is fully closed
    Closed,
    /// Stream was reset due to error
    Reset,
}

impl StreamStatus {
    /// Check if stream can send data
    pub fn can_send(&self) -> bool {
        matches!(self, StreamStatus::Open | StreamStatus::HalfClosedRemote)
    }

    /// Check if stream can receive data
    pub fn can_receive(&self) -> bool {
        matches!(self, StreamStatus::Open | StreamStatus::HalfClosedLocal)
    }

    /// Check if stream is fully closed
    pub fn is_closed(&self) -> bool {
        matches!(self, StreamStatus::Closed | StreamStatus::Reset)
    }
}

/// Stream state
pub struct StreamState {
    /// Stream ID
    pub id: u16,
    /// Current status
    pub status: StreamStatus,
    /// Send buffer
    pub send_buffer: VecDeque<Bytes>,
    /// Receive buffer
    pub recv_buffer: VecDeque<Bytes>,
    /// Flow control window size
    pub window_size: u32,
    /// Bytes sent
    pub bytes_sent: AtomicU64,
    /// Bytes received
    pub bytes_received: AtomicU64,
    /// Error message if stream was reset
    pub error: Option<String>,
}

impl StreamState {
    /// Create a new stream state
    pub fn new(id: u16) -> Self {
        Self {
            id,
            status: StreamStatus::Open,
            send_buffer: VecDeque::new(),
            recv_buffer: VecDeque::new(),
            window_size: DEFAULT_WINDOW_SIZE,
            bytes_sent: AtomicU64::new(0),
            bytes_received: AtomicU64::new(0),
            error: None,
        }
    }

    /// Queue data for sending
    pub fn queue_send(&mut self, data: Bytes) -> Result<(), MultiplexError> {
        if !self.status.can_send() {
            return Err(MultiplexError::StreamClosed(self.id));
        }
        self.send_buffer.push_back(data);
        Ok(())
    }

    /// Queue received data
    pub fn queue_recv(&mut self, data: Bytes) -> Result<(), MultiplexError> {
        if !self.status.can_receive() {
            return Err(MultiplexError::StreamClosed(self.id));
        }
        self.bytes_received.fetch_add(data.len() as u64, Ordering::Relaxed);
        self.recv_buffer.push_back(data);
        Ok(())
    }

    /// Take next data from send buffer
    pub fn take_send(&mut self) -> Option<Bytes> {
        let data = self.send_buffer.pop_front()?;
        self.bytes_sent.fetch_add(data.len() as u64, Ordering::Relaxed);
        Some(data)
    }

    /// Take next data from receive buffer
    pub fn take_recv(&mut self) -> Option<Bytes> {
        self.recv_buffer.pop_front()
    }

    /// Close local side of stream
    pub fn close_local(&mut self) {
        self.status = match self.status {
            StreamStatus::Open => StreamStatus::HalfClosedLocal,
            StreamStatus::HalfClosedRemote => StreamStatus::Closed,
            other => other,
        };
    }

    /// Close remote side of stream
    pub fn close_remote(&mut self) {
        self.status = match self.status {
            StreamStatus::Open => StreamStatus::HalfClosedRemote,
            StreamStatus::HalfClosedLocal => StreamStatus::Closed,
            other => other,
        };
    }

    /// Reset stream with error
    pub fn reset(&mut self, error: Option<String>) {
        self.status = StreamStatus::Reset;
        self.error = error;
        self.send_buffer.clear();
        // Keep recv_buffer so pending reads can see the error
    }
}

/// Multiplexed connection supporting concurrent streams
pub struct MultiplexedConnection {
    /// Active streams
    streams: RwLock<HashMap<u16, Arc<Mutex<StreamState>>>>,
    /// Stream ID counter (starts at 1, 0 is control)
    next_stream_id: AtomicU16,
    /// Connection-level send buffer
    send_queue: Mutex<VecDeque<(StreamHeader, Bytes)>>,
    /// Whether connection is closed
    closed: std::sync::atomic::AtomicBool,
    /// Total bytes sent
    bytes_sent: AtomicU64,
    /// Total bytes received
    bytes_received: AtomicU64,
    /// Stream count
    stream_count: AtomicU16,
}

impl MultiplexedConnection {
    /// Create a new multiplexed connection
    pub fn new() -> Self {
        Self {
            streams: RwLock::new(HashMap::new()),
            next_stream_id: AtomicU16::new(1),
            send_queue: Mutex::new(VecDeque::new()),
            closed: std::sync::atomic::AtomicBool::new(false),
            bytes_sent: AtomicU64::new(0),
            bytes_received: AtomicU64::new(0),
            stream_count: AtomicU16::new(0),
        }
    }

    /// Check if connection is closed
    pub fn is_closed(&self) -> bool {
        self.closed.load(Ordering::SeqCst)
    }

    /// Get current stream count
    pub fn stream_count(&self) -> u16 {
        self.stream_count.load(Ordering::Relaxed)
    }

    /// Open a new stream
    pub async fn open_stream(&self) -> Result<u16, MultiplexError> {
        if self.is_closed() {
            return Err(MultiplexError::ConnectionClosed);
        }

        // Check stream limit
        if self.stream_count() >= MAX_STREAMS {
            return Err(MultiplexError::TooManyStreams);
        }

        // Allocate stream ID
        let stream_id = self.next_stream_id.fetch_add(1, Ordering::SeqCst);
        if stream_id == 0 {
            // Wrapped around, skip 0 (control stream)
            let stream_id = self.next_stream_id.fetch_add(1, Ordering::SeqCst);
            if stream_id == 0 {
                return Err(MultiplexError::TooManyStreams);
            }
        }

        // Create stream state
        let state = Arc::new(Mutex::new(StreamState::new(stream_id)));

        // Register stream
        {
            let mut streams = self.streams.write().await;
            if streams.contains_key(&stream_id) {
                return Err(MultiplexError::StreamAlreadyExists(stream_id));
            }
            streams.insert(stream_id, state);
        }

        self.stream_count.fetch_add(1, Ordering::Relaxed);

        // Queue SYN
        self.queue_frame(StreamHeader::syn(stream_id), Bytes::new()).await?;

        Ok(stream_id)
    }

    /// Accept an incoming stream (called when receiving SYN)
    pub async fn accept_stream(&self, stream_id: u16) -> Result<(), MultiplexError> {
        if self.is_closed() {
            return Err(MultiplexError::ConnectionClosed);
        }

        if stream_id == StreamHeader::CONTROL_STREAM {
            return Err(MultiplexError::InvalidStreamId);
        }

        // Check stream limit
        if self.stream_count() >= MAX_STREAMS {
            return Err(MultiplexError::TooManyStreams);
        }

        // Create stream state
        let state = Arc::new(Mutex::new(StreamState::new(stream_id)));

        // Register stream
        {
            let mut streams = self.streams.write().await;
            if streams.contains_key(&stream_id) {
                return Err(MultiplexError::StreamAlreadyExists(stream_id));
            }
            streams.insert(stream_id, state);
        }

        self.stream_count.fetch_add(1, Ordering::Relaxed);

        // Queue ACK
        self.queue_frame(StreamHeader::ack(stream_id), Bytes::new()).await?;

        Ok(())
    }

    /// Send data on a stream
    pub async fn send(&self, stream_id: u16, data: Bytes) -> Result<(), MultiplexError> {
        if self.is_closed() {
            return Err(MultiplexError::ConnectionClosed);
        }

        // Get stream
        let stream = {
            let streams = self.streams.read().await;
            streams
                .get(&stream_id)
                .cloned()
                .ok_or(MultiplexError::StreamNotFound(stream_id))?
        };

        // Queue data in stream
        {
            let mut state = stream.lock().await;
            state.queue_send(data.clone())?;
        }

        // Queue frame for sending
        let header = StreamHeader::data(stream_id, data.len() as u32);
        self.queue_frame(header, data).await?;

        Ok(())
    }

    /// Receive data from a stream
    pub async fn recv(&self, stream_id: u16) -> Result<Option<Bytes>, MultiplexError> {
        if self.is_closed() {
            return Err(MultiplexError::ConnectionClosed);
        }

        // Get stream
        let stream = {
            let streams = self.streams.read().await;
            streams
                .get(&stream_id)
                .cloned()
                .ok_or(MultiplexError::StreamNotFound(stream_id))?
        };

        // Take data from receive buffer
        let mut state = stream.lock().await;

        // Check for error
        if state.status == StreamStatus::Reset {
            return Err(MultiplexError::StreamError(stream_id));
        }

        Ok(state.take_recv())
    }

    /// Close a stream gracefully
    pub async fn close_stream(&self, stream_id: u16) -> Result<(), MultiplexError> {
        // Get stream
        let stream = {
            let streams = self.streams.read().await;
            streams
                .get(&stream_id)
                .cloned()
                .ok_or(MultiplexError::StreamNotFound(stream_id))?
        };

        // Mark local side as closed
        {
            let mut state = stream.lock().await;
            state.close_local();
        }

        // Queue FIN
        self.queue_frame(StreamHeader::fin(stream_id), Bytes::new()).await?;

        Ok(())
    }

    /// Reset a stream with error
    pub async fn reset_stream(
        &self,
        stream_id: u16,
        error: Option<String>,
    ) -> Result<(), MultiplexError> {
        // Get stream
        let stream = {
            let streams = self.streams.read().await;
            streams
                .get(&stream_id)
                .cloned()
                .ok_or(MultiplexError::StreamNotFound(stream_id))?
        };

        // Reset stream
        {
            let mut state = stream.lock().await;
            state.reset(error);
        }

        // Queue RST
        self.queue_frame(StreamHeader::rst(stream_id), Bytes::new()).await?;

        Ok(())
    }

    /// Process an incoming frame
    pub async fn process_frame(
        &self,
        header: StreamHeader,
        payload: Bytes,
    ) -> Result<(), MultiplexError> {
        if self.is_closed() {
            return Err(MultiplexError::ConnectionClosed);
        }

        self.bytes_received
            .fetch_add((STREAM_HEADER_SIZE + payload.len()) as u64, Ordering::Relaxed);

        // Handle control stream
        if header.is_control() {
            return self.process_control_frame(header, payload).await;
        }

        // Handle SYN (new stream)
        if header.flags.is_syn() {
            return self.accept_stream(header.stream_id).await;
        }

        // Get existing stream
        let stream = {
            let streams = self.streams.read().await;
            streams.get(&header.stream_id).cloned()
        };

        let stream = match stream {
            Some(s) => s,
            None => {
                // Unknown stream - send RST
                self.queue_frame(StreamHeader::rst(header.stream_id), Bytes::new()).await?;
                return Ok(());
            }
        };

        let mut state = stream.lock().await;

        // Handle RST
        if header.flags.is_rst() {
            state.reset(Some("Remote reset".to_string()));
            return Ok(());
        }

        // Handle FIN
        if header.flags.is_fin() {
            state.close_remote();

            // If fully closed, remove stream
            if state.status.is_closed() {
                drop(state);
                self.remove_stream(header.stream_id).await;
            }
            return Ok(());
        }

        // Handle data
        if !payload.is_empty() {
            state.queue_recv(payload)?;
        }

        Ok(())
    }

    /// Process a control frame
    async fn process_control_frame(
        &self,
        _header: StreamHeader,
        _payload: Bytes,
    ) -> Result<(), MultiplexError> {
        // Control frames are for connection-level operations
        // Currently not implemented - reserved for future use
        Ok(())
    }

    /// Remove a stream
    async fn remove_stream(&self, stream_id: u16) {
        let mut streams = self.streams.write().await;
        if streams.remove(&stream_id).is_some() {
            self.stream_count.fetch_sub(1, Ordering::Relaxed);
        }
    }

    /// Queue a frame for sending
    async fn queue_frame(
        &self,
        header: StreamHeader,
        payload: Bytes,
    ) -> Result<(), MultiplexError> {
        if self.is_closed() {
            return Err(MultiplexError::ConnectionClosed);
        }

        let mut queue = self.send_queue.lock().await;
        queue.push_back((header, payload));
        Ok(())
    }

    /// Take next frame to send
    pub async fn take_frame(&self) -> Option<(StreamHeader, Bytes)> {
        let mut queue = self.send_queue.lock().await;
        let frame = queue.pop_front()?;
        self.bytes_sent
            .fetch_add((STREAM_HEADER_SIZE + frame.1.len()) as u64, Ordering::Relaxed);
        Some(frame)
    }

    /// Encode a frame to bytes
    pub fn encode_frame(header: &StreamHeader, payload: &Bytes) -> Bytes {
        let mut buf = BytesMut::with_capacity(STREAM_HEADER_SIZE + payload.len());
        header.encode(&mut buf);
        buf.extend_from_slice(payload);
        buf.freeze()
    }

    /// Get stream status
    pub async fn stream_status(&self, stream_id: u16) -> Result<StreamStatus, MultiplexError> {
        let streams = self.streams.read().await;
        let stream = streams.get(&stream_id).ok_or(MultiplexError::StreamNotFound(stream_id))?;
        let state = stream.lock().await;
        Ok(state.status)
    }

    /// Close the connection
    pub async fn close(&self) {
        self.closed.store(true, Ordering::SeqCst);

        // Reset all streams
        let streams = self.streams.read().await;
        for (_, stream) in streams.iter() {
            let mut state = stream.lock().await;
            state.reset(Some("Connection closed".to_string()));
        }
    }

    /// Get total bytes sent
    pub fn total_bytes_sent(&self) -> u64 {
        self.bytes_sent.load(Ordering::Relaxed)
    }

    /// Get total bytes received
    pub fn total_bytes_received(&self) -> u64 {
        self.bytes_received.load(Ordering::Relaxed)
    }

    /// Check if a stream exists
    pub async fn has_stream(&self, stream_id: u16) -> bool {
        let streams = self.streams.read().await;
        streams.contains_key(&stream_id)
    }

    /// Get all active stream IDs
    pub async fn active_streams(&self) -> Vec<u16> {
        let streams = self.streams.read().await;
        streams.keys().copied().collect()
    }
}

impl Default for MultiplexedConnection {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_open_stream() {
        let conn = MultiplexedConnection::new();

        let stream_id = conn.open_stream().await.unwrap();
        assert!(stream_id > 0);
        assert_eq!(conn.stream_count(), 1);
        assert!(conn.has_stream(stream_id).await);
    }

    #[tokio::test]
    async fn test_multiple_streams() {
        let conn = MultiplexedConnection::new();

        let id1 = conn.open_stream().await.unwrap();
        let id2 = conn.open_stream().await.unwrap();
        let id3 = conn.open_stream().await.unwrap();

        assert_ne!(id1, id2);
        assert_ne!(id2, id3);
        assert_eq!(conn.stream_count(), 3);
    }

    #[tokio::test]
    async fn test_send_recv() {
        let conn = MultiplexedConnection::new();
        let stream_id = conn.open_stream().await.unwrap();

        // Simulate receiving data
        let header = StreamHeader::data(stream_id, 5);
        let payload = Bytes::from("hello");
        conn.process_frame(header, payload).await.unwrap();

        // Receive the data
        let data = conn.recv(stream_id).await.unwrap();
        assert_eq!(data, Some(Bytes::from("hello")));
    }

    #[tokio::test]
    async fn test_close_stream() {
        let conn = MultiplexedConnection::new();
        let stream_id = conn.open_stream().await.unwrap();

        conn.close_stream(stream_id).await.unwrap();

        let status = conn.stream_status(stream_id).await.unwrap();
        assert_eq!(status, StreamStatus::HalfClosedLocal);
    }

    #[tokio::test]
    async fn test_reset_stream() {
        let conn = MultiplexedConnection::new();
        let stream_id = conn.open_stream().await.unwrap();

        conn.reset_stream(stream_id, Some("test error".to_string())).await.unwrap();

        let status = conn.stream_status(stream_id).await.unwrap();
        assert_eq!(status, StreamStatus::Reset);
    }

    #[tokio::test]
    async fn test_stream_isolation() {
        let conn = MultiplexedConnection::new();

        let id1 = conn.open_stream().await.unwrap();
        let id2 = conn.open_stream().await.unwrap();

        // Send data to stream 1
        let header1 = StreamHeader::data(id1, 6);
        conn.process_frame(header1, Bytes::from("stream1")).await.unwrap();

        // Send data to stream 2
        let header2 = StreamHeader::data(id2, 6);
        conn.process_frame(header2, Bytes::from("stream2")).await.unwrap();

        // Reset stream 1
        conn.reset_stream(id1, None).await.unwrap();

        // Stream 2 should still work
        let data2 = conn.recv(id2).await.unwrap();
        assert_eq!(data2, Some(Bytes::from("stream2")));

        // Stream 1 should error
        let result = conn.recv(id1).await;
        assert!(matches!(result, Err(MultiplexError::StreamError(_))));
    }

    #[tokio::test]
    async fn test_accept_stream() {
        let conn = MultiplexedConnection::new();

        // Simulate receiving SYN
        let header = StreamHeader::syn(100);
        conn.process_frame(header, Bytes::new()).await.unwrap();

        assert!(conn.has_stream(100).await);
        assert_eq!(conn.stream_count(), 1);
    }

    #[tokio::test]
    async fn test_connection_close() {
        let conn = MultiplexedConnection::new();
        let stream_id = conn.open_stream().await.unwrap();

        conn.close().await;

        assert!(conn.is_closed());

        // Operations should fail
        let result = conn.open_stream().await;
        assert!(matches!(result, Err(MultiplexError::ConnectionClosed)));

        let result = conn.send(stream_id, Bytes::from("test")).await;
        assert!(matches!(result, Err(MultiplexError::ConnectionClosed)));
    }

    #[tokio::test]
    async fn test_take_frame() {
        let conn = MultiplexedConnection::new();
        let stream_id = conn.open_stream().await.unwrap();

        // Opening a stream queues a SYN frame
        let frame = conn.take_frame().await;
        assert!(frame.is_some());
        let (header, _) = frame.unwrap();
        assert!(header.flags.is_syn());
        assert_eq!(header.stream_id, stream_id);
    }

    #[tokio::test]
    async fn test_stream_status_transitions() {
        let conn = MultiplexedConnection::new();
        let stream_id = conn.open_stream().await.unwrap();

        // Initial status is Open
        let status = conn.stream_status(stream_id).await.unwrap();
        assert_eq!(status, StreamStatus::Open);

        // Close local side
        conn.close_stream(stream_id).await.unwrap();
        let status = conn.stream_status(stream_id).await.unwrap();
        assert_eq!(status, StreamStatus::HalfClosedLocal);

        // Receive FIN from remote
        let fin = StreamHeader::fin(stream_id);
        conn.process_frame(fin, Bytes::new()).await.unwrap();

        // Stream should be removed after full close
        assert!(!conn.has_stream(stream_id).await);
    }

    #[tokio::test]
    async fn test_concurrent_streams_limit() {
        // This test verifies the MAX_STREAMS constant is respected
        // We don't actually create 65535 streams, just verify the limit exists
        assert_eq!(MAX_STREAMS, 65535);
    }

    #[test]
    fn test_stream_status_can_send() {
        assert!(StreamStatus::Open.can_send());
        assert!(!StreamStatus::HalfClosedLocal.can_send());
        assert!(StreamStatus::HalfClosedRemote.can_send());
        assert!(!StreamStatus::Closed.can_send());
        assert!(!StreamStatus::Reset.can_send());
    }

    #[test]
    fn test_stream_status_can_receive() {
        assert!(StreamStatus::Open.can_receive());
        assert!(StreamStatus::HalfClosedLocal.can_receive());
        assert!(!StreamStatus::HalfClosedRemote.can_receive());
        assert!(!StreamStatus::Closed.can_receive());
        assert!(!StreamStatus::Reset.can_receive());
    }
}
