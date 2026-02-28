//! Persistent cache using memory-mapped files

use dx_bundle_core::{ContentHash, ModuleId};
use memmap2::Mmap;
use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};

/// Cache file magic bytes
const CACHE_MAGIC: &[u8; 4] = b"DXPC";
/// Cache version
const CACHE_VERSION: u32 = 1;

/// Cache entry header size (module_id: 8 + hash: 16 + offset: 8 + size: 4 = 36 bytes)
const ENTRY_HEADER_SIZE: usize = 36;

/// Memory-mapped persistent cache with binary search
pub struct PersistentCache {
    /// Cache directory (kept for potential future use)
    #[allow(dead_code)]
    cache_dir: PathBuf,
    /// Index file path
    index_file: PathBuf,
    /// Data file path
    data_file: PathBuf,
    /// Memory-mapped index
    index_mmap: Option<Mmap>,
    /// Memory-mapped data
    data_mmap: Option<Mmap>,
    /// In-memory index for fast lookup (module_id -> (hash, offset, size))
    index: HashMap<ModuleId, (ContentHash, u64, u32)>,
    /// Pending writes (not yet flushed)
    pending: HashMap<ModuleId, (ContentHash, Vec<u8>)>,
}

impl PersistentCache {
    /// Open persistent cache
    pub fn open(cache_dir: &Path) -> std::io::Result<Self> {
        std::fs::create_dir_all(cache_dir)?;

        let index_file = cache_dir.join("cache-index.bin");
        let data_file = cache_dir.join("cache-data.bin");

        let mut cache = Self {
            cache_dir: cache_dir.to_path_buf(),
            index_file,
            data_file,
            index_mmap: None,
            data_mmap: None,
            index: HashMap::new(),
            pending: HashMap::new(),
        };

        // Load existing cache
        cache.load()?;

        Ok(cache)
    }

    /// Load cache from disk
    fn load(&mut self) -> std::io::Result<()> {
        if !self.index_file.exists() {
            return Ok(());
        }

        // Memory-map index file
        let index_file = File::open(&self.index_file)?;
        let index_mmap = unsafe { Mmap::map(&index_file)? };

        // Validate header
        if index_mmap.len() < 8 {
            return Ok(()); // Empty or invalid
        }

        if &index_mmap[0..4] != CACHE_MAGIC {
            // Invalid magic, rebuild cache
            self.invalidate()?;
            return Ok(());
        }

        let version = u32::from_le_bytes(index_mmap[4..8].try_into().unwrap());
        if version != CACHE_VERSION {
            // Version mismatch, rebuild cache
            self.invalidate()?;
            return Ok(());
        }

        // Parse index entries
        let mut offset = 8;
        while offset + ENTRY_HEADER_SIZE <= index_mmap.len() {
            let module_id = u64::from_le_bytes(index_mmap[offset..offset + 8].try_into().unwrap());
            let hash_bytes: [u8; 16] = index_mmap[offset + 8..offset + 24].try_into().unwrap();
            let data_offset =
                u64::from_le_bytes(index_mmap[offset + 24..offset + 32].try_into().unwrap());
            let data_size =
                u32::from_le_bytes(index_mmap[offset + 32..offset + 36].try_into().unwrap());

            self.index
                .insert(module_id, (ContentHash::from_bytes(hash_bytes), data_offset, data_size));
            offset += ENTRY_HEADER_SIZE;
        }

        self.index_mmap = Some(index_mmap);

        // Memory-map data file
        if self.data_file.exists() {
            let data_file = File::open(&self.data_file)?;
            self.data_mmap = Some(unsafe { Mmap::map(&data_file)? });
        }

        Ok(())
    }

    /// Check if module is cached with matching hash
    pub fn has(&self, module_id: ModuleId, source_hash: &ContentHash) -> bool {
        if let Some((cached_hash, _, _)) = self.index.get(&module_id) {
            return cached_hash == source_hash;
        }
        if let Some((cached_hash, _)) = self.pending.get(&module_id) {
            return cached_hash == source_hash;
        }
        false
    }

    /// Get cached module data (if valid)
    pub fn get(&self, module_id: ModuleId, source_hash: &ContentHash) -> Option<Vec<u8>> {
        // Check pending writes first
        if let Some((cached_hash, data)) = self.pending.get(&module_id) {
            if cached_hash == source_hash {
                return Some(data.clone());
            }
        }

        // Check persisted cache
        if let Some((cached_hash, offset, size)) = self.index.get(&module_id) {
            if cached_hash != source_hash {
                return None; // Hash mismatch
            }

            if let Some(ref mmap) = self.data_mmap {
                let start = *offset as usize;
                let end = start + *size as usize;
                if end <= mmap.len() {
                    return Some(mmap[start..end].to_vec());
                }
            }
        }

        None
    }

