//! dx-pkg-store: Content-addressed package storage with memory-mapped index
//!
//! This crate implements a content-addressed storage system for DXP packages.
//! Packages are stored by their content hash for deduplication, with a
//! memory-mapped index for O(1) lookups.

use dx_pkg_core::{error::Error, hash::ContentHash, Result};
use dx_pkg_format::DxpPackage;
use memmap2::MmapMut;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Store structure on disk:
/// ```text
/// ~/.dx-pkg/store/
/// ├── index.bin       # Memory-mapped hash table
/// ├── packages/       # Content-addressed packages
/// │   ├── ab/cd/abcd1234...dxp
/// │   └── ef/gh/efgh5678...dxp
/// └── cache/          # Hot package cache
const INDEX_HEADER_SIZE: usize = 64;
const INDEX_ENTRY_SIZE: usize = 40; // hash(16) + offset(8) + size(8) + flags(8)
const INITIAL_INDEX_SIZE: usize = 1024; // Start with 1K entries
#[allow(dead_code)]
const MAX_INDEX_SIZE: usize = 1024 * 1024; // Max 1M entries

/// Store index header (properly aligned)
#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct IndexHeader {
    magic: [u8; 8], // "DXSTORE\0"
    version: u32,
    entry_count: u32,
    capacity: u32,
    reserved: [u8; 44],
}

/// Store index entry
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
struct IndexEntry {
    hash: u128,  // Content hash
    offset: u64, // Offset in packages directory (encoded path)
    size: u64,   // Package size in bytes
    flags: u64,  // Reserved for future use
}

/// Content-addressed package store
pub struct DxpStore {
    #[allow(dead_code)]
    root: PathBuf,
    index_path: PathBuf,
    packages_path: PathBuf,
    #[allow(dead_code)]
    cache_path: PathBuf,
    index: Arc<RwLock<StoreIndex>>,
    lru_cache: Arc<RwLock<LruCache>>,
}

struct StoreIndex {
    entries: HashMap<u128, IndexEntry>,
}

struct LruCache {
    packages: HashMap<u128, Arc<DxpPackage>>,
    access_order: Vec<u128>,
    max_size: usize,
}

impl DxpStore {
    /// Open or create a store at the given path
    pub fn open(root: impl AsRef<Path>) -> Result<Self> {
        let root = root.as_ref().to_path_buf();
        let index_path = root.join("index.bin");
        let packages_path = root.join("packages");
        let cache_path = root.join("cache");

        // Create directories
        fs::create_dir_all(&packages_path)?;
        fs::create_dir_all(&cache_path)?;

        // Load or create index
        let index = if index_path.exists() {
            Self::load_index(&index_path)?
        } else {
            Self::create_index(&index_path)?
        };

        Ok(Self {
            root,
            index_path,
            packages_path,
            cache_path,
            index: Arc::new(RwLock::new(index)),
            lru_cache: Arc::new(RwLock::new(LruCache::new(100))),
        })
    }

    /// Get package by content hash (O(1) lookup)
    pub fn get(&self, hash: ContentHash) -> Result<Arc<DxpPackage>> {
        let hash_u128 = hash;

        // Check LRU cache first
        {
            let mut cache = self.lru_cache.write();
            if let Some(pkg) = cache.get(hash_u128) {
                return Ok(pkg);
            }
        }

        // Lookup in index
        let index = self.index.read();
        let _entry = index
            .entries
            .get(&hash_u128)
            .ok_or_else(|| Error::package_not_found(format!("{:032x}", hash_u128)))?;

        // Construct package path from hash
        let path = self.package_path(hash_u128);

        // Load package
        let package = DxpPackage::open(&path)?;
        let package = Arc::new(package);

        // Add to cache
        let mut cache = self.lru_cache.write();
        cache.put(hash_u128, Arc::clone(&package));

        Ok(package)
    }

    /// Store package with deduplication
    pub fn put(&self, package_data: &[u8]) -> Result<ContentHash> {
        // Calculate content hash
        let hash_u128 = dx_pkg_core::hash::xxhash128(package_data);

        // Check if already exists
        {
            let index = self.index.read();
            if index.entries.contains_key(&hash_u128) {
                return Ok(hash_u128); // Already stored
            }
        }

        // Write package to disk
        let path = self.package_path(hash_u128);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let mut file = File::create(&path)?;
        file.write_all(package_data)?;
        file.sync_all()?;

        // Update index
        let entry = IndexEntry {
            hash: hash_u128,
            offset: 0, // Encoded in path
            size: package_data.len() as u64,
            flags: 0,
        };

        let mut index = self.index.write();
        index.entries.insert(hash_u128, entry);
        Self::write_index(&self.index_path, &index)?;

        Ok(hash_u128)
    }

