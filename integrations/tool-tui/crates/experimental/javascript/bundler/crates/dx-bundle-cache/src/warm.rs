//! Warm cache - in-memory hot cache with persistent backing

use crate::{CacheStats, CachedTransform};
use dashmap::DashMap;
use dx_bundle_core::hash::PathHasher;
use dx_bundle_core::{ContentHash, ModuleId};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

/// In-memory warm cache
pub struct WarmCache {
    /// Hot cache (in-memory)
    hot: DashMap<ModuleId, CachedTransform>,
    /// Cache directory
    cache_dir: PathBuf,
    /// Statistics
    hits: AtomicUsize,
    misses: AtomicUsize,
}

impl Clone for WarmCache {
    fn clone(&self) -> Self {
        Self {
            hot: self.hot.clone(),
            cache_dir: self.cache_dir.clone(),
            hits: AtomicUsize::new(self.hits.load(Ordering::Relaxed)),
            misses: AtomicUsize::new(self.misses.load(Ordering::Relaxed)),
        }
    }
}

impl WarmCache {
    /// Create new warm cache
    pub fn new(cache_dir: PathBuf) -> Self {
        // Create cache directory if it doesn't exist
        std::fs::create_dir_all(&cache_dir).ok();

        Self {
            hot: DashMap::new(),
            cache_dir,
            hits: AtomicUsize::new(0),
            misses: AtomicUsize::new(0),
        }
    }

    /// Load cache from disk
    pub fn load(cache_dir: PathBuf) -> std::io::Result<Self> {
        let cache = Self::new(cache_dir);

        // Load persistent cache if it exists
        let cache_file = cache.cache_dir.join("warm-cache.bin");
        if cache_file.exists() {
            match cache.load_from_file(&cache_file) {
                Ok(count) => {
                    eprintln!("Loaded {} cached modules", count);
                }
                Err(e) => {
                    eprintln!("Failed to load cache: {}", e);
                }
            }
        }

        Ok(cache)
    }

    /// Get cached transform
    pub fn get(&self, path: &Path) -> Option<CachedTransform> {
        let module_id = PathHasher::hash(path);

        if let Some(entry) = self.hot.get(&module_id) {
            // Validate cache entry
            if entry.is_valid(path) {
                self.hits.fetch_add(1, Ordering::Relaxed);
                return Some(entry.clone());
            } else {
                // Invalid - remove from cache
                drop(entry);
                self.hot.remove(&module_id);
            }
        }

        self.misses.fetch_add(1, Ordering::Relaxed);
        None
    }

    /// Store transformed module
    pub fn put(&self, path: &Path, transform: CachedTransform) {
        let module_id = PathHasher::hash(path);
        self.hot.insert(module_id, transform);
    }

    /// Invalidate cache entry
    pub fn invalidate(&self, path: &Path) {
        let module_id = PathHasher::hash(path);
        self.hot.remove(&module_id);
    }

    /// Clear all cache
    pub fn clear(&self) {
        self.hot.clear();
    }

    /// Get cache statistics
    pub fn stats(&self) -> CacheStats {
        let hits = self.hits.load(Ordering::Relaxed);
        let misses = self.misses.load(Ordering::Relaxed);

        let bytes_saved: usize = self.hot.iter().map(|entry| entry.transformed.len()).sum();

        CacheStats {
            hits,
            misses,
            bytes_saved,
            cache_size: self.hot.len() * std::mem::size_of::<CachedTransform>(),
        }
    }

    /// Flush cache to disk (async)
    pub fn flush(&self) -> std::io::Result<()> {
        let cache_file = self.cache_dir.join("warm-cache.bin");
        self.save_to_file(&cache_file)
    }

    // ========== Persistence ==========

