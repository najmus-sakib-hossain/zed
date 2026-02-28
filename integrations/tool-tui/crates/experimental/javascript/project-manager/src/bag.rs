//! Binary Affected Graph (BAG) format
//!
//! Pre-computed change propagation paths for instant impact detection.

use crate::error::WorkspaceError;
use crate::{BAG_MAGIC, FORMAT_VERSION};
use bytemuck::{bytes_of, cast_slice, Pod, Zeroable};

/// Binary Affected Graph header
#[repr(C, packed)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct BagHeader {
    /// Magic bytes: "DXAG"
    pub magic: [u8; 4],
    /// Format version
    pub version: u32,
    /// Number of packages
    pub package_count: u32,
    /// Offset to inverse dependency index
    pub inverse_deps_offset: u64,
    /// Offset to transitive closure cache
    pub transitive_offset: u64,
    /// Offset to file-to-package mapping
    pub file_map_offset: u64,
    /// Blake3 hash of content
    pub content_hash: [u8; 32],
}

impl BagHeader {
    /// Size of header in bytes
    pub const SIZE: usize = std::mem::size_of::<Self>();

    /// Create a new header
    pub fn new(package_count: u32) -> Self {
        Self {
            magic: BAG_MAGIC,
            version: FORMAT_VERSION,
            package_count,
            inverse_deps_offset: Self::SIZE as u64,
            transitive_offset: 0,
            file_map_offset: 0,
            content_hash: [0; 32],
        }
    }
}

/// Inverse dependency entry
#[repr(C, packed)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct InverseDepsEntry {
    /// Package index
    pub package_idx: u32,
    /// Offset to list of dependents
    pub dependents_offset: u32,
    /// Number of direct dependents
    pub dependents_count: u16,
    /// Padding
    _padding: u16,
}

/// File-to-package mapping entry
#[repr(C, packed)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct FileMapEntry {
    /// Hash of file path
    pub path_hash: u64,
    /// Owning package index
    pub package_idx: u32,
    /// Padding
    _padding: u32,
}

/// Affected graph data for serialization
#[derive(Debug, Clone, Default)]
pub struct AffectedGraphData {
    /// Number of packages
    pub package_count: u32,
    /// Inverse dependencies: package_idx -> list of packages that depend on it
    pub inverse_deps: Vec<Vec<u32>>,
    /// Transitive closure: package_idx -> all transitive dependents
    pub transitive_closure: Vec<Vec<u32>>,
    /// File path hash -> package index
    pub file_map: Vec<(u64, u32)>,
}

impl AffectedGraphData {
    /// Create from dependency edges
    ///
    /// Edges are (from, to) meaning "from depends on to".
    /// When package X changes, all packages that depend on X (directly or transitively) are affected.
    pub fn from_edges(package_count: u32, edges: &[(u32, u32)]) -> Self {
        let n = package_count as usize;

        // Build dependents index: dependents[i] = packages that depend on i
        // If edge is (from, to) meaning "from depends on to", then from is in dependents[to]
        let mut dependents = vec![Vec::new(); n];
        for &(from, to) in edges {
            dependents[to as usize].push(from);
        }

        // Compute transitive closure: for each package, find all packages affected when it changes
        // If X changes, all packages that depend on X (directly or transitively) are affected
        let mut transitive_closure = vec![Vec::new(); n];

        for i in 0..n {
            let mut visited = vec![false; n];
            let mut stack = dependents[i].clone();

            while let Some(pkg) = stack.pop() {
                if visited[pkg as usize] {
                    continue;
                }
                visited[pkg as usize] = true;

                // Also add packages that depend on this one
                for &dep in &dependents[pkg as usize] {
                    if !visited[dep as usize] {
                        stack.push(dep);
                    }
                }
            }

            transitive_closure[i] = (0..n as u32).filter(|&j| visited[j as usize]).collect();
        }

        Self {
            package_count,
            inverse_deps: dependents,
            transitive_closure,
            file_map: Vec::new(),
        }
    }

    /// Add file-to-package mapping
    pub fn add_file_mapping(&mut self, path: &str, package_idx: u32) {
        let path_hash = xxhash_rust::xxh3::xxh3_64(path.as_bytes());
        self.file_map.push((path_hash, package_idx));
    }