    /// Verify package integrity
    pub fn verify(&self, hash: ContentHash) -> Result<bool> {
        let hash_u128 = hash;
        let path = self.package_path(hash_u128);

        if !path.exists() {
            return Ok(false);
        }

        // Read and hash file
        let data = fs::read(&path)?;
        let computed_hash = dx_pkg_core::hash::xxhash128(&data);

        Ok(computed_hash == hash_u128)
    }

    /// Garbage collect unused packages
    pub fn gc(&self, keep_hashes: &[ContentHash]) -> Result<usize> {
        let keep_set: std::collections::HashSet<u128> = keep_hashes.iter().copied().collect();

        let mut removed = 0;
        let mut index = self.index.write();

        let to_remove: Vec<u128> =
            index.entries.keys().filter(|h| !keep_set.contains(h)).copied().collect();

        for hash in to_remove {
            let path = self.package_path(hash);
            if path.exists() {
                fs::remove_file(path)?;
                removed += 1;
            }
            index.entries.remove(&hash);
        }

        Self::write_index(&self.index_path, &index)?;
        Ok(removed)
    }

    /// List all stored packages
    pub fn list(&self) -> Result<Vec<ContentHash>> {
        let index = self.index.read();
        Ok(index.entries.keys().copied().collect())
    }

    /// Get store statistics
    pub fn stats(&self) -> StoreStats {
        let index = self.index.read();
        let total_size: u64 = index.entries.values().map(|e| e.size).sum();
        let cache_size = self.lru_cache.read().packages.len();

        StoreStats {
            package_count: index.entries.len(),
            total_size,
            cache_hit_count: cache_size,
        }
    }

    // Internal helpers

    fn package_path(&self, hash: u128) -> PathBuf {
        let hex = format!("{:032x}", hash);
        let dir1 = &hex[0..2];
        let dir2 = &hex[2..4];
        self.packages_path.join(dir1).join(dir2).join(format!("{}.dxp", hex))
    }

    fn load_index(path: &Path) -> Result<StoreIndex> {
        let file = File::open(path)?;
        let mmap = unsafe { MmapMut::map_mut(&file)? };

        // Parse header
        if mmap.len() < INDEX_HEADER_SIZE {
            return Err(Error::CorruptedData);
        }

        let header = unsafe { &*(mmap.as_ptr() as *const IndexHeader) };
        if &header.magic != b"DXSTORE\0" {
            return Err(Error::InvalidMagic {
                expected: *b"DXST",
                found: [
                    header.magic[0],
                    header.magic[1],
                    header.magic[2],
                    header.magic[3],
                ],
            });
        }

        // Parse entries
        let mut entries = HashMap::new();
        let entry_count = header.entry_count as usize;
        let entries_offset = INDEX_HEADER_SIZE;

        for i in 0..entry_count {
            let offset = entries_offset + i * INDEX_ENTRY_SIZE;
            if offset + INDEX_ENTRY_SIZE > mmap.len() {
                break;
            }

            // Use manual byte copy to avoid alignment issues
            let mut entry = IndexEntry::default();
            unsafe {
                let entry_bytes = std::slice::from_raw_parts_mut(
                    &mut entry as *mut IndexEntry as *mut u8,
                    INDEX_ENTRY_SIZE,
                );
                entry_bytes.copy_from_slice(&mmap[offset..offset + INDEX_ENTRY_SIZE]);
            }

            if entry.hash != 0 {
                entries.insert(entry.hash, entry);
            }
        }

        Ok(StoreIndex { entries })
    }

    fn create_index(path: &Path) -> Result<StoreIndex> {
        let size = INDEX_HEADER_SIZE + INITIAL_INDEX_SIZE * INDEX_ENTRY_SIZE;
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .open(path)?;

        file.set_len(size as u64)?;
        let mut mmap = unsafe { MmapMut::map_mut(&file)? };

        // Write header
        let header = IndexHeader {
            magic: *b"DXSTORE\0",
            version: 1,
            entry_count: 0,
            capacity: INITIAL_INDEX_SIZE as u32,
            reserved: [0; 44],
        };

        unsafe {
            let header_ptr = mmap.as_mut_ptr() as *mut IndexHeader;
            *header_ptr = header;
        }

        mmap.flush()?;

        Ok(StoreIndex {
            entries: HashMap::new(),
        })
    }

