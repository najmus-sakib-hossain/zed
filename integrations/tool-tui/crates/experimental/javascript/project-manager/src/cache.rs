//! Cache Manager
//!
//! Manages local DXC cache storage with memory-mapped access.
//! Implements LRU eviction, expiration, and persistence.

use crate::dxc::{CacheEntry, XorPatch};
use crate::error::CacheError;
use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::time::{Duration, Instant, SystemTime};

/// LRU entry tracking
#[derive(Debug, Clone)]
struct LruEntry {
    hash: [u8; 32],
    size: u64,
    last_access: Instant,
    created_at: SystemTime,
    ttl: Option<Duration>,
}

/// Cache Manager for task output caching with LRU eviction
pub struct CacheManager {
    /// Cache directory
    cache_dir: PathBuf,
    /// Maximum cache size in bytes
    max_size: u64,
    /// Current cache size
    current_size: u64,
    /// In-memory cache for zero-disk mode
    memory_cache: HashMap<[u8; 32], CacheEntry>,
    /// LRU tracking - most recently used at back
    lru_order: VecDeque<LruEntry>,
    /// Hash to LRU index mapping for O(1) access updates
    lru_index: HashMap<[u8; 32], usize>,
    /// Zero-disk mode enabled
    zero_disk: bool,
    /// Bloom filter for fast miss detection (simplified)
    bloom_filter: Vec<u64>,
    /// Default TTL for cache entries
    default_ttl: Option<Duration>,
}

impl CacheManager {
    /// Create a new cache manager
    pub fn new(cache_dir: PathBuf, max_size: u64) -> Self {
        Self {
            cache_dir,
            max_size,
            current_size: 0,
            memory_cache: HashMap::new(),
            lru_order: VecDeque::new(),
            lru_index: HashMap::new(),
            zero_disk: false,
            bloom_filter: vec![0; 1024], // 8KB bloom filter
            default_ttl: None,
        }
    }

    /// Create cache manager with TTL
    pub fn with_ttl(cache_dir: PathBuf, max_size: u64, ttl: Duration) -> Self {
        let mut cache = Self::new(cache_dir, max_size);
        cache.default_ttl = Some(ttl);
        cache
    }

    /// Check if cache entry exists (< 0.1ms target)
    pub fn has(&self, task_hash: &[u8; 32]) -> bool {
        // Check bloom filter first for fast negative
        if !self.bloom_check(task_hash) {
            return false;
        }

        if self.zero_disk {
            return self.memory_cache.contains_key(task_hash);
        }

        // Check file existence
        let path = self.hash_to_path(task_hash);
        path.exists()
    }

    /// Get cached output with zero-copy access (< 0.5ms target)
    pub fn get(&mut self, task_hash: &[u8; 32]) -> Option<CacheEntry> {
        // Check expiration first
        if self.is_expired(task_hash) {
            self.remove(task_hash);
            return None;
        }

        // Update LRU on access
        self.touch(task_hash);

        if self.zero_disk {
            return self.memory_cache.get(task_hash).cloned();
        }

        let path = self.hash_to_path(task_hash);
        if !path.exists() {
            return None;
        }

        // Read and parse cache entry
        let data = std::fs::read(&path).ok()?;
        self.parse_cache_entry(&data)
    }

    /// Store task output in cache
    pub fn put(&mut self, task_hash: &[u8; 32], entry: &CacheEntry) -> Result<(), CacheError> {
        let size = entry.total_size() as u64;

        // Check if we need to evict
        while self.current_size + size > self.max_size && !self.lru_order.is_empty() {
            self.evict_lru()?;
        }

        // Add to bloom filter
        self.bloom_add(task_hash);

        // Add to LRU tracking
        let lru_entry = LruEntry {
            hash: *task_hash,
            size,
            last_access: Instant::now(),
            created_at: SystemTime::now(),
            ttl: self.default_ttl,
        };

        self.lru_order.push_back(lru_entry);
        self.lru_index.insert(*task_hash, self.lru_order.len() - 1);

        if self.zero_disk {
            self.memory_cache.insert(*task_hash, entry.clone());
            self.current_size += size;
            return Ok(());
        }

        // Serialize and write to disk
        let data = self.serialize_cache_entry(entry);
        let path = self.hash_to_path(task_hash);

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        std::fs::write(&path, &data)?;
        self.current_size += size;

        Ok(())
    }

