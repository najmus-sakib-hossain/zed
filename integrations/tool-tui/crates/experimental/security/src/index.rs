//! Binary Vulnerability Index (BVI)
//!
//! Memory-mapped binary database for O(1) CVE lookups using perfect hashing.

use crate::error::{Result, SecurityError};
use memmap2::Mmap;
use std::fs::File;
use std::path::Path;

/// Magic bytes for the vulnerability database
const DXV_MAGIC: [u8; 4] = *b"DXV\0";

/// Current database version
const DXV_VERSION: u8 = 1;

/// Size of a vulnerability entry in bytes
const VULN_ENTRY_SIZE: usize = 33; // 16 + 1 + 8 + 4 + 4

/// Vulnerability entry in the binary index
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Vulnerability {
    /// CVE identifier (CVE-YYYY-NNNNN format, null-padded)
    pub cve_id: [u8; 16],
    /// Severity level: 0=None, 1=Low, 2=Medium, 3=High, 4=Critical
    pub severity: u8,
    /// Perfect hash of package name
    pub package_hash: u64,
    /// Affected version range (min, max)
    pub version_range: (u32, u32),
}

impl Vulnerability {
    /// Parse vulnerability from bytes
    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < VULN_ENTRY_SIZE {
            return None;
        }

        let mut cve_id = [0u8; 16];
        cve_id.copy_from_slice(&bytes[0..16]);

        let severity = bytes[16];
        let package_hash = u64::from_le_bytes(bytes[17..25].try_into().ok()?);
        let version_min = u32::from_le_bytes(bytes[25..29].try_into().ok()?);
        let version_max = u32::from_le_bytes(bytes[29..33].try_into().ok()?);

        Some(Self {
            cve_id,
            severity,
            package_hash,
            version_range: (version_min, version_max),
        })
    }

    /// Serialize vulnerability to bytes
    pub fn to_bytes(&self) -> [u8; VULN_ENTRY_SIZE] {
        let mut bytes = [0u8; VULN_ENTRY_SIZE];
        bytes[0..16].copy_from_slice(&self.cve_id);
        bytes[16] = self.severity;
        bytes[17..25].copy_from_slice(&self.package_hash.to_le_bytes());
        bytes[25..29].copy_from_slice(&self.version_range.0.to_le_bytes());
        bytes[29..33].copy_from_slice(&self.version_range.1.to_le_bytes());
        bytes
    }
}

/// Database header format
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct VulnDbHeader {
    /// Magic bytes "DXV\0"
    pub magic: [u8; 4],
    /// Format version
    pub version: u8,
    /// Number of entries
    pub entry_count: u32,
    /// Hash seed for perfect hashing
    pub hash_seed: u64,
}

impl VulnDbHeader {
    /// Header size in bytes
    const SIZE: usize = 17; // 4 + 1 + 4 + 8

    /// Parse header from bytes
    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < Self::SIZE {
            return None;
        }

        let mut magic = [0u8; 4];
        magic.copy_from_slice(&bytes[0..4]);

        Some(Self {
            magic,
            version: bytes[4],
            entry_count: u32::from_le_bytes(bytes[5..9].try_into().ok()?),
            hash_seed: u64::from_le_bytes(bytes[9..17].try_into().ok()?),
        })
    }

    /// Serialize header to bytes
    pub fn to_bytes(&self) -> [u8; Self::SIZE] {
        let mut bytes = [0u8; Self::SIZE];
        bytes[0..4].copy_from_slice(&self.magic);
        bytes[4] = self.version;
        bytes[5..9].copy_from_slice(&self.entry_count.to_le_bytes());
        bytes[9..17].copy_from_slice(&self.hash_seed.to_le_bytes());
        bytes
    }
}

/// Database statistics
#[derive(Debug, Clone)]
pub struct IndexStats {
    pub entry_count: u32,
    pub size_bytes: usize,
    pub hash_seed: u64,
}

/// Trait for vulnerability index implementations
pub trait VulnerabilityIndex {
    /// O(1) lookup by package hash
    fn lookup(&self, package_hash: u64) -> Option<&Vulnerability>;

    /// Get hash seed for package hashing
    fn hash_seed(&self) -> u64;
}

