//! DXL-Workspace Lockfile format
//!
//! O(1) dependency resolution with CRDT merge support.

use crate::error::LockfileError;
use crate::{DXL_MAGIC, FORMAT_VERSION};
use bytemuck::{Pod, Zeroable};

/// DXL-Workspace lockfile header
#[repr(C, packed)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct DxlHeader {
    /// Magic bytes: "DXLW"
    pub magic: [u8; 4],
    /// Format version
    pub version: u32,
    /// Number of resolved packages
    pub package_count: u32,
    /// Offset to package index (for O(1) lookup)
    pub index_offset: u64,
    /// Offset to package entries
    pub entries_offset: u64,
    /// Offset to peer dependency conflict matrix
    pub conflicts_offset: u64,
    /// Offset to hoisting strategy
    pub hoisting_offset: u64,
    /// CRDT vector clock for merge support
    pub vector_clock: [u64; 8],
    /// Blake3 hash of content
    pub content_hash: [u8; 32],
}

impl DxlHeader {
    /// Size of header in bytes
    pub const SIZE: usize = std::mem::size_of::<Self>();

    /// Create a new header
    pub fn new(package_count: u32) -> Self {
        Self {
            magic: DXL_MAGIC,
            version: FORMAT_VERSION,
            package_count,
            index_offset: Self::SIZE as u64,
            entries_offset: 0,
            conflicts_offset: 0,
            hoisting_offset: 0,
            vector_clock: [0; 8],
            content_hash: [0; 32],
        }
    }

    /// Validate magic bytes
    pub fn validate_magic(&self) -> Result<(), LockfileError> {
        if self.magic != DXL_MAGIC {
            return Err(LockfileError::InvalidMagic { found: self.magic });
        }
        Ok(())
    }

    /// Increment vector clock for a node
    pub fn increment_clock(&mut self, node_id: usize) {
        if node_id < 8 {
            self.vector_clock[node_id] += 1;
        }
    }

    /// Merge vector clocks (take max of each component)
    pub fn merge_clocks(&mut self, other: &[u64; 8]) {
        for (i, &other_val) in other.iter().enumerate() {
            self.vector_clock[i] = self.vector_clock[i].max(other_val);
        }
    }

    /// Check if this clock is concurrent with another
    pub fn is_concurrent(&self, other: &[u64; 8]) -> bool {
        let mut less = false;
        let mut greater = false;

        for (i, &other_val) in other.iter().enumerate() {
            if self.vector_clock[i] < other_val {
                less = true;
            }
            if self.vector_clock[i] > other_val {
                greater = true;
            }
        }

        less && greater
    }
}

/// Resolved package entry
#[repr(C, packed)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct ResolvedPackage {
    /// Index into string table for name
    pub name_idx: u32,
    /// Version packed as (major << 20) | (minor << 10) | patch
    pub version_packed: u32,
    /// Integrity hash (SHA-512 truncated)
    pub integrity_hash: [u8; 32],
    /// Index into string table for tarball URL
    pub tarball_url_idx: u32,
    /// Offset to dependency list
    pub dependencies_offset: u32,
    /// Number of dependencies
    pub dependencies_count: u16,
    /// Flags
    pub flags: u16,
}

impl ResolvedPackage {
    /// Size in bytes
    pub const SIZE: usize = std::mem::size_of::<Self>();

    /// Unpack version
    pub fn version(&self) -> (u16, u16, u16) {
        let major = (self.version_packed >> 20) as u16;
        let minor = ((self.version_packed >> 10) & 0x3FF) as u16;
        let patch = (self.version_packed & 0x3FF) as u16;
        (major, minor, patch)
    }
}

/// Peer dependency conflict
#[derive(Debug, Clone)]
pub struct PeerConflict {
    /// Package with the conflict
    pub package: String,
    /// Required peer
    pub peer: String,
    /// Required version range
    pub required: String,
    /// Actual resolved version
    pub resolved: String,
}

/// Hoisting strategy
#[derive(Debug, Clone, Default)]
pub struct HoistingStrategy {
    /// Packages to hoist to root
    pub hoisted: Vec<String>,
    /// Packages that cannot be hoisted (conflicts)
    pub nested: Vec<(String, String)>, // (parent, package)
}

/// Lockfile data for serialization
#[derive(Debug, Clone, PartialEq)]
pub struct LockfileData {
    /// Resolved packages
    pub packages: Vec<PackageResolution>,
    /// Vector clock
    pub vector_clock: [u64; 8],
}