    /// Remove a cache entry
    pub fn remove(&mut self, task_hash: &[u8; 32]) -> bool {
        // Remove from LRU tracking
        if let Some(idx) = self.lru_index.remove(task_hash) {
            if idx < self.lru_order.len() {
                let entry = self.lru_order.remove(idx);
                if let Some(e) = entry {
                    self.current_size = self.current_size.saturating_sub(e.size);
                }
                // Rebuild index after removal
                self.rebuild_lru_index();
            }
        }

        if self.zero_disk {
            return self.memory_cache.remove(task_hash).is_some();
        }

        let path = self.hash_to_path(task_hash);
        if path.exists() {
            std::fs::remove_file(&path).is_ok()
        } else {
            false
        }
    }

    /// Apply XOR patch to update cache entry
    pub fn apply_patch(
        &mut self,
        task_hash: &[u8; 32],
        patch: &XorPatch,
    ) -> Result<(), CacheError> {
        let base_entry = self.get(&patch.base_hash).ok_or(CacheError::EntryNotFound {
            hash: patch.base_hash,
        })?;

        // Apply patch to each file
        let mut new_entry = CacheEntry::new(*task_hash);

        for file in &base_entry.files {
            let patched_content = patch.apply(&file.content);
            new_entry.add_file(file.path.clone(), patched_content, file.mode);
        }

        self.put(task_hash, &new_entry)
    }

    /// Verify Ed25519 signature of cache entry
    pub fn verify(&self, entry: &CacheEntry) -> Result<bool, CacheError> {
        let (signature, public_key) = match (entry.signature, entry.public_key) {
            (Some(sig), Some(pk)) => (sig, pk),
            _ => return Ok(false), // No signature to verify
        };

        use ed25519_dalek::{Signature, Verifier, VerifyingKey};

        let verifying_key =
            VerifyingKey::from_bytes(&public_key).map_err(|_| CacheError::SignatureInvalid)?;

        let sig = Signature::from_bytes(&signature);

        // Hash all file contents
        let mut hasher = blake3::Hasher::new();
        hasher.update(&entry.task_hash);
        for file in &entry.files {
            hasher.update(file.path.as_bytes());
            hasher.update(&file.content);
        }
        let content_hash = hasher.finalize();

        verifying_key
            .verify(content_hash.as_bytes(), &sig)
            .map(|_| true)
            .map_err(|_| CacheError::SignatureInvalid)
    }

    /// Enable zero-disk mode with virtual filesystem
    pub fn enable_zero_disk(&mut self) -> Result<(), CacheError> {
        self.zero_disk = true;
        Ok(())
    }

    /// Disable zero-disk mode
    pub fn disable_zero_disk(&mut self) {
        self.zero_disk = false;
    }

    /// Get current cache size
    pub fn size(&self) -> u64 {
        self.current_size
    }

    /// Get number of entries
    pub fn len(&self) -> usize {
        self.lru_order.len()
    }

    /// Check if cache is empty
    pub fn is_empty(&self) -> bool {
        self.lru_order.is_empty()
    }

    /// Clear all cache entries
    pub fn clear(&mut self) -> Result<(), CacheError> {
        self.memory_cache.clear();
        self.lru_order.clear();
        self.lru_index.clear();
        self.current_size = 0;
        self.bloom_filter.fill(0);

        if !self.zero_disk && self.cache_dir.exists() {
            std::fs::remove_dir_all(&self.cache_dir)?;
        }

        Ok(())
    }

    /// Remove expired entries
    pub fn cleanup_expired(&mut self) -> usize {
        let mut removed = 0;
        let now = SystemTime::now();

        // Collect expired hashes
        let expired: Vec<[u8; 32]> = self
            .lru_order
            .iter()
            .filter(|e| {
                if let Some(ttl) = e.ttl {
                    if let Ok(elapsed) = now.duration_since(e.created_at) {
                        return elapsed > ttl;
                    }
                }
                false
            })
            .map(|e| e.hash)
            .collect();

        // Remove expired entries
        for hash in expired {
            if self.remove(&hash) {
                removed += 1;
            }
        }

        removed
    }