    /// Get direct dependents of a package
    pub fn dependents(&self, package_idx: u32) -> &[u32] {
        self.inverse_deps.get(package_idx as usize).map(|v| v.as_slice()).unwrap_or(&[])
    }

    /// Get all transitive dependents of a package
    pub fn transitive_dependents(&self, package_idx: u32) -> &[u32] {
        self.transitive_closure
            .get(package_idx as usize)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// Find package that owns a file
    pub fn file_to_package(&self, path: &str) -> Option<u32> {
        let path_hash = xxhash_rust::xxh3::xxh3_64(path.as_bytes());
        self.file_map.iter().find(|(h, _)| *h == path_hash).map(|(_, idx)| *idx)
    }
}

/// BAG Serializer for binary format
pub struct BagSerializer;

impl BagSerializer {
    /// Serialize affected graph data to binary format
    pub fn serialize(data: &AffectedGraphData) -> Result<Vec<u8>, WorkspaceError> {
        let mut buffer = Vec::new();

        // Reserve space for header
        let header_size = BagHeader::SIZE;
        buffer.resize(header_size, 0);

        // Write inverse dependency entries
        let inverse_deps_offset = buffer.len() as u64;
        let mut dependents_data: Vec<u8> = Vec::new();
        let mut entries: Vec<InverseDepsEntry> = Vec::new();

        for (pkg_idx, deps) in data.inverse_deps.iter().enumerate() {
            let entry = InverseDepsEntry {
                package_idx: pkg_idx as u32,
                dependents_offset: dependents_data.len() as u32,
                dependents_count: deps.len() as u16,
                _padding: 0,
            };
            entries.push(entry);

            // Write dependent indices
            for &dep in deps {
                dependents_data.extend_from_slice(&dep.to_le_bytes());
            }
        }

        // Write entries
        for entry in &entries {
            buffer.extend_from_slice(bytes_of(entry));
        }

        // Write dependents data
        let _dependents_data_offset = buffer.len();
        buffer.extend_from_slice(&dependents_data);

        // Write transitive closure
        let transitive_offset = buffer.len() as u64;
        let mut transitive_data: Vec<u8> = Vec::new();
        let mut transitive_entries: Vec<(u32, u32)> = Vec::new(); // (offset, count)

        for closure in &data.transitive_closure {
            transitive_entries.push((transitive_data.len() as u32, closure.len() as u32));
            for &pkg in closure {
                transitive_data.extend_from_slice(&pkg.to_le_bytes());
            }
        }

        // Write transitive entry table
        for (offset, count) in &transitive_entries {
            buffer.extend_from_slice(&offset.to_le_bytes());
            buffer.extend_from_slice(&count.to_le_bytes());
        }

        // Write transitive data
        buffer.extend_from_slice(&transitive_data);

        // Write file-to-package mapping
        let file_map_offset = buffer.len() as u64;
        for (path_hash, pkg_idx) in &data.file_map {
            let entry = FileMapEntry {
                path_hash: *path_hash,
                package_idx: *pkg_idx,
                _padding: 0,
            };
            buffer.extend_from_slice(bytes_of(&entry));
        }

        // Compute content hash
        let content_hash = blake3::hash(&buffer[header_size..]);

        // Write header
        let header = BagHeader {
            magic: BAG_MAGIC,
            version: FORMAT_VERSION,
            package_count: data.package_count,
            inverse_deps_offset,
            transitive_offset,
            file_map_offset,
            content_hash: *content_hash.as_bytes(),
        };

        buffer[..header_size].copy_from_slice(bytes_of(&header));

        Ok(buffer)
    }

