//! # Build Cache
//!
//! Implements content-based caching for incremental compilation.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use serde::{Deserialize, Serialize};

use crate::build::BinaryObject;
use crate::error::DxResult;
use crate::parser::ComponentType;

/// Build cache for incremental compilation.
pub struct BuildCache {
    /// Cache entries by file path
    entries: HashMap<PathBuf, CacheEntry>,
    /// Whether cache has been modified
    dirty: bool,
}

/// A cache entry for a compiled file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry {
    /// Source file path
    pub source_path: PathBuf,
    /// Output file path
    pub output_path: PathBuf,
    /// Content hash of source
    pub source_hash: String,
    /// Content hash of output
    pub output_hash: String,
    /// Last modified time
    pub mtime: u64,
    /// Component type
    pub component_type: u8,
    /// Output size in bytes
    pub size: usize,
    /// Dependencies
    pub dependencies: Vec<PathBuf>,
}

impl CacheEntry {
    /// Convert to BinaryObject.
    pub fn to_binary_object(&self) -> BinaryObject {
        BinaryObject {
            path: self.output_path.clone(),
            size: self.size,
            hash: self.output_hash.clone(),
            dependencies: self
                .dependencies
                .iter()
                .map(|p| p.to_string_lossy().to_string())
                .collect(),
            component_type: match self.component_type {
                0x01 => ComponentType::Page,
                0x02 => ComponentType::Component,
                0x03 => ComponentType::Layout,
                _ => ComponentType::Component,
            },
        }
    }
}

/// Cache file format
#[derive(Debug, Serialize, Deserialize)]
struct CacheFile {
    version: u32,
    entries: Vec<CacheEntry>,
}

impl BuildCache {
    /// Create a new empty cache.
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
            dirty: false,
        }
    }

    /// Load cache from disk.
    pub async fn load(&mut self, cache_dir: &Path) -> DxResult<()> {
        let cache_file = cache_dir.join("build-cache.json");

        if !cache_file.exists() {
            return Ok(());
        }

        let content = tokio::fs::read_to_string(&cache_file).await?;
        let cache: CacheFile =
            serde_json::from_str(&content).map_err(|e| crate::error::DxError::CacheError {
                message: format!("Failed to parse cache file: {}", e),
            })?;

        // Validate cache version
        if cache.version != 1 {
            tracing::warn!("Cache version mismatch, ignoring cache");
            return Ok(());
        }

        // Load entries
        for entry in cache.entries {
            self.entries.insert(entry.source_path.clone(), entry);
        }

        Ok(())
    }

    /// Save cache to disk.
    pub async fn save(&self, cache_dir: &Path) -> DxResult<()> {
        if !self.dirty {
            return Ok(());
        }

        tokio::fs::create_dir_all(cache_dir).await?;

        let cache_file = cache_dir.join("build-cache.json");
        let cache = CacheFile {
            version: 1,
            entries: self.entries.values().cloned().collect(),
        };

        let content = serde_json::to_string_pretty(&cache).map_err(|e| {
            crate::error::DxError::CacheError {
                message: format!("Failed to serialize cache: {}", e),
            }
        })?;

        tokio::fs::write(&cache_file, content).await?;

        Ok(())
    }

    /// Check if a file is cached and still valid.
    pub async fn check(&self, path: &Path) -> DxResult<Option<CacheEntry>> {
        if let Some(entry) = self.entries.get(path) {
            // Check if source file still exists
            if !path.exists() {
                return Ok(None);
            }

            // Check if mtime has changed
            let metadata = tokio::fs::metadata(path).await?;
            let mtime = metadata
                .modified()
                .ok()
                .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
                .map(|d| d.as_secs())
                .unwrap_or(0);

            if mtime != entry.mtime {
                // mtime changed, check hash
                let content = tokio::fs::read(path).await?;
                let hash = blake3::hash(&content).to_hex()[..16].to_string();

                if hash != entry.source_hash {
                    return Ok(None);
                }
            }

            // Check if output file exists
            if !entry.output_path.exists() {
                return Ok(None);
            }

            // Check dependencies
            for dep in &entry.dependencies {
                if !dep.exists() {
                    return Ok(None);
                }

                // Check dep mtime
                if let Ok(dep_meta) = tokio::fs::metadata(dep).await {
                    let dep_mtime = dep_meta
                        .modified()
                        .ok()
                        .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
                        .map(|d| d.as_secs())
                        .unwrap_or(0);

                    if dep_mtime > entry.mtime {
                        return Ok(None);
                    }
                }
            }

            return Ok(Some(entry.clone()));
        }

        Ok(None)
    }

    /// Get a cached entry.
    pub fn get(&self, path: &Path) -> Option<&CacheEntry> {
        self.entries.get(path)
    }

    /// Set a cache entry.
    pub async fn set(&mut self, path: PathBuf, compiled: crate::build::CompiledComponent) {
        let metadata = tokio::fs::metadata(&path).await.ok();
        let mtime = metadata
            .and_then(|m| m.modified().ok())
            .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);

        // Calculate source hash
        let source_hash = if let Ok(content) = tokio::fs::read(&path).await {
            blake3::hash(&content).to_hex()[..16].to_string()
        } else {
            String::new()
        };

        let entry = CacheEntry {
            source_path: path.clone(),
            output_path: PathBuf::new(), // Set by caller
            source_hash,
            output_hash: compiled.hash.clone(),
            mtime,
            component_type: match compiled.parsed.component_type {
                ComponentType::Page => 0x01,
                ComponentType::Component => 0x02,
                ComponentType::Layout => 0x03,
            },
            size: compiled.script_bytes.len()
                + compiled.template_bytes.len()
                + compiled.style_bytes.len(),
            dependencies: compiled.dependencies.iter().map(PathBuf::from).collect(),
        };

        self.entries.insert(path, entry);
        self.dirty = true;
    }

    /// Invalidate a cache entry.
    pub fn invalidate(&mut self, path: &Path) {
        if self.entries.remove(path).is_some() {
            self.dirty = true;
        }
    }

    /// Invalidate all entries that depend on a file.
    pub fn invalidate_dependents(&mut self, path: &Path) {
        let mut to_invalidate = Vec::new();

        for (source_path, entry) in &self.entries {
            if entry.dependencies.contains(&path.to_path_buf()) {
                to_invalidate.push(source_path.clone());
            }
        }

        for path in to_invalidate {
            self.entries.remove(&path);
            self.dirty = true;
        }
    }

    /// Clear all cache entries.
    pub fn clear(&mut self) {
        self.entries.clear();
        self.dirty = true;
    }

    /// Get cache statistics.
    pub fn stats(&self) -> CacheStats {
        let total_size: usize = self.entries.values().map(|e| e.size).sum();

        CacheStats {
            entry_count: self.entries.len(),
            total_size,
        }
    }

    /// Prune stale entries (source files no longer exist).
    pub async fn prune(&mut self) -> usize {
        let mut stale = Vec::new();

        for path in self.entries.keys() {
            if !path.exists() {
                stale.push(path.clone());
            }
        }

        let count = stale.len();
        for path in stale {
            self.entries.remove(&path);
        }

        if count > 0 {
            self.dirty = true;
        }

        count
    }
}

