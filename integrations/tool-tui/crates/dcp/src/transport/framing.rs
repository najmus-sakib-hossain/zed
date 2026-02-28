//! Message framing for DCP protocol.
//!
//! Provides length-prefixed framing for reliable message boundaries over TCP streams.

use bytes::{Buf, BufMut, Bytes, BytesMut};
use thiserror::Error;

/// Protocol version byte
pub const PROTOCOL_VERSION: u8 = 1;

/// Maximum message size (16MB)
pub const MAX_MESSAGE_SIZE: u32 = 16 * 1024 * 1024;

/// Frame header size (1 byte version + 4 bytes length)
pub const FRAME_HEADER_SIZE: usize = 5;

/// Frame header - 5 bytes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameHeader {
    /// Protocol version (1 byte)
    pub version: u8,
    /// Payload length in bytes (4 bytes, big-endian)
    pub length: u32,
}

impl FrameHeader {
    /// Create a new frame header
    pub fn new(length: u32) -> Self {
        Self {
            version: PROTOCOL_VERSION,
            length,
        }
    }

    /// Encode header to bytes
    pub fn encode(&self, dst: &mut BytesMut) {
        dst.put_u8(self.version);
        dst.put_u32(self.length);
    }

    /// Decode header from bytes
    pub fn decode(src: &[u8]) -> Option<Self> {
        if src.len() < FRAME_HEADER_SIZE {
            return None;
        }
        Some(Self {
            version: src[0],
            length: u32::from_be_bytes([src[1], src[2], src[3], src[4]]),
        })
    }

    /// Total frame size (header + payload)
    pub fn frame_size(&self) -> usize {
        FRAME_HEADER_SIZE + self.length as usize
    }
}

/// Frame errors
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum FrameError {
    #[error("message too large: {0} bytes (max: {1})")]
    MessageTooLarge(u32, u32),
    #[error("invalid protocol version: {0}")]
    InvalidVersion(u8),
    #[error("incomplete frame")]
    Incomplete,
    #[error("empty message")]
    EmptyMessage,
}

/// Frame codec for encoding/decoding messages
#[derive(Debug)]
pub struct FrameCodec {
    /// Maximum allowed message size
    max_size: u32,
    /// Read buffer for partial frames
    read_buffer: BytesMut,
}

impl Default for FrameCodec {
    fn default() -> Self {
        Self::new()
    }
}

impl FrameCodec {
    /// Create a new frame codec with default max size
    pub fn new() -> Self {
        Self {
            max_size: MAX_MESSAGE_SIZE,
            read_buffer: BytesMut::with_capacity(8192),
        }
    }

    /// Create a frame codec with custom max size
    pub fn with_max_size(max_size: u32) -> Self {
        Self {
            max_size,
            read_buffer: BytesMut::with_capacity(8192),
        }
    }

    /// Get the maximum message size
    pub fn max_size(&self) -> u32 {
        self.max_size
    }

    /// Feed data into the codec's internal buffer
    pub fn feed(&mut self, data: &[u8]) {
        self.read_buffer.extend_from_slice(data);
    }

    /// Check if there's enough data for a complete frame
    pub fn has_complete_frame(&self) -> bool {
        if self.read_buffer.len() < FRAME_HEADER_SIZE {
            return false;
        }
        if let Some(header) = FrameHeader::decode(&self.read_buffer) {
            self.read_buffer.len() >= header.frame_size()
        } else {
            false
        }
    }

    /// Get the current buffer size
    pub fn buffer_size(&self) -> usize {
        self.read_buffer.len()
    }

    /// Clear the internal buffer
    pub fn clear(&mut self) {
        self.read_buffer.clear();
    }

    /// Decode a frame from the internal buffer
    /// Returns None if incomplete, Some(bytes) if complete
    pub fn decode(&mut self) -> Result<Option<Bytes>, FrameError> {
        self.decode_from(&mut self.read_buffer.clone()).map(|opt| {
            if let Some(ref bytes) = opt {
                // Advance the actual buffer
                let consumed = FRAME_HEADER_SIZE + bytes.len();
                self.read_buffer.advance(consumed);
            }
            opt
        })
    }

