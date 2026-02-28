//! Stream header for multiplexing.
//!
//! Defines the header format for multiplexed streams.

use bytes::{Buf, BufMut, Bytes, BytesMut};

/// Stream header size in bytes
pub const STREAM_HEADER_SIZE: usize = 8;

/// Stream flags for control
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct StreamFlags(u8);

impl StreamFlags {
    /// No flags set
    pub const NONE: StreamFlags = StreamFlags(0);
    /// FIN - stream is closing
    pub const FIN: StreamFlags = StreamFlags(1);
    /// RST - stream reset/error
    pub const RST: StreamFlags = StreamFlags(2);
    /// ACK - acknowledgment
    pub const ACK: StreamFlags = StreamFlags(4);
    /// SYN - stream open request
    pub const SYN: StreamFlags = StreamFlags(8);

    /// Create flags from raw byte
    pub fn from_byte(b: u8) -> Self {
        StreamFlags(b)
    }

    /// Get raw byte value
    pub fn as_byte(self) -> u8 {
        self.0
    }

    /// Check if FIN flag is set
    pub fn is_fin(self) -> bool {
        self.0 & Self::FIN.0 != 0
    }

    /// Check if RST flag is set
    pub fn is_rst(self) -> bool {
        self.0 & Self::RST.0 != 0
    }

    /// Check if ACK flag is set
    pub fn is_ack(self) -> bool {
        self.0 & Self::ACK.0 != 0
    }

    /// Check if SYN flag is set
    pub fn is_syn(self) -> bool {
        self.0 & Self::SYN.0 != 0
    }

    /// Combine flags
    pub fn with(self, other: StreamFlags) -> StreamFlags {
        StreamFlags(self.0 | other.0)
    }
}

impl std::ops::BitOr for StreamFlags {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        StreamFlags(self.0 | rhs.0)
    }
}

/// Stream header for multiplexing
///
/// Format (8 bytes):
/// - stream_id: u16 (0 = control, 1-65535 = data streams)
/// - flags: u8 (FIN=1, RST=2, ACK=4, SYN=8)
/// - reserved: u8
/// - length: u32 (payload length, big-endian)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StreamHeader {
    /// Stream ID (0 = control, 1-65535 = data streams)
    pub stream_id: u16,
    /// Flags: FIN=1, RST=2, ACK=4, SYN=8
    pub flags: StreamFlags,
    /// Reserved for future use
    pub reserved: u8,
    /// Payload length
    pub length: u32,
}

impl StreamHeader {
    /// Control stream ID
    pub const CONTROL_STREAM: u16 = 0;

    /// Maximum stream ID
    pub const MAX_STREAM_ID: u16 = 65535;

    /// Create a new stream header
    pub fn new(stream_id: u16, flags: StreamFlags, length: u32) -> Self {
        Self {
            stream_id,
            flags,
            reserved: 0,
            length,
        }
    }

    /// Create a data header
    pub fn data(stream_id: u16, length: u32) -> Self {
        Self::new(stream_id, StreamFlags::NONE, length)
    }

    /// Create a SYN header (open stream)
    pub fn syn(stream_id: u16) -> Self {
        Self::new(stream_id, StreamFlags::SYN, 0)
    }

    /// Create a FIN header (close stream)
    pub fn fin(stream_id: u16) -> Self {
        Self::new(stream_id, StreamFlags::FIN, 0)
    }

    /// Create a RST header (reset stream)
    pub fn rst(stream_id: u16) -> Self {
        Self::new(stream_id, StreamFlags::RST, 0)
    }

    /// Create an ACK header
    pub fn ack(stream_id: u16) -> Self {
        Self::new(stream_id, StreamFlags::ACK, 0)
    }

    /// Check if this is a control stream message
    pub fn is_control(&self) -> bool {
        self.stream_id == Self::CONTROL_STREAM
    }

    /// Encode header to bytes
    pub fn encode(&self, dst: &mut BytesMut) {
        dst.put_u16(self.stream_id);
        dst.put_u8(self.flags.as_byte());
        dst.put_u8(self.reserved);
        dst.put_u32(self.length);
    }

    /// Encode header to a new BytesMut
    pub fn to_bytes(&self) -> Bytes {
        let mut buf = BytesMut::with_capacity(STREAM_HEADER_SIZE);
        self.encode(&mut buf);
        buf.freeze()
    }

    /// Decode header from bytes
    pub fn decode(src: &[u8]) -> Option<Self> {
        if src.len() < STREAM_HEADER_SIZE {
            return None;
        }

        Some(Self {
            stream_id: u16::from_be_bytes([src[0], src[1]]),
            flags: StreamFlags::from_byte(src[2]),
            reserved: src[3],
            length: u32::from_be_bytes([src[4], src[5], src[6], src[7]]),
        })
    }

    /// Decode header from BytesMut, advancing the buffer
    pub fn decode_from(src: &mut BytesMut) -> Option<Self> {
        if src.len() < STREAM_HEADER_SIZE {
            return None;
        }

        let header = Self::decode(src)?;
        src.advance(STREAM_HEADER_SIZE);
        Some(header)
    }

    /// Total frame size (header + payload)
    pub fn frame_size(&self) -> usize {
        STREAM_HEADER_SIZE + self.length as usize
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stream_flags() {
        let flags = StreamFlags::FIN | StreamFlags::ACK;
        assert!(flags.is_fin());
        assert!(flags.is_ack());
        assert!(!flags.is_rst());
        assert!(!flags.is_syn());
    }

    #[test]
    fn test_stream_header_encode_decode() {
        let header = StreamHeader::new(42, StreamFlags::FIN, 1234);
        let mut buf = BytesMut::new();
        header.encode(&mut buf);

        assert_eq!(buf.len(), STREAM_HEADER_SIZE);

        let decoded = StreamHeader::decode(&buf).unwrap();
        assert_eq!(decoded.stream_id, 42);
        assert!(decoded.flags.is_fin());
        assert_eq!(decoded.length, 1234);
    }

    #[test]
    fn test_stream_header_helpers() {
        let syn = StreamHeader::syn(1);
        assert!(syn.flags.is_syn());
        assert_eq!(syn.stream_id, 1);

        let fin = StreamHeader::fin(2);
        assert!(fin.flags.is_fin());
        assert_eq!(fin.stream_id, 2);

        let rst = StreamHeader::rst(3);
        assert!(rst.flags.is_rst());
        assert_eq!(rst.stream_id, 3);
    }

    #[test]
    fn test_control_stream() {
        let header = StreamHeader::data(StreamHeader::CONTROL_STREAM, 100);
        assert!(header.is_control());

        let header = StreamHeader::data(1, 100);
        assert!(!header.is_control());
    }

    #[test]
    fn test_frame_size() {
        let header = StreamHeader::data(1, 100);
        assert_eq!(header.frame_size(), STREAM_HEADER_SIZE + 100);
    }
}
