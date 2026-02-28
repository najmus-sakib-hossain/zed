//! DX-Machine header format
//!
//! The header is a compact 4-byte structure:
//! - 2 bytes: Magic (0x5A 0x44)
//! - 1 byte: Version (0x01)
//! - 1 byte: Flags

use std::fmt;

/// DX-Machine file header (4 bytes)
#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct DxMachineHeader {
    /// Magic bytes: 0x5A 0x44 ("ZD" little-endian)
    pub magic: [u8; 2],
    /// Format version (currently 0x01)
    pub version: u8,
    /// Feature flags (see FLAG_* constants)
    pub flags: u8,
}

/// Flag: File contains heap section
pub const FLAG_HAS_HEAP: u8 = 0b0000_0001;

/// Flag: File contains string intern table
pub const FLAG_HAS_INTERN: u8 = 0b0000_0010;

/// Flag: Data is little-endian (always set for v1)
pub const FLAG_LITTLE_ENDIAN: u8 = 0b0000_0100;

/// Flag: File contains length table for SIMD validation
pub const FLAG_HAS_LENGTH_TABLE: u8 = 0b0000_1000;

/// Reserved flags (must be zero in v1)
const FLAG_RESERVED_MASK: u8 = 0b1111_0000;

impl DxMachineHeader {
    /// Create a new header with default flags
    #[inline]
    pub const fn new() -> Self {
        Self {
            magic: crate::machine::MAGIC,
            version: crate::machine::VERSION,
            flags: FLAG_LITTLE_ENDIAN,
        }
    }

    /// Create header with specific flags
    #[inline]
    pub const fn with_flags(flags: u8) -> Self {
        Self {
            magic: crate::machine::MAGIC,
            version: crate::machine::VERSION,
            flags: flags | FLAG_LITTLE_ENDIAN,
        }
    }

    /// Parse header from byte slice
    #[inline]
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, HeaderError> {
        if bytes.len() < 4 {
            return Err(HeaderError::BufferTooSmall);
        }

        let header = Self {
            magic: [bytes[0], bytes[1]],
            version: bytes[2],
            flags: bytes[3],
        };

        header.validate()?;
        Ok(header)
    }

    /// Validate header
    #[inline]
    pub fn validate(&self) -> Result<(), HeaderError> {
        // Check magic bytes
        if self.magic != crate::machine::MAGIC {
            return Err(HeaderError::InvalidMagic {
                expected: crate::machine::MAGIC,
                found: self.magic,
            });
        }

        // Check version
        if self.version != crate::machine::VERSION {
            return Err(HeaderError::UnsupportedVersion {
                supported: crate::machine::VERSION,
                found: self.version,
            });
        }

        // Check reserved flags
        if self.flags & FLAG_RESERVED_MASK != 0 {
            return Err(HeaderError::ReservedFlagsSet);
        }

        // Verify little-endian flag is set
        if self.flags & FLAG_LITTLE_ENDIAN == 0 {
            return Err(HeaderError::UnsupportedEndianness);
        }

        Ok(())
    }

    /// Check if heap section exists
    #[inline]
    pub const fn has_heap(&self) -> bool {
        self.flags & FLAG_HAS_HEAP != 0
    }

    /// Check if intern table exists
    #[inline]
    pub const fn has_intern_table(&self) -> bool {
        self.flags & FLAG_HAS_INTERN != 0
    }

    /// Check if length table exists
    #[inline]
    pub const fn has_length_table(&self) -> bool {
        self.flags & FLAG_HAS_LENGTH_TABLE != 0
    }

    /// Set heap flag
    #[inline]
    pub fn set_has_heap(&mut self, value: bool) {
        if value {
            self.flags |= FLAG_HAS_HEAP;
        } else {
            self.flags &= !FLAG_HAS_HEAP;
        }
    }

    /// Set intern table flag
    #[inline]
    pub fn set_has_intern_table(&mut self, value: bool) {
        if value {
            self.flags |= FLAG_HAS_INTERN;
        } else {
            self.flags &= !FLAG_HAS_INTERN;
        }
    }

    /// Set length table flag
    #[inline]
    pub fn set_has_length_table(&mut self, value: bool) {
        if value {
            self.flags |= FLAG_HAS_LENGTH_TABLE;
        } else {
            self.flags &= !FLAG_HAS_LENGTH_TABLE;
        }
    }

