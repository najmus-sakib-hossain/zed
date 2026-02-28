//! Binary Workspace Manifest (BWM) format
//!
//! Memory-mapped workspace structure with pre-computed dependency graphs.

use crate::error::WorkspaceError;
use crate::types::PackageEntry;
use crate::{BWM_MAGIC, FORMAT_VERSION};
use bytemuck::{Pod, Zeroable};

/// Binary Workspace Manifest header
#[repr(C, packed)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct BwmHeader {
    /// Magic bytes for format identification: "DXWM"
    pub magic: [u8; 4],
    /// Format version for compatibility
    pub version: u32,
    /// Total number of packages in workspace
    pub package_count: u32,
    /// Offset to package metadata table
    pub packages_offset: u64,
    /// Offset to dependency graph
    pub graph_offset: u64,
    /// Offset to topological order array
    pub topo_order_offset: u64,
    /// Offset to string table
    pub strings_offset: u64,
    /// Blake3 hash of content for integrity verification
    pub content_hash: [u8; 32],
}

impl BwmHeader {
    /// Size of header in bytes
    pub const SIZE: usize = std::mem::size_of::<Self>();

    /// Create a new header with default values
    pub fn new(package_count: u32) -> Self {
        Self {
            magic: BWM_MAGIC,
            version: FORMAT_VERSION,
            package_count,
            packages_offset: Self::SIZE as u64,
            graph_offset: 0,
            topo_order_offset: 0,
            strings_offset: 0,
            content_hash: [0; 32],
        }
    }

    /// Validate magic bytes
    pub fn validate_magic(&self) -> Result<(), WorkspaceError> {
        if self.magic != BWM_MAGIC {
            return Err(WorkspaceError::InvalidMagic { found: self.magic });
        }
        Ok(())
    }

    /// Validate version
    pub fn validate_version(&self) -> Result<(), WorkspaceError> {
        if self.version != FORMAT_VERSION {
            return Err(WorkspaceError::VersionMismatch {
                expected: FORMAT_VERSION,
                found: self.version,
            });
        }
        Ok(())
    }
}

/// BWM Serializer for creating and reading Binary Workspace Manifests
pub struct BwmSerializer;

impl BwmSerializer {
    /// Serialize a workspace configuration to BWM format
    pub fn serialize(config: &WorkspaceData) -> Result<Vec<u8>, WorkspaceError> {
        let mut buffer = Vec::new();

        // Calculate offsets
        let header_size = BwmHeader::SIZE;
        let packages_size = config.packages.len() * PackageEntry::SIZE;
        let graph_size = config.dependency_edges.len() * 8; // u32 pairs
        let topo_size = config.packages.len() * 4; // u32 indices

        let packages_offset = header_size as u64;
        let graph_offset = packages_offset + packages_size as u64;
        let topo_order_offset = graph_offset + graph_size as u64;
        let strings_offset = topo_order_offset + topo_size as u64;

        // Build string table
        let (string_table, string_indices) = Self::build_string_table(config);

        // Create header
        let mut header = BwmHeader::new(config.packages.len() as u32);
        header.packages_offset = packages_offset;
        header.graph_offset = graph_offset;
        header.topo_order_offset = topo_order_offset;
        header.strings_offset = strings_offset;

        // Write header
        buffer.extend_from_slice(bytemuck::bytes_of(&header));

        // Write package entries
        for pkg in config.packages.iter() {
            let entry = PackageEntry::new(
                string_indices[&pkg.name] as u32,
                string_indices[&pkg.path] as u32,
                pkg.version,
                0, // deps_offset filled later
                pkg.dependencies.len() as u16,
            );
            buffer.extend_from_slice(bytemuck::bytes_of(&entry));
        }

        // Write dependency edges
        for (from, to) in &config.dependency_edges {
            buffer.extend_from_slice(&from.to_le_bytes());
            buffer.extend_from_slice(&to.to_le_bytes());
        }

        // Write topological order
        for idx in &config.topological_order {
            buffer.extend_from_slice(&idx.to_le_bytes());
        }

        // Write string table
        buffer.extend_from_slice(&string_table);

        // Compute content hash
        // content_hash offset in packed BwmHeader: 4 + 4 + 4 + 8 + 8 + 8 + 8 = 44
        let content_hash = blake3::hash(&buffer[BwmHeader::SIZE..]);
        buffer[44..76].copy_from_slice(content_hash.as_bytes());

        Ok(buffer)
    }

