//! Persistent Compilation Cache

use dashmap::DashMap;
use std::fs::{self, File};
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};

use crate::{CachedArtifact, CodeStorage, CompilationTier, FunctionSignature, PccError};

/// Cache index entry
#[derive(Debug, Clone)]
struct IndexEntry {
    signature: FunctionSignature,
    artifact: CachedArtifact,
}

/// Persistent Compilation Cache
pub struct PersistentCompilationCache {
    /// Cache directory
    cache_dir: PathBuf,
    /// Index mapping signatures to artifacts
    index: DashMap<u128, IndexEntry>,
    /// Code storage
    code_storage: CodeStorage,
    /// Maximum cache size in bytes
    max_size: u64,
    /// LRU cleanup threshold (fraction of max_size)
    cleanup_threshold: f64,
}

impl PersistentCompilationCache {
    /// Open or create a persistent compilation cache
    pub fn open<P: AsRef<Path>>(cache_dir: P) -> Result<Self, PccError> {
        let cache_dir = cache_dir.as_ref().to_path_buf();
        fs::create_dir_all(&cache_dir)?;

        let code_path = cache_dir.join("code.bin");
        let code_storage = CodeStorage::open(&code_path)?;

        let mut cache = Self {
            cache_dir,
            index: DashMap::new(),
            code_storage,
            max_size: 256 * 1024 * 1024, // 256MB default
            cleanup_threshold: 0.9,
        };

        cache.load_index()?;

        Ok(cache)
    }

    /// Create an in-memory cache (for testing)
    pub fn in_memory() -> Self {
        Self {
            cache_dir: PathBuf::new(),
            index: DashMap::new(),
            code_storage: CodeStorage::new(),
            max_size: 64 * 1024 * 1024,
            cleanup_threshold: 0.9,
        }
    }

    /// Set maximum cache size
    pub fn set_max_size(&mut self, size: u64) {
        self.max_size = size;
    }

    /// Get a cached artifact by signature
    pub fn get(&self, signature: &FunctionSignature) -> Option<(&CachedArtifact, *const u8)> {
        let key = signature.cache_key();

        if let Some(mut entry) = self.index.get_mut(&key) {
            // Verify full signature match
            if entry.signature.source_hash == signature.source_hash
                && entry.signature.bytecode_hash == signature.bytecode_hash
                && entry.signature.type_profile_hash == signature.type_profile_hash
            {
                entry.artifact.record_access();
                let code_ptr = self.code_storage.get_ptr(entry.artifact.code_offset)?;

                // Return references - caller must ensure cache outlives usage
                let artifact_ref = unsafe { &*(&entry.artifact as *const CachedArtifact) };
                return Some((artifact_ref, code_ptr));
            }
        }

        None
    }

    /// Check if a signature is cached
    pub fn contains(&self, signature: &FunctionSignature) -> bool {
        let key = signature.cache_key();
        if let Some(entry) = self.index.get(&key) {
            entry.signature.source_hash == signature.source_hash
                && entry.signature.bytecode_hash == signature.bytecode_hash
                && entry.signature.type_profile_hash == signature.type_profile_hash
        } else {
            false
        }
    }

    /// Save compiled code to the cache
    pub fn save(
        &self,
        signature: FunctionSignature,
        tier: CompilationTier,
        code: &[u8],
        profile_data: Vec<u8>,
    ) -> Result<(), PccError> {
        // Check if cleanup is needed
        if self.code_storage.current_offset() as f64 > self.max_size as f64 * self.cleanup_threshold
        {
            self.cleanup()?;
        }

        // Allocate space for code
        let code_offset = self.code_storage.allocate(code.len(), 16)?;

        // Write code
        self.code_storage.write(code_offset, code)?;

        // Create artifact
        let artifact = CachedArtifact::new(
            tier,
            code_offset,
            code.len() as u32,
            vec![], // Relocations handled separately
            profile_data,
        );

        // Add to index
        let key = signature.cache_key();
        self.index.insert(
            key,
            IndexEntry {
                signature,
                artifact,
            },
        );

        Ok(())
    }

    /// Invalidate cache entries for a source file
    pub fn invalidate_source(&self, source_hash: &[u8; 32]) {
        self.index.retain(|_, entry| entry.signature.source_hash != *source_hash);
    }