    fn load_from_file(&self, path: &Path) -> std::io::Result<usize> {
        let file = std::fs::File::open(path)?;
        let mut reader = std::io::BufReader::new(file);

        // Read header
        let mut magic = [0u8; 4];
        reader.read_exact(&mut magic)?;

        if &magic != b"DXWC" {
            return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "Invalid cache file"));
        }

        let mut count_bytes = [0u8; 4];
        reader.read_exact(&mut count_bytes)?;
        let count = u32::from_le_bytes(count_bytes) as usize;

        // Read entries
        let mut loaded = 0;
        for _ in 0..count {
            match self.load_entry(&mut reader) {
                Ok((module_id, transform)) => {
                    self.hot.insert(module_id, transform);
                    loaded += 1;
                }
                Err(_) => break,
            }
        }

        Ok(loaded)
    }

    fn load_entry(
        &self,
        reader: &mut impl std::io::Read,
    ) -> std::io::Result<(ModuleId, CachedTransform)> {
        // Read module ID
        let mut id_bytes = [0u8; 8];
        reader.read_exact(&mut id_bytes)?;
        let module_id = u64::from_le_bytes(id_bytes);

        // Read content hash
        let mut hash_bytes = [0u8; 16];
        reader.read_exact(&mut hash_bytes)?;
        let content_hash = ContentHash::from_bytes(hash_bytes);

        // Read mtime
        let mut mtime_bytes = [0u8; 8];
        reader.read_exact(&mut mtime_bytes)?;
        let mtime = u64::from_le_bytes(mtime_bytes);

        // Read transformed size
        let mut size_bytes = [0u8; 4];
        reader.read_exact(&mut size_bytes)?;
        let size = u32::from_le_bytes(size_bytes) as usize;

        // Read transformed data
        let mut transformed = vec![0u8; size];
        reader.read_exact(&mut transformed)?;

        // Read imports count
        let mut imports_count_bytes = [0u8; 4];
        reader.read_exact(&mut imports_count_bytes)?;
        let imports_count = u32::from_le_bytes(imports_count_bytes) as usize;

        // Read imports
        let mut imports = Vec::with_capacity(imports_count);
        for _ in 0..imports_count {
            let mut import_bytes = [0u8; 8];
            reader.read_exact(&mut import_bytes)?;
            imports.push(u64::from_le_bytes(import_bytes));
        }

        Ok((
            module_id,
            CachedTransform {
                content_hash,
                transformed,
                imports,
                mtime,
            },
        ))
    }

    fn save_to_file(&self, path: &Path) -> std::io::Result<()> {
        let file = std::fs::File::create(path)?;
        let mut writer = std::io::BufWriter::new(file);

        // Write header
        writer.write_all(b"DXWC")?; // Magic
        writer.write_all(&(self.hot.len() as u32).to_le_bytes())?; // Count

        // Write entries
        for entry in self.hot.iter() {
            self.save_entry(&mut writer, *entry.key(), entry.value())?;
        }

        writer.flush()?;
        Ok(())
    }

    fn save_entry(
        &self,
        writer: &mut impl std::io::Write,
        module_id: ModuleId,
        transform: &CachedTransform,
    ) -> std::io::Result<()> {
        // Write module ID
        writer.write_all(&module_id.to_le_bytes())?;

        // Write content hash
        writer.write_all(transform.content_hash.as_bytes())?;

        // Write mtime
        writer.write_all(&transform.mtime.to_le_bytes())?;

        // Write transformed size and data
        writer.write_all(&(transform.transformed.len() as u32).to_le_bytes())?;
        writer.write_all(&transform.transformed)?;

        // Write imports
        writer.write_all(&(transform.imports.len() as u32).to_le_bytes())?;
        for import in &transform.imports {
            writer.write_all(&import.to_le_bytes())?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_warm_cache() {
        let temp_dir = std::env::temp_dir().join("dx-cache-test");
        let cache = WarmCache::new(temp_dir.clone());

        // Create a test file
        let test_file = temp_dir.join("test.js");
        std::fs::write(&test_file, b"console.log('test');").unwrap();

        // Create cache entry
        let hash = ContentHash::hash_file(&test_file).unwrap();
        let transform = CachedTransform {
            content_hash: hash,
            transformed: b"console.log('transformed');".to_vec(),
            imports: vec![],
            mtime: 0,
        };

        // Store and retrieve
        cache.put(&test_file, transform.clone());
        let retrieved = cache.get(&test_file).unwrap();

        assert_eq!(retrieved.transformed, transform.transformed);

        // Clean up
        std::fs::remove_dir_all(&temp_dir).ok();
    }
}