    /// Decode a frame from the provided buffer
    /// Returns None if incomplete, Some(bytes) if complete
    pub fn decode_from(&self, src: &mut BytesMut) -> Result<Option<Bytes>, FrameError> {
        // Need at least header
        if src.len() < FRAME_HEADER_SIZE {
            return Ok(None);
        }

        // Parse header
        let header = FrameHeader::decode(src).ok_or(FrameError::Incomplete)?;

        // Validate version
        if header.version != PROTOCOL_VERSION {
            return Err(FrameError::InvalidVersion(header.version));
        }

        // Validate size before allocating
        if header.length > self.max_size {
            return Err(FrameError::MessageTooLarge(header.length, self.max_size));
        }

        // Check if we have complete frame
        let frame_size = header.frame_size();
        if src.len() < frame_size {
            return Ok(None);
        }

        // Extract payload
        src.advance(FRAME_HEADER_SIZE);
        let payload = src.split_to(header.length as usize).freeze();

        Ok(Some(payload))
    }

    /// Encode a message into a frame
    pub fn encode(&self, msg: &[u8], dst: &mut BytesMut) -> Result<(), FrameError> {
        let len = msg.len() as u32;

        // Validate size
        if len > self.max_size {
            return Err(FrameError::MessageTooLarge(len, self.max_size));
        }

        // Reserve space
        dst.reserve(FRAME_HEADER_SIZE + msg.len());

        // Write header
        let header = FrameHeader::new(len);
        header.encode(dst);

        // Write payload
        dst.extend_from_slice(msg);

        Ok(())
    }

    /// Encode a message and return the framed bytes
    pub fn encode_to_bytes(&self, msg: &[u8]) -> Result<Bytes, FrameError> {
        let mut dst = BytesMut::with_capacity(FRAME_HEADER_SIZE + msg.len());
        self.encode(msg, &mut dst)?;
        Ok(dst.freeze())
    }
}

/// Convenience function to frame a message
pub fn frame_message(msg: &[u8]) -> Result<Bytes, FrameError> {
    FrameCodec::new().encode_to_bytes(msg)
}