/// Package resolution data
#[derive(Debug, Clone, PartialEq)]
pub struct PackageResolution {
    /// Package name
    pub name: String,
    /// Resolved version
    pub version: (u16, u16, u16),
    /// Integrity hash
    pub integrity: [u8; 32],
    /// Tarball URL
    pub tarball_url: String,
    /// Dependencies (name, version requirement)
    pub dependencies: Vec<(String, String)>,
}

impl LockfileData {
    /// Create empty lockfile
    pub fn new() -> Self {
        Self {
            packages: Vec::new(),
            vector_clock: [0; 8],
        }
    }

    /// Merge with another lockfile using CRDT semantics
    pub fn merge(&mut self, other: &LockfileData) -> Result<(), LockfileError> {
        // Merge vector clocks
        for i in 0..8 {
            self.vector_clock[i] = self.vector_clock[i].max(other.vector_clock[i]);
        }

        // Merge packages (last-write-wins based on vector clock)
        use std::collections::HashMap;
        let mut package_map: HashMap<String, &PackageResolution> = HashMap::new();

        for pkg in &self.packages {
            package_map.insert(pkg.name.clone(), pkg);
        }

        for pkg in &other.packages {
            // Simple LWW merge - in practice would use more sophisticated CRDT
            package_map.insert(pkg.name.clone(), pkg);
        }

        self.packages = package_map.values().map(|&p| p.clone()).collect();

        Ok(())
    }
}

impl Default for LockfileData {
    fn default() -> Self {
        Self::new()
    }
}

/// DXL Serializer
pub struct DxlSerializer;

impl DxlSerializer {
    /// Serialize lockfile to DXL format
    pub fn serialize(data: &LockfileData) -> Result<Vec<u8>, LockfileError> {
        let mut buffer = Vec::new();

        // Build string table
        let (string_table, string_indices) = Self::build_string_table(data);

        // Calculate offsets
        let index_offset = DxlHeader::SIZE as u64;
        let index_size = data.packages.len() * 16; // (hash: u64, offset: u64) pairs
        let entries_offset = index_offset + index_size as u64;
        let entries_size = data.packages.len() * ResolvedPackage::SIZE;
        let _strings_offset = entries_offset + entries_size as u64;

        // Create header
        let mut header = DxlHeader::new(data.packages.len() as u32);
        header.index_offset = index_offset;
        header.entries_offset = entries_offset;
        header.vector_clock = data.vector_clock;

        // Write header
        buffer.extend_from_slice(bytemuck::bytes_of(&header));

        // Write index (name hash -> entry offset)
        for (i, pkg) in data.packages.iter().enumerate() {
            let name_hash = xxhash_rust::xxh3::xxh3_64(pkg.name.as_bytes());
            let entry_offset = entries_offset + (i * ResolvedPackage::SIZE) as u64;
            buffer.extend_from_slice(&name_hash.to_le_bytes());
            buffer.extend_from_slice(&entry_offset.to_le_bytes());
        }

        // Write package entries
        for pkg in &data.packages {
            let version_packed = ((pkg.version.0 as u32) << 20)
                | ((pkg.version.1 as u32 & 0x3FF) << 10)
                | (pkg.version.2 as u32 & 0x3FF);

            let entry = ResolvedPackage {
                name_idx: string_indices[&pkg.name] as u32,
                version_packed,
                integrity_hash: pkg.integrity,
                tarball_url_idx: string_indices[&pkg.tarball_url] as u32,
                dependencies_offset: 0,
                dependencies_count: pkg.dependencies.len() as u16,
                flags: 0,
            };
            buffer.extend_from_slice(bytemuck::bytes_of(&entry));
        }

        // Write string table
        buffer.extend_from_slice(&string_table);

        // Compute content hash
        // content_hash offset in packed DxlHeader: 4 + 4 + 4 + 8 + 8 + 8 + 8 + 64 = 108
        let content_hash = blake3::hash(&buffer[DxlHeader::SIZE..]);
        buffer[108..140].copy_from_slice(content_hash.as_bytes());

        Ok(buffer)
    }