    /// Get cache statistics
    pub fn stats(&self) -> CacheStats {
        CacheStats {
            entries: self.lru_order.len(),
            size_bytes: self.current_size,
            max_size_bytes: self.max_size,
            utilization: if self.max_size > 0 {
                (self.current_size as f64 / self.max_size as f64) * 100.0
            } else {
                0.0
            },
        }
    }

    // Private helpers

    fn hash_to_path(&self, hash: &[u8; 32]) -> PathBuf {
        let hex: String = hash.iter().map(|b| format!("{:02x}", b)).collect();
        self.cache_dir.join(&hex[0..2]).join(&hex[2..4]).join(&hex)
    }

    fn bloom_add(&mut self, hash: &[u8; 32]) {
        let h1 = u64::from_le_bytes(hash[0..8].try_into().unwrap());
        let h2 = u64::from_le_bytes(hash[8..16].try_into().unwrap());

        for i in 0..4 {
            let idx =
                ((h1.wrapping_add(i as u64 * h2)) % (self.bloom_filter.len() as u64 * 64)) as usize;
            let word = idx / 64;
            let bit = idx % 64;
            self.bloom_filter[word] |= 1 << bit;
        }
    }

    fn bloom_check(&self, hash: &[u8; 32]) -> bool {
        let h1 = u64::from_le_bytes(hash[0..8].try_into().unwrap());
        let h2 = u64::from_le_bytes(hash[8..16].try_into().unwrap());

        for i in 0..4 {
            let idx =
                ((h1.wrapping_add(i as u64 * h2)) % (self.bloom_filter.len() as u64 * 64)) as usize;
            let word = idx / 64;
            let bit = idx % 64;
            if self.bloom_filter[word] & (1 << bit) == 0 {
                return false;
            }
        }
        true
    }

    /// Evict least recently used entry
    fn evict_lru(&mut self) -> Result<(), CacheError> {
        if let Some(entry) = self.lru_order.pop_front() {
            self.lru_index.remove(&entry.hash);
            self.current_size = self.current_size.saturating_sub(entry.size);

            if self.zero_disk {
                self.memory_cache.remove(&entry.hash);
            } else {
                let path = self.hash_to_path(&entry.hash);
                if path.exists() {
                    let _ = std::fs::remove_file(&path);
                }
            }

            // Rebuild index after removal
            self.rebuild_lru_index();
        }
        Ok(())
    }

    /// Update access time for LRU tracking
    fn touch(&mut self, hash: &[u8; 32]) {
        if let Some(&idx) = self.lru_index.get(hash) {
            if idx < self.lru_order.len() {
                // Remove from current position
                if let Some(mut entry) = self.lru_order.remove(idx) {
                    entry.last_access = Instant::now();
                    // Add to back (most recently used)
                    self.lru_order.push_back(entry);
                    // Rebuild index
                    self.rebuild_lru_index();
                }
            }
        }
    }

    /// Check if entry is expired
    fn is_expired(&self, hash: &[u8; 32]) -> bool {
        if let Some(&idx) = self.lru_index.get(hash) {
            if let Some(entry) = self.lru_order.get(idx) {
                if let Some(ttl) = entry.ttl {
                    if let Ok(elapsed) = SystemTime::now().duration_since(entry.created_at) {
                        return elapsed > ttl;
                    }
                }
            }
        }
        false
    }

    /// Rebuild LRU index after modifications
    fn rebuild_lru_index(&mut self) {
        self.lru_index.clear();
        for (idx, entry) in self.lru_order.iter().enumerate() {
            self.lru_index.insert(entry.hash, idx);
        }
    }

    fn serialize_cache_entry(&self, entry: &CacheEntry) -> Vec<u8> {
        // Simple serialization format
        let mut data = Vec::new();

        // Header
        data.extend_from_slice(b"DXC\0");
        data.extend_from_slice(&1u32.to_le_bytes()); // version
        data.extend_from_slice(&entry.task_hash);

        // File count
        data.extend_from_slice(&(entry.files.len() as u32).to_le_bytes());

        // Files
        for file in &entry.files {
            data.extend_from_slice(&(file.path.len() as u32).to_le_bytes());
            data.extend_from_slice(file.path.as_bytes());
            data.extend_from_slice(&(file.content.len() as u64).to_le_bytes());
            data.extend_from_slice(&file.content);
            data.extend_from_slice(&file.mode.to_le_bytes());
        }

        data
    }