    /// Invalidate all entries for a module
    pub fn invalidate_module(&self, module: &str) {
        self.index.retain(|_, entry| entry.signature.module != module);
    }

    /// Cleanup old entries using LRU eviction
    pub fn cleanup(&self) -> Result<(), PccError> {
        let target_size = (self.max_size as f64 * 0.7) as u64;

        // Collect entries with access info
        let mut entries: Vec<(u128, u64, u32)> = self
            .index
            .iter()
            .map(|e| (*e.key(), e.artifact.last_accessed, e.artifact.access_count))
            .collect();

        // Sort by last access time (oldest first)
        entries.sort_by_key(|(_, last_accessed, _)| *last_accessed);

        // Remove oldest entries until we're under target
        for (key, _, _) in entries {
            if self.code_storage.current_offset() <= target_size {
                break;
            }
            self.index.remove(&key);
        }

        // Note: This doesn't actually reclaim space in the code storage
        // A full compaction would require rewriting the entire cache

        Ok(())
    }

    /// Flush all changes to disk
    pub fn flush(&self) -> Result<(), PccError> {
        self.code_storage.flush()?;
        self.save_index()?;
        Ok(())
    }

    /// Load index from disk
    fn load_index(&mut self) -> Result<(), PccError> {
        let index_path = self.cache_dir.join("index.bin");

        if !index_path.exists() {
            return Ok(());
        }

        let file = File::open(&index_path)?;
        let mut reader = BufReader::new(file);

        // Read entry count
        let mut count_bytes = [0u8; 4];
        if reader.read_exact(&mut count_bytes).is_err() {
            return Ok(()); // Empty or corrupted, start fresh
        }
        let count = u32::from_le_bytes(count_bytes) as usize;

        for _ in 0..count {
            // Read entry size
            let mut size_bytes = [0u8; 4];
            if reader.read_exact(&mut size_bytes).is_err() {
                break;
            }
            let size = u32::from_le_bytes(size_bytes) as usize;

            // Read entry data
            let mut data = vec![0u8; size];
            if reader.read_exact(&mut data).is_err() {
                break;
            }

            // Parse signature and artifact
            if let Some((sig, artifact)) = Self::parse_index_entry(&data) {
                let key = sig.cache_key();
                self.index.insert(
                    key,
                    IndexEntry {
                        signature: sig,
                        artifact,
                    },
                );
            }
        }

        Ok(())
    }

    /// Save index to disk
    fn save_index(&self) -> Result<(), PccError> {
        if self.cache_dir.as_os_str().is_empty() {
            return Ok(()); // In-memory cache
        }

        let index_path = self.cache_dir.join("index.bin");
        let file = File::create(&index_path)?;
        let mut writer = BufWriter::new(file);

        // Write entry count
        let count = self.index.len() as u32;
        writer.write_all(&count.to_le_bytes())?;

        // Write each entry
        for entry in self.index.iter() {
            let data = Self::serialize_index_entry(&entry.signature, &entry.artifact);
            writer.write_all(&(data.len() as u32).to_le_bytes())?;
            writer.write_all(&data)?;
        }

        writer.flush()?;
        Ok(())
    }

    fn serialize_index_entry(sig: &FunctionSignature, artifact: &CachedArtifact) -> Vec<u8> {
        let sig_bytes = sig.to_bytes();
        let artifact_bytes = artifact.to_bytes();

        let mut data = Vec::with_capacity(8 + sig_bytes.len() + artifact_bytes.len());
        data.extend_from_slice(&(sig_bytes.len() as u32).to_le_bytes());
        data.extend_from_slice(&sig_bytes);
        data.extend_from_slice(&(artifact_bytes.len() as u32).to_le_bytes());
        data.extend_from_slice(&artifact_bytes);

        data
    }

    fn parse_index_entry(data: &[u8]) -> Option<(FunctionSignature, CachedArtifact)> {
        if data.len() < 8 {
            return None;
        }

        let sig_len = u32::from_le_bytes(data[0..4].try_into().ok()?) as usize;
        if data.len() < 4 + sig_len + 4 {
            return None;
        }

        let sig = FunctionSignature::from_bytes(&data[4..4 + sig_len])?;

        let artifact_offset = 4 + sig_len;
        let artifact_len =
            u32::from_le_bytes(data[artifact_offset..artifact_offset + 4].try_into().ok()?)
                as usize;

        if data.len() < artifact_offset + 4 + artifact_len {
            return None;
        }

        let artifact = CachedArtifact::from_bytes(
            &data[artifact_offset + 4..artifact_offset + 4 + artifact_len],
        )?;

        Some((sig, artifact))
    }

