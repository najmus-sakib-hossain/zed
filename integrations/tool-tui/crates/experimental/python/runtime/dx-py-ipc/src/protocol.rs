//! HBTP Protocol definitions - Header and message types
//!
//! High-performance Binary Transfer Protocol for Python IPC.

use bitflags::bitflags;

/// Magic bytes for HBTP protocol identification
pub const HBTP_MAGIC: u16 = 0xDEAD;

/// HBTP Header - 8 bytes, compact for low overhead
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct HbtpHeader {
    /// Magic bytes (0xDEAD)
    pub magic: u16,
    /// Message type
    pub msg_type: MessageType,
    /// Message flags
    pub flags: HbtpFlags,
    /// Payload length in bytes
    pub payload_len: u32,
}

impl HbtpHeader {
    /// Create a new HBTP header
    pub fn new(msg_type: MessageType, flags: HbtpFlags, payload_len: u32) -> Self {
        Self {
            magic: HBTP_MAGIC,
            msg_type,
            flags,
            payload_len,
        }
    }

    /// Validate the magic bytes
    #[inline]
    pub fn validate_magic(&self) -> bool {
        self.magic == HBTP_MAGIC
    }

    /// Serialize to bytes
    pub fn to_bytes(&self) -> [u8; 8] {
        let mut bytes = [0u8; 8];
        bytes[0..2].copy_from_slice(&self.magic.to_le_bytes());
        bytes[2] = self.msg_type as u8;
        bytes[3] = self.flags.bits();
        bytes[4..8].copy_from_slice(&self.payload_len.to_le_bytes());
        bytes
    }

    /// Deserialize from bytes
    pub fn from_bytes(bytes: &[u8; 8]) -> Option<Self> {
        let magic = u16::from_le_bytes([bytes[0], bytes[1]]);
        if magic != HBTP_MAGIC {
            return None;
        }

        let msg_type = MessageType::from_u8(bytes[2])?;
        let flags = HbtpFlags::from_bits(bytes[3])?;
        let payload_len = u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);

        Some(Self {
            magic,
            msg_type,
            flags,
            payload_len,
        })
    }

    /// Get the header size
    pub const fn size() -> usize {
        8
    }
}

/// Message types for HBTP protocol
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageType {
    /// Transfer a Python object
    TransferObject = 0,
    /// Transfer an array (NumPy-compatible)
    TransferArray = 1,
    /// Call a remote function
    CallFunction = 2,
    /// Return value from function call
    ReturnValue = 3,
    /// Exception/error response
    Exception = 4,
    /// Acknowledgment
    Ack = 5,
    /// Ping for keepalive
    Ping = 6,
    /// Pong response
    Pong = 7,
    /// Shared memory handle transfer
    SharedMemHandle = 8,
    /// Close connection
    Close = 9,
}

impl MessageType {
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(Self::TransferObject),
            1 => Some(Self::TransferArray),
            2 => Some(Self::CallFunction),
            3 => Some(Self::ReturnValue),
            4 => Some(Self::Exception),
            5 => Some(Self::Ack),
            6 => Some(Self::Ping),
            7 => Some(Self::Pong),
            8 => Some(Self::SharedMemHandle),
            9 => Some(Self::Close),
            _ => None,
        }
    }
}

bitflags! {
    /// Flags for HBTP messages
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct HbtpFlags: u8 {
        /// Payload is compressed
        const COMPRESSED = 0x01;
        /// Uses shared memory for payload
        const SHARED_MEMORY = 0x02;
        /// Requires acknowledgment
        const REQUIRES_ACK = 0x04;
        /// Message is a response
        const IS_RESPONSE = 0x08;
        /// Payload contains multiple items
        const BATCH = 0x10;
        /// High priority message
        const HIGH_PRIORITY = 0x20;
    }
}

/// Array data type for HBTP transfers
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArrayDtype {
    Int8 = 0,
    Int16 = 1,
    Int32 = 2,
    Int64 = 3,
    UInt8 = 4,
    UInt16 = 5,
    UInt32 = 6,
    UInt64 = 7,
    Float32 = 8,
    Float64 = 9,
    Bool = 10,
    Complex64 = 11,
    Complex128 = 12,
}

impl ArrayDtype {
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(Self::Int8),
            1 => Some(Self::Int16),
            2 => Some(Self::Int32),
            3 => Some(Self::Int64),
            4 => Some(Self::UInt8),
            5 => Some(Self::UInt16),
            6 => Some(Self::UInt32),
            7 => Some(Self::UInt64),
            8 => Some(Self::Float32),
            9 => Some(Self::Float64),
            10 => Some(Self::Bool),
            11 => Some(Self::Complex64),
            12 => Some(Self::Complex128),
            _ => None,
        }
    }

    /// Get the size in bytes of this dtype
    pub fn size(&self) -> usize {
        match self {
            Self::Int8 | Self::UInt8 | Self::Bool => 1,
            Self::Int16 | Self::UInt16 => 2,
            Self::Int32 | Self::UInt32 | Self::Float32 => 4,
            Self::Int64 | Self::UInt64 | Self::Float64 | Self::Complex64 => 8,
            Self::Complex128 => 16,
        }
    }
}

/// Array metadata for HBTP transfers
#[repr(C)]
#[derive(Debug, Clone)]
pub struct ArrayMetadata {
    /// Data type
    pub dtype: ArrayDtype,
    /// Number of dimensions
    pub ndim: u8,
    /// Shape (up to 8 dimensions)
    pub shape: [u64; 8],
    /// Strides in bytes
    pub strides: [i64; 8],
    /// Total number of elements
    pub size: u64,
}

impl ArrayMetadata {
    pub fn new(dtype: ArrayDtype, shape: &[u64]) -> Self {
        let ndim = shape.len().min(8) as u8;
        let mut shape_arr = [0u64; 8];
        let mut strides_arr = [0i64; 8];

        for (i, &s) in shape.iter().take(8).enumerate() {
            shape_arr[i] = s;
        }

        // Calculate C-contiguous strides
        let elem_size = dtype.size() as i64;
        let mut stride = elem_size;
        for i in (0..ndim as usize).rev() {
            strides_arr[i] = stride;
            stride *= shape_arr[i] as i64;
        }

        let size = shape.iter().product();

        Self {
            dtype,
            ndim,
            shape: shape_arr,
            strides: strides_arr,
            size,
        }
    }

    /// Get the total size in bytes
    pub fn byte_size(&self) -> usize {
        self.size as usize * self.dtype.size()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_header_roundtrip() {
        let header = HbtpHeader::new(
            MessageType::TransferArray,
            HbtpFlags::SHARED_MEMORY | HbtpFlags::REQUIRES_ACK,
            12345,
        );

        let bytes = header.to_bytes();
        let parsed = HbtpHeader::from_bytes(&bytes).unwrap();

        assert_eq!(parsed.msg_type, MessageType::TransferArray);
        assert!(parsed.flags.contains(HbtpFlags::SHARED_MEMORY));
        assert!(parsed.flags.contains(HbtpFlags::REQUIRES_ACK));
        // Access packed field through a copy
        let payload_len = parsed.payload_len;
        assert_eq!(payload_len, 12345);
    }

    #[test]
    fn test_header_size() {
        assert_eq!(HbtpHeader::size(), 8);
    }

    #[test]
    fn test_array_metadata() {
        let meta = ArrayMetadata::new(ArrayDtype::Float64, &[100, 200]);
        assert_eq!(meta.ndim, 2);
        assert_eq!(meta.size, 20000);
        assert_eq!(meta.byte_size(), 160000);
    }
}
