//! DCP Stream with Blake3 checksum for integrity verification.
//!
//! Provides streaming support with chunk sequencing, integrity verification,
//! and retransmission request handling.

use blake3::Hasher;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Mutex;

use crate::binary::{ChunkFlags, StreamChunk};
use crate::stream::ring_buffer::StreamRingBuffer;
use crate::DCPError;

/// Retransmission request for lost chunks
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RetransmitRequest {
    /// Starting sequence number
    pub start_seq: u32,
    /// Number of chunks to retransmit
    pub count: u32,
}

/// DCP Stream with integrity verification
pub struct DcpStream {
    /// Underlying ring buffer
    buffer: StreamRingBuffer,
    /// Running Blake3 hasher for integrity
    hasher: Mutex<Hasher>,
    /// Next expected sequence number (consumer side)
    next_seq: AtomicU32,
    /// Next sequence to send (producer side)
    send_seq: AtomicU32,
    /// Stream ID
    stream_id: u32,
    /// Whether the stream is complete
    complete: std::sync::atomic::AtomicBool,
}

impl DcpStream {
    /// Create a new DCP stream with the given buffer capacity
    pub fn new(stream_id: u32, capacity: usize) -> Self {
        Self {
            buffer: StreamRingBuffer::new(capacity),
            hasher: Mutex::new(Hasher::new()),
            next_seq: AtomicU32::new(0),
            send_seq: AtomicU32::new(0),
            stream_id,
            complete: std::sync::atomic::AtomicBool::new(false),
        }
    }

    /// Get the stream ID
    pub fn stream_id(&self) -> u32 {
        self.stream_id
    }

    /// Get the underlying buffer
    pub fn buffer(&self) -> &StreamRingBuffer {
        &self.buffer
    }

    /// Check if the stream is complete
    pub fn is_complete(&self) -> bool {
        self.complete.load(Ordering::Acquire)
    }

    /// Write a chunk to the stream (producer side)
    /// Returns the chunk header that should be sent
    pub fn write_chunk(&self, data: &[u8], is_last: bool) -> Result<StreamChunk, DCPError> {
        if data.len() > u16::MAX as usize {
            return Err(DCPError::OutOfBounds);
        }

        let seq = self.send_seq.fetch_add(1, Ordering::AcqRel);
        let is_first = seq == 0;

        let flags = if is_first && is_last {
            ChunkFlags::FIRST | ChunkFlags::LAST
        } else if is_first {
            ChunkFlags::FIRST
        } else if is_last {
            ChunkFlags::LAST
        } else {
            ChunkFlags::CONTINUE
        };

        // Write chunk header + data to buffer
        let chunk = StreamChunk::new(seq, flags, data.len() as u16);

        // Write header
        self.buffer.push(chunk.as_bytes())?;

        // Write payload
        if !data.is_empty() {
            self.buffer.push(data)?;
        }

        // Update hasher
        {
            let mut hasher = self.hasher.lock().unwrap();
            hasher.update(data);
        }

        if is_last {
            self.complete.store(true, Ordering::Release);
        }

        Ok(chunk)
    }

    /// Read a chunk from the stream (consumer side)
    /// Returns the chunk header and data, or None if no data available
    pub fn read_chunk(&self) -> Result<Option<(StreamChunk, Vec<u8>)>, DCPError> {
        // First peek at the header
        let mut header_buf = [0u8; StreamChunk::SIZE];
        let peeked = self.buffer.peek(&mut header_buf);

        if peeked < StreamChunk::SIZE {
            return Ok(None);
        }

        let chunk = StreamChunk::from_bytes(&header_buf)?;
        let chunk_len = chunk.len as usize;
        let total_len = StreamChunk::SIZE + chunk_len;

        // Check if we have the full chunk
        if self.buffer.len() < total_len {
            return Ok(None);
        }

        // Verify sequence number
        let expected_seq = self.next_seq.load(Ordering::Acquire);
        if chunk.sequence != expected_seq {
            // Out of order - request retransmission
            return Err(DCPError::ChecksumMismatch);
        }

        // Read the full chunk
        let mut full_buf = vec![0u8; total_len];
        self.buffer.pop(&mut full_buf);

        // Extract payload
        let payload = full_buf[StreamChunk::SIZE..].to_vec();

        // Update sequence
        self.next_seq.store(expected_seq + 1, Ordering::Release);

        // Copy chunk values before returning
        let result_chunk = StreamChunk::new(chunk.sequence, chunk.flags, chunk.len);

        Ok(Some((result_chunk, payload)))
    }

