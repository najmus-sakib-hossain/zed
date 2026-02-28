//! Memory-Mapped Rule Files
//!
//! Zero-copy loading via mmap for instant rule access.

use crate::{DrivenError, Result};
use memmap2::Mmap;
use std::fs::File;
use std::path::Path;

/// Memory-mapped rule file
#[derive(Debug)]
pub struct MappedRule {
    /// Memory mapping
    mmap: Mmap,
    /// File path (for debugging)
    path: std::path::PathBuf,
}

impl MappedRule {
    /// Open a rule file with memory mapping
    pub fn open(path: &Path) -> Result<Self> {
        let file = File::open(path).map_err(|e| {
            DrivenError::Io(std::io::Error::new(
                e.kind(),
                format!("Failed to open {}: {}", path.display(), e),
            ))
        })?;

        // Safety: The file is opened read-only and we maintain the mapping
        let mmap = unsafe { Mmap::map(&file) }.map_err(|e| {
            DrivenError::Io(std::io::Error::other(format!(
                "Failed to mmap {}: {}",
                path.display(),
                e
            )))
        })?;

        Ok(Self {
            mmap,
            path: path.to_path_buf(),
        })
    }

    /// Get the raw bytes (zero-copy)
    pub fn as_bytes(&self) -> &[u8] {
        &self.mmap
    }

    /// Get file size
    pub fn len(&self) -> usize {
        self.mmap.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.mmap.is_empty()
    }

    /// Get the file path
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Advise the kernel about access patterns
    pub fn advise_sequential(&self) -> Result<()> {
        #[cfg(unix)]
        {
            self.mmap.advise(memmap2::Advice::Sequential).map_err(|e| {
                DrivenError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))
            })?;
        }
        Ok(())
    }

    /// Advise the kernel to keep pages in memory
    pub fn advise_willneed(&self) -> Result<()> {
        #[cfg(unix)]
        {
            self.mmap.advise(memmap2::Advice::WillNeed).map_err(|e| {
                DrivenError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))
            })?;
        }
        Ok(())
    }
}

/// Rule mapping utilities
#[derive(Debug, Default)]
pub struct RuleMapping;

impl RuleMapping {
    /// Check if a path can be memory-mapped
    pub fn can_map(path: &Path) -> bool {
        path.is_file() && path.metadata().map(|m| m.len() > 0).unwrap_or(false)
    }

    /// Get recommended mapping for file size
    pub fn strategy(size: u64) -> MappingStrategy {
        if size < 4096 {
            MappingStrategy::ReadAll // Small files: just read
        } else if size < 1024 * 1024 {
            MappingStrategy::MapPrivate // Medium: private mapping
        } else {
            MappingStrategy::MapShared // Large: shared mapping
        }
    }
}

/// Memory mapping strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MappingStrategy {
    /// Read entire file into memory
    ReadAll,
    /// Private memory mapping (copy-on-write)
    MapPrivate,
    /// Shared memory mapping
    MapShared,
}

/// Pre-fetched rule data (for when mmap isn't suitable)
#[derive(Debug)]
pub struct PreloadedRule {
    /// Rule data
    data: Vec<u8>,
    /// Original path
    path: std::path::PathBuf,
}

impl PreloadedRule {
    /// Load a rule file completely into memory
    pub fn load(path: &Path) -> Result<Self> {
        let data = std::fs::read(path)?;
        Ok(Self {
            data,
            path: path.to_path_buf(),
        })
    }

    /// Get the raw bytes
    pub fn as_bytes(&self) -> &[u8] {
        &self.data
    }

    /// Get file size
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Get the file path
    pub fn path(&self) -> &Path {
        &self.path
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mapping_strategy() {
        assert_eq!(RuleMapping::strategy(100), MappingStrategy::ReadAll);
        assert_eq!(RuleMapping::strategy(10_000), MappingStrategy::MapPrivate);
        assert_eq!(RuleMapping::strategy(10_000_000), MappingStrategy::MapShared);
    }
}
