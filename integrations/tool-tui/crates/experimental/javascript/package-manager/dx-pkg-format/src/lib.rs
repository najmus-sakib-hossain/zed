//! dx-pkg-format: DXP binary package format implementation
//!
//! Provides zero-copy memory-mapped access to DXP packages with O(1) file lookups.

use dx_pkg_core::{xxhash128, xxhash64, DxpHeader, Error, Result, DXP_MAGIC};
use memmap2::Mmap;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

mod compression;
mod index;

use compression::{
    choose_compression, compress_lz4, compress_zstd, decompress, COMPRESSION_LZ4, COMPRESSION_NONE,
    COMPRESSION_ZSTD,
};
use index::FileIndex;
pub use index::FileIndexEntry;

/// DXP Package (memory-mapped)
pub struct DxpPackage {
    mmap: Mmap,
    header: DxpHeader,
    index: FileIndex,
}

impl DxpPackage {
    /// Open a DXP package file (zero-copy memory mapping)
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let file = File::open(path)?;
        let mmap = unsafe { Mmap::map(&file)? };

        // Verify magic number
        if &mmap[0..4] != DXP_MAGIC {
            return Err(Error::InvalidMagic {
                expected: *DXP_MAGIC,
                found: [mmap[0], mmap[1], mmap[2], mmap[3]],
            });
        }

        // Read header (zero-copy cast)
        let header = *bytemuck::from_bytes::<DxpHeader>(&mmap[0..128]);

        // Verify content hash
        let content_end = header.total_size as usize;
        if content_end > mmap.len() {
            return Err(Error::CorruptedData);
        }

        let computed_hash = xxhash128(&mmap[128..content_end]);
        if computed_hash != header.content_hash {
            return Err(Error::CorruptedData);
        }

        // Load file index
        let index = FileIndex::from_mmap(&mmap, &header)?;

        Ok(Self {
            mmap,
            header,
            index,
        })
    }

    /// Calculate the offset where file data section starts
    fn data_section_offset(&self) -> usize {
        let entry_size = std::mem::size_of::<FileIndexEntry>();
        let table_size = (self.header.file_count * 2).next_power_of_two().max(4);
        let index_size = table_size as usize * entry_size;
        self.header.index_offset as usize + index_size
    }

    /// Get file content by path (zero-copy or decompressed)
    pub fn get_file(&self, path: &str) -> Result<Vec<u8>> {
        let path_hash = xxhash64(path.as_bytes());
        let entry = self.index.find(path_hash)?;

        // entry.offset is relative to file data section, add data section offset
        let data_offset = self.data_section_offset();
        let start = data_offset + entry.offset as usize;
        let end = start + entry.compressed_size as usize;
        let data = &self.mmap[start..end];

        if entry.compressed_size > 0 && entry.compressed_size != entry.size {
            // Decompress
            decompress(data, entry.size as usize, entry.flags)
        } else {
            // Uncompressed, return copy
            Ok(data.to_vec())
        }
    }

    /// List all files in package (returns actual file paths from metadata)
    pub fn list_files(&self) -> Vec<String> {
        // Try to get file paths from metadata first
        if let Ok(metadata) = self.get_metadata() {
            if let Some(files) = metadata.get("files").and_then(|f| f.as_array()) {
                return files.iter().filter_map(|f| f.as_str().map(|s| s.to_string())).collect();
            }
        }

        // Fallback to hash-based names if metadata doesn't have file paths
        self.index.list()
    }

    /// Get package metadata
    pub fn header(&self) -> &DxpHeader {
        &self.header
    }

    /// Get file count
    pub fn file_count(&self) -> u32 {
        self.header.file_count
    }

    /// Get package dependencies
    /// Returns a list of (name_hash, version_hash) pairs
    /// Note: Original names are not stored, only hashes for fast lookup
    pub fn get_dependencies(&self) -> Vec<(u64, u64)> {
        let deps_offset = self.header.deps_offset as usize;
        let deps_count = self.header.deps_count as usize;

        if deps_count == 0 || deps_offset >= self.mmap.len() {
            return Vec::new();
        }

        let mut deps = Vec::with_capacity(deps_count);
        let mut offset = deps_offset;

        for _ in 0..deps_count {
            if offset + 16 > self.mmap.len() {
                break;
            }
            let name_hash =
                u64::from_le_bytes(self.mmap[offset..offset + 8].try_into().unwrap_or([0; 8]));
            let version_hash =
                u64::from_le_bytes(self.mmap[offset + 8..offset + 16].try_into().unwrap_or([0; 8]));
            deps.push((name_hash, version_hash));
            offset += 16;
        }

        deps
    }

    /// Get package metadata as JSON
    pub fn get_metadata(&self) -> Result<serde_json::Value> {
        let metadata_offset = self.header.metadata_offset as usize;
        let metadata_size = self.header.metadata_size as usize;

        if metadata_size == 0 || metadata_offset >= self.mmap.len() {
            return Ok(serde_json::json!({}));
        }

        let end = std::cmp::min(metadata_offset + metadata_size, self.mmap.len());
        let metadata_bytes = &self.mmap[metadata_offset..end];

        serde_json::from_slice(metadata_bytes)
            .map_err(|e| Error::Compression(format!("Invalid metadata JSON: {}", e)))
    }

    /// Get package name from metadata
    pub fn get_name(&self) -> Option<String> {
        self.get_metadata()
            .ok()
            .and_then(|m| m.get("name").and_then(|n| n.as_str()).map(|s| s.to_string()))
    }

    /// Get package version from metadata
    pub fn get_version(&self) -> Option<String> {
        self.get_metadata()
            .ok()
            .and_then(|m| m.get("version").and_then(|v| v.as_str()).map(|s| s.to_string()))
    }

    /// Get dependencies with names from metadata
    pub fn get_dependencies_with_names(&self) -> Vec<(String, String)> {
        self.get_metadata()
            .ok()
            .and_then(|m| m.get("dependencies").and_then(|d| d.as_array()).cloned())
            .map(|deps| {
                deps.iter()
                    .filter_map(|d| {
                        let name = d.get("name").and_then(|n| n.as_str())?;
                        let version = d.get("version").and_then(|v| v.as_str())?;
                        Some((name.to_string(), version.to_string()))
                    })
                    .collect()
            })
            .unwrap_or_default()
    }
}

