//! Binary format reader.

use super::header::BinaryHeader;
use super::string_table::StringTable;
use crate::{Error, Result, WorkspaceConfig};
use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom};
use std::path::Path;

/// Reader for binary workspace configuration.
pub struct BinaryReader {
    /// Whether to use memory mapping for large files.
    use_mmap: bool,
    /// Minimum file size for memory mapping (default: 1MB).
    mmap_threshold: u64,
}

impl BinaryReader {
    /// Create a new binary reader.
    pub fn new() -> Self {
        Self {
            use_mmap: true,
            mmap_threshold: 1024 * 1024, // 1MB
        }
    }

    /// Create a reader that always uses buffered I/O.
    pub fn without_mmap() -> Self {
        Self {
            use_mmap: false,
            mmap_threshold: u64::MAX,
        }
    }

    /// Set the memory mapping threshold.
    pub fn with_mmap_threshold(mut self, threshold: u64) -> Self {
        self.mmap_threshold = threshold;
        self
    }

    /// Read workspace configuration from a binary file.
    pub fn read(&self, path: impl AsRef<Path>) -> Result<WorkspaceConfig> {
        let path = path.as_ref();
        let file = File::open(path).map_err(|e| Error::io(path, e))?;
        let metadata = file.metadata().map_err(|e| Error::io(path, e))?;

        // For now, always use buffered I/O to avoid unsafe blocks
        // Memory-mapped I/O can be added later with proper safety documentation
        let _ = (self.use_mmap, self.mmap_threshold, metadata.len());
        self.read_buffered(file, path)
    }

    /// Read using buffered I/O.
    fn read_buffered(&self, file: File, path: &Path) -> Result<WorkspaceConfig> {
        let mut reader = BufReader::new(file);

        // Read header
        let header = BinaryHeader::read_from(&mut reader)?;
        header.validate()?;

        // Seek to config data
        reader
            .seek(SeekFrom::Start(header.config_data_offset))
            .map_err(|e| Error::io(path, e))?;

        // Read config data
        let config_size = (header.total_size - header.config_data_offset) as usize;
        let mut config_bytes = vec![0u8; config_size];
        reader.read_exact(&mut config_bytes).map_err(|e| Error::io(path, e))?;

        // Deserialize config
        let config: WorkspaceConfig =
            serde_json::from_slice(&config_bytes).map_err(|e| Error::Serialization {
                format: "json".into(),
                details: e.to_string(),
            })?;

        Ok(config)
    }

    /// Read from a byte slice (used by mmap path).
    fn read_from_bytes(&self, bytes: &[u8]) -> Result<WorkspaceConfig> {
        // Parse header
        let header = BinaryHeader::from_bytes(bytes)?;
        header.validate()?;

        // Get config data slice
        let config_start = header.config_data_offset as usize;
        let config_end = header.total_size as usize;

        if bytes.len() < config_end {
            return Err(Error::InvalidBinaryFormat {
                reason: format!(
                    "File truncated: expected {} bytes, got {}",
                    config_end,
                    bytes.len()
                ),
            });
        }

        let config_bytes = &bytes[config_start..config_end];

        // Deserialize config
        let config: WorkspaceConfig =
            serde_json::from_slice(config_bytes).map_err(|e| Error::Serialization {
                format: "json".into(),
                details: e.to_string(),
            })?;

        Ok(config)
    }

    /// Validate a binary file without fully parsing it.
    pub fn validate(&self, path: impl AsRef<Path>) -> Result<bool> {
        let path = path.as_ref();
        let file = File::open(path).map_err(|e| Error::io(path, e))?;
        let mut reader = BufReader::new(file);

        // Read and validate header
        let header = BinaryHeader::read_from(&mut reader)?;
        header.validate()?;

        // Verify file size matches
        let metadata = reader.get_ref().metadata().map_err(|e| Error::io(path, e))?;
        if metadata.len() != header.total_size {
            return Ok(false);
        }

        // Verify content hash
        reader
            .seek(SeekFrom::Start(header.config_data_offset))
            .map_err(|e| Error::io(path, e))?;

        let config_size = (header.total_size - header.config_data_offset) as usize;
        let mut config_bytes = vec![0u8; config_size];
        reader.read_exact(&mut config_bytes).map_err(|e| Error::io(path, e))?;

        let computed_hash = blake3::hash(&config_bytes);
        Ok(*computed_hash.as_bytes() == header.content_hash)
    }

    /// Get the content hash without fully parsing the file.
    pub fn get_content_hash(&self, path: impl AsRef<Path>) -> Result<[u8; 32]> {
        let path = path.as_ref();
        let file = File::open(path).map_err(|e| Error::io(path, e))?;
        let mut reader = BufReader::new(file);

        let header = BinaryHeader::read_from(&mut reader)?;
        Ok(header.content_hash)
    }

    /// Read string table from a binary file.
    pub fn read_string_table(&self, path: impl AsRef<Path>) -> Result<StringTable> {
        let path = path.as_ref();
        let file = File::open(path).map_err(|e| Error::io(path, e))?;
        let mut reader = BufReader::new(file);

        let header = BinaryHeader::read_from(&mut reader)?;

        if !header.flags.has_string_table() {
            return Ok(StringTable::new());
        }

        let string_table_size = (header.config_data_offset - header.string_table_offset) as usize;
        StringTable::read_from(&mut reader, string_table_size)
    }
}

impl Default for BinaryReader {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::binary::BinaryWriter;
    use tempfile::tempdir;

    #[test]
    fn test_read_write_roundtrip() {
        let config = WorkspaceConfig::new("test-project");
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.dxws");

        // Write
        let writer = BinaryWriter::new();
        writer.write(&config, &path).unwrap();

        // Read
        let reader = BinaryReader::new();
        let loaded = reader.read(&path).unwrap();

        assert_eq!(config.name, loaded.name);
    }

    #[test]
    fn test_read_without_mmap() {
        let config = WorkspaceConfig::new("test-project");
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.dxws");

        let writer = BinaryWriter::new();
        writer.write(&config, &path).unwrap();

        let reader = BinaryReader::without_mmap();
        let loaded = reader.read(&path).unwrap();

        assert_eq!(config.name, loaded.name);
    }

    #[test]
    fn test_validate_valid_file() {
        let config = WorkspaceConfig::new("test-project");
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.dxws");

        let writer = BinaryWriter::new();
        writer.write(&config, &path).unwrap();

        let reader = BinaryReader::new();
        assert!(reader.validate(&path).unwrap());
    }

    #[test]
    fn test_content_hash() {
        let config = WorkspaceConfig::new("test-project");
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.dxws");

        let writer = BinaryWriter::new();
        writer.write(&config, &path).unwrap();

        let reader = BinaryReader::new();
        let hash = reader.get_content_hash(&path).unwrap();

        // Hash should be non-zero
        assert_ne!(hash, [0u8; 32]);
    }
}
