//! Binary Rule Format Implementation
//!
//! Provides serialization and deserialization of rules to/from the .dxm binary format.
//! Uses dx-serializer for high-performance binary encoding.
//!
//! # Binary Format (.dxm)
//!
//! ```text
//! ┌─────────────────────────────────────────┐
//! │ Header (32 bytes)                       │
//! │ - Magic: "DXRULES\0" (8 bytes)          │
//! │ - Version: u32                          │
//! │ - Rule count: u32                       │
//! │ - Index offset: u64                     │
//! │ - Data offset: u64                      │
//! │ - Checksum: u32                         │
//! ├─────────────────────────────────────────┤
//! │ Rule Index                              │
//! │ - [RuleId, NameOffset, DataOffset, Len] │
//! ├─────────────────────────────────────────┤
//! │ Rule Data                               │
//! │ - Name, Category, Severity, Fixable     │
//! │ - Description, Pattern, Options Schema  │
//! └─────────────────────────────────────────┘
//! ```

use super::schema::DxRuleDatabase;
use bincode::{Decode, Encode, config};
use std::io::{Read, Write};
use std::path::Path;
use thiserror::Error;

/// Magic bytes for the binary rule format
pub const MAGIC: &[u8; 8] = b"DXRULES\0";

/// Current version of the binary format
pub const VERSION: u32 = 1;

/// Errors that can occur during rule serialization/deserialization
#[derive(Debug, Error)]
pub enum BinaryRuleError {
    #[error("Invalid magic bytes")]
    InvalidMagic,

    #[error("Version mismatch: expected {expected}, got {actual}")]
    VersionMismatch { expected: u32, actual: u32 },

    #[error("Checksum mismatch")]
    ChecksumMismatch,

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Encoding error: {0}")]
    EncodeError(String),

    #[error("Decoding error: {0}")]
    DecodeError(String),

    #[error("Rule not found: {0}")]
    RuleNotFound(String),
}

/// Binary header for the rule file
#[derive(Debug, Clone, Encode, Decode)]
pub struct BinaryHeader {
    /// Magic bytes: "DXRULES\0"
    pub magic: [u8; 8],
    /// Format version
    pub version: u32,
    /// Number of rules
    pub rule_count: u32,
    /// Offset to rule index
    pub index_offset: u64,
    /// Offset to rule data
    pub data_offset: u64,
    /// CRC32 checksum of data
    pub checksum: u32,
}

impl BinaryHeader {
    #[must_use]
    pub fn new(rule_count: u32) -> Self {
        Self {
            magic: *MAGIC,
            version: VERSION,
            rule_count,
            index_offset: 0,
            data_offset: 0,
            checksum: 0,
        }
    }

    pub fn validate(&self) -> Result<(), BinaryRuleError> {
        if &self.magic != MAGIC {
            return Err(BinaryRuleError::InvalidMagic);
        }
        if self.version != VERSION {
            return Err(BinaryRuleError::VersionMismatch {
                expected: VERSION,
                actual: self.version,
            });
        }
        Ok(())
    }
}

/// Index entry for fast rule lookup
#[derive(Debug, Clone, Encode, Decode)]
pub struct RuleIndexEntry {
    /// Rule ID
    pub rule_id: u16,
    /// Offset to rule name in string table
    pub name_offset: u32,
    /// Offset to rule data
    pub data_offset: u32,
    /// Length of rule data
    pub data_len: u32,
}

/// Serializer for rule databases
pub struct RuleSerializer;

impl RuleSerializer {
    /// Serialize a rule database to binary format
    pub fn serialize(db: &DxRuleDatabase) -> Result<Vec<u8>, BinaryRuleError> {
        let config = config::standard();

        // Serialize the entire database
        let data = bincode::encode_to_vec(db, config)
            .map_err(|e| BinaryRuleError::EncodeError(e.to_string()))?;

        // Calculate checksum
        let checksum = crc32fast::hash(&data);

        // Create header
        let mut header = BinaryHeader::new(db.rule_count);
        header.checksum = checksum;

        // Serialize header
        let header_bytes = bincode::encode_to_vec(&header, config)
            .map_err(|e| BinaryRuleError::EncodeError(e.to_string()))?;

        // Combine header and data
        let mut result = Vec::with_capacity(header_bytes.len() + data.len());
        result.extend_from_slice(&header_bytes);
        result.extend_from_slice(&data);

        Ok(result)
    }