/// Builder for creating DXP packages
pub struct DxpBuilder {
    name: String,
    version: String,
    files: HashMap<String, Vec<u8>>,
    dependencies: Vec<(String, String)>, // (name, version)
    metadata: Option<Vec<u8>>,
    file_paths: Vec<String>, // Store actual file paths for listing
}

impl DxpBuilder {
    pub fn new(name: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            version: version.into(),
            files: HashMap::new(),
            dependencies: Vec::new(),
            metadata: None,
            file_paths: Vec::new(),
        }
    }

    pub fn add_file(&mut self, path: impl Into<String>, content: Vec<u8>) {
        let path_str = path.into();
        self.file_paths.push(path_str.clone());
        self.files.insert(path_str, content);
    }

    pub fn add_dependency(&mut self, name: impl Into<String>, version: impl Into<String>) {
        self.dependencies.push((name.into(), version.into()));
    }

    pub fn set_metadata(&mut self, metadata: Vec<u8>) {
        self.metadata = Some(metadata);
    }

    pub fn build<P: AsRef<Path>>(self, output: P) -> Result<()> {
        let file = File::create(output)?;
        let mut writer = BufWriter::new(file);

        // Calculate table size for hash table (next power of 2, 2x file count)
        let file_count = self.files.len() as u32;
        let table_size = (file_count * 2).next_power_of_two().max(4); // Minimum 4 entries
        let entry_size = std::mem::size_of::<FileIndexEntry>();

        // Prepare compressed file data and index entries
        let mut file_data: Vec<u8> = Vec::new();
        let mut index_entries: Vec<FileIndexEntry> =
            vec![FileIndexEntry::default(); table_size as usize];

        for (path, content) in &self.files {
            let path_hash = xxhash64(path.as_bytes());

            // Choose compression based on file size
            let compression_type = choose_compression(content.len());
            let (compressed_data, flags) = match compression_type {
                COMPRESSION_LZ4 => {
                    let compressed = compress_lz4(content)?;
                    // Only use compression if it actually saves space
                    if compressed.len() < content.len() {
                        (compressed, COMPRESSION_LZ4)
                    } else {
                        (content.clone(), COMPRESSION_NONE)
                    }
                }
                COMPRESSION_ZSTD => {
                    let compressed = compress_zstd(content, 3)?;
                    if compressed.len() < content.len() {
                        (compressed, COMPRESSION_ZSTD)
                    } else {
                        (content.clone(), COMPRESSION_NONE)
                    }
                }
                _ => (content.clone(), COMPRESSION_NONE),
            };

            // Create index entry
            let entry = FileIndexEntry {
                path_hash,
                offset: file_data.len() as u64,
                size: content.len() as u32,
                compressed_size: compressed_data.len() as u32,
                flags,
                _reserved: [0; 3],
            };

            // Insert into hash table using quadratic probing
            let mut idx = (path_hash % table_size as u64) as usize;
            let mut probes = 0;
            loop {
                if index_entries[idx].path_hash == 0 {
                    index_entries[idx] = entry;
                    break;
                }
                probes += 1;
                idx = (path_hash as usize + probes * probes) % table_size as usize;
                if probes > table_size as usize {
                    return Err(Error::Compression("Hash table full".into()));
                }
            }

            // Append compressed data
            file_data.extend_from_slice(&compressed_data);
        }

        // Prepare metadata section (JSON with package info including file paths)
        let metadata = self.metadata.unwrap_or_else(|| {
            let meta_json = serde_json::json!({
                "name": self.name,
                "version": self.version,
                "files": self.file_paths,
                "dependencies": self.dependencies.iter()
                    .map(|(n, v)| serde_json::json!({"name": n, "version": v}))
                    .collect::<Vec<_>>()
            });
            serde_json::to_vec(&meta_json).unwrap_or_default()
        });

        // Calculate offsets
        let header_size = 128u64;
        let metadata_offset = header_size;
        let metadata_size = metadata.len() as u32;
        let deps_offset = metadata_offset + metadata_size as u64;
        let deps_size = self.dependencies.len() * 16; // 8 bytes name hash + 8 bytes version
        let index_offset = deps_offset + deps_size as u64;
        let index_size = table_size as u64 * entry_size as u64;
        let data_offset = index_offset + index_size;
        let total_size = data_offset + file_data.len() as u64;

        // Calculate content hash (everything after header)
        let mut content_for_hash: Vec<u8> = Vec::new();
        content_for_hash.extend_from_slice(&metadata);
        // Add deps
        for (name, version) in &self.dependencies {
            content_for_hash.extend_from_slice(&xxhash64(name.as_bytes()).to_le_bytes());
            content_for_hash.extend_from_slice(&xxhash64(version.as_bytes()).to_le_bytes());
        }
        // Add index
        for entry in &index_entries {
            content_for_hash.extend_from_slice(bytemuck::bytes_of(entry));
        }
        // Add file data
        content_for_hash.extend_from_slice(&file_data);
        let content_hash = xxhash128(&content_for_hash);

        // Create header
        let header = DxpHeader {
            magic: *DXP_MAGIC,
            version: 1,
            flags: 0,
            name_hash: xxhash64(self.name.as_bytes()),
            version_num: encode_version_string(&self.version),
            total_size,
            index_offset,
            file_count,
            metadata_size,
            metadata_offset,
            deps_offset,
            deps_count: self.dependencies.len() as u16,
            _pad: [0; 6],
            content_hash,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
            reserved: [0; 32],
        };

        // Write header
        writer.write_all(bytemuck::bytes_of(&header))?;

        // Write metadata
        writer.write_all(&metadata)?;

        // Write dependencies
        for (name, version) in &self.dependencies {
            writer.write_all(&xxhash64(name.as_bytes()).to_le_bytes())?;
            writer.write_all(&xxhash64(version.as_bytes()).to_le_bytes())?;
        }

        // Write index
        for entry in &index_entries {
            writer.write_all(bytemuck::bytes_of(entry))?;
        }

        // Write file data
        writer.write_all(&file_data)?;

        writer.flush()?;
        Ok(())
    }
}