    /// Get the current checksum of all data written so far
    pub fn checksum(&self) -> [u8; 32] {
        let hasher = self.hasher.lock().unwrap();
        *hasher.finalize().as_bytes()
    }

    /// Verify the stream checksum against an expected value
    pub fn verify_checksum(&self, expected: &[u8; 32]) -> bool {
        &self.checksum() == expected
    }

    /// Create a retransmission request for missing chunks
    pub fn request_retransmit(&self, missing_seq: u32) -> RetransmitRequest {
        let expected = self.next_seq.load(Ordering::Acquire);
        RetransmitRequest {
            start_seq: expected,
            count: missing_seq.saturating_sub(expected) + 1,
        }
    }

    /// Get the next expected sequence number
    pub fn next_expected_seq(&self) -> u32 {
        self.next_seq.load(Ordering::Acquire)
    }

    /// Get the next sequence number to send
    pub fn next_send_seq(&self) -> u32 {
        self.send_seq.load(Ordering::Acquire)
    }

    /// Check if backpressure is active
    pub fn is_backpressure(&self) -> bool {
        self.buffer.backpressure().is_full()
    }

    /// Get available buffer space
    pub fn available_space(&self) -> usize {
        self.buffer.available_space()
    }

    /// Reset the stream for reuse
    pub fn reset(&self) {
        self.buffer.clear();
        self.next_seq.store(0, Ordering::Release);
        self.send_seq.store(0, Ordering::Release);
        self.complete.store(false, Ordering::Release);
        *self.hasher.lock().unwrap() = Hasher::new();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stream_basic() {
        let stream = DcpStream::new(1, 1024);

        // Write first chunk
        let chunk1 = stream.write_chunk(b"hello", false).unwrap();
        assert!(chunk1.is_first());
        assert!(!chunk1.is_last());
        let seq1 = chunk1.sequence;
        assert_eq!(seq1, 0);

        // Write last chunk
        let chunk2 = stream.write_chunk(b"world", true).unwrap();
        assert!(!chunk2.is_first());
        assert!(chunk2.is_last());
        let seq2 = chunk2.sequence;
        assert_eq!(seq2, 1);

        assert!(stream.is_complete());
    }

    #[test]
    fn test_stream_read_write() {
        let stream = DcpStream::new(1, 1024);

        stream.write_chunk(b"test", false).unwrap();
        stream.write_chunk(b"data", true).unwrap();

        // Read first chunk
        let (chunk1, data1) = stream.read_chunk().unwrap().unwrap();
        let seq1 = chunk1.sequence;
        assert_eq!(seq1, 0);
        assert_eq!(data1, b"test");

        // Read second chunk
        let (chunk2, data2) = stream.read_chunk().unwrap().unwrap();
        let seq2 = chunk2.sequence;
        assert_eq!(seq2, 1);
        assert_eq!(data2, b"data");

        // No more data
        assert!(stream.read_chunk().unwrap().is_none());
    }

    #[test]
    fn test_stream_checksum() {
        let stream = DcpStream::new(1, 1024);

        stream.write_chunk(b"hello", false).unwrap();
        let checksum1 = stream.checksum();

        stream.write_chunk(b"world", true).unwrap();
        let checksum2 = stream.checksum();

        // Checksums should be different
        assert_ne!(checksum1, checksum2);

        // Verify checksum
        assert!(stream.verify_checksum(&checksum2));
        assert!(!stream.verify_checksum(&checksum1));
    }

    #[test]
    fn test_stream_single_chunk() {
        let stream = DcpStream::new(1, 1024);

        // Single chunk that is both first and last
        let chunk = stream.write_chunk(b"single", true).unwrap();
        assert!(chunk.is_first());
        assert!(chunk.is_last());
        let seq = chunk.sequence;
        assert_eq!(seq, 0);
    }

    #[test]
    fn test_retransmit_request() {
        let stream = DcpStream::new(1, 1024);

        let req = stream.request_retransmit(5);
        assert_eq!(req.start_seq, 0);
        assert_eq!(req.count, 6);
    }

    #[test]
    fn test_stream_reset() {
        let stream = DcpStream::new(1, 1024);

        stream.write_chunk(b"data", true).unwrap();
        assert!(stream.is_complete());
        assert_eq!(stream.next_send_seq(), 1);

        stream.reset();
        assert!(!stream.is_complete());
        assert_eq!(stream.next_send_seq(), 0);
        assert_eq!(stream.next_expected_seq(), 0);
    }
}