impl Default for BuildCache {
    fn default() -> Self {
        Self::new()
    }
}

/// Cache statistics.
#[derive(Debug, Clone)]
pub struct CacheStats {
    /// Number of cached entries
    pub entry_count: usize,
    /// Total size of cached output in bytes
    pub total_size: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_new() {
        let cache = BuildCache::new();
        assert_eq!(cache.entries.len(), 0);
        assert!(!cache.dirty);
    }

    #[test]
    fn test_cache_stats() {
        let cache = BuildCache::new();
        let stats = cache.stats();
        assert_eq!(stats.entry_count, 0);
        assert_eq!(stats.total_size, 0);
    }

    #[test]
    fn test_cache_invalidate() {
        let mut cache = BuildCache::new();

        let entry = CacheEntry {
            source_path: PathBuf::from("/test/file.pg"),
            output_path: PathBuf::from("/test/file.dxob"),
            source_hash: "abc123".to_string(),
            output_hash: "def456".to_string(),
            mtime: 12345,
            component_type: 0x01,
            size: 100,
            dependencies: vec![],
        };

        cache.entries.insert(entry.source_path.clone(), entry);
        assert_eq!(cache.entries.len(), 1);

        cache.invalidate(Path::new("/test/file.pg"));
        assert_eq!(cache.entries.len(), 0);
        assert!(cache.dirty);
    }

    #[test]
    fn test_cache_clear() {
        let mut cache = BuildCache::new();

        cache.entries.insert(
            PathBuf::from("/a"),
            CacheEntry {
                source_path: PathBuf::from("/a"),
                output_path: PathBuf::from("/a.dxob"),
                source_hash: "a".to_string(),
                output_hash: "a".to_string(),
                mtime: 0,
                component_type: 0x01,
                size: 10,
                dependencies: vec![],
            },
        );

        cache.entries.insert(
            PathBuf::from("/b"),
            CacheEntry {
                source_path: PathBuf::from("/b"),
                output_path: PathBuf::from("/b.dxob"),
                source_hash: "b".to_string(),
                output_hash: "b".to_string(),
                mtime: 0,
                component_type: 0x02,
                size: 20,
                dependencies: vec![],
            },
        );

        assert_eq!(cache.entries.len(), 2);

        cache.clear();
        assert_eq!(cache.entries.len(), 0);
        assert!(cache.dirty);
    }

    #[test]
    fn test_cache_entry_to_binary_object() {
        let entry = CacheEntry {
            source_path: PathBuf::from("/test/page.pg"),
            output_path: PathBuf::from("/out/page.dxob"),
            source_hash: "src123".to_string(),
            output_hash: "out456".to_string(),
            mtime: 999,
            component_type: 0x01,
            size: 500,
            dependencies: vec![PathBuf::from("/test/shared.cp")],
        };

        let obj = entry.to_binary_object();
        assert_eq!(obj.path, PathBuf::from("/out/page.dxob"));
        assert_eq!(obj.size, 500);
        assert_eq!(obj.hash, "out456");
        assert_eq!(obj.dependencies.len(), 1);
        assert!(matches!(obj.component_type, ComponentType::Page));
    }
}