/// Convenience function to unframe a message
pub fn unframe_message(data: &[u8]) -> Result<Option<Bytes>, FrameError> {
    let mut src = BytesMut::from(data);
    FrameCodec::new().decode_from(&mut src)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frame_header_encode_decode() {
        let header = FrameHeader::new(1234);
        let mut buf = BytesMut::new();
        header.encode(&mut buf);

        assert_eq!(buf.len(), FRAME_HEADER_SIZE);

        let decoded = FrameHeader::decode(&buf).unwrap();
        assert_eq!(decoded.version, PROTOCOL_VERSION);
        assert_eq!(decoded.length, 1234);
    }

    #[test]
    fn test_frame_header_frame_size() {
        let header = FrameHeader::new(100);
        assert_eq!(header.frame_size(), FRAME_HEADER_SIZE + 100);
    }

    #[test]
    fn test_frame_codec_encode_decode() {
        let codec = FrameCodec::new();
        let message = b"Hello, DCP!";

        // Encode
        let mut encoded = BytesMut::new();
        codec.encode(message, &mut encoded).unwrap();

        // Decode
        let decoded = codec.decode_from(&mut encoded).unwrap().unwrap();
        assert_eq!(&decoded[..], message);
    }

    #[test]
    fn test_frame_codec_round_trip() {
        let codec = FrameCodec::new();
        let messages = vec![
            b"".to_vec(),
            b"short".to_vec(),
            b"a longer message with more content".to_vec(),
            vec![0u8; 1000],   // Binary data
            vec![0xFF; 10000], // Larger binary
        ];

        for msg in messages {
            let framed = codec.encode_to_bytes(&msg).unwrap();
            let mut src = BytesMut::from(&framed[..]);
            let decoded = codec.decode_from(&mut src).unwrap().unwrap();
            assert_eq!(&decoded[..], &msg[..]);
        }
    }

    #[test]
    fn test_frame_codec_partial_message() {
        let codec = FrameCodec::new();
        let message = b"Complete message";

        // Encode full message
        let framed = codec.encode_to_bytes(message).unwrap();

        // Try to decode with only header
        let mut partial = BytesMut::from(&framed[..FRAME_HEADER_SIZE]);
        let result = codec.decode_from(&mut partial).unwrap();
        assert!(result.is_none());

        // Try with partial payload
        let mut partial = BytesMut::from(&framed[..FRAME_HEADER_SIZE + 5]);
        let result = codec.decode_from(&mut partial).unwrap();
        assert!(result.is_none());

        // Full message should decode
        let mut full = BytesMut::from(&framed[..]);
        let result = codec.decode_from(&mut full).unwrap();
        assert!(result.is_some());
        assert_eq!(&result.unwrap()[..], message);
    }

    #[test]
    fn test_frame_codec_message_too_large() {
        let codec = FrameCodec::with_max_size(100);
        let message = vec![0u8; 200];

        let result = codec.encode_to_bytes(&message);
        assert!(matches!(result, Err(FrameError::MessageTooLarge(200, 100))));
    }

    #[test]
    fn test_frame_codec_invalid_version() {
        let mut data = BytesMut::new();
        data.put_u8(99); // Invalid version
        data.put_u32(5);
        data.extend_from_slice(b"hello");

        let codec = FrameCodec::new();
        let result = codec.decode_from(&mut data);
        assert!(matches!(result, Err(FrameError::InvalidVersion(99))));
    }

    #[test]
    fn test_frame_codec_feed_and_decode() {
        let mut codec = FrameCodec::new();
        let message = b"Test message";

        // Encode
        let framed = codec.encode_to_bytes(message).unwrap();

        // Feed in chunks
        codec.feed(&framed[..3]); // Partial header
        assert!(!codec.has_complete_frame());

        codec.feed(&framed[3..FRAME_HEADER_SIZE]); // Rest of header
        assert!(!codec.has_complete_frame());

        codec.feed(&framed[FRAME_HEADER_SIZE..]); // Payload
        assert!(codec.has_complete_frame());

        // Decode
        let decoded = codec.decode().unwrap().unwrap();
        assert_eq!(&decoded[..], message);
        assert_eq!(codec.buffer_size(), 0);
    }

    #[test]
    fn test_frame_codec_multiple_messages() {
        let mut codec = FrameCodec::new();
        let msg1 = b"First";
        let msg2 = b"Second";

        // Encode both
        let framed1 = codec.encode_to_bytes(msg1).unwrap();
        let framed2 = codec.encode_to_bytes(msg2).unwrap();

        // Feed both at once
        codec.feed(&framed1);
        codec.feed(&framed2);

        // Decode first
        let decoded1 = codec.decode().unwrap().unwrap();
        assert_eq!(&decoded1[..], msg1);

        // Decode second
        let decoded2 = codec.decode().unwrap().unwrap();
        assert_eq!(&decoded2[..], msg2);

        // No more
        assert!(!codec.has_complete_frame());
    }

    #[test]
    fn test_convenience_functions() {
        let message = b"Quick test";

        let framed = frame_message(message).unwrap();
        let unframed = unframe_message(&framed).unwrap().unwrap();

        assert_eq!(&unframed[..], message);
    }

    #[test]
    fn test_frame_header_decode_insufficient_data() {
        let data = [0u8; 3]; // Less than header size
        assert!(FrameHeader::decode(&data).is_none());
    }

    #[test]
    fn test_max_message_size_validation() {
        // Create header claiming huge size
        let mut data = BytesMut::new();
        data.put_u8(PROTOCOL_VERSION);
        data.put_u32(MAX_MESSAGE_SIZE + 1);

        let codec = FrameCodec::new();
        let result = codec.decode_from(&mut data);
        assert!(matches!(result, Err(FrameError::MessageTooLarge(_, _))));
    }
}