    /// Deserialize BWM format to workspace data
    pub fn deserialize(data: &[u8]) -> Result<WorkspaceData, WorkspaceError> {
        if data.len() < BwmHeader::SIZE {
            return Err(WorkspaceError::ManifestCorrupted {
                reason: "data too small for header".to_string(),
                hash_mismatch: false,
            });
        }

        let header: &BwmHeader = bytemuck::from_bytes(&data[..BwmHeader::SIZE]);
        header.validate_magic()?;
        header.validate_version()?;

        // Verify content hash
        let computed_hash = blake3::hash(&data[BwmHeader::SIZE..]);
        if computed_hash.as_bytes() != &header.content_hash {
            return Err(WorkspaceError::ManifestCorrupted {
                reason: "content hash mismatch".to_string(),
                hash_mismatch: true,
            });
        }

        // Read string table
        let strings_start = header.strings_offset as usize;
        let string_table = Self::parse_string_table(&data[strings_start..]);

        // Read package entries
        let packages_start = header.packages_offset as usize;
        let mut packages = Vec::with_capacity(header.package_count as usize);

        for i in 0..header.package_count as usize {
            let offset = packages_start + i * PackageEntry::SIZE;
            let entry: &PackageEntry =
                bytemuck::from_bytes(&data[offset..offset + PackageEntry::SIZE]);

            packages.push(PackageData {
                name: string_table[entry.name_idx as usize].clone(),
                path: string_table[entry.path_idx as usize].clone(),
                version: entry.version(),
                dependencies: Vec::new(), // Filled from graph
                is_private: entry.is_private(),
            });
        }

        // Read dependency edges
        let graph_start = header.graph_offset as usize;
        let topo_start = header.topo_order_offset as usize;
        let edge_count = (topo_start - graph_start) / 8;
        let mut dependency_edges = Vec::with_capacity(edge_count);

        for i in 0..edge_count {
            let offset = graph_start + i * 8;
            let from = u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap());
            let to = u32::from_le_bytes(data[offset + 4..offset + 8].try_into().unwrap());
            dependency_edges.push((from, to));
        }

        // Read topological order
        let topo_count = header.package_count as usize;
        let mut topological_order = Vec::with_capacity(topo_count);