    /// Get cache statistics
    pub fn stats(&self) -> CacheStats {
        let mut total_code_size = 0u64;
        let mut tier_counts = [0u32; 4];

        for entry in self.index.iter() {
            total_code_size += entry.artifact.code_size as u64;
            tier_counts[entry.artifact.tier as usize] += 1;
        }

        CacheStats {
            entry_count: self.index.len(),
            total_code_size,
            storage_used: self.code_storage.current_offset(),
            storage_capacity: self.code_storage.capacity(),
            tier_counts,
        }
    }
}

/// Cache statistics
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub entry_count: usize,
    pub total_code_size: u64,
    pub storage_used: u64,
    pub storage_capacity: u64,
    pub tier_counts: [u32; 4],
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_cache_creation() {
        let temp = TempDir::new().unwrap();
        let cache = PersistentCompilationCache::open(temp.path()).unwrap();

        let stats = cache.stats();
        assert_eq!(stats.entry_count, 0);
    }

    #[test]
    fn test_save_and_get() {
        let cache = PersistentCompilationCache::in_memory();

        let sig = FunctionSignature::new(
            b"def foo(): pass",
            b"\x00\x01",
            b"",
            "foo".to_string(),
            "test".to_string(),
        );

        let code = vec![0x90; 64]; // NOP sled
        cache.save(sig.clone(), CompilationTier::Baseline, &code, vec![]).unwrap();

        assert!(cache.contains(&sig));

        let (artifact, code_ptr) = cache.get(&sig).unwrap();
        assert_eq!(artifact.tier, CompilationTier::Baseline);
        assert_eq!(artifact.code_size, 64);
        assert!(!code_ptr.is_null());
    }

    #[test]
    fn test_invalidate_source() {
        let cache = PersistentCompilationCache::in_memory();

        let sig1 =
            FunctionSignature::new(b"source1", b"bc1", b"", "f1".to_string(), "m".to_string());

        let sig2 =
            FunctionSignature::new(b"source2", b"bc2", b"", "f2".to_string(), "m".to_string());

        cache.save(sig1.clone(), CompilationTier::Baseline, &[0x90], vec![]).unwrap();
        cache.save(sig2.clone(), CompilationTier::Baseline, &[0x90], vec![]).unwrap();

        assert!(cache.contains(&sig1));
        assert!(cache.contains(&sig2));

        cache.invalidate_source(&sig1.source_hash);

        assert!(!cache.contains(&sig1));
        assert!(cache.contains(&sig2));
    }

    #[test]
    fn test_persistence() {
        let temp = TempDir::new().unwrap();

        let sig = FunctionSignature::new(
            b"persistent",
            b"code",
            b"",
            "func".to_string(),
            "mod".to_string(),
        );

        // Save to cache
        {
            let cache = PersistentCompilationCache::open(temp.path()).unwrap();
            cache
                .save(sig.clone(), CompilationTier::Optimized, &[0xCC; 100], vec![1, 2, 3])
                .unwrap();
            cache.flush().unwrap();
        }

        // Reopen and verify
        {
            let cache = PersistentCompilationCache::open(temp.path()).unwrap();
            assert!(cache.contains(&sig));

            let (artifact, _) = cache.get(&sig).unwrap();
            assert_eq!(artifact.tier, CompilationTier::Optimized);
            assert_eq!(artifact.code_size, 100);
        }
    }

    #[test]
    fn test_stats() {
        let cache = PersistentCompilationCache::in_memory();

        for i in 0..10 {
            let sig = FunctionSignature::new(
                format!("source{}", i).as_bytes(),
                b"bc",
                b"",
                format!("f{}", i),
                "m".to_string(),
            );
            let tier = if i % 2 == 0 {
                CompilationTier::Baseline
            } else {
                CompilationTier::Optimized
            };
            cache.save(sig, tier, &[0x90; 100], vec![]).unwrap();
        }

        let stats = cache.stats();
        assert_eq!(stats.entry_count, 10);
        assert_eq!(stats.tier_counts[CompilationTier::Baseline as usize], 5);
        assert_eq!(stats.tier_counts[CompilationTier::Optimized as usize], 5);
    }
}