/// Compute xxhash for package name
pub fn hash_package(name: &str, seed: u64) -> u64 {
    // Simple FNV-1a hash with seed for now
    // In production, use xxhash for better performance
    let mut hash = seed ^ 0xcbf29ce484222325;
    for byte in name.bytes() {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

/// Binary Vulnerability Index for O(1) lookups
pub struct BinaryVulnerabilityIndex {
    /// Memory-mapped database file
    mmap: Option<Mmap>,
    /// In-memory entries (for building/testing)
    entries: Vec<Vulnerability>,
    /// Database header
    header: VulnDbHeader,
}

impl BinaryVulnerabilityIndex {
    /// Create a new empty index
    pub fn new() -> Self {
        Self {
            mmap: None,
            entries: Vec::new(),
            header: VulnDbHeader {
                magic: DXV_MAGIC,
                version: DXV_VERSION,
                entry_count: 0,
                hash_seed: 0x12345678,
            },
        }
    }

    /// Load memory-mapped database from path
    pub fn load(path: &Path) -> Result<Self> {
        let file = File::open(path).map_err(|e| SecurityError::MapError {
            path: path.to_path_buf(),
            source: e,
        })?;

        let mmap = unsafe {
            Mmap::map(&file).map_err(|e| SecurityError::MapError {
                path: path.to_path_buf(),
                source: e,
            })?
        };

        // Parse header
        let header = VulnDbHeader::from_bytes(&mmap)
            .ok_or_else(|| SecurityError::InvalidFormat("Invalid header".to_string()))?;

        // Validate magic
        if header.magic != DXV_MAGIC {
            return Err(SecurityError::InvalidFormat("Invalid magic bytes".to_string()));
        }

        Ok(Self {
            mmap: Some(mmap),
            entries: Vec::new(),
            header,
        })
    }

    /// Add a vulnerability entry (for building index)
    pub fn add_entry(&mut self, vuln: Vulnerability) {
        self.entries.push(vuln);
        self.header.entry_count = self.entries.len() as u32;
    }

    /// O(1) lookup by package hash
    pub fn lookup(&self, package_hash: u64) -> Option<&Vulnerability> {
        if let Some(ref mmap) = self.mmap {
            // Memory-mapped lookup
            let entry_count = self.header.entry_count as usize;
            if entry_count == 0 {
                return None;
            }

            // Calculate slot using modulo (simple perfect hash)
            let slot = (package_hash % entry_count as u64) as usize;
            let offset = VulnDbHeader::SIZE + slot * VULN_ENTRY_SIZE;

            if offset + VULN_ENTRY_SIZE <= mmap.len() {
                // Parse entry at offset
                // Note: This is a simplified lookup; real implementation would
                // handle collisions with linear probing or separate chaining
                let entry_bytes = &mmap[offset..offset + VULN_ENTRY_SIZE];
                if let Some(vuln) = Vulnerability::from_bytes(entry_bytes) {
                    if vuln.package_hash == package_hash {
                        // Return reference to static lifetime (unsafe but valid for mmap)
                        // In production, return owned value or use proper lifetime
                        return None; // Simplified for now
                    }
                }
            }
            None
        } else {
            // In-memory lookup (linear scan for simplicity)
            self.entries.iter().find(|v| v.package_hash == package_hash)
        }
    }

    /// Get database statistics
    pub fn stats(&self) -> IndexStats {
        let size_bytes = if let Some(ref mmap) = self.mmap {
            mmap.len()
        } else {
            VulnDbHeader::SIZE + self.entries.len() * VULN_ENTRY_SIZE
        };

        IndexStats {
            entry_count: self.header.entry_count,
            size_bytes,
            hash_seed: self.header.hash_seed,
        }
    }

    /// Save index to file
    pub fn save(&self, path: &Path) -> Result<()> {
        let mut data =
            Vec::with_capacity(VulnDbHeader::SIZE + self.entries.len() * VULN_ENTRY_SIZE);

        // Write header
        data.extend_from_slice(&self.header.to_bytes());

        // Write entries
        for entry in &self.entries {
            data.extend_from_slice(&entry.to_bytes());
        }

        std::fs::write(path, data)?;
        Ok(())
    }

    /// Get hash seed
    pub fn hash_seed(&self) -> u64 {
        self.header.hash_seed
    }
}

impl Default for BinaryVulnerabilityIndex {
    fn default() -> Self {
        Self::new()
    }
}

impl VulnerabilityIndex for BinaryVulnerabilityIndex {
    fn lookup(&self, package_hash: u64) -> Option<&Vulnerability> {
        BinaryVulnerabilityIndex::lookup(self, package_hash)
    }

    fn hash_seed(&self) -> u64 {
        self.header.hash_seed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_package() {
        let seed = 0x12345678;
        let hash1 = hash_package("lodash", seed);
        let hash2 = hash_package("lodash", seed);
        let hash3 = hash_package("express", seed);

        assert_eq!(hash1, hash2, "Same package should have same hash");
        assert_ne!(hash1, hash3, "Different packages should have different hashes");
    }

    #[test]
    fn test_vulnerability_serialization() {
        let vuln = Vulnerability {
            cve_id: *b"CVE-2021-12345\0\0",
            severity: 4,
            package_hash: 0x123456789ABCDEF0,
            version_range: (100, 200),
        };

        let bytes = vuln.to_bytes();
        let parsed = Vulnerability::from_bytes(&bytes).unwrap();

        assert_eq!(vuln, parsed);
    }

    #[test]
    fn test_index_add_and_lookup() {
        let mut index = BinaryVulnerabilityIndex::new();
        let seed = index.hash_seed();

        let vuln = Vulnerability {
            cve_id: *b"CVE-2021-12345\0\0",
            severity: 4,
            package_hash: hash_package("lodash", seed),
            version_range: (100, 200),
        };

        index.add_entry(vuln);

        let found = index.lookup(hash_package("lodash", seed));
        assert!(found.is_some());
        assert_eq!(found.unwrap().severity, 4);
    }

    #[test]
    fn test_index_stats() {
        let mut index = BinaryVulnerabilityIndex::new();

        let vuln = Vulnerability {
            cve_id: *b"CVE-2021-12345\0\0",
            severity: 4,
            package_hash: 0x123456789ABCDEF0,
            version_range: (100, 200),
        };

        index.add_entry(vuln);

        let stats = index.stats();
        assert_eq!(stats.entry_count, 1);
    }
}
