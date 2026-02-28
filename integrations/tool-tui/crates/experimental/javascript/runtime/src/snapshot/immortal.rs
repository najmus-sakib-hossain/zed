//! Persistent code cache
//!
//! The immortal cache stores compiled MIR (Mid-level Intermediate Representation)
//! to disk, allowing fast cold starts by skipping parsing and type checking.

use crate::compiler::mir::TypedMIR;
use crate::compiler::CompiledModule;
use crate::constants::{CACHE_FILE_EXTENSION, CACHE_MAGIC, HASH_FULL_LEN};
use crate::error::{DxError, DxResult};
use crate::CacheStats;
use blake3::Hasher;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

/// Cache file version - increment when format changes
const CACHE_VERSION: u32 = 1;

/// Hash of source code
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct SourceHash([u8; HASH_FULL_LEN]);

impl SourceHash {
    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }

    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() == HASH_FULL_LEN {
            let mut hash = [0u8; HASH_FULL_LEN];
            hash.copy_from_slice(bytes);
            Some(Self(hash))
        } else {
            None
        }
    }
}

/// Cached MIR entry stored on disk
#[derive(Serialize, Deserialize)]
struct CachedEntry {
    /// Version of the cache format
    version: u32,
    /// Source hash for validation
    source_hash: [u8; HASH_FULL_LEN],
    /// Runtime version that created this cache
    runtime_version: String,
    /// Target architecture
    arch: String,
    /// The cached MIR
    mir: TypedMIR,
}

/// Persistent code cache - compiled modules survive restarts
pub struct ImmortalCache {
    cache_dir: PathBuf,
    /// In-memory index of cached modules
    index: HashMap<SourceHash, CacheEntry>,
    /// Statistics
    hits: AtomicU64,
    misses: AtomicU64,
    /// Verbose logging enabled
    verbose: bool,
}

#[derive(Clone)]
struct CacheEntry {
    path: PathBuf,
    size: u64,
}

impl ImmortalCache {
    /// Open or create the cache
    pub fn open_or_create(cache_dir: &Path) -> DxResult<Self> {
        Self::open_or_create_with_options(cache_dir, false)
    }