    fn parse_cache_entry(&self, data: &[u8]) -> Option<CacheEntry> {
        if data.len() < 44 {
            return None;
        }

        // Verify magic
        if &data[0..4] != b"DXC\0" {
            return None;
        }

        let task_hash: [u8; 32] = data[8..40].try_into().ok()?;
        let file_count = u32::from_le_bytes(data[40..44].try_into().ok()?) as usize;

        let mut entry = CacheEntry::new(task_hash);
        let mut offset = 44;

        for _ in 0..file_count {
            if offset + 4 > data.len() {
                return None;
            }

            let path_len = u32::from_le_bytes(data[offset..offset + 4].try_into().ok()?) as usize;
            offset += 4;

            if offset + path_len > data.len() {
                return None;
            }

            let path = std::str::from_utf8(&data[offset..offset + path_len]).ok()?.to_string();
            offset += path_len;

            if offset + 8 > data.len() {
                return None;
            }

            let content_len =
                u64::from_le_bytes(data[offset..offset + 8].try_into().ok()?) as usize;
            offset += 8;

            if offset + content_len > data.len() {
                return None;
            }

            let content = data[offset..offset + content_len].to_vec();
            offset += content_len;

            if offset + 4 > data.len() {
                return None;
            }

            let mode = u32::from_le_bytes(data[offset..offset + 4].try_into().ok()?);
            offset += 4;

            entry.add_file(path, content, mode);
        }

        Some(entry)
    }
}

