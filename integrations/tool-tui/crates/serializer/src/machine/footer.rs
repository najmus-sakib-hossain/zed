//! DX-Machine footer format (negative-offset)
//!
//! The footer is an 8-byte structure appended at the end of the file:
//! - 4 bytes: Magic ("DXM\0")
//! - 1 byte: Version
//! - 1 byte: Flags
//! - 2 bytes: CRC-16 checksum
//!
//! Layout:
//! ```text
//! [RKYV data at offset 0][8-byte footer at end]
//! ```

use std::fmt;

/// DX-Machine footer (8 bytes, appended at end)
#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct DxFooter {
    /// Magic bytes: "DXM\0"
    pub magic: [u8; 4],
    /// Format version (currently 0x01)
    pub version: u8,
    /// Feature flags
    pub flags: u8,
    /// CRC-16 checksum of RKYV data
    pub checksum: u16,
}

/// Footer magic bytes: "DXM\0"
pub const FOOTER_MAGIC: [u8; 4] = [b'D', b'X', b'M', 0x00];

/// Footer version
pub const FOOTER_VERSION: u8 = 0x01;

/// Footer size in bytes
pub const FOOTER_SIZE: usize = 8;

impl DxFooter {
    /// Create a new footer with checksum
    #[inline]
    pub const fn new(checksum: u16) -> Self {
        Self {
            magic: FOOTER_MAGIC,
            version: FOOTER_VERSION,
            flags: 0,
            checksum,
        }
    }

    /// Create footer with specific flags
    #[inline]
    pub const fn with_flags(checksum: u16, flags: u8) -> Self {
        Self {
            magic: FOOTER_MAGIC,
            version: FOOTER_VERSION,
            flags,
            checksum,
        }
    }

    /// Parse footer from byte slice (reads last 8 bytes)
    #[inline]
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, FooterError> {
        if bytes.len() < FOOTER_SIZE {
            return Err(FooterError::BufferTooSmall);
        }

        // Read footer from end of buffer
        let footer_start = bytes.len() - FOOTER_SIZE;
        let footer_bytes = &bytes[footer_start..];

        let footer = Self {
            magic: [
                footer_bytes[0],
                footer_bytes[1],
                footer_bytes[2],
                footer_bytes[3],
            ],
            version: footer_bytes[4],
            flags: footer_bytes[5],
            checksum: u16::from_le_bytes([footer_bytes[6], footer_bytes[7]]),
        };

        footer.validate()?;
        Ok(footer)
    }

    /// Validate footer
    #[inline]
    pub fn validate(&self) -> Result<(), FooterError> {
        // Check magic bytes
        if self.magic != FOOTER_MAGIC {
            return Err(FooterError::InvalidMagic {
                expected: FOOTER_MAGIC,
                found: self.magic,
            });
        }

        // Check version
        if self.version != FOOTER_VERSION {
            return Err(FooterError::UnsupportedVersion {
                supported: FOOTER_VERSION,
                found: self.version,
            });
        }

        Ok(())
    }

    /// Verify checksum against data
    #[inline]
    pub fn verify_checksum(&self, data: &[u8]) -> Result<(), FooterError> {
        let computed = compute_crc16(data);
        if computed != self.checksum {
            return Err(FooterError::ChecksumMismatch {
                expected: self.checksum,
                computed,
            });
        }
        Ok(())
    }

    /// Write footer to byte slice
    #[inline]
    pub fn write_to(&self, bytes: &mut [u8]) {
        bytes[0] = self.magic[0];
        bytes[1] = self.magic[1];
        bytes[2] = self.magic[2];
        bytes[3] = self.magic[3];
        bytes[4] = self.version;
        bytes[5] = self.flags;
        let checksum_bytes = self.checksum.to_le_bytes();
        bytes[6] = checksum_bytes[0];
        bytes[7] = checksum_bytes[1];
    }

    /// Get footer size in bytes
    #[inline]
    pub const fn size() -> usize {
        FOOTER_SIZE
    }
}

impl Default for DxFooter {
    fn default() -> Self {
        Self::new(0)
    }
}

impl fmt::Debug for DxFooter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Copy checksum to avoid unaligned reference to packed field
        let checksum = self.checksum;
        f.debug_struct("DxFooter")
            .field(
                "magic",
                &format!("{:?}", std::str::from_utf8(&self.magic).unwrap_or("<invalid>")),
            )
            .field("version", &self.version)
            .field("flags", &format!("0b{:08b}", self.flags))
            .field("checksum", &format!("0x{:04X}", checksum))
            .finish()
    }
}

/// Compute CRC-16 checksum (CCITT variant)
#[inline]
pub fn compute_crc16(data: &[u8]) -> u16 {
    let mut crc: u16 = 0xFFFF;

    for &byte in data {
        crc ^= (byte as u16) << 8;
        for _ in 0..8 {
            if crc & 0x8000 != 0 {
                crc = (crc << 1) ^ 0x1021;
            } else {
                crc <<= 1;
            }
        }
    }

    crc
}

/// Extract RKYV data from buffer with footer
#[inline]
pub fn extract_data(bytes: &[u8]) -> Result<&[u8], FooterError> {
    if bytes.len() < FOOTER_SIZE {
        return Err(FooterError::BufferTooSmall);
    }

    // Data is everything except the last 8 bytes (footer)
    let data_len = bytes.len() - FOOTER_SIZE;
    Ok(&bytes[..data_len])
}

