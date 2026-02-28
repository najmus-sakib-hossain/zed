//! Binary header format.

use crate::{Error, Result};
use std::io::{Read, Write};

/// Magic bytes identifying a dx-workspace binary file.
pub const MAGIC: [u8; 4] = *b"DXWS";

/// Current binary format version.
pub const VERSION: u32 = 1;

/// Header size in bytes.
pub const HEADER_SIZE: usize = 72;

/// Flags for binary format features.
#[derive(Debug, Clone, Copy, Default)]
pub struct BinaryFlags(u32);

impl BinaryFlags {
    /// Create new flags with defaults.
    pub fn new() -> Self {
        Self(0)
    }

    /// Check if compression is enabled.
    pub fn is_compressed(&self) -> bool {
        self.0 & 0x01 != 0
    }

    /// Set compression flag.
    pub fn set_compressed(&mut self, value: bool) {
        if value {
            self.0 |= 0x01;
        } else {
            self.0 &= !0x01;
        }
    }

    /// Check if string deduplication is enabled.
    pub fn has_string_table(&self) -> bool {
        self.0 & 0x02 != 0
    }

    /// Set string table flag.
    pub fn set_string_table(&mut self, value: bool) {
        if value {
            self.0 |= 0x02;
        } else {
            self.0 &= !0x02;
        }
    }

    /// Get raw flags value.
    pub fn raw(&self) -> u32 {
        self.0
    }

    /// Create from raw value.
    pub fn from_raw(value: u32) -> Self {
        Self(value)
    }
}

/// Binary file header.
#[derive(Debug, Clone)]
pub struct BinaryHeader {
    /// Magic bytes (must be MAGIC).
    pub magic: [u8; 4],
    /// Format version.
    pub version: u32,
    /// Feature flags.
    pub flags: BinaryFlags,
    /// Blake3 hash of the config content.
    pub content_hash: [u8; 32],
    /// Offset to string table from start of file.
    pub string_table_offset: u64,
    /// Offset to config data from start of file.
    pub config_data_offset: u64,
    /// Total file size.
    pub total_size: u64,
}

impl BinaryHeader {
    /// Create a new header with default values.
    pub fn new() -> Self {
        Self {
            magic: MAGIC,
            version: VERSION,
            flags: BinaryFlags::new(),
            content_hash: [0u8; 32],
            string_table_offset: HEADER_SIZE as u64,
            config_data_offset: 0,
            total_size: 0,
        }
    }

    /// Read header from bytes.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < HEADER_SIZE {
            return Err(Error::InvalidBinaryFormat {
                reason: format!("Header too small: {} bytes", bytes.len()),
            });
        }

        let magic: [u8; 4] = bytes[0..4].try_into().unwrap();
        if magic != MAGIC {
            return Err(Error::InvalidBinaryFormat {
                reason: format!("Invalid magic bytes: {:?}", magic),
            });
        }

        let version = u32::from_le_bytes(bytes[4..8].try_into().unwrap());
        if version > VERSION {
            return Err(Error::InvalidBinaryFormat {
                reason: format!("Unsupported version: {} (max: {})", version, VERSION),
            });
        }

        let flags = BinaryFlags::from_raw(u32::from_le_bytes(bytes[8..12].try_into().unwrap()));
        let content_hash: [u8; 32] = bytes[12..44].try_into().unwrap();
        let string_table_offset = u64::from_le_bytes(bytes[44..52].try_into().unwrap());
        let config_data_offset = u64::from_le_bytes(bytes[52..60].try_into().unwrap());
        let total_size = u64::from_le_bytes(bytes[60..68].try_into().unwrap());

        Ok(Self {
            magic,
            version,
            flags,
            content_hash,
            string_table_offset,
            config_data_offset,
            total_size,
        })
    }

    /// Write header to bytes.
    pub fn to_bytes(&self) -> [u8; HEADER_SIZE] {
        let mut bytes = [0u8; HEADER_SIZE];

        bytes[0..4].copy_from_slice(&self.magic);
        bytes[4..8].copy_from_slice(&self.version.to_le_bytes());
        bytes[8..12].copy_from_slice(&self.flags.raw().to_le_bytes());
        bytes[12..44].copy_from_slice(&self.content_hash);
        bytes[44..52].copy_from_slice(&self.string_table_offset.to_le_bytes());
        bytes[52..60].copy_from_slice(&self.config_data_offset.to_le_bytes());
        bytes[60..68].copy_from_slice(&self.total_size.to_le_bytes());

        bytes
    }

    /// Read header from a reader.
    pub fn read_from<R: Read>(reader: &mut R) -> Result<Self> {
        let mut bytes = [0u8; HEADER_SIZE];
        reader.read_exact(&mut bytes).map_err(|e| Error::io("header", e))?;
        Self::from_bytes(&bytes)
    }

    /// Write header to a writer.
    pub fn write_to<W: Write>(&self, writer: &mut W) -> Result<()> {
        let bytes = self.to_bytes();
        writer.write_all(&bytes).map_err(|e| Error::io("header", e))?;
        Ok(())
    }

    /// Validate the header.
    pub fn validate(&self) -> Result<()> {
        if self.magic != MAGIC {
            return Err(Error::InvalidBinaryFormat {
                reason: "Invalid magic bytes".into(),
            });
        }

        if self.version > VERSION {
            return Err(Error::InvalidBinaryFormat {
                reason: format!("Unsupported version: {}", self.version),
            });
        }

        if self.config_data_offset < self.string_table_offset {
            return Err(Error::InvalidBinaryFormat {
                reason: "Config data offset before string table".into(),
            });
        }

        if self.total_size < self.config_data_offset {
            return Err(Error::InvalidBinaryFormat {
                reason: "Total size smaller than config data offset".into(),
            });
        }

        Ok(())
    }
}

impl Default for BinaryHeader {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_header_roundtrip() {
        let mut header = BinaryHeader::new();
        header.content_hash = [42u8; 32];
        header.string_table_offset = 64;
        header.config_data_offset = 128;
        header.total_size = 256;

        let bytes = header.to_bytes();
        let parsed = BinaryHeader::from_bytes(&bytes).unwrap();

        assert_eq!(parsed.magic, MAGIC);
        assert_eq!(parsed.version, VERSION);
        assert_eq!(parsed.content_hash, [42u8; 32]);
        assert_eq!(parsed.string_table_offset, 64);
        assert_eq!(parsed.config_data_offset, 128);
        assert_eq!(parsed.total_size, 256);
    }

    #[test]
    fn test_flags() {
        let mut flags = BinaryFlags::new();
        assert!(!flags.is_compressed());
        assert!(!flags.has_string_table());

        flags.set_compressed(true);
        assert!(flags.is_compressed());

        flags.set_string_table(true);
        assert!(flags.has_string_table());
    }

    #[test]
    fn test_invalid_magic() {
        let mut bytes = [0u8; HEADER_SIZE];
        bytes[0..4].copy_from_slice(b"XXXX");

        let result = BinaryHeader::from_bytes(&bytes);
        assert!(result.is_err());
    }
}
