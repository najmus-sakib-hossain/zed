//! Binary format for dx-workspace configuration.
//!
//! This module provides zero-copy binary serialization for workspace configurations,
//! enabling microsecond-level loading times instead of millisecond JSON parsing.
//!
//! # Format
//!
//! The binary format uses a compact representation optimized for:
//! - Zero-copy deserialization where possible
//! - Content-based caching with Blake3 hashes
//! - Fast validation without full parsing
//!
//! ```text
//! ┌─────────────────────────────────────┐
//! │  HEADER (64 bytes)                  │
//! │  - Magic: b"DXWS"                   │
//! │  - Version: u32                     │
//! │  - Flags: u32                       │
//! │  - Content Hash: [u8; 32]           │
//! │  - String Table Offset: u64         │
//! │  - Config Data Offset: u64          │
//! │  - Total Size: u64                  │
//! ├─────────────────────────────────────┤
//! │  STRING TABLE                       │
//! │  - Count: u32                       │
//! │  - Offsets: [u32; count]            │
//! │  - UTF-8 strings (null-terminated)  │
//! ├─────────────────────────────────────┤
//! │  CONFIG DATA                        │
//! │  - Bincode-encoded WorkspaceConfig  │
//! └─────────────────────────────────────┘
//! ```

mod header;
mod reader;
mod string_table;
mod writer;

pub use header::{BinaryHeader, MAGIC, VERSION};
pub use reader::BinaryReader;
pub use string_table::StringTable;
pub use writer::BinaryWriter;

use crate::{Result, WorkspaceConfig};
use std::path::Path;

/// File extension for binary workspace configuration.
pub const BINARY_EXTENSION: &str = "dxws";

/// Save workspace configuration to binary format.
pub fn save_binary(config: &WorkspaceConfig, path: impl AsRef<Path>) -> Result<()> {
    let writer = BinaryWriter::new();
    writer.write(config, path)
}

/// Load workspace configuration from binary format.
pub fn load_binary(path: impl AsRef<Path>) -> Result<WorkspaceConfig> {
    let reader = BinaryReader::new();
    reader.read(path)
}

/// Check if a binary file is valid without fully parsing it.
pub fn validate_binary(path: impl AsRef<Path>) -> Result<bool> {
    let reader = BinaryReader::new();
    reader.validate(path)
}

/// Get the content hash of a binary file without fully parsing it.
pub fn get_content_hash(path: impl AsRef<Path>) -> Result<[u8; 32]> {
    let reader = BinaryReader::new();
    reader.get_content_hash(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_roundtrip() {
        let config = WorkspaceConfig::new("test-project");
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.dxws");

        save_binary(&config, &path).unwrap();
        let loaded = load_binary(&path).unwrap();

        assert_eq!(config.name, loaded.name);
    }

    #[test]
    fn test_validate() {
        let config = WorkspaceConfig::new("test-project");
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.dxws");

        save_binary(&config, &path).unwrap();
        assert!(validate_binary(&path).unwrap());
    }

    #[test]
    fn test_content_hash_consistency() {
        let config = WorkspaceConfig::new("test-project");
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.dxws");

        save_binary(&config, &path).unwrap();
        let hash1 = get_content_hash(&path).unwrap();
        let hash2 = get_content_hash(&path).unwrap();

        assert_eq!(hash1, hash2);
    }
}