/// Deserialize with footer validation
///
/// This function:
/// 1. Validates the footer at the end
/// 2. Extracts RKYV data (everything except footer)
/// 3. Verifies checksum
/// 4. Returns reference to RKYV data
#[inline]
pub fn deserialize_with_footer(bytes: &[u8]) -> Result<&[u8], FooterError> {
    // Parse and validate footer
    let footer = DxFooter::from_bytes(bytes)?;

    // Extract RKYV data
    let data = extract_data(bytes)?;

    // Verify checksum
    footer.verify_checksum(data)?;

    Ok(data)
}

/// Footer validation errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FooterError {
    /// Buffer too small to contain footer
    BufferTooSmall,
    /// Invalid magic bytes
    InvalidMagic { expected: [u8; 4], found: [u8; 4] },
    /// Unsupported format version
    UnsupportedVersion { supported: u8, found: u8 },
    /// Checksum mismatch
    ChecksumMismatch { expected: u16, computed: u16 },
}

impl fmt::Display for FooterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BufferTooSmall => {
                write!(f, "Buffer too small to contain footer (need {} bytes)", FOOTER_SIZE)
            }
            Self::InvalidMagic { expected, found } => {
                write!(f, "Invalid magic bytes: expected {:?}, found {:?}", expected, found)
            }
            Self::UnsupportedVersion { supported, found } => write!(
                f,
                "Unsupported footer version: this implementation supports v{:02X}, found v{:02X}",
                supported, found
            ),
            Self::ChecksumMismatch { expected, computed } => write!(
                f,
                "Checksum mismatch: expected 0x{:04X}, computed 0x{:04X}",
                expected, computed
            ),
        }
    }
}

impl std::error::Error for FooterError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_footer_new() {
        let footer = DxFooter::new(0x1234);
        assert_eq!(footer.magic, FOOTER_MAGIC);
        assert_eq!(footer.version, FOOTER_VERSION);
        assert_eq!(footer.flags, 0);
        // Copy to avoid unaligned reference
        let checksum = footer.checksum;
        assert_eq!(checksum, 0x1234);
    }

    #[test]
    fn test_footer_roundtrip() {
        let mut bytes = [0u8; 8];
        let footer = DxFooter::with_flags(0xABCD, 0b0000_0001);

        footer.write_to(&mut bytes);
        let parsed = DxFooter::from_bytes(&bytes).unwrap();

        assert_eq!(parsed.magic, footer.magic);
        assert_eq!(parsed.version, footer.version);
        assert_eq!(parsed.flags, footer.flags);
        // Copy to avoid unaligned reference
        let checksum1 = parsed.checksum;
        let checksum2 = footer.checksum;
        assert_eq!(checksum1, checksum2);
    }

    #[test]
    fn test_footer_validation() {
        // Valid footer
        let mut bytes = [0u8; 8];
        let footer = DxFooter::new(0);
        footer.write_to(&mut bytes);
        assert!(DxFooter::from_bytes(&bytes).is_ok());

        // Invalid magic
        let bytes = [0x00, 0x00, 0x00, 0x00, FOOTER_VERSION, 0, 0, 0];
        assert!(matches!(DxFooter::from_bytes(&bytes), Err(FooterError::InvalidMagic { .. })));

        // Wrong version
        let bytes = [b'D', b'X', b'M', 0x00, 0x99, 0, 0, 0];
        assert!(matches!(
            DxFooter::from_bytes(&bytes),
            Err(FooterError::UnsupportedVersion { .. })
        ));
    }

    #[test]
    fn test_crc16() {
        let data = b"Hello, World!";
        let crc = compute_crc16(data);
        assert_ne!(crc, 0); // Should produce non-zero checksum

        // Same data should produce same checksum
        let crc2 = compute_crc16(data);
        assert_eq!(crc, crc2);

        // Different data should produce different checksum
        let crc3 = compute_crc16(b"Hello, World?");
        assert_ne!(crc, crc3);
    }

    #[test]
    fn test_extract_data() {
        let mut buffer = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
        // Append footer
        buffer.extend_from_slice(&[b'D', b'X', b'M', 0x00, FOOTER_VERSION, 0, 0, 0]);

        let data = extract_data(&buffer).unwrap();
        assert_eq!(data, &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
    }

    #[test]
    fn test_deserialize_with_footer() {
        let data = b"Test RKYV data";
        let checksum = compute_crc16(data);

        // Create buffer with data + footer
        let mut buffer = Vec::from(&data[..]);
        let footer = DxFooter::new(checksum);
        let mut footer_bytes = [0u8; 8];
        footer.write_to(&mut footer_bytes);
        buffer.extend_from_slice(&footer_bytes);

        // Deserialize
        let extracted = deserialize_with_footer(&buffer).unwrap();
        assert_eq!(extracted, data);
    }

    #[test]
    fn test_deserialize_with_footer_bad_checksum() {
        let data = b"Test RKYV data";
        let wrong_checksum = 0x0000; // Wrong checksum

        // Create buffer with data + footer
        let mut buffer = Vec::from(&data[..]);
        let footer = DxFooter::new(wrong_checksum);
        let mut footer_bytes = [0u8; 8];
        footer.write_to(&mut footer_bytes);
        buffer.extend_from_slice(&footer_bytes);

        // Should fail checksum verification
        let result = deserialize_with_footer(&buffer);
        assert!(matches!(result, Err(FooterError::ChecksumMismatch { .. })));
    }

    #[test]
    fn test_footer_size() {
        assert_eq!(DxFooter::size(), 8);
        assert_eq!(std::mem::size_of::<DxFooter>(), 8);
    }
}
