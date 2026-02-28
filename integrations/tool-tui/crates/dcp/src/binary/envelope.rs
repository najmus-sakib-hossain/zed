//! Binary Message Envelope (BME) - 8-byte fixed header for DCP messages.

use crate::DCPError;

/// DCP v1 magic number
pub const DCP_MAGIC: u16 = 0xDC01;

/// Binary Message Envelope - 8 bytes fixed header
#[repr(C, packed)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BinaryMessageEnvelope {
    /// Magic number: 0xDC01 for DCP v1
    pub magic: u16,
    /// Message type (Tool=1, Resource=2, Prompt=3, Response=4, Error=5, Stream=6)
    pub message_type: u8,
    /// Flags: bit 0=streaming, bit 1=compressed, bit 2=signed
    pub flags: u8,
    /// Payload length in bytes
    pub payload_len: u32,
}

/// Message types enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum MessageType {
    Tool = 1,
    Resource = 2,
    Prompt = 3,
    Response = 4,
    Error = 5,
    Stream = 6,
}

impl MessageType {
    /// Convert from u8 to MessageType
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            1 => Some(Self::Tool),
            2 => Some(Self::Resource),
            3 => Some(Self::Prompt),
            4 => Some(Self::Response),
            5 => Some(Self::Error),
            6 => Some(Self::Stream),
            _ => None,
        }
    }
}

/// Flag bits
#[allow(non_snake_case)]
pub mod Flags {
    pub const STREAMING: u8 = 0b0000_0001;
    pub const COMPRESSED: u8 = 0b0000_0010;
    pub const SIGNED: u8 = 0b0000_0100;
}

impl BinaryMessageEnvelope {
    /// Size of the envelope in bytes
    pub const SIZE: usize = 8;

    /// Create a new envelope
    pub fn new(message_type: MessageType, flags: u8, payload_len: u32) -> Self {
        Self {
            magic: DCP_MAGIC,
            message_type: message_type as u8,
            flags,
            payload_len,
        }
    }

    /// Parse envelope from bytes - O(1) via pointer cast
    #[inline(always)]
    pub fn from_bytes(bytes: &[u8]) -> Result<&Self, DCPError> {
        if bytes.len() < Self::SIZE {
            return Err(DCPError::InsufficientData);
        }
        // SAFETY: We've verified the slice is at least 8 bytes
        let envelope = unsafe { &*(bytes.as_ptr() as *const Self) };
        if envelope.magic != DCP_MAGIC {
            return Err(DCPError::InvalidMagic);
        }
        Ok(envelope)
    }

    /// Parse envelope from mutable bytes - O(1) via pointer cast
    #[inline(always)]
    pub fn from_bytes_mut(bytes: &mut [u8]) -> Result<&mut Self, DCPError> {
        if bytes.len() < Self::SIZE {
            return Err(DCPError::InsufficientData);
        }
        // SAFETY: We've verified the slice is at least 8 bytes
        let envelope = unsafe { &mut *(bytes.as_mut_ptr() as *mut Self) };
        if envelope.magic != DCP_MAGIC {
            return Err(DCPError::InvalidMagic);
        }
        Ok(envelope)
    }

    /// Serialize envelope to bytes - zero allocation
    #[inline(always)]
    pub fn as_bytes(&self) -> &[u8] {
        // SAFETY: BinaryMessageEnvelope is repr(C, packed) and has no padding
        unsafe { std::slice::from_raw_parts(self as *const Self as *const u8, Self::SIZE) }
    }

    /// Get the message type as enum
    pub fn get_message_type(&self) -> Option<MessageType> {
        MessageType::from_u8(self.message_type)
    }

    /// Check if streaming flag is set
    #[inline(always)]
    pub fn is_streaming(&self) -> bool {
        self.flags & Flags::STREAMING != 0
    }

    /// Check if compressed flag is set
    #[inline(always)]
    pub fn is_compressed(&self) -> bool {
        self.flags & Flags::COMPRESSED != 0
    }

    /// Check if signed flag is set
    #[inline(always)]
    pub fn is_signed(&self) -> bool {
        self.flags & Flags::SIGNED != 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_envelope_size() {
        assert_eq!(std::mem::size_of::<BinaryMessageEnvelope>(), 8);
    }

    #[test]
    fn test_envelope_round_trip() {
        let envelope =
            BinaryMessageEnvelope::new(MessageType::Tool, Flags::STREAMING | Flags::SIGNED, 1024);
        let bytes = envelope.as_bytes();
        let parsed = BinaryMessageEnvelope::from_bytes(bytes).unwrap();

        // Copy values from packed struct before comparing
        let magic = parsed.magic;
        let message_type = parsed.message_type;
        let flags = parsed.flags;
        let payload_len = parsed.payload_len;

        assert_eq!(magic, DCP_MAGIC);
        assert_eq!(message_type, MessageType::Tool as u8);
        assert_eq!(flags, Flags::STREAMING | Flags::SIGNED);
        assert_eq!(payload_len, 1024);
    }

    #[test]
    fn test_invalid_magic() {
        let mut bytes = [0u8; 8];
        bytes[0] = 0xFF;
        bytes[1] = 0xFF;
        assert_eq!(BinaryMessageEnvelope::from_bytes(&bytes), Err(DCPError::InvalidMagic));
    }

    #[test]
    fn test_insufficient_data() {
        let bytes = [0u8; 4];
        assert_eq!(BinaryMessageEnvelope::from_bytes(&bytes), Err(DCPError::InsufficientData));
    }

    #[test]
    fn test_flag_helpers() {
        let envelope = BinaryMessageEnvelope::new(
            MessageType::Stream,
            Flags::STREAMING | Flags::COMPRESSED,
            0,
        );
        assert!(envelope.is_streaming());
        assert!(envelope.is_compressed());
        assert!(!envelope.is_signed());
    }
}