    /// Deserialize a rule database from binary format
    pub fn deserialize(bytes: &[u8]) -> Result<DxRuleDatabase, BinaryRuleError> {
        let config = config::standard();

        // Decode header first
        let (header, header_len): (BinaryHeader, usize) = bincode::decode_from_slice(bytes, config)
            .map_err(|e| BinaryRuleError::DecodeError(e.to_string()))?;

        header.validate()?;

        // Get data portion
        let data = &bytes[header_len..];

        // Verify checksum
        let actual_checksum = crc32fast::hash(data);
        if actual_checksum != header.checksum {
            return Err(BinaryRuleError::ChecksumMismatch);
        }

        // Decode database
        let (db, _): (DxRuleDatabase, usize) = bincode::decode_from_slice(data, config)
            .map_err(|e| BinaryRuleError::DecodeError(e.to_string()))?;

        Ok(db)
    }

    /// Write rule database to a file
    pub fn write_to_file(db: &DxRuleDatabase, path: &Path) -> Result<(), BinaryRuleError> {
        let bytes = Self::serialize(db)?;
        let mut file = std::fs::File::create(path)?;
        file.write_all(&bytes)?;
        Ok(())
    }

    /// Read rule database from a file
    pub fn read_from_file(path: &Path) -> Result<DxRuleDatabase, BinaryRuleError> {
        let mut file = std::fs::File::open(path)?;
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)?;
        Self::deserialize(&bytes)
    }
}

/// Compressed serializer using LZ4
#[cfg(feature = "compression")]
pub struct CompressedRuleSerializer;

#[cfg(feature = "compression")]
impl CompressedRuleSerializer {
    /// Serialize with LZ4 compression
    pub fn serialize(db: &DxRuleDatabase) -> Result<Vec<u8>, BinaryRuleError> {
        let uncompressed = RuleSerializer::serialize(db)?;
        let compressed = lz4_flex::compress_prepend_size(&uncompressed);
        Ok(compressed)
    }

    /// Deserialize with LZ4 decompression
    pub fn deserialize(bytes: &[u8]) -> Result<DxRuleDatabase, BinaryRuleError> {
        let decompressed = lz4_flex::decompress_size_prepended(bytes)
            .map_err(|e| BinaryRuleError::DecodeError(e.to_string()))?;
        RuleSerializer::deserialize(&decompressed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::schema::{DxCategory, DxRule, Language, RuleSource};

    fn create_test_database() -> DxRuleDatabase {
        let mut db = DxRuleDatabase::new();

        db.add_rule(DxRule::new(
            1,
            Language::JavaScript,
            "no-console",
            "Disallow console statements",
            DxCategory::Suspicious,
            RuleSource::DxCheck,
        ));

        db.add_rule(DxRule::new(
            2,
            Language::JavaScript,
            "no-debugger",
            "Disallow debugger statements",
            DxCategory::Suspicious,
            RuleSource::DxCheck,
        ));

        db
    }

    #[test]
    fn test_serialize_deserialize_round_trip() {
        let db = create_test_database();

        // Serialize
        let bytes = RuleSerializer::serialize(&db).expect("Serialization failed");

        // Deserialize
        let restored = RuleSerializer::deserialize(&bytes).expect("Deserialization failed");

        // Verify
        assert_eq!(restored.rule_count, db.rule_count);
        assert_eq!(restored.rules.len(), db.rules.len());
        assert!(restored.get_by_name("js/no-console").is_some());
        assert!(restored.get_by_name("js/no-debugger").is_some());
    }

    #[test]
    fn test_invalid_magic() {
        let mut bytes = RuleSerializer::serialize(&create_test_database()).unwrap();
        // Corrupt magic bytes
        bytes[0] = 0xFF;

        let result = RuleSerializer::deserialize(&bytes);
        assert!(matches!(result, Err(BinaryRuleError::InvalidMagic)));
    }

    #[cfg(feature = "compression")]
    #[test]
    fn test_compressed_round_trip() {
        let db = create_test_database();

        let compressed = CompressedRuleSerializer::serialize(&db).expect("Compression failed");
        let restored =
            CompressedRuleSerializer::deserialize(&compressed).expect("Decompression failed");

        assert_eq!(restored.rule_count, db.rule_count);
    }
}