    /// Deserialize DXL format
    pub fn deserialize(data: &[u8]) -> Result<LockfileData, LockfileError> {
        if data.len() < DxlHeader::SIZE {
            return Err(LockfileError::Corrupted {
                reason: "data too small for header".to_string(),
            });
        }

        let header: &DxlHeader = bytemuck::from_bytes(&data[..DxlHeader::SIZE]);
        header.validate_magic()?;

        // Read string table (at end of file)
        let entries_end =
            header.entries_offset as usize + header.package_count as usize * ResolvedPackage::SIZE;
        let string_table = Self::parse_string_table(&data[entries_end..]);

        // Read package entries
        let mut packages = Vec::with_capacity(header.package_count as usize);

        for i in 0..header.package_count as usize {
            let offset = header.entries_offset as usize + i * ResolvedPackage::SIZE;
            let entry: &ResolvedPackage =
                bytemuck::from_bytes(&data[offset..offset + ResolvedPackage::SIZE]);

            packages.push(PackageResolution {
                name: string_table.get(entry.name_idx as usize).cloned().unwrap_or_default(),
                version: entry.version(),
                integrity: entry.integrity_hash,
                tarball_url: string_table
                    .get(entry.tarball_url_idx as usize)
                    .cloned()
                    .unwrap_or_default(),
                dependencies: Vec::new(),
            });
        }

        Ok(LockfileData {
            packages,
            vector_clock: header.vector_clock,
        })
    }

    fn build_string_table(
        data: &LockfileData,
    ) -> (Vec<u8>, std::collections::HashMap<String, usize>) {
        use std::collections::HashMap;

        let mut table = Vec::new();
        let mut indices = HashMap::new();
        let mut string_index = 0usize;

        for pkg in &data.packages {
            for s in [&pkg.name, &pkg.tarball_url] {
                if !indices.contains_key(s) {
                    indices.insert(s.clone(), string_index);
                    table.extend_from_slice(s.as_bytes());
                    table.push(0);
                    string_index += 1;
                }
            }
        }

        (table, indices)
    }

    fn parse_string_table(data: &[u8]) -> Vec<String> {
        let mut strings = Vec::new();
        let mut start = 0;

        for (i, &byte) in data.iter().enumerate() {
            if byte == 0 {
                if let Ok(s) = std::str::from_utf8(&data[start..i]) {
                    strings.push(s.to_string());
                }
                start = i + 1;
            }
        }

        strings
    }
}

/// Lockfile Resolver for O(1) package resolution
pub struct LockfileResolver {
    /// Loaded lockfile data
    data: Option<LockfileData>,
    /// Package name to index lookup (for O(1) resolution)
    name_index: rustc_hash::FxHashMap<String, usize>,
    /// Workspace package versions (for workspace:* resolution)
    workspace_versions: rustc_hash::FxHashMap<String, (u16, u16, u16)>,
}

impl LockfileResolver {
    /// Create a new lockfile resolver
    pub fn new() -> Self {
        Self {
            data: None,
            name_index: rustc_hash::FxHashMap::default(),
            workspace_versions: rustc_hash::FxHashMap::default(),
        }
    }

    /// Load lockfile from serialized bytes
    pub fn load_from_bytes(&mut self, bytes: &[u8]) -> Result<(), LockfileError> {
        let data = DxlSerializer::deserialize(bytes)?;
        self.build_name_index(&data);
        self.data = Some(data);
        Ok(())
    }

    /// Build name-to-index lookup table for O(1) resolution
    fn build_name_index(&mut self, data: &LockfileData) {
        self.name_index.clear();
        for (idx, pkg) in data.packages.iter().enumerate() {
            self.name_index.insert(pkg.name.clone(), idx);
        }
    }

    /// Register workspace package version (for workspace:* resolution)
    pub fn register_workspace_package(&mut self, name: &str, version: (u16, u16, u16)) {
        self.workspace_versions.insert(name.to_string(), version);
    }

    /// Resolve package with O(1) lookup
    pub fn resolve(&self, name: &str) -> Option<&PackageResolution> {
        let idx = *self.name_index.get(name)?;
        self.data.as_ref()?.packages.get(idx)
    }

    /// Resolve package by name and version requirement
    /// For simplicity, this returns the first match (real impl would check version)
    pub fn resolve_with_version(
        &self,
        name: &str,
        _version_req: &str,
    ) -> Option<&PackageResolution> {
        self.resolve(name)
    }

    /// Get pre-resolved workspace protocol reference
    pub fn resolve_workspace(&self, name: &str) -> Option<(u16, u16, u16)> {
        self.workspace_versions.get(name).copied()
    }

    /// Get all resolved packages
    pub fn packages(&self) -> &[PackageResolution] {
        self.data.as_ref().map(|d| d.packages.as_slice()).unwrap_or(&[])
    }

    /// Get package count
    pub fn package_count(&self) -> usize {
        self.data.as_ref().map(|d| d.packages.len()).unwrap_or(0)
    }

    /// Check if lockfile is loaded
    pub fn is_loaded(&self) -> bool {
        self.data.is_some()
    }
}