    /// Open or create the cache with verbose logging option
    pub fn open_or_create_with_options(cache_dir: &Path, verbose: bool) -> DxResult<Self> {
        fs::create_dir_all(cache_dir)?;

        // Scan existing cache entries
        let mut index = HashMap::new();
        if let Ok(entries) = fs::read_dir(cache_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().is_some_and(|e| e == CACHE_FILE_EXTENSION) {
                    if let Some(stem) = path.file_stem() {
                        let hex_str = stem.to_string_lossy();
                        if let Ok(bytes) = hex::decode(&*hex_str) {
                            if bytes.len() == HASH_FULL_LEN {
                                let mut hash = [0u8; HASH_FULL_LEN];
                                hash.copy_from_slice(&bytes);
                                if let Ok(meta) = entry.metadata() {
                                    index.insert(
                                        SourceHash(hash),
                                        CacheEntry {
                                            path: path.clone(),
                                            size: meta.len(),
                                        },
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }

        if verbose {
            eprintln!("[cache] Immortal cache opened with {} entries", index.len());
        }

        Ok(Self {
            cache_dir: cache_dir.to_path_buf(),
            index,
            hits: AtomicU64::new(0),
            misses: AtomicU64::new(0),
            verbose,
        })
    }

    /// Hash source code with version and architecture info
    pub fn hash_source(&self, source: &str) -> SourceHash {
        let mut hasher = Hasher::new();
        hasher.update(source.as_bytes());
        hasher.update(env!("CARGO_PKG_VERSION").as_bytes());
        hasher.update(std::env::consts::ARCH.as_bytes());
        SourceHash(hasher.finalize().into())
    }

    /// Get cached MIR from disk
    pub fn get_mir(&self, hash: &SourceHash) -> DxResult<Option<TypedMIR>> {
        if let Some(entry) = self.index.get(hash) {
            match self.load_from_disk(&entry.path, hash) {
                Ok(mir) => {
                    self.hits.fetch_add(1, Ordering::Relaxed);
                    if self.verbose {
                        eprintln!("[cache] Cache hit for {}", hash.to_hex());
                    }
                    Ok(Some(mir))
                }
                Err(e) => {
                    // Cache corrupted, log and return None to trigger recompile
                    eprintln!("[cache] Cache entry corrupted for {}: {}", hash.to_hex(), e);
                    self.misses.fetch_add(1, Ordering::Relaxed);
                    Ok(None)
                }
            }
        } else {
            self.misses.fetch_add(1, Ordering::Relaxed);
            if self.verbose {
                eprintln!("[cache] Cache miss for {}", hash.to_hex());
            }
            Ok(None)
        }
    }

    /// Get cached module (for backward compatibility - returns None, use get_mir instead)
    pub fn get(&self, hash: &SourceHash) -> DxResult<Option<CompiledModule>> {
        // CompiledModule contains JIT code which cannot be serialized
        // This method exists for API compatibility but always returns None
        // Use get_mir() to retrieve cached MIR and recompile
        if self.index.contains_key(hash) {
            self.hits.fetch_add(1, Ordering::Relaxed);
        } else {
            self.misses.fetch_add(1, Ordering::Relaxed);
        }
        Ok(None)
    }

    /// Load MIR from disk with validation
    fn load_from_disk(&self, path: &Path, expected_hash: &SourceHash) -> DxResult<TypedMIR> {
        let mut file = File::open(path)?;
        let mut data = Vec::new();
        file.read_to_end(&mut data)?;

        // Validate magic header
        if data.len() < CACHE_MAGIC.len() {
            return Err(DxError::CacheError("Cache file too small".to_string()));
        }

        if &data[..CACHE_MAGIC.len()] != CACHE_MAGIC {
            return Err(DxError::CacheError("Invalid cache magic".to_string()));
        }

        // Deserialize the cached entry
        let cached: CachedEntry = bincode::deserialize(&data[CACHE_MAGIC.len()..])
            .map_err(|e| DxError::CacheError(format!("Failed to deserialize cache: {}", e)))?;

        // Validate version
        if cached.version != CACHE_VERSION {
            return Err(DxError::CacheError(format!(
                "Cache version mismatch: expected {}, got {}",
                CACHE_VERSION, cached.version
            )));
        }

        // Validate source hash
        if cached.source_hash != expected_hash.0 {
            return Err(DxError::CacheError("Source hash mismatch".to_string()));
        }

        // Validate runtime version
        if cached.runtime_version != env!("CARGO_PKG_VERSION") {
            return Err(DxError::CacheError(format!(
                "Runtime version mismatch: expected {}, got {}",
                env!("CARGO_PKG_VERSION"),
                cached.runtime_version
            )));
        }

        // Validate architecture
        if cached.arch != std::env::consts::ARCH {
            return Err(DxError::CacheError(format!(
                "Architecture mismatch: expected {}, got {}",
                std::env::consts::ARCH,
                cached.arch
            )));
        }

        Ok(cached.mir)
    }

    /// Store MIR to cache
    pub fn store_mir(&mut self, hash: &SourceHash, mir: &TypedMIR) -> DxResult<()> {
        let path = self.cache_path(hash);

        let cached = CachedEntry {
            version: CACHE_VERSION,
            source_hash: hash.0,
            runtime_version: env!("CARGO_PKG_VERSION").to_string(),
            arch: std::env::consts::ARCH.to_string(),
            mir: mir.clone(),
        };

        // Serialize the cached entry
        let serialized = bincode::serialize(&cached)
            .map_err(|e| DxError::CacheError(format!("Failed to serialize cache: {}", e)))?;

        // Write with magic header
        let mut file = File::create(&path)?;
        file.write_all(CACHE_MAGIC)?;
        file.write_all(&serialized)?;
        file.sync_all()?;

        let size = CACHE_MAGIC.len() as u64 + serialized.len() as u64;
        self.index.insert(*hash, CacheEntry { path, size });

        if self.verbose {
            eprintln!("[cache] Cached MIR for {} ({} bytes)", hash.to_hex(), size);
        }

        Ok(())
    }

    /// Store compiled module to cache (for backward compatibility)
    pub fn store(&mut self, hash: &SourceHash, _module: &CompiledModule) -> DxResult<()> {
        let path = self.cache_path(hash);

        // Write a cache marker file (legacy format)
        let mut file = File::create(&path)?;
        file.write_all(CACHE_MAGIC)?;
        file.write_all(env!("CARGO_PKG_VERSION").as_bytes())?;

        let size = file.metadata().map(|m| m.len()).unwrap_or(0);
        self.index.insert(*hash, CacheEntry { path, size });

        Ok(())
    }

    fn cache_path(&self, hash: &SourceHash) -> PathBuf {
        self.cache_dir.join(format!("{}.{}", hash.to_hex(), CACHE_FILE_EXTENSION))
    }

    /// Invalidate cache entry for a source hash
    pub fn invalidate(&mut self, hash: &SourceHash) -> DxResult<()> {
        if let Some(entry) = self.index.remove(hash) {
            if let Err(e) = fs::remove_file(&entry.path) {
                eprintln!("[cache] Failed to remove cache file {}: {}", entry.path.display(), e);
            }
        }
        Ok(())
    }

    /// Check if source has changed and invalidate if needed
    pub fn check_and_invalidate(&mut self, source: &str, hash: &SourceHash) -> bool {
        let current_hash = self.hash_source(source);
        if current_hash != *hash {
            let _ = self.invalidate(hash);
            true
        } else {
            false
        }
    }

    /// Get cache statistics
    pub fn stats(&self) -> CacheStats {
        let total_size: u64 = self.index.values().map(|e| e.size).sum();
        let hits = self.hits.load(Ordering::Relaxed);
        let misses = self.misses.load(Ordering::Relaxed);

        if self.verbose {
            let hit_rate = if hits + misses > 0 {
                (hits as f64 / (hits + misses) as f64) * 100.0
            } else {
                0.0
            };
            eprintln!(
                "[cache] Cache stats: {} hits, {} misses ({:.1}% hit rate), {} modules, {} bytes",
                hits,
                misses,
                hit_rate,
                self.index.len(),
                total_size
            );
        }

        CacheStats {
            hits,
            misses,
            modules_cached: self.index.len(),
            total_size_bytes: total_size,
        }
    }

    /// Log cache statistics (for verbose mode)
    pub fn log_stats(&self) {
        let stats = self.stats();
        let hit_rate = if stats.hits + stats.misses > 0 {
            (stats.hits as f64 / (stats.hits + stats.misses) as f64) * 100.0
        } else {
            0.0
        };
        println!(
            "Cache: {} hits, {} misses ({:.1}% hit rate), {} modules cached ({} bytes)",
            stats.hits, stats.misses, hit_rate, stats.modules_cached, stats.total_size_bytes
        );
    }

    /// Clear the cache
    pub fn clear(&mut self) -> DxResult<()> {
        for entry in self.index.values() {
            let _ = fs::remove_file(&entry.path);
        }
        self.index.clear();
        self.hits.store(0, Ordering::Relaxed);
        self.misses.store(0, Ordering::Relaxed);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::mir::{
        BlockId, FunctionId, PrimitiveType, SourceSpan, Terminator, Type, TypedBlock, TypedFunction,
    };
    use tempfile::TempDir;

    fn create_test_mir() -> TypedMIR {
        TypedMIR {
            functions: vec![TypedFunction {
                id: FunctionId(0),
                name: "test".to_string(),
                params: vec![],
                return_type: Type::Primitive(PrimitiveType::F64),
                blocks: vec![TypedBlock {
                    id: BlockId(0),
                    instructions: vec![],
                    terminator: Terminator::Return(None),
                    instruction_spans: vec![],
                    terminator_span: SourceSpan::unknown(),
                }],
                locals: vec![],
                is_pure: true,
                span: SourceSpan::unknown(),
            }],
            globals: vec![],
            entry_point: Some(FunctionId(0)),
            type_layouts: HashMap::new(),
            source_file: String::new(),
        }
    }

    #[test]
    fn test_cache_round_trip() {
        let temp_dir = TempDir::new().unwrap();
        let mut cache = ImmortalCache::open_or_create(temp_dir.path()).unwrap();

        let source = "const x = 42;";
        let hash = cache.hash_source(source);
        let mir = create_test_mir();

        // Store MIR
        cache.store_mir(&hash, &mir).unwrap();

        // Retrieve MIR
        let loaded = cache.get_mir(&hash).unwrap();
        assert!(loaded.is_some());

        let loaded_mir = loaded.unwrap();
        assert_eq!(loaded_mir.functions.len(), mir.functions.len());
        assert_eq!(loaded_mir.functions[0].name, mir.functions[0].name);
    }

    #[test]
    fn test_cache_invalidation() {
        let temp_dir = TempDir::new().unwrap();
        let mut cache = ImmortalCache::open_or_create(temp_dir.path()).unwrap();

        let source = "const x = 42;";
        let hash = cache.hash_source(source);
        let mir = create_test_mir();

        cache.store_mir(&hash, &mir).unwrap();
        assert!(cache.get_mir(&hash).unwrap().is_some());

        cache.invalidate(&hash).unwrap();
        assert!(cache.get_mir(&hash).unwrap().is_none());
    }

    #[test]
    fn test_cache_corruption_recovery() {
        let temp_dir = TempDir::new().unwrap();
        let mut cache = ImmortalCache::open_or_create(temp_dir.path()).unwrap();

        let source = "const x = 42;";
        let hash = cache.hash_source(source);
        let mir = create_test_mir();

        cache.store_mir(&hash, &mir).unwrap();

        // Corrupt the cache file
        let path = cache.cache_path(&hash);
        fs::write(&path, b"corrupted data").unwrap();

        // Should return None (not error) for corrupted cache
        let result = cache.get_mir(&hash).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_cache_stats() {
        let temp_dir = TempDir::new().unwrap();
        let mut cache = ImmortalCache::open_or_create(temp_dir.path()).unwrap();

        let source = "const x = 42;";
        let hash = cache.hash_source(source);
        let mir = create_test_mir();

        cache.store_mir(&hash, &mir).unwrap();

        // Hit
        let _ = cache.get_mir(&hash);
        // Miss
        let other_hash = cache.hash_source("const y = 100;");
        let _ = cache.get_mir(&other_hash);

        let stats = cache.stats();
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 1);
        assert_eq!(stats.modules_cached, 1);
    }
}