        for i in 0..topo_count {
            let offset = topo_start + i * 4;
            let idx = u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap());
            topological_order.push(idx);
        }

        Ok(WorkspaceData {
            packages,
            dependency_edges,
            topological_order,
        })
    }

    fn build_string_table(
        config: &WorkspaceData,
    ) -> (Vec<u8>, std::collections::HashMap<String, usize>) {
        use std::collections::HashMap;

        let mut table = Vec::new();
        let mut indices = HashMap::new();
        let mut string_index = 0usize;

        for pkg in &config.packages {
            if !indices.contains_key(&pkg.name) {
                indices.insert(pkg.name.clone(), string_index);
                table.extend_from_slice(pkg.name.as_bytes());
                table.push(0); // null terminator
                string_index += 1;
            }
            if !indices.contains_key(&pkg.path) {
                indices.insert(pkg.path.clone(), string_index);
                table.extend_from_slice(pkg.path.as_bytes());
                table.push(0);
                string_index += 1;
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

/// Workspace data for serialization/deserialization
#[derive(Debug, Clone, PartialEq)]
pub struct WorkspaceData {
    /// Package metadata
    pub packages: Vec<PackageData>,
    /// Dependency edges as (from_idx, to_idx) pairs
    pub dependency_edges: Vec<(u32, u32)>,
    /// Pre-computed topological order
    pub topological_order: Vec<u32>,
}

/// Package data for serialization
#[derive(Debug, Clone, PartialEq)]
pub struct PackageData {
    /// Package name
    pub name: String,
    /// Package path relative to workspace root
    pub path: String,
    /// Version as (major, minor, patch)
    pub version: (u16, u16, u16),
    /// Dependency package names
    pub dependencies: Vec<String>,
    /// Whether package is private
    pub is_private: bool,
}

impl WorkspaceData {
    /// Create empty workspace data
    pub fn new() -> Self {
        Self {
            packages: Vec::new(),
            dependency_edges: Vec::new(),
            topological_order: Vec::new(),
        }
    }

    /// Compute topological order from dependency edges
    pub fn compute_topological_order(&mut self) -> Result<(), WorkspaceError> {
        let n = self.packages.len();
        let mut in_degree = vec![0u32; n];
        let mut adj: Vec<Vec<u32>> = vec![Vec::new(); n];

        for &(from, to) in &self.dependency_edges {
            adj[from as usize].push(to);
            in_degree[to as usize] += 1;
        }

        let mut queue: Vec<u32> = (0..n as u32).filter(|&i| in_degree[i as usize] == 0).collect();

        let mut order = Vec::with_capacity(n);

        while let Some(node) = queue.pop() {
            order.push(node);
            for &neighbor in &adj[node as usize] {
                in_degree[neighbor as usize] -= 1;
                if in_degree[neighbor as usize] == 0 {
                    queue.push(neighbor);
                }
            }
        }

        if order.len() != n {
            // Find cycle for error message
            let in_cycle: Vec<String> = (0..n)
                .filter(|&i| in_degree[i] > 0)
                .map(|i| self.packages[i].name.clone())
                .collect();
            return Err(WorkspaceError::CyclicDependency { cycle: in_cycle });
        }

        self.topological_order = order;
        Ok(())
    }
}

impl Default for WorkspaceData {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bwm_header_size() {
        // Packed struct size: 4 + 4 + 4 + 8 + 8 + 8 + 8 + 32 = 76 bytes
        assert_eq!(BwmHeader::SIZE, 76);
    }

    #[test]
    fn test_bwm_header_validation() {
        let header = BwmHeader::new(10);
        assert!(header.validate_magic().is_ok());
        assert!(header.validate_version().is_ok());

        let mut bad_header = header;
        bad_header.magic = *b"XXXX";
        assert!(bad_header.validate_magic().is_err());
    }

    #[test]
    fn test_topological_sort() {
        let mut data = WorkspaceData {
            packages: vec![
                PackageData {
                    name: "a".to_string(),
                    path: "packages/a".to_string(),
                    version: (1, 0, 0),
                    dependencies: vec![],
                    is_private: false,
                },
                PackageData {
                    name: "b".to_string(),
                    path: "packages/b".to_string(),
                    version: (1, 0, 0),
                    dependencies: vec!["a".to_string()],
                    is_private: false,
                },
                PackageData {
                    name: "c".to_string(),
                    path: "packages/c".to_string(),
                    version: (1, 0, 0),
                    dependencies: vec!["b".to_string()],
                    is_private: false,
                },
            ],
            dependency_edges: vec![(0, 1), (1, 2)], // a -> b -> c
            topological_order: vec![],
        };

        data.compute_topological_order().unwrap();

        // a should come before b, b before c
        let pos_a = data.topological_order.iter().position(|&x| x == 0).unwrap();
        let pos_b = data.topological_order.iter().position(|&x| x == 1).unwrap();
        let pos_c = data.topological_order.iter().position(|&x| x == 2).unwrap();

        assert!(pos_a < pos_b);
        assert!(pos_b < pos_c);
    }

    #[test]
    fn test_cyclic_dependency_detection() {
        let mut data = WorkspaceData {
            packages: vec![
                PackageData {
                    name: "a".to_string(),
                    path: "packages/a".to_string(),
                    version: (1, 0, 0),
                    dependencies: vec![],
                    is_private: false,
                },
                PackageData {
                    name: "b".to_string(),
                    path: "packages/b".to_string(),
                    version: (1, 0, 0),
                    dependencies: vec![],
                    is_private: false,
                },
            ],
            dependency_edges: vec![(0, 1), (1, 0)], // a -> b -> a (cycle)
            topological_order: vec![],
        };

        let result = data.compute_topological_order();
        assert!(matches!(result, Err(WorkspaceError::CyclicDependency { .. })));
    }
}