impl Default for LockfileResolver {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dxl_header_size() {
        // Packed struct size: 4 + 4 + 4 + 8 + 8 + 8 + 8 + 64 + 32 = 140 bytes
        assert_eq!(DxlHeader::SIZE, 140);
    }

    #[test]
    fn test_vector_clock_merge() {
        let mut header = DxlHeader::new(0);
        header.vector_clock = [1, 2, 3, 0, 0, 0, 0, 0];

        let other = [0, 5, 1, 4, 0, 0, 0, 0];
        header.merge_clocks(&other);

        // Copy to avoid unaligned access on packed struct
        let clock = header.vector_clock;
        assert_eq!(clock, [1, 5, 3, 4, 0, 0, 0, 0]);
    }

    #[test]
    fn test_concurrent_detection() {
        let mut header = DxlHeader::new(0);
        header.vector_clock = [1, 2, 0, 0, 0, 0, 0, 0];

        // Concurrent: one is ahead in some components, behind in others
        let concurrent = [0, 3, 0, 0, 0, 0, 0, 0];
        assert!(header.is_concurrent(&concurrent));

        // Not concurrent: one dominates
        let dominated = [0, 1, 0, 0, 0, 0, 0, 0];
        assert!(!header.is_concurrent(&dominated));
    }

    #[test]
    fn test_lockfile_merge() {
        let mut lockfile1 = LockfileData {
            packages: vec![PackageResolution {
                name: "pkg-a".to_string(),
                version: (1, 0, 0),
                integrity: [0; 32],
                tarball_url: "https://example.com/a".to_string(),
                dependencies: vec![],
            }],
            vector_clock: [1, 0, 0, 0, 0, 0, 0, 0],
        };

        let lockfile2 = LockfileData {
            packages: vec![PackageResolution {
                name: "pkg-b".to_string(),
                version: (2, 0, 0),
                integrity: [1; 32],
                tarball_url: "https://example.com/b".to_string(),
                dependencies: vec![],
            }],
            vector_clock: [0, 1, 0, 0, 0, 0, 0, 0],
        };

        lockfile1.merge(&lockfile2).unwrap();

        assert_eq!(lockfile1.packages.len(), 2);
        assert_eq!(lockfile1.vector_clock, [1, 1, 0, 0, 0, 0, 0, 0]);
    }

    #[test]
    fn test_lockfile_resolver_o1_lookup() {
        let data = LockfileData {
            packages: vec![
                PackageResolution {
                    name: "lodash".to_string(),
                    version: (4, 17, 21),
                    integrity: [1; 32],
                    tarball_url: "https://registry.npmjs.org/lodash".to_string(),
                    dependencies: vec![],
                },
                PackageResolution {
                    name: "react".to_string(),
                    version: (18, 2, 0),
                    integrity: [2; 32],
                    tarball_url: "https://registry.npmjs.org/react".to_string(),
                    dependencies: vec![],
                },
                PackageResolution {
                    name: "typescript".to_string(),
                    version: (5, 0, 0),
                    integrity: [3; 32],
                    tarball_url: "https://registry.npmjs.org/typescript".to_string(),
                    dependencies: vec![],
                },
            ],
            vector_clock: [1, 0, 0, 0, 0, 0, 0, 0],
        };

        let bytes = DxlSerializer::serialize(&data).unwrap();
        let mut resolver = LockfileResolver::new();
        resolver.load_from_bytes(&bytes).unwrap();

        // O(1) lookup by name
        let lodash = resolver.resolve("lodash").unwrap();
        assert_eq!(lodash.version, (4, 17, 21));

        let react = resolver.resolve("react").unwrap();
        assert_eq!(react.version, (18, 2, 0));

        let ts = resolver.resolve("typescript").unwrap();
        assert_eq!(ts.version, (5, 0, 0));

        // Non-existent package
        assert!(resolver.resolve("nonexistent").is_none());
    }

    #[test]
    fn test_lockfile_resolver_workspace_protocol() {
        let mut resolver = LockfileResolver::new();

        // Register workspace packages
        resolver.register_workspace_package("@myorg/core", (1, 0, 0));
        resolver.register_workspace_package("@myorg/utils", (1, 2, 3));

        // Resolve workspace:* references
        let core_version = resolver.resolve_workspace("@myorg/core").unwrap();
        assert_eq!(core_version, (1, 0, 0));

        let utils_version = resolver.resolve_workspace("@myorg/utils").unwrap();
        assert_eq!(utils_version, (1, 2, 3));

        // Non-existent workspace package
        assert!(resolver.resolve_workspace("@myorg/nonexistent").is_none());
    }
}