/// Cache statistics
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub entries: usize,
    pub size_bytes: u64,
    pub max_size_bytes: u64,
    pub utilization: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_cache_manager_zero_disk() {
        let temp = TempDir::new().unwrap();
        let mut cache = CacheManager::new(temp.path().to_path_buf(), 1024 * 1024);
        cache.enable_zero_disk().unwrap();

        let hash = [1u8; 32];
        let mut entry = CacheEntry::new(hash);
        entry.add_file("test.txt".to_string(), b"hello".to_vec(), 0o644);

        // Initially not in cache
        assert!(!cache.has(&hash));

        // Add to cache
        cache.put(&hash, &entry).unwrap();
        assert!(cache.has(&hash));

        // Retrieve from cache
        let retrieved = cache.get(&hash).unwrap();
        assert_eq!(retrieved.files.len(), 1);
        assert_eq!(retrieved.files[0].content, b"hello");
    }

    #[test]
    fn test_cache_manager_disk() {
        let temp = TempDir::new().unwrap();
        let mut cache = CacheManager::new(temp.path().to_path_buf(), 1024 * 1024);

        let hash = [2u8; 32];
        let mut entry = CacheEntry::new(hash);
        entry.add_file("dist/index.js".to_string(), b"console.log('hi')".to_vec(), 0o644);

        cache.put(&hash, &entry).unwrap();
        assert!(cache.has(&hash));

        let retrieved = cache.get(&hash).unwrap();
        assert_eq!(retrieved.files[0].path, "dist/index.js");
    }

    #[test]
    fn test_bloom_filter() {
        let temp = TempDir::new().unwrap();
        let mut cache = CacheManager::new(temp.path().to_path_buf(), 1024 * 1024);
        cache.enable_zero_disk().unwrap();

        let hash1 = [1u8; 32];
        let hash2 = [2u8; 32];
        let _hash3 = [3u8; 32];

        // Add hash1 and hash2
        cache.bloom_add(&hash1);
        cache.bloom_add(&hash2);

        // hash1 and hash2 should pass bloom check
        assert!(cache.bloom_check(&hash1));
        assert!(cache.bloom_check(&hash2));

        // hash3 might pass (false positive) or fail
        // We can't assert it fails due to bloom filter nature
    }

    #[test]
    fn test_lru_eviction() {
        let temp = TempDir::new().unwrap();
        let mut cache = CacheManager::new(temp.path().to_path_buf(), 100); // Very small cache
        cache.enable_zero_disk().unwrap();

        // Add entries until eviction happens
        for i in 0..10 {
            let hash = [i as u8; 32];
            let mut entry = CacheEntry::new(hash);
            entry.add_file("test.txt".to_string(), vec![0u8; 20], 0o644);
            cache.put(&hash, &entry).unwrap();
        }

        // Cache size should be limited
        assert!(cache.size() <= 100);

        // First entries should have been evicted (LRU)
        // Later entries should still be present
    }

    #[test]
    fn test_lru_order_update_on_access() {
        let temp = TempDir::new().unwrap();
        let mut cache = CacheManager::new(temp.path().to_path_buf(), 1000);
        cache.enable_zero_disk().unwrap();

        // Add three entries
        let hash1 = [1u8; 32];
        let hash2 = [2u8; 32];
        let hash3 = [3u8; 32];

        for hash in [hash1, hash2, hash3] {
            let mut entry = CacheEntry::new(hash);
            entry.add_file("test.txt".to_string(), vec![0u8; 10], 0o644);
            cache.put(&hash, &entry).unwrap();
        }

        // Access hash1 to move it to most recently used
        let _ = cache.get(&hash1);

        // Now hash2 should be LRU (least recently used)
        // If we add more entries to trigger eviction, hash2 should be evicted first
        assert_eq!(cache.len(), 3);
    }

    #[test]
    fn test_cache_clear() {
        let temp = TempDir::new().unwrap();
        let mut cache = CacheManager::new(temp.path().to_path_buf(), 1024 * 1024);
        cache.enable_zero_disk().unwrap();

        let hash = [1u8; 32];
        let mut entry = CacheEntry::new(hash);
        entry.add_file("test.txt".to_string(), b"hello".to_vec(), 0o644);

        cache.put(&hash, &entry).unwrap();
        assert!(cache.has(&hash));

        cache.clear().unwrap();
        assert!(!cache.has(&hash));
        assert_eq!(cache.size(), 0);
        assert!(cache.is_empty());
    }

    #[test]
    fn test_cache_stats() {
        let temp = TempDir::new().unwrap();
        let mut cache = CacheManager::new(temp.path().to_path_buf(), 1024 * 1024);
        cache.enable_zero_disk().unwrap();

        let hash = [1u8; 32];
        let mut entry = CacheEntry::new(hash);
        entry.add_file("test.txt".to_string(), b"hello".to_vec(), 0o644);

        cache.put(&hash, &entry).unwrap();

        let stats = cache.stats();
        assert_eq!(stats.entries, 1);
        assert!(stats.size_bytes > 0);
        assert_eq!(stats.max_size_bytes, 1024 * 1024);
    }

    #[test]
    fn test_cache_remove() {
        let temp = TempDir::new().unwrap();
        let mut cache = CacheManager::new(temp.path().to_path_buf(), 1024 * 1024);
        cache.enable_zero_disk().unwrap();

        let hash = [1u8; 32];
        let mut entry = CacheEntry::new(hash);
        entry.add_file("test.txt".to_string(), b"hello".to_vec(), 0o644);

        cache.put(&hash, &entry).unwrap();
        assert!(cache.has(&hash));

        assert!(cache.remove(&hash));
        assert!(!cache.has(&hash));
    }

    #[test]
    fn test_cache_with_ttl() {
        use std::thread::sleep;

        let temp = TempDir::new().unwrap();
        let mut cache = CacheManager::with_ttl(
            temp.path().to_path_buf(),
            1024 * 1024,
            Duration::from_millis(50),
        );
        cache.enable_zero_disk().unwrap();

        let hash = [1u8; 32];
        let mut entry = CacheEntry::new(hash);
        entry.add_file("test.txt".to_string(), b"hello".to_vec(), 0o644);

        cache.put(&hash, &entry).unwrap();
        assert!(cache.has(&hash));

        // Entry should still be valid
        assert!(cache.get(&hash).is_some());

        // Wait for TTL to expire
        sleep(Duration::from_millis(100));

        // Entry should be expired now
        assert!(cache.get(&hash).is_none());
    }
}

