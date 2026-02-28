//! Binary stream chunk for streaming support.

use crate::DCPError;

/// Chunk flag constants
#[allow(non_snake_case)]
pub mod ChunkFlags {
    pub const FIRST: u8 = 0b0001;
    pub const CONTINUE: u8 = 0b0010;
    pub const LAST: u8 = 0b0100;
    pub const ERROR: u8 = 0b1000;
}

/// Binary stream chunk - 7 bytes header
#[repr(C, packed)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StreamChunk {
    /// Sequence number for ordering
    pub sequence: u32,
    /// Flags: FIRST=1, CONTINUE=2, LAST=4, ERROR=8
    pub flags: u8,
    /// Chunk payload length
    pub len: u16,
    // Payload follows
}

impl StreamChunk {
    /// Size of the chunk header in bytes
    pub const SIZE: usize = 7;

    /// Create a new stream chunk
    pub fn new(sequence: u32, flags: u8, len: u16) -> Self {
        Self {
            sequence,
            flags,
            len,
        }
    }

    /// Create the first chunk in a stream
    pub fn first(sequence: u32, len: u16) -> Self {
        Self::new(sequence, ChunkFlags::FIRST, len)
    }

    /// Create a continuation chunk
    pub fn continuation(sequence: u32, len: u16) -> Self {
        Self::new(sequence, ChunkFlags::CONTINUE, len)
    }

    /// Create the last chunk in a stream
    pub fn last(sequence: u32, len: u16) -> Self {
        Self::new(sequence, ChunkFlags::LAST, len)
    }

    /// Create an error chunk
    pub fn error(sequence: u32, len: u16) -> Self {
        Self::new(sequence, ChunkFlags::ERROR, len)
    }

    /// Parse from bytes
    #[inline(always)]
    pub fn from_bytes(bytes: &[u8]) -> Result<&Self, DCPError> {
        if bytes.len() < Self::SIZE {
            return Err(DCPError::InsufficientData);
        }
        // SAFETY: We've verified the slice is at least SIZE bytes
        Ok(unsafe { &*(bytes.as_ptr() as *const Self) })
    }

    /// Serialize to bytes
    #[inline(always)]
    pub fn as_bytes(&self) -> &[u8] {
        // SAFETY: StreamChunk is repr(C, packed) with no padding
        unsafe { std::slice::from_raw_parts(self as *const Self as *const u8, Self::SIZE) }
    }

    /// Check if this is the first chunk
    #[inline(always)]
    pub fn is_first(&self) -> bool {
        self.flags & ChunkFlags::FIRST != 0
    }

    /// Check if this is a continuation chunk
    #[inline(always)]
    pub fn is_continuation(&self) -> bool {
        self.flags & ChunkFlags::CONTINUE != 0
    }

    /// Check if this is the last chunk
    #[inline(always)]
    pub fn is_last(&self) -> bool {
        self.flags & ChunkFlags::LAST != 0
    }

    /// Check if this is an error chunk
    #[inline(always)]
    pub fn is_error(&self) -> bool {
        self.flags & ChunkFlags::ERROR != 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunk_size() {
        assert_eq!(std::mem::size_of::<StreamChunk>(), 7);
    }

    #[test]
    fn test_chunk_round_trip() {
        let chunk = StreamChunk::new(12345, ChunkFlags::FIRST | ChunkFlags::LAST, 1024);
        let bytes = chunk.as_bytes();
        let parsed = StreamChunk::from_bytes(bytes).unwrap();

        // Copy values from packed struct before comparing
        let sequence = parsed.sequence;
        let flags = parsed.flags;
        let len = parsed.len;

        assert_eq!(sequence, 12345);
        assert_eq!(flags, ChunkFlags::FIRST | ChunkFlags::LAST);
        assert_eq!(len, 1024);
    }

    #[test]
    fn test_flag_helpers() {
        let first = StreamChunk::first(0, 100);
        assert!(first.is_first());
        assert!(!first.is_last());

        let last = StreamChunk::last(1, 50);
        assert!(last.is_last());
        assert!(!last.is_first());

        let error = StreamChunk::error(2, 0);
        assert!(error.is_error());
    }

    #[test]
    fn test_insufficient_data() {
        let bytes = [0u8; 4];
        assert_eq!(StreamChunk::from_bytes(&bytes), Err(DCPError::InsufficientData));
    }
}
