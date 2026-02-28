//! Binary frame encoding for the DX Agent protocol.
//!
//! Frame format:
//! ```text
//! [version: u8][type: u8][length: u32 BE][payload: [u8; length]][checksum: u32 BE]
//! ```

use bytes::{Buf, BufMut, Bytes, BytesMut};
use serde::{Deserialize, Serialize};

use crate::PROTOCOL_VERSION;

/// Frame types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum FrameType {
    /// JSON text message
    Text = 0x01,
    /// Binary data (DX Serializer format)
    Binary = 0x02,
    /// Ping heartbeat
    Ping = 0x03,
    /// Pong heartbeat
    Pong = 0x04,
    /// Close connection
    Close = 0x05,
    /// Authentication frame
    Auth = 0x06,
    /// Stream chunk (for LLM streaming responses)
    StreamChunk = 0x07,
    /// Stream end marker
    StreamEnd = 0x08,
}

impl TryFrom<u8> for FrameType {
    type Error = FrameError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0x01 => Ok(Self::Text),
            0x02 => Ok(Self::Binary),
            0x03 => Ok(Self::Ping),
            0x04 => Ok(Self::Pong),
            0x05 => Ok(Self::Close),
            0x06 => Ok(Self::Auth),
            0x07 => Ok(Self::StreamChunk),
            0x08 => Ok(Self::StreamEnd),
            _ => Err(FrameError::InvalidFrameType(value)),
        }
    }
}

/// A protocol frame
#[derive(Debug, Clone)]
pub struct Frame {
    pub frame_type: FrameType,
    pub payload: Bytes,
}

/// Frame encoding/decoding errors
#[derive(Debug, thiserror::Error)]
pub enum FrameError {
    #[error("invalid frame type: 0x{0:02x}")]
    InvalidFrameType(u8),
    #[error("frame too large: {0} bytes (max: {1})")]
    FrameTooLarge(usize, usize),
    #[error("incomplete frame: need {0} more bytes")]
    Incomplete(usize),
    #[error("checksum mismatch: expected 0x{expected:08x}, got 0x{actual:08x}")]
    ChecksumMismatch { expected: u32, actual: u32 },
    #[error("protocol version mismatch: expected {expected}, got {actual}")]
    VersionMismatch { expected: u32, actual: u32 },
}

/// Header size: version(1) + type(1) + length(4) = 6 bytes
const HEADER_SIZE: usize = 6;
/// Checksum size: 4 bytes
const CHECKSUM_SIZE: usize = 4;
/// Maximum payload size: 1MB
const MAX_PAYLOAD_SIZE: usize = 1_048_576;

impl Frame {
    /// Create a new text frame
    pub fn text(data: impl Into<Bytes>) -> Self {
        Self {
            frame_type: FrameType::Text,
            payload: data.into(),
        }
    }

    /// Create a new binary frame
    pub fn binary(data: impl Into<Bytes>) -> Self {
        Self {
            frame_type: FrameType::Binary,
            payload: data.into(),
        }
    }

    /// Create a ping frame
    pub fn ping() -> Self {
        let ts = chrono::Utc::now().timestamp().to_be_bytes();
        Self {
            frame_type: FrameType::Ping,
            payload: Bytes::copy_from_slice(&ts),
        }
    }

    /// Create a pong frame
    pub fn pong(ping_payload: &[u8]) -> Self {
        Self {
            frame_type: FrameType::Pong,
            payload: Bytes::copy_from_slice(ping_payload),
        }
    }

    /// Create an auth frame
    pub fn auth(data: impl Into<Bytes>) -> Self {
        Self {
            frame_type: FrameType::Auth,
            payload: data.into(),
        }
    }

    /// Create a stream chunk frame
    pub fn stream_chunk(data: impl Into<Bytes>) -> Self {
        Self {
            frame_type: FrameType::StreamChunk,
            payload: data.into(),
        }
    }

    /// Encode frame to bytes
    pub fn encode(&self) -> BytesMut {
        let payload_len = self.payload.len();
        let total_len = HEADER_SIZE + payload_len + CHECKSUM_SIZE;
        let mut buf = BytesMut::with_capacity(total_len);

        // Header
        buf.put_u8(PROTOCOL_VERSION as u8);
        buf.put_u8(self.frame_type as u8);
        buf.put_u32(payload_len as u32);

        // Payload
        buf.put_slice(&self.payload);

        // Checksum (simple CRC32-like using wrapping adds)
        let checksum = compute_checksum(&self.payload);
        buf.put_u32(checksum);

        buf
    }