    fn write_index(path: &Path, index: &StoreIndex) -> Result<()> {
        let entry_count = index.entries.len();
        let capacity = entry_count.max(INITIAL_INDEX_SIZE).next_power_of_two();
        let size = INDEX_HEADER_SIZE + capacity * INDEX_ENTRY_SIZE;

        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .open(path)?;

        file.set_len(size as u64)?;
        let mut mmap = unsafe { MmapMut::map_mut(&file)? };

        // Write header
        let header = IndexHeader {
            magic: *b"DXSTORE\0",
            version: 1,
            entry_count: entry_count as u32,
            capacity: capacity as u32,
            reserved: [0; 44],
        };

        unsafe {
            let header_ptr = mmap.as_mut_ptr() as *mut IndexHeader;
            *header_ptr = header;
        }

        // Write entries
        let entries_offset = INDEX_HEADER_SIZE;
        for (i, entry) in index.entries.values().enumerate() {
            let offset = entries_offset + i * INDEX_ENTRY_SIZE;
            // Use manual byte copy to avoid alignment issues
            let bytes = unsafe {
                std::slice::from_raw_parts(
                    entry as *const IndexEntry as *const u8,
                    INDEX_ENTRY_SIZE,
                )
            };
            mmap[offset..offset + INDEX_ENTRY_SIZE].copy_from_slice(bytes);
        }

        mmap.flush()?;
        Ok(())
    }
}

impl LruCache {
    fn new(max_size: usize) -> Self {
        Self {
            packages: HashMap::new(),
            access_order: Vec::new(),
            max_size,
        }
    }

    fn get(&mut self, hash: u128) -> Option<Arc<DxpPackage>> {
        if let Some(pkg) = self.packages.get(&hash) {
            // Move to end (most recently used)
            if let Some(pos) = self.access_order.iter().position(|&h| h == hash) {
                self.access_order.remove(pos);
            }
            self.access_order.push(hash);
            Some(Arc::clone(pkg))
        } else {
            None
        }
    }

    fn put(&mut self, hash: u128, package: Arc<DxpPackage>) {
        // Evict if full
        if self.packages.len() >= self.max_size && !self.packages.contains_key(&hash) {
            if let Some(oldest) = self.access_order.first().copied() {
                self.access_order.remove(0);
                self.packages.remove(&oldest);
            }
        }

        self.packages.insert(hash, package);
        if let Some(pos) = self.access_order.iter().position(|&h| h == hash) {
            self.access_order.remove(pos);
        }
        self.access_order.push(hash);
    }
}

/// Store statistics
#[derive(Debug, Clone)]
pub struct StoreStats {
    pub package_count: usize,
    pub total_size: u64,
    pub cache_hit_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_store_create() -> Result<()> {
        let temp = TempDir::new()?;
        let store = DxpStore::open(temp.path())?;

        assert!(store.index_path.exists());
        assert!(store.packages_path.exists());
        assert!(store.cache_path.exists());

        Ok(())
    }

    #[test]
    fn test_store_put_get() -> Result<()> {
        let temp = TempDir::new()?;
        let store = DxpStore::open(temp.path())?;

        // Create test package data
        let data = b"test package data";

        // Store package
        let hash = store.put(data)?;

        // Verify it exists
        assert!(store.verify(hash)?);

        // List packages
        let packages = store.list()?;
        assert_eq!(packages.len(), 1);
        assert_eq!(packages[0], hash);

        Ok(())
    }

    #[test]
    fn test_store_deduplication() -> Result<()> {
        let temp = TempDir::new()?;
        let store = DxpStore::open(temp.path())?;

        let data = b"duplicate data";

        let hash1 = store.put(data)?;
        let hash2 = store.put(data)?;

        assert_eq!(hash1, hash2);

        let packages = store.list()?;
        assert_eq!(packages.len(), 1);

        Ok(())
    }

    #[test]
    fn test_store_gc() -> Result<()> {
        let temp = TempDir::new()?;
        let store = DxpStore::open(temp.path())?;

        let hash1 = store.put(b"package1")?;
        let hash2 = store.put(b"package2")?;
        let _hash3 = store.put(b"package3")?;

        // Keep only hash1 and hash2
        let removed = store.gc(&[hash1, hash2])?;
        assert_eq!(removed, 1);

        let packages = store.list()?;
        assert_eq!(packages.len(), 2);

        Ok(())
    }

    #[test]
    fn test_store_stats() -> Result<()> {
        let temp = TempDir::new()?;
        let store = DxpStore::open(temp.path())?;

        store.put(b"test1")?;
        store.put(b"test2")?;

        let stats = store.stats();
        assert_eq!(stats.package_count, 2);
        assert!(stats.total_size > 0);

        Ok(())
    }
}