/// Encode version string to u64 (simplified: hash for now)
fn encode_version_string(version: &str) -> u64 {
    // Try to parse as semver, otherwise hash
    let parts: Vec<&str> = version.split('.').collect();
    if parts.len() >= 3 {
        if let (Ok(major), Ok(minor), Ok(patch)) = (
            parts[0].parse::<u16>(),
            parts[1].parse::<u16>(),
            parts[2].split('-').next().unwrap_or("0").parse::<u32>(),
        ) {
            return ((major as u64) << 48) | ((minor as u64) << 32) | (patch as u64);
        }
    }
    xxhash64(version.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_builder() {
        let mut builder = DxpBuilder::new("test-package", "1.0.0");
        builder.add_file("index.js", b"console.log('hello');".to_vec());
        assert_eq!(builder.files.len(), 1);
    }

    #[test]
    fn test_build_and_read() {
        let temp = tempdir().unwrap();
        let output_path = temp.path().join("test.dxp");

        // Build package
        let mut builder = DxpBuilder::new("test-package", "1.0.0");
        builder.add_file("index.js", b"console.log('hello');".to_vec());
        builder.add_file("package.json", b"{\"name\":\"test\"}".to_vec());
        builder.add_dependency("lodash", "4.17.21");
        builder.build(&output_path).unwrap();

        // Read package
        let pkg = DxpPackage::open(&output_path).unwrap();
        assert_eq!(pkg.file_count(), 2);

        // Get file content
        let content = pkg.get_file("index.js").unwrap();
        assert_eq!(content, b"console.log('hello');");
    }

    #[test]
    fn test_encode_version() {
        let v = encode_version_string("1.2.3");
        assert_eq!(v >> 48, 1);
        assert_eq!((v >> 32) & 0xFFFF, 2);
        assert_eq!(v & 0xFFFFFFFF, 3);
    }
}