    /// Deserialize binary format to affected graph data
    pub fn deserialize(data: &[u8]) -> Result<AffectedGraphData, WorkspaceError> {
        if data.len() < BagHeader::SIZE {
            return Err(WorkspaceError::ManifestCorrupted {
                reason: "data too small for BAG header".to_string(),
                hash_mismatch: false,
            });
        }

        // Read header
        let header: BagHeader = *bytemuck::from_bytes(&data[..BagHeader::SIZE]);

        // Verify magic
        if header.magic != BAG_MAGIC {
            return Err(WorkspaceError::ManifestCorrupted {
                reason: "invalid BAG magic bytes".to_string(),
                hash_mismatch: false,
            });
        }

        // Verify version
        let found_version = header.version;
        if found_version != FORMAT_VERSION {
            return Err(WorkspaceError::ManifestCorrupted {
                reason: format!(
                    "BAG version mismatch: expected {}, found {}",
                    FORMAT_VERSION, found_version
                ),
                hash_mismatch: false,
            });
        }

        // Verify content hash
        let content_hash = blake3::hash(&data[BagHeader::SIZE..]);
        if content_hash.as_bytes() != &header.content_hash {
            return Err(WorkspaceError::ManifestCorrupted {
                reason: "BAG content hash mismatch".to_string(),
                hash_mismatch: true,
            });
        }

        let package_count = header.package_count as usize;

        // Read inverse dependency entries
        let entry_size = std::mem::size_of::<InverseDepsEntry>();
        let entries_start = header.inverse_deps_offset as usize;
        let entries_end = entries_start + package_count * entry_size;

        let entries: &[InverseDepsEntry] = cast_slice(&data[entries_start..entries_end]);

        // Read dependents data (after entries)
        let dependents_data_start = entries_end;

        let mut inverse_deps = Vec::with_capacity(package_count);
        for entry in entries {
            let offset = dependents_data_start + entry.dependents_offset as usize;
            let count = entry.dependents_count as usize;

            let mut deps = Vec::with_capacity(count);
            for i in 0..count {
                let idx_offset = offset + i * 4;
                let dep = u32::from_le_bytes([
                    data[idx_offset],
                    data[idx_offset + 1],
                    data[idx_offset + 2],
                    data[idx_offset + 3],
                ]);
                deps.push(dep);
            }
            inverse_deps.push(deps);
        }

        // Read transitive closure
        let transitive_start = header.transitive_offset as usize;
        let transitive_entry_size = 8; // u32 offset + u32 count
        let transitive_entries_end = transitive_start + package_count * transitive_entry_size;

        let mut transitive_closure = Vec::with_capacity(package_count);
        for i in 0..package_count {
            let entry_offset = transitive_start + i * transitive_entry_size;
            let offset = u32::from_le_bytes([
                data[entry_offset],
                data[entry_offset + 1],
                data[entry_offset + 2],
                data[entry_offset + 3],
            ]) as usize;
            let count = u32::from_le_bytes([
                data[entry_offset + 4],
                data[entry_offset + 5],
                data[entry_offset + 6],
                data[entry_offset + 7],
            ]) as usize;

            let data_offset = transitive_entries_end + offset;
            let mut closure = Vec::with_capacity(count);
            for j in 0..count {
                let idx_offset = data_offset + j * 4;
                let pkg = u32::from_le_bytes([
                    data[idx_offset],
                    data[idx_offset + 1],
                    data[idx_offset + 2],
                    data[idx_offset + 3],
                ]);
                closure.push(pkg);
            }
            transitive_closure.push(closure);
        }

        // Read file-to-package mapping
        let file_map_start = header.file_map_offset as usize;
        let file_entry_size = std::mem::size_of::<FileMapEntry>();
        let remaining = data.len() - file_map_start;
        let file_count = remaining / file_entry_size;

        let mut file_map = Vec::with_capacity(file_count);
        for i in 0..file_count {
            let entry_offset = file_map_start + i * file_entry_size;
            let entry: FileMapEntry =
                *bytemuck::from_bytes(&data[entry_offset..entry_offset + file_entry_size]);
            file_map.push((entry.path_hash, entry.package_idx));
        }

        Ok(AffectedGraphData {
            package_count: header.package_count,
            inverse_deps,
            transitive_closure,
            file_map,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bag_header_size() {
        // Packed struct size: 4 + 4 + 4 + 8 + 8 + 8 + 32 = 68 bytes
        assert_eq!(BagHeader::SIZE, 68);
    }

    #[test]
    fn test_inverse_deps() {
        // a -> b -> c means: a depends on b, b depends on c
        // Edges: (from, to) means "from depends on to"
        let edges = vec![(0, 1), (1, 2)];
        let graph = AffectedGraphData::from_edges(3, &edges);

        // a depends on b, so a is in dependents(1) (packages that depend on b)
        assert!(graph.dependents(1).contains(&0));
        // b depends on c, so b is in dependents(2) (packages that depend on c)
        assert!(graph.dependents(2).contains(&1));
        // c has no packages depending on it directly except b
        // a has no packages depending on it
        assert!(graph.dependents(0).is_empty());
    }

    #[test]
    fn test_transitive_closure() {
        // a -> b -> c means: a depends on b, b depends on c
        // Edges: (from, to) means "from depends on to"
        let edges = vec![(0, 1), (1, 2)];
        let graph = AffectedGraphData::from_edges(3, &edges);

        // Changing a affects nothing (nothing depends on a)
        assert!(graph.transitive_dependents(0).is_empty());
        // Changing b affects a (a depends on b)
        assert!(graph.transitive_dependents(1).contains(&0));
        // Changing c affects both a and b (b depends on c, a depends on b)
        let c_deps = graph.transitive_dependents(2);
        assert!(c_deps.contains(&0));
        assert!(c_deps.contains(&1));
    }

    #[test]
    fn test_file_mapping() {
        let mut graph = AffectedGraphData::from_edges(3, &[]);
        graph.add_file_mapping("packages/a/src/index.ts", 0);
        graph.add_file_mapping("packages/b/src/index.ts", 1);

        assert_eq!(graph.file_to_package("packages/a/src/index.ts"), Some(0));
        assert_eq!(graph.file_to_package("packages/b/src/index.ts"), Some(1));
        assert_eq!(graph.file_to_package("packages/c/src/index.ts"), None);
    }

    #[test]
    fn test_bag_serializer_roundtrip() {
        // Create a graph with dependencies and file mappings
        let edges = vec![(0, 1), (1, 2), (0, 2)];
        let mut graph = AffectedGraphData::from_edges(3, &edges);
        graph.add_file_mapping("packages/a/src/index.ts", 0);
        graph.add_file_mapping("packages/b/src/index.ts", 1);
        graph.add_file_mapping("packages/c/src/index.ts", 2);

        // Serialize
        let bytes = BagSerializer::serialize(&graph).unwrap();

        // Deserialize
        let restored = BagSerializer::deserialize(&bytes).unwrap();

        // Verify package count
        assert_eq!(restored.package_count, graph.package_count);

        // Verify inverse deps
        for i in 0..graph.package_count as usize {
            let orig = graph.dependents(i as u32);
            let rest = restored.dependents(i as u32);
            assert_eq!(orig.len(), rest.len(), "inverse deps count mismatch for pkg {}", i);
            for dep in orig {
                assert!(rest.contains(dep), "missing dependent {} for pkg {}", dep, i);
            }
        }

        // Verify transitive closure
        for i in 0..graph.package_count as usize {
            let orig = graph.transitive_dependents(i as u32);
            let rest = restored.transitive_dependents(i as u32);
            assert_eq!(orig.len(), rest.len(), "transitive deps count mismatch for pkg {}", i);
            for dep in orig {
                assert!(rest.contains(dep), "missing transitive dependent {} for pkg {}", dep, i);
            }
        }

        // Verify file mappings
        assert_eq!(restored.file_to_package("packages/a/src/index.ts"), Some(0));
        assert_eq!(restored.file_to_package("packages/b/src/index.ts"), Some(1));
        assert_eq!(restored.file_to_package("packages/c/src/index.ts"), Some(2));
    }

    #[test]
    fn test_bag_serializer_empty_graph() {
        let graph = AffectedGraphData::from_edges(0, &[]);
        let bytes = BagSerializer::serialize(&graph).unwrap();
        let restored = BagSerializer::deserialize(&bytes).unwrap();
        assert_eq!(restored.package_count, 0);
    }

    #[test]
    fn test_bag_serializer_single_package() {
        let mut graph = AffectedGraphData::from_edges(1, &[]);
        graph.add_file_mapping("packages/solo/index.ts", 0);

        let bytes = BagSerializer::serialize(&graph).unwrap();
        let restored = BagSerializer::deserialize(&bytes).unwrap();

        assert_eq!(restored.package_count, 1);
        assert_eq!(restored.file_to_package("packages/solo/index.ts"), Some(0));
    }
}
