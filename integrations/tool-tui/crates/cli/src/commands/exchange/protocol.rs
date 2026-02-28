//! Exchange Protocol Definition
//!
//! Wire protocol for daemon communication

/// Protocol version
pub const PROTOCOL_VERSION: u8 = 1;

/// Magic bytes for message framing
pub const MAGIC: [u8; 4] = [0x44, 0x58, 0x53, 0x52]; // "DXSR"

/// Maximum message size (16MB)
pub const MAX_MESSAGE_SIZE: usize = 16 * 1024 * 1024;

/// Wire frame format:
/// ```text
/// +--------+--------+--------+--------+
/// | Magic (4 bytes) | "DXSR"          |
/// +--------+--------+--------+--------+
/// | Version| Flags  | Reserved        |
/// +--------+--------+--------+--------+
/// | Message Length (4 bytes, BE)      |
/// +--------+--------+--------+--------+
/// | Message Type (2 bytes, BE)        |
/// +--------+--------+--------+--------+
/// | Header Length (2 bytes, BE)       |
/// +--------+--------+--------+--------+
/// | Header (variable)                 |
/// +--------+--------+--------+--------+
/// | Payload (variable, .sr format)    |
/// +--------+--------+--------+--------+
/// | CRC32 (4 bytes)                   |
/// +--------+--------+--------+--------+
/// ```

/// Frame header
#[derive(Debug, Clone)]
pub struct FrameHeader {
    /// Protocol version
    pub version: u8,

    /// Flags
    pub flags: FrameFlags,

    /// Total message length
    pub message_length: u32,

    /// Message type code
    pub message_type: u16,

    /// Header length
    pub header_length: u16,
}

/// Frame flags
#[derive(Debug, Clone, Copy, Default)]
pub struct FrameFlags {
    /// Message is compressed
    pub compressed: bool,

    /// Message is encrypted
    pub encrypted: bool,

    /// Message requires acknowledgment
    pub requires_ack: bool,

    /// Message is a response
    pub is_response: bool,

    /// Message is part of a batch
    pub is_batch: bool,
}

impl FrameFlags {
    /// Convert to byte
    pub fn to_byte(&self) -> u8 {
        let mut flags = 0u8;
        if self.compressed {
            flags |= 0x01;
        }
        if self.encrypted {
            flags |= 0x02;
        }
        if self.requires_ack {
            flags |= 0x04;
        }
        if self.is_response {
            flags |= 0x08;
        }
        if self.is_batch {
            flags |= 0x10;
        }
        flags
    }

    /// Parse from byte
    pub fn from_byte(byte: u8) -> Self {
        Self {
            compressed: byte & 0x01 != 0,
            encrypted: byte & 0x02 != 0,
            requires_ack: byte & 0x04 != 0,
            is_response: byte & 0x08 != 0,
            is_batch: byte & 0x10 != 0,
        }
    }
}

/// Message type codes
#[repr(u16)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageTypeCode {
    // Check operations (0x01xx)
    CheckRequest = 0x0100,
    CheckResult = 0x0101,

    // Score operations (0x02xx)
    ScoreRequest = 0x0200,
    ScoreResult = 0x0201,

    // Sync operations (0x03xx)
    SyncRequest = 0x0300,
    SyncProgress = 0x0301,
    SyncComplete = 0x0302,

    // Branch operations (0x04xx)
    BranchConfig = 0x0400,
    BranchUpdate = 0x0401,

    // AI operations (0x05xx)
    AiUpdateRequest = 0x0500,
    AiUpdateApproval = 0x0501,
    AiUpdateResult = 0x0502,

    // Health operations (0x06xx)
    Heartbeat = 0x0600,
    StatusRequest = 0x0601,
    StatusResponse = 0x0602,

    // Error handling (0xFFxx)
    Error = 0xFF00,
    Ack = 0xFF01,
}

impl MessageTypeCode {
    /// Parse from u16
    pub fn from_u16(code: u16) -> Option<Self> {
        match code {
            0x0100 => Some(Self::CheckRequest),
            0x0101 => Some(Self::CheckResult),
            0x0200 => Some(Self::ScoreRequest),
            0x0201 => Some(Self::ScoreResult),
            0x0300 => Some(Self::SyncRequest),
            0x0301 => Some(Self::SyncProgress),
            0x0302 => Some(Self::SyncComplete),
            0x0400 => Some(Self::BranchConfig),
            0x0401 => Some(Self::BranchUpdate),
            0x0500 => Some(Self::AiUpdateRequest),
            0x0501 => Some(Self::AiUpdateApproval),
            0x0502 => Some(Self::AiUpdateResult),
            0x0600 => Some(Self::Heartbeat),
            0x0601 => Some(Self::StatusRequest),
            0x0602 => Some(Self::StatusResponse),
            0xFF00 => Some(Self::Error),
            0xFF01 => Some(Self::Ack),
            _ => None,
        }
    }
}

