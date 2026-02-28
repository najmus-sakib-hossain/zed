//! HBTP protocol types and parsing.

use bitflags::bitflags;
use std::mem::size_of;

/// HBTP OpCodes - 1 byte for common operations.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HbtpOpcode {
    /// Ping request (keepalive).
    Ping = 0x00,
    /// Pong response (keepalive).
    Pong = 0x01,
    /// Close connection.
    Close = 0x02,
    /// Full state synchronization.
    StateSync = 0x10,
    /// Delta state update.
    StateDelta = 0x11,
    /// HTIP clone operation.
    HtipClone = 0x20,
    /// HTIP text patch operation.
    HtipPatchText = 0x21,
    /// RPC call request.
    RpcCall = 0x30,
    /// RPC call response.
    RpcResponse = 0x31,
    /// RPC error response.
    RpcError = 0x32,
    /// Client-side event.
    ClientEvent = 0x40,
    /// Extended opcode (uses payload for actual opcode).
    Extended = 0xFF,
}

impl HbtpOpcode {
    /// Try to convert a u8 to an opcode.
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0x00 => Some(Self::Ping),
            0x01 => Some(Self::Pong),
            0x02 => Some(Self::Close),
            0x10 => Some(Self::StateSync),
            0x11 => Some(Self::StateDelta),
            0x20 => Some(Self::HtipClone),
            0x21 => Some(Self::HtipPatchText),
            0x30 => Some(Self::RpcCall),
            0x31 => Some(Self::RpcResponse),
            0x32 => Some(Self::RpcError),
            0x40 => Some(Self::ClientEvent),
            0xFF => Some(Self::Extended),
            _ => None,
        }
    }

    /// Get all defined opcodes.
    pub fn all() -> &'static [HbtpOpcode] {
        &[
            Self::Ping,
            Self::Pong,
            Self::Close,
            Self::StateSync,
            Self::StateDelta,
            Self::HtipClone,
            Self::HtipPatchText,
            Self::RpcCall,
            Self::RpcResponse,
            Self::RpcError,
            Self::ClientEvent,
            Self::Extended,
        ]
    }
}

bitflags! {
    /// HBTP message flags.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct HbtpFlags: u8 {
        /// Payload is compressed (LZ4).
        const COMPRESSED = 0b0000_0001;
        /// Payload is encrypted.
        const ENCRYPTED = 0b0000_0010;
        /// Sender expects a response.
        const EXPECTS_RESPONSE = 0b0000_0100;
        /// This is the final message in a sequence.
        const FINAL = 0b0000_1000;
    }
}

/// 8-byte header for all HBTP messages.
///
/// Layout:
/// - Byte 0: opcode (HbtpOpcode)
/// - Byte 1: flags (HbtpFlags)
/// - Bytes 2-3: sequence number (u16, little-endian)
/// - Bytes 4-7: payload length (u32, little-endian)
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct HbtpHeader {
    /// Operation type.
    pub opcode: u8,
    /// Message flags.
    pub flags: u8,
    /// Request sequence number for correlation.
    pub sequence: u16,
    /// Payload length in bytes.
    pub length: u32,
}

impl HbtpHeader {
    /// Header size in bytes (always 8).
    pub const SIZE: usize = 8;

    /// Create a new header.
    pub fn new(opcode: HbtpOpcode, flags: HbtpFlags, sequence: u16, length: u32) -> Self {
        Self {
            opcode: opcode as u8,
            flags: flags.bits(),
            sequence,
            length,
        }
    }

    /// Parse a header from a byte slice (zero-copy).
    ///
    /// Returns `Some` if the slice is at least 8 bytes, `None` otherwise.
    pub fn from_bytes(bytes: &[u8]) -> Option<&Self> {
        if bytes.len() < Self::SIZE {
            return None;
        }
        // SAFETY: We've verified the slice is large enough, and HbtpHeader
        // is repr(C, packed) with no padding requirements.
        Some(unsafe { &*(bytes.as_ptr() as *const Self) })
    }

    /// Serialize the header to bytes.
    pub fn to_bytes(&self) -> [u8; Self::SIZE] {
        let mut bytes = [0u8; Self::SIZE];
        bytes[0] = self.opcode;
        bytes[1] = self.flags;
        bytes[2..4].copy_from_slice(&self.sequence.to_le_bytes());
        bytes[4..8].copy_from_slice(&self.length.to_le_bytes());
        bytes
    }

    /// Get the opcode as an enum.
    pub fn opcode(&self) -> Option<HbtpOpcode> {
        HbtpOpcode::from_u8(self.opcode)
    }

    /// Get the flags as a bitflags struct.
    pub fn flags(&self) -> HbtpFlags {
        HbtpFlags::from_bits_truncate(self.flags)
    }

    /// Get the payload slice from a message buffer.
    pub fn payload<'a>(&self, buffer: &'a [u8]) -> Option<&'a [u8]> {
        let start = Self::SIZE;
        let end = start + self.length as usize;
        if buffer.len() >= end {
            Some(&buffer[start..end])
        } else {
            None
        }
    }
}

// Compile-time assertion that HbtpHeader is exactly 8 bytes
const _: () = assert!(size_of::<HbtpHeader>() == 8);

/// HBTP protocol errors.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum HbtpError {
    /// Invalid or incomplete header.
    #[error("invalid header: buffer too small")]
    InvalidHeader,
    /// Invalid payload.
    #[error("invalid payload: {0}")]
    InvalidPayload(String),
    /// Route not found.
    #[error("route not found: {0}")]
    RouteNotFound(u8),
    /// Unknown opcode.
    #[error("unknown opcode: {0}")]
    UnknownOpcode(u8),
}

/// Handler function type for HBTP messages.
pub type HbtpHandler = fn(&HbtpHeader, &[u8]) -> Result<Vec<u8>, HbtpError>;

/// HBTP protocol handler with O(1) route lookup.
#[derive(Clone)]
pub struct HbtpProtocol {
    /// Route handlers indexed by opcode (0-255).
    handlers: [Option<HbtpHandler>; 256],
}

impl HbtpProtocol {
    /// Create a new protocol handler.
    pub fn new() -> Self {
        Self {
            handlers: [None; 256],
        }
    }

    /// Register a handler for an opcode.
    ///
    /// This provides O(1) lookup by using the opcode as an array index.
    pub fn route(&mut self, opcode: HbtpOpcode, handler: HbtpHandler) -> &mut Self {
        self.handlers[opcode as usize] = Some(handler);
        self
    }

    /// Process an incoming message.
    pub fn process(&self, buffer: &[u8]) -> Result<Vec<u8>, HbtpError> {
        let header = HbtpHeader::from_bytes(buffer).ok_or(HbtpError::InvalidHeader)?;

        let handler =
            self.handlers[header.opcode as usize].ok_or(HbtpError::RouteNotFound(header.opcode))?;

        let payload = header.payload(buffer).unwrap_or(&[]);
        handler(header, payload)
    }

    /// Get a handler for an opcode (O(1) lookup).
    pub fn get_handler(&self, opcode: u8) -> Option<HbtpHandler> {
        self.handlers[opcode as usize]
    }
}

impl Default for HbtpProtocol {
    fn default() -> Self {
        Self::new()
    }
}