    /// Store module in cache
    pub fn put(&mut self, module_id: ModuleId, source_hash: ContentHash, data: Vec<u8>) {
        self.pending.insert(module_id, (source_hash, data));
    }

    /// Flush pending writes to disk
    pub fn flush(&mut self) -> std::io::Result<()> {
        if self.pending.is_empty() {
            return Ok(());
        }

        // Append to data file
        let mut data_file =
            std::fs::OpenOptions::new().create(true).append(true).open(&self.data_file)?;

        let mut current_offset = data_file.metadata()?.len();

        for (module_id, (hash, data)) in self.pending.drain() {
            let size = data.len() as u32;
            data_file.write_all(&data)?;
            self.index.insert(module_id, (hash, current_offset, size));
            current_offset += data.len() as u64;
        }

        data_file.flush()?;

        // Rewrite index file
        self.write_index()?;

        // Reload mmaps
        self.reload_mmaps()?;

        Ok(())
    }

    /// Write index to disk
    fn write_index(&self) -> std::io::Result<()> {
        let mut file = File::create(&self.index_file)?;

        // Write header
        file.write_all(CACHE_MAGIC)?;
        file.write_all(&CACHE_VERSION.to_le_bytes())?;

        // Write entries (sorted by module_id for binary search)
        let mut entries: Vec<_> = self.index.iter().collect();
        entries.sort_by_key(|(id, _)| *id);

        for (module_id, (hash, offset, size)) in entries {
            file.write_all(&module_id.to_le_bytes())?;
            file.write_all(hash.as_bytes())?;
            file.write_all(&offset.to_le_bytes())?;
            file.write_all(&size.to_le_bytes())?;
        }

        file.flush()?;
        Ok(())
    }

    /// Reload memory maps after flush
    fn reload_mmaps(&mut self) -> std::io::Result<()> {
        if self.index_file.exists() {
            let file = File::open(&self.index_file)?;
            self.index_mmap = Some(unsafe { Mmap::map(&file)? });
        }

        if self.data_file.exists() {
            let file = File::open(&self.data_file)?;
            self.data_mmap = Some(unsafe { Mmap::map(&file)? });
        }

        Ok(())
    }

    /// Invalidate entire cache
    pub fn invalidate(&mut self) -> std::io::Result<()> {
        self.index_mmap = None;
        self.data_mmap = None;
        self.index.clear();
        self.pending.clear();

        if self.index_file.exists() {
            std::fs::remove_file(&self.index_file)?;
        }
        if self.data_file.exists() {
            std::fs::remove_file(&self.data_file)?;
        }

        Ok(())
    }

    /// Invalidate a single module
    pub fn invalidate_module(&mut self, module_id: ModuleId) {
        self.index.remove(&module_id);
        self.pending.remove(&module_id);
        // Note: Data is not removed from file, will be cleaned up on compaction
    }

    /// Get cache statistics
    pub fn stats(&self) -> CacheStats {
        let entry_count = self.index.len() + self.pending.len();
        let data_size = self.data_mmap.as_ref().map(|m| m.len()).unwrap_or(0)
            + self.pending.values().map(|(_, d)| d.len()).sum::<usize>();

        CacheStats {
            entry_count,
            data_size,
            index_size: self.index_mmap.as_ref().map(|m| m.len()).unwrap_or(0),
        }
    }
}

/// Cache statistics
#[derive(Clone, Debug, Default)]
pub struct CacheStats {
    /// Number of cached entries
    pub entry_count: usize,
    /// Total data size in bytes
    pub data_size: usize,
    /// Index size in bytes
    pub index_size: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_persistent_cache() {
        let temp_dir = std::env::temp_dir().join("dx-persistent-test");
        let _ = std::fs::remove_dir_all(&temp_dir);

        let mut cache = PersistentCache::open(&temp_dir).unwrap();

        let module_id = 123u64;
        let hash = ContentHash::xxh3(b"test content");
        let data = b"transformed content".to_vec();

        // Store
        cache.put(module_id, hash, data.clone());

        // Should be in pending
        assert!(cache.has(module_id, &hash));

        // Flush
        cache.flush().unwrap();

        // Should still be accessible
        let retrieved = cache.get(module_id, &hash).unwrap();
        assert_eq!(retrieved, data);

        // Reload cache
        let cache2 = PersistentCache::open(&temp_dir).unwrap();
        let retrieved2 = cache2.get(module_id, &hash).unwrap();
        assert_eq!(retrieved2, data);

        // Clean up
        std::fs::remove_dir_all(&temp_dir).ok();
    }