    /// Decode frame from bytes. Returns (frame, bytes_consumed) or error.
    pub fn decode(data: &[u8]) -> Result<(Self, usize), FrameError> {
        if data.len() < HEADER_SIZE {
            return Err(FrameError::Incomplete(HEADER_SIZE - data.len()));
        }

        let mut cursor = &data[..];

        // Read header
        let version = cursor.get_u8();
        if version as u32 != PROTOCOL_VERSION {
            return Err(FrameError::VersionMismatch {
                expected: PROTOCOL_VERSION,
                actual: version as u32,
            });
        }

        let frame_type_byte = cursor.get_u8();
        let frame_type = FrameType::try_from(frame_type_byte)?;

        let payload_len = cursor.get_u32() as usize;
        if payload_len > MAX_PAYLOAD_SIZE {
            return Err(FrameError::FrameTooLarge(payload_len, MAX_PAYLOAD_SIZE));
        }

        let total_len = HEADER_SIZE + payload_len + CHECKSUM_SIZE;
        if data.len() < total_len {
            return Err(FrameError::Incomplete(total_len - data.len()));
        }

        // Read payload
        let payload = Bytes::copy_from_slice(&cursor[..payload_len]);
        cursor.advance(payload_len);

        // Verify checksum
        let expected_checksum = cursor.get_u32();
        let actual_checksum = compute_checksum(&payload);
        if expected_checksum != actual_checksum {
            return Err(FrameError::ChecksumMismatch {
                expected: expected_checksum,
                actual: actual_checksum,
            });
        }

        Ok((
            Frame {
                frame_type,
                payload,
            },
            total_len,
        ))
    }
}

/// Simple checksum computation using wrapping addition
fn compute_checksum(data: &[u8]) -> u32 {
    let mut hash: u32 = 0x811c_9dc5; // FNV offset basis
    for &byte in data {
        hash = hash.wrapping_mul(0x0100_0193); // FNV prime
        hash ^= byte as u32;
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frame_roundtrip_text() {
        let frame = Frame::text(Bytes::from("hello world"));
        let encoded = frame.encode();
        let (decoded, consumed) = Frame::decode(&encoded).expect("decode");
        assert_eq!(consumed, encoded.len());
        assert_eq!(decoded.frame_type, FrameType::Text);
        assert_eq!(decoded.payload, Bytes::from("hello world"));
    }

    #[test]
    fn test_frame_roundtrip_binary() {
        let data = vec![0u8, 1, 2, 3, 255, 254, 253];
        let frame = Frame::binary(Bytes::from(data.clone()));
        let encoded = frame.encode();
        let (decoded, _) = Frame::decode(&encoded).expect("decode");
        assert_eq!(decoded.frame_type, FrameType::Binary);
        assert_eq!(decoded.payload.as_ref(), &data[..]);
    }

    #[test]
    fn test_frame_ping_pong() {
        let ping = Frame::ping();
        let encoded_ping = ping.encode();
        let (decoded_ping, _) = Frame::decode(&encoded_ping).expect("decode ping");
        assert_eq!(decoded_ping.frame_type, FrameType::Ping);

        let pong = Frame::pong(&decoded_ping.payload);
        let encoded_pong = pong.encode();
        let (decoded_pong, _) = Frame::decode(&encoded_pong).expect("decode pong");
        assert_eq!(decoded_pong.frame_type, FrameType::Pong);
        assert_eq!(decoded_pong.payload, decoded_ping.payload);
    }

    #[test]
    fn test_frame_incomplete() {
        let frame = Frame::text(Bytes::from("test"));
        let encoded = frame.encode();
        let result = Frame::decode(&encoded[..3]);
        assert!(matches!(result, Err(FrameError::Incomplete(_))));
    }

    #[test]
    fn test_frame_checksum_corruption() {
        let frame = Frame::text(Bytes::from("test"));
        let mut encoded = frame.encode();
        // Corrupt the last byte (part of checksum)
        let len = encoded.len();
        encoded[len - 1] ^= 0xFF;
        let result = Frame::decode(&encoded);
        assert!(matches!(result, Err(FrameError::ChecksumMismatch { .. })));
    }

    #[test]
    fn test_frame_type_conversion() {
        assert_eq!(FrameType::try_from(0x01).expect("ok"), FrameType::Text);
        assert_eq!(FrameType::try_from(0x07).expect("ok"), FrameType::StreamChunk);
        assert!(FrameType::try_from(0xFF).is_err());
    }
}