    /// Write header to byte slice
    #[inline]
    pub fn write_to(&self, bytes: &mut [u8]) {
        bytes[0] = self.magic[0];
        bytes[1] = self.magic[1];
        bytes[2] = self.version;
        bytes[3] = self.flags;
    }

    /// Get header size in bytes
    #[inline]
    pub const fn size() -> usize {
        4
    }
}

impl Default for DxMachineHeader {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for DxMachineHeader {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DxMachineHeader")
            .field("magic", &format!("0x{:02X} 0x{:02X}", self.magic[0], self.magic[1]))
            .field("version", &self.version)
            .field("flags", &format!("0b{:08b}", self.flags))
            .field("has_heap", &self.has_heap())
            .field("has_intern_table", &self.has_intern_table())
            .field("has_length_table", &self.has_length_table())
            .finish()
    }
}

/// Header validation errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HeaderError {
    /// Buffer too small to contain header
    BufferTooSmall,
    /// Invalid magic bytes
    InvalidMagic { expected: [u8; 2], found: [u8; 2] },
    /// Unsupported format version
    UnsupportedVersion { supported: u8, found: u8 },
    /// Reserved flags are set (future version)
    ReservedFlagsSet,
    /// Unsupported endianness
    UnsupportedEndianness,
}

impl fmt::Display for HeaderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BufferTooSmall => write!(f, "Buffer too small to contain header (need 4 bytes)"),
            Self::InvalidMagic { expected, found } => {
                write!(f, "Invalid magic bytes: expected {:02X?}, found {:02X?}", expected, found)
            }
            Self::UnsupportedVersion { supported, found } => write!(
                f,
                "Unsupported format version: this implementation supports v{:02X}, found v{:02X}",
                supported, found
            ),
            Self::ReservedFlagsSet => {
                write!(f, "Reserved flags are set (file from future version?)")
            }
            Self::UnsupportedEndianness => {
                write!(f, "Unsupported endianness (big-endian not supported in v1)")
            }
        }
    }
}

impl std::error::Error for HeaderError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_header_new() {
        let header = DxMachineHeader::new();
        assert_eq!(header.magic, [0x5A, 0x44]);
        assert_eq!(header.version, 0x01);
        assert_eq!(header.flags & FLAG_LITTLE_ENDIAN, FLAG_LITTLE_ENDIAN);
        assert!(!header.has_heap());
        assert!(!header.has_intern_table());
    }

    #[test]
    fn test_header_flags() {
        let mut header = DxMachineHeader::new();

        assert!(!header.has_heap());
        header.set_has_heap(true);
        assert!(header.has_heap());

        assert!(!header.has_intern_table());
        header.set_has_intern_table(true);
        assert!(header.has_intern_table());
    }

    #[test]
    fn test_header_roundtrip() {
        let mut bytes = [0u8; 4];
        let header = DxMachineHeader::with_flags(FLAG_HAS_HEAP | FLAG_LITTLE_ENDIAN);

        header.write_to(&mut bytes);
        let parsed = DxMachineHeader::from_bytes(&bytes).unwrap();

        assert_eq!(parsed.magic, header.magic);
        assert_eq!(parsed.version, header.version);
        assert_eq!(parsed.flags, header.flags);
    }

    #[test]
    fn test_header_validation() {
        // Valid header
        let bytes = [0x5A, 0x44, 0x01, FLAG_LITTLE_ENDIAN];
        assert!(DxMachineHeader::from_bytes(&bytes).is_ok());

        // Invalid magic
        let bytes = [0x00, 0x00, 0x01, FLAG_LITTLE_ENDIAN];
        assert!(matches!(
            DxMachineHeader::from_bytes(&bytes),
            Err(HeaderError::InvalidMagic { .. })
        ));

        // Wrong version
        let bytes = [0x5A, 0x44, 0x99, FLAG_LITTLE_ENDIAN];
        assert!(matches!(
            DxMachineHeader::from_bytes(&bytes),
            Err(HeaderError::UnsupportedVersion { .. })
        ));

        // Reserved flags set
        let bytes = [0x5A, 0x44, 0x01, FLAG_LITTLE_ENDIAN | 0b1000_0000];
        assert!(matches!(
            DxMachineHeader::from_bytes(&bytes),
            Err(HeaderError::ReservedFlagsSet)
        ));
    }

    #[test]
    fn test_header_size() {
        assert_eq!(DxMachineHeader::size(), 4);
        assert_eq!(std::mem::size_of::<DxMachineHeader>(), 4);
    }
}