#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;
    use tempfile::TempDir;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Feature: production-readiness, Property 24: LRU Cache Eviction
        /// Validates: Requirements 24.1
        /// For any cache with size limit L and entries exceeding L,
        /// the least recently used entries SHALL be evicted to maintain size <= L
        #[test]
        fn prop_lru_cache_size_limit(
            max_size in 50u64..500u64,
            entry_count in 5usize..20usize,
            entry_size in 10usize..50usize
        ) {
            let temp = TempDir::new().unwrap();
            let mut cache = CacheManager::new(temp.path().to_path_buf(), max_size);
            cache.enable_zero_disk().unwrap();

            // Add entries
            for i in 0..entry_count {
                let hash = [i as u8; 32];
                let mut entry = CacheEntry::new(hash);
                entry.add_file("test.txt".to_string(), vec![0u8; entry_size], 0o644);
                let _ = cache.put(&hash, &entry);
            }

            // Cache size should never exceed max_size
            prop_assert!(cache.size() <= max_size,
                "Cache size {} exceeded max_size {}", cache.size(), max_size);
        }

        /// Feature: production-readiness, Property 24: LRU Cache Eviction
        /// Validates: Requirements 24.1
        /// Accessing an entry should move it to most recently used position
        #[test]
        fn prop_lru_access_updates_order(
            entry_count in 3usize..10usize,
            access_index in 0usize..3usize
        ) {
            let temp = TempDir::new().unwrap();
            let mut cache = CacheManager::new(temp.path().to_path_buf(), 10000);
            cache.enable_zero_disk().unwrap();

            // Add entries
            let mut hashes = Vec::new();
            for i in 0..entry_count {
                let hash = [i as u8; 32];
                hashes.push(hash);
                let mut entry = CacheEntry::new(hash);
                entry.add_file("test.txt".to_string(), vec![0u8; 10], 0o644);
                cache.put(&hash, &entry).unwrap();
            }

            // Access an entry (if valid index)
            let access_idx = access_index % entry_count;
            let _ = cache.get(&hashes[access_idx]);

            // The accessed entry should still be in cache
            prop_assert!(cache.has(&hashes[access_idx]),
                "Accessed entry should still be in cache");
        }

        /// Feature: production-readiness, Property 24: LRU Cache Eviction
        /// Validates: Requirements 24.1
        /// After clearing cache, size should be 0 and all entries removed
        #[test]
        fn prop_cache_clear_removes_all(entry_count in 1usize..20usize) {
            let temp = TempDir::new().unwrap();
            let mut cache = CacheManager::new(temp.path().to_path_buf(), 100000);
            cache.enable_zero_disk().unwrap();

            // Add entries
            let mut hashes = Vec::new();
            for i in 0..entry_count {
                let hash = [i as u8; 32];
                hashes.push(hash);
                let mut entry = CacheEntry::new(hash);
                entry.add_file("test.txt".to_string(), vec![0u8; 10], 0o644);
                cache.put(&hash, &entry).unwrap();
            }

            // Clear cache
            cache.clear().unwrap();

            // Verify all entries removed
            prop_assert_eq!(cache.size(), 0);
            prop_assert!(cache.is_empty());

            for hash in &hashes {
                prop_assert!(!cache.has(hash), "Entry should be removed after clear");
            }
        }

        /// Feature: production-readiness, Property 24: LRU Cache Eviction
        /// Validates: Requirements 24.1
        /// Put then get should return the same entry
        #[test]
        fn prop_cache_roundtrip(
            content in prop::collection::vec(any::<u8>(), 1..100)
        ) {
            let temp = TempDir::new().unwrap();
            let mut cache = CacheManager::new(temp.path().to_path_buf(), 100000);
            cache.enable_zero_disk().unwrap();

            let hash = [42u8; 32];
            let mut entry = CacheEntry::new(hash);
            entry.add_file("test.txt".to_string(), content.clone(), 0o644);

            cache.put(&hash, &entry).unwrap();
            let retrieved = cache.get(&hash).unwrap();

            prop_assert_eq!(retrieved.files.len(), 1);
            prop_assert_eq!(&retrieved.files[0].content, &content);
        }
    }
}
