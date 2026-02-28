//! HBTP (Hyper Binary Transport Protocol) header - 20 bytes.

use crate::DCPError;

/// HBTP magic number: "HBTP" in ASCII
pub const HBTP_MAGIC: u32 = 0x48425450;

/// HBTP header - 20 bytes
#[repr(C, packed)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HbtpHeader {
    /// Magic: 0x48425450 ("HBTP")
    pub magic: u32,
    /// Protocol version
    pub version: u8,
    /// Message type
    pub msg_type: u8,
    /// Flags (compression, priority, etc.)
    pub flags: u16,
    /// Multiplexed stream ID
    pub stream_id: u32,
    /// Payload length
    pub length: u32,
    /// CRC32 checksum of header fields (excluding checksum itself)
    pub checksum: u32,
}

impl HbtpHeader {
    /// Size of the header in bytes
    pub const SIZE: usize = 20;

    /// Create a new HBTP header with computed checksum
    pub fn new(version: u8, msg_type: u8, flags: u16, stream_id: u32, length: u32) -> Self {
        let mut header = Self {
            magic: HBTP_MAGIC,
            version,
            msg_type,
            flags,
            stream_id,
            length,
            checksum: 0,
        };
        header.checksum = header.compute_checksum();
        header
    }

    /// Compute CRC32 checksum of header fields (excluding checksum)
    pub fn compute_checksum(&self) -> u32 {
        let bytes = self.as_bytes();
        // Checksum covers first 16 bytes (everything except the checksum field)
        crc32fast::hash(&bytes[..16])
    }

    /// Verify the checksum is correct
    pub fn verify_checksum(&self) -> bool {
        self.checksum == self.compute_checksum()
    }

    /// Parse header from bytes - O(1) via pointer cast
    #[inline(always)]
    pub fn from_bytes(bytes: &[u8]) -> Result<&Self, DCPError> {
        if bytes.len() < Self::SIZE {
            return Err(DCPError::InsufficientData);
        }
        // SAFETY: We've verified the slice is at least 20 bytes
        let header = unsafe { &*(bytes.as_ptr() as *const Self) };
        if header.magic != HBTP_MAGIC {
            return Err(DCPError::InvalidMagic);
        }
        Ok(header)
    }

    /// Parse header from bytes and verify checksum
    pub fn from_bytes_verified(bytes: &[u8]) -> Result<&Self, DCPError> {
        let header = Self::from_bytes(bytes)?;
        if !header.verify_checksum() {
            return Err(DCPError::ChecksumMismatch);
        }
        Ok(header)
    }

    /// Serialize header to bytes - zero allocation
    #[inline(always)]
    pub fn as_bytes(&self) -> &[u8] {
        // SAFETY: HbtpHeader is repr(C, packed) and has no padding
        unsafe { std::slice::from_raw_parts(self as *const Self as *const u8, Self::SIZE) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_header_size() {
        assert_eq!(std::mem::size_of::<HbtpHeader>(), 20);
    }

    #[test]
    fn test_header_round_trip() {
        let header = HbtpHeader::new(1, 2, 0x0100, 12345, 4096);
        let bytes = header.as_bytes();
        let parsed = HbtpHeader::from_bytes(bytes).unwrap();

        // Copy values from packed struct before comparing
        let magic = parsed.magic;
        let version = parsed.version;
        let msg_type = parsed.msg_type;
        let flags = parsed.flags;
        let stream_id = parsed.stream_id;
        let length = parsed.length;

        assert_eq!(magic, HBTP_MAGIC);
        assert_eq!(version, 1);
        assert_eq!(msg_type, 2);
        assert_eq!(flags, 0x0100);
        assert_eq!(stream_id, 12345);
        assert_eq!(length, 4096);
        assert!(parsed.verify_checksum());
    }

    #[test]
    fn test_checksum_verification() {
        let header = HbtpHeader::new(1, 2, 0, 0, 100);
        assert!(header.verify_checksum());

        // Corrupt the header
        let mut bytes = header.as_bytes().to_vec();
        bytes[4] ^= 0xFF; // Flip version byte
        let corrupted = HbtpHeader::from_bytes(&bytes).unwrap();
        assert!(!corrupted.verify_checksum());
    }

    #[test]
    fn test_invalid_magic() {
        let bytes = [0u8; 20];
        assert_eq!(HbtpHeader::from_bytes(&bytes), Err(DCPError::InvalidMagic));
    }

    #[test]
    fn test_insufficient_data() {
        let bytes = [0u8; 10];
        assert_eq!(HbtpHeader::from_bytes(&bytes), Err(DCPError::InsufficientData));
    }
}