/// Frame reader for parsing messages
pub struct FrameReader {
    buffer: Vec<u8>,
    position: usize,
}

impl FrameReader {
    /// Create new reader
    pub fn new() -> Self {
        Self {
            buffer: Vec::with_capacity(4096),
            position: 0,
        }
    }

    /// Feed data into buffer
    pub fn feed(&mut self, data: &[u8]) {
        self.buffer.extend_from_slice(data);
    }

    /// Try to read next frame
    pub fn try_read(&mut self) -> Option<Frame> {
        // Need at least header size (16 bytes)
        if self.buffer.len() < 16 {
            return None;
        }

        // Check magic
        if &self.buffer[0..4] != &MAGIC {
            // Invalid frame, skip byte
            self.buffer.remove(0);
            return None;
        }

        // Parse header
        let version = self.buffer[4];
        let flags = FrameFlags::from_byte(self.buffer[5]);

        let message_length = u32::from_be_bytes([
            self.buffer[8],
            self.buffer[9],
            self.buffer[10],
            self.buffer[11],
        ]) as usize;

        // Check we have full message
        if self.buffer.len() < 16 + message_length {
            return None;
        }

        let message_type = u16::from_be_bytes([self.buffer[12], self.buffer[13]]);
        let header_length = u16::from_be_bytes([self.buffer[14], self.buffer[15]]) as usize;

        // Extract header and payload
        let header_start = 16;
        let header_end = header_start + header_length;
        let payload_start = header_end;
        let payload_end = header_start + message_length - 4; // -4 for CRC
        let crc_start = payload_end;

        let header = self.buffer[header_start..header_end].to_vec();
        let payload = self.buffer[payload_start..payload_end].to_vec();
        let crc = u32::from_be_bytes([
            self.buffer[crc_start],
            self.buffer[crc_start + 1],
            self.buffer[crc_start + 2],
            self.buffer[crc_start + 3],
        ]);

        // Verify CRC
        let computed_crc = crc32fast::hash(&self.buffer[header_start..payload_end]);
        if computed_crc != crc {
            // CRC mismatch, drop frame
            self.buffer.drain(0..16 + message_length);
            return None;
        }

        // Remove processed data
        self.buffer.drain(0..16 + message_length);

        Some(Frame {
            header: FrameHeader {
                version,
                flags,
                message_length: message_length as u32,
                message_type,
                header_length: header_length as u16,
            },
            header_data: header,
            payload,
        })
    }
}

/// Parsed frame
#[derive(Debug)]
pub struct Frame {
    pub header: FrameHeader,
    pub header_data: Vec<u8>,
    pub payload: Vec<u8>,
}

/// Frame writer for creating messages
pub struct FrameWriter;

impl FrameWriter {
    /// Write frame to buffer
    pub fn write(
        message_type: MessageTypeCode,
        flags: FrameFlags,
        header: &[u8],
        payload: &[u8],
    ) -> Vec<u8> {
        let message_length = header.len() + payload.len() + 4; // +4 for CRC

        let mut buffer = Vec::with_capacity(16 + message_length);

        // Magic
        buffer.extend_from_slice(&MAGIC);

        // Version
        buffer.push(PROTOCOL_VERSION);

        // Flags
        buffer.push(flags.to_byte());

        // Reserved
        buffer.extend_from_slice(&[0, 0]);

        // Message length
        buffer.extend_from_slice(&(message_length as u32).to_be_bytes());

        // Message type
        buffer.extend_from_slice(&(message_type as u16).to_be_bytes());

        // Header length
        buffer.extend_from_slice(&(header.len() as u16).to_be_bytes());

        // Header
        buffer.extend_from_slice(header);

        // Payload
        buffer.extend_from_slice(payload);

        // CRC32
        let crc = crc32fast::hash(&buffer[16..]);
        buffer.extend_from_slice(&crc.to_be_bytes());

        buffer
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frame_roundtrip() {
        let header = b"test-header";
        let payload = b"test-payload";

        let frame_data =
            FrameWriter::write(MessageTypeCode::Heartbeat, FrameFlags::default(), header, payload);

        let mut reader = FrameReader::new();
        reader.feed(&frame_data);

        let frame = reader.try_read().expect("Should parse frame");
        assert_eq!(frame.header.message_type, MessageTypeCode::Heartbeat as u16);
        assert_eq!(frame.header_data, header);
        assert_eq!(frame.payload, payload);
    }
}
