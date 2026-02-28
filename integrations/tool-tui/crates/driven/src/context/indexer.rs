//! Codebase indexer for binary format

use crate::Result;
use std::path::Path;

/// Indexes a codebase into binary format for fast access
#[derive(Debug, Default)]
pub struct CodebaseIndexer {
    /// Whether to use SIMD acceleration
    use_simd: bool,
}

impl CodebaseIndexer {
    /// Create a new indexer
    pub fn new() -> Self {
        Self { use_simd: false }
    }

    /// Enable SIMD acceleration
    pub fn with_simd(mut self, enabled: bool) -> Self {
        self.use_simd = enabled;
        self
    }

    /// Index a project directory
    pub fn index(&self, _path: &Path) -> Result<CodebaseIndex> {
        // TODO: Implement binary indexing
        Ok(CodebaseIndex::default())
    }

    /// Load an existing index
    pub fn load(&self, path: &Path) -> Result<CodebaseIndex> {
        let data = std::fs::read(path)?;
        CodebaseIndex::from_bytes(&data)
    }
}

/// Binary codebase index
#[derive(Debug, Default)]
pub struct CodebaseIndex {
    /// Version of the index format
    pub version: u32,
    /// Number of files indexed
    pub file_count: u32,
    /// Raw index data
    data: Vec<u8>,
}

impl CodebaseIndex {
    /// Create from binary data
    pub fn from_bytes(data: &[u8]) -> Result<Self> {
        if data.len() < 8 {
            return Ok(Self::default());
        }

        let version = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
        let file_count = u32::from_le_bytes([data[4], data[5], data[6], data[7]]);

        Ok(Self {
            version,
            file_count,
            data: data[8..].to_vec(),
        })
    }

    /// Serialize to binary
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(8 + self.data.len());
        bytes.extend_from_slice(&self.version.to_le_bytes());
        bytes.extend_from_slice(&self.file_count.to_le_bytes());
        bytes.extend_from_slice(&self.data);
        bytes
    }

    /// Save index to file
    pub fn save(&self, path: &Path) -> Result<()> {
        std::fs::write(path, self.to_bytes())?;
        Ok(())
    }

    /// Get the file count
    pub fn file_count(&self) -> u32 {
        self.file_count
    }

    /// Get the size of the index in bytes
    pub fn size_bytes(&self) -> usize {
        8 + self.data.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_indexer_new() {
        let indexer = CodebaseIndexer::new();
        assert!(!indexer.use_simd);
    }

    #[test]
    fn test_index_roundtrip() {
        let index = CodebaseIndex {
            version: 1,
            file_count: 42,
            data: vec![1, 2, 3, 4],
        };

        let bytes = index.to_bytes();
        let loaded = CodebaseIndex::from_bytes(&bytes).unwrap();

        assert_eq!(loaded.version, 1);
        assert_eq!(loaded.file_count, 42);
    }
}