    #[test]
    fn test_cache_invalidation() {
        let temp_dir = std::env::temp_dir().join("dx-persistent-invalidate-test");
        let _ = std::fs::remove_dir_all(&temp_dir);

        let mut cache = PersistentCache::open(&temp_dir).unwrap();

        let module_id = 456u64;
        let hash = ContentHash::xxh3(b"test");
        cache.put(module_id, hash, b"data".to_vec());
        cache.flush().unwrap();

        // Invalidate
        cache.invalidate_module(module_id);
        assert!(!cache.has(module_id, &hash));

        // Clean up
        std::fs::remove_dir_all(&temp_dir).ok();
    }
}

/// Property-based tests for cache
/// **Feature: production-readiness, Property 18: Cache Round-Trip**
/// **Validates: Requirements 12.1, 12.2, 12.3**
#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Property 18: Cache Round-Trip
        /// For any module_id, hash, and data, storing then retrieving should return the same data
        #[test]
        fn prop_cache_round_trip(
            module_id in any::<u64>(),
            data in prop::collection::vec(any::<u8>(), 0..1000)
        ) {
            let temp_dir = tempfile::tempdir().unwrap();
            let mut cache = PersistentCache::open(temp_dir.path()).unwrap();

            let hash = ContentHash::xxh3(&data);

            // Store
            cache.put(module_id, hash, data.clone());

            // Retrieve from pending
            let retrieved = cache.get(module_id, &hash);
            prop_assert_eq!(retrieved, Some(data.clone()));

            // Flush and retrieve from disk
            cache.flush().unwrap();
            let retrieved_after_flush = cache.get(module_id, &hash);
            prop_assert_eq!(retrieved_after_flush, Some(data.clone()));

            // Reload and retrieve
            let cache2 = PersistentCache::open(temp_dir.path()).unwrap();
            let retrieved_after_reload = cache2.get(module_id, &hash);
            prop_assert_eq!(retrieved_after_reload, Some(data));
        }

        /// Property: Hash mismatch returns None
        /// For any module_id and data, retrieving with wrong hash should return None
        #[test]
        fn prop_hash_mismatch_returns_none(
            module_id in any::<u64>(),
            data in prop::collection::vec(any::<u8>(), 1..100)
        ) {
            let temp_dir = tempfile::tempdir().unwrap();
            let mut cache = PersistentCache::open(temp_dir.path()).unwrap();

            let correct_hash = ContentHash::xxh3(&data);
            let wrong_hash = ContentHash::xxh3(b"wrong data");

            cache.put(module_id, correct_hash, data);
            cache.flush().unwrap();

            // Should not find with wrong hash
            let retrieved = cache.get(module_id, &wrong_hash);
            prop_assert!(retrieved.is_none());
        }

        /// Property 19: Cache Invalidation on Change
        /// For any module_id and data, after invalidation the cache should not contain the entry
        /// **Validates: Requirements 12.5**
        #[test]
        fn prop_cache_invalidation(
            module_id in any::<u64>(),
            data in prop::collection::vec(any::<u8>(), 1..100)
        ) {
            let temp_dir = tempfile::tempdir().unwrap();
            let mut cache = PersistentCache::open(temp_dir.path()).unwrap();

            let hash = ContentHash::xxh3(&data);

            // Store and flush
            cache.put(module_id, hash, data.clone());
            cache.flush().unwrap();

            // Verify it's there
            prop_assert!(cache.has(module_id, &hash));

            // Invalidate
            cache.invalidate_module(module_id);

            // Should no longer be accessible
            prop_assert!(!cache.has(module_id, &hash));
            prop_assert!(cache.get(module_id, &hash).is_none());
        }

        /// Property: Multiple entries don't interfere
        /// Storing multiple entries should not affect each other
        #[test]
        fn prop_multiple_entries_independent(
            entries in prop::collection::vec(
                (any::<u64>(), prop::collection::vec(any::<u8>(), 1..50)),
                1..10
            )
        ) {
            let temp_dir = tempfile::tempdir().unwrap();
            let mut cache = PersistentCache::open(temp_dir.path()).unwrap();

            // Store all entries
            let mut stored: Vec<(u64, ContentHash, Vec<u8>)> = Vec::new();
            for (module_id, data) in entries {
                let hash = ContentHash::xxh3(&data);
                cache.put(module_id, hash, data.clone());
                stored.push((module_id, hash, data));
            }

            cache.flush().unwrap();

            // Verify all entries are retrievable
            for (module_id, hash, expected_data) in stored {
                let retrieved = cache.get(module_id, &hash);
                prop_assert_eq!(retrieved, Some(expected_data));
            }
        }
    }
}
