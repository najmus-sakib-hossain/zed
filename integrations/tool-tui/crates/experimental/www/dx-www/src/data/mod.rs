//! # Data Loading
//!
//! This module provides the data loader interface and execution.
//!
//! Data loaders allow pages to fetch data before rendering, passing
//! the results as props to components.

use std::collections::HashMap;
use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::Arc;
use std::time::{Duration, Instant};

use dashmap::DashMap;
use serde::{Serialize, de::DeserializeOwned};

use crate::error::{DxError, DxResult};

// =============================================================================
// Data Loader Trait
// =============================================================================

/// Trait for data loaders that fetch data for pages.
pub trait DataLoaderTrait: Send + Sync {
    /// The type of data this loader returns.
    type Data: Serialize + DeserializeOwned + Send + Sync + Clone + 'static;

    /// Load data for the given route parameters.
    fn load(
        &self,
        params: &HashMap<String, String>,
        context: &LoaderContext,
    ) -> Pin<Box<dyn Future<Output = DataLoaderResult<Self::Data>> + Send + '_>>;

    /// Get the cache key for this loader.
    fn cache_key(&self, params: &HashMap<String, String>) -> String {
        let mut key = String::new();
        for (k, v) in params {
            if !key.is_empty() {
                key.push('&');
            }
            key.push_str(k);
            key.push('=');
            key.push_str(v);
        }
        key
    }

    /// Get the cache duration (TTL).
    fn cache_ttl(&self) -> Option<Duration> {
        None // No caching by default
    }

    /// Whether this loader can run in parallel with others.
    fn parallel(&self) -> bool {
        true
    }
}

// =============================================================================
// Loader Context
// =============================================================================

/// Context passed to data loaders.
#[derive(Debug, Clone)]
pub struct LoaderContext {
    /// Request headers
    pub headers: HashMap<String, String>,
    /// Request cookies
    pub cookies: HashMap<String, String>,
    /// Request URL
    pub url: String,
    /// Request method
    pub method: String,
    /// Project root path
    pub project_root: PathBuf,
}

impl Default for LoaderContext {
    fn default() -> Self {
        Self {
            headers: HashMap::new(),
            cookies: HashMap::new(),
            url: String::new(),
            method: "GET".to_string(),
            project_root: PathBuf::new(),
        }
    }
}

// =============================================================================
// Data Loader Registry
// =============================================================================

/// Registry of data loaders for a project.
#[derive(Debug)]
pub struct DataLoader {
    /// Loaders indexed by page path
    loaders: HashMap<String, LoaderInfo>,
    /// Cache for loader results
    cache: Arc<DataLoaderCache>,
}

/// Information about a registered loader.
#[derive(Debug, Clone)]
pub struct LoaderInfo {
    /// Page path this loader is for
    pub page_path: String,
    /// Source file
    pub source_file: PathBuf,
    /// Whether the loader is async
    pub is_async: bool,
    /// Cache TTL
    pub cache_ttl: Option<Duration>,
}

impl DataLoader {
    /// Create a new data loader registry.
    pub fn new() -> Self {
        Self {
            loaders: HashMap::new(),
            cache: Arc::new(DataLoaderCache::new()),
        }
    }

    /// Discover data loaders from parsed components.
    pub fn discover(&mut self, project_root: &PathBuf) -> DxResult<()> {
        // Scan pages directory for components with data loaders
        let pages_dir = project_root.join("pages");
        if pages_dir.exists() {
            self.scan_directory(&pages_dir, &pages_dir)?;
        }
        Ok(())
    }

    /// Scan a directory for components with data loaders.
    fn scan_directory(&mut self, dir: &PathBuf, pages_root: &PathBuf) -> DxResult<()> {
        let entries = std::fs::read_dir(dir).map_err(|e| DxError::IoError {
            path: Some(dir.clone()),
            message: e.to_string(),
        })?;

        for entry in entries {
            let entry = entry.map_err(|e| DxError::IoError {
                path: Some(dir.clone()),
                message: e.to_string(),
            })?;
            let path = entry.path();

            if path.is_dir() {
                self.scan_directory(&path, pages_root)?;
            } else if let Some(ext) = path.extension() {
                if ext == "pg" {
                    self.check_for_loader(&path, pages_root)?;
                }
            }
        }

        Ok(())
    }

    /// Check if a page file has a data loader.
    fn check_for_loader(&mut self, file: &PathBuf, pages_root: &PathBuf) -> DxResult<()> {
        let content = std::fs::read_to_string(file).map_err(|e| DxError::IoError {
            path: Some(file.clone()),
            message: e.to_string(),
        })?;

        // Look for data loader function signatures
        let has_loader = content.contains("pub async fn load")
            || content.contains("pub fn load")
            || content.contains("export async function load")
            || content.contains("async def load")
            || content.contains("func Load");

        if has_loader {
            let relative = file.strip_prefix(pages_root).map_err(|_| DxError::IoError {
                path: Some(file.clone()),
                message: "Failed to get relative path".to_string(),
            })?;

            let page_path = self.file_to_page_path(relative);
            let is_async =
                content.contains("async fn load") || content.contains("async function load");

            self.loaders.insert(
                page_path.clone(),
                LoaderInfo {
                    page_path,
                    source_file: file.clone(),
                    is_async,
                    cache_ttl: None,
                },
            );
        }

        Ok(())
    }

    /// Convert file path to page path.
    fn file_to_page_path(&self, relative: &std::path::Path) -> String {
        let mut path = String::new();

        for component in relative.components() {
            if let std::path::Component::Normal(part) = component {
                let part_str = part.to_string_lossy();
                let name = if let Some(idx) = part_str.rfind('.') {
                    &part_str[..idx]
                } else {
                    &part_str
                };

                if name == "index" {
                    continue;
                }

                path.push('/');
                path.push_str(name);
            }
        }

        if path.is_empty() {
            "/".to_string()
        } else {
            path
        }
    }

    /// Get loader info for a page.
    pub fn get_loader(&self, page_path: &str) -> Option<&LoaderInfo> {
        self.loaders.get(page_path)
    }

    /// Check if a page has a data loader.
    pub fn has_loader(&self, page_path: &str) -> bool {
        self.loaders.contains_key(page_path)
    }

    /// Get all registered loaders.
    pub fn loaders(&self) -> &HashMap<String, LoaderInfo> {
        &self.loaders
    }

    /// Get the cache.
    pub fn cache(&self) -> &Arc<DataLoaderCache> {
        &self.cache
    }
}

impl Default for DataLoader {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Data Loader Cache
// =============================================================================

/// Cache for data loader results.
#[derive(Debug)]
pub struct DataLoaderCache {
    /// Cached entries
    entries: DashMap<String, CachedData>,
    /// Maximum cache size in bytes
    max_size: usize,
    /// Current size in bytes
    current_size: std::sync::atomic::AtomicUsize,
}

/// A cached data entry.
#[derive(Debug, Clone)]
pub struct CachedData {
    /// Serialized data
    pub data: Vec<u8>,
    /// When the entry was created
    pub created_at: Instant,
    /// Time-to-live
    pub ttl: Duration,
    /// Size in bytes
    pub size: usize,
}

impl DataLoaderCache {
    /// Create a new cache.
    pub fn new() -> Self {
        Self {
            entries: DashMap::new(),
            max_size: 100 * 1024 * 1024, // 100MB default
            current_size: std::sync::atomic::AtomicUsize::new(0),
        }
    }

    /// Create a cache with a custom max size.
    pub fn with_max_size(max_size: usize) -> Self {
        Self {
            entries: DashMap::new(),
            max_size,
            current_size: std::sync::atomic::AtomicUsize::new(0),
        }
    }

    /// Get a cached entry.
    pub fn get(&self, key: &str) -> Option<CachedData> {
        self.entries.get(key).and_then(|entry| {
            if entry.created_at.elapsed() < entry.ttl {
                Some(entry.clone())
            } else {
                // Entry expired
                drop(entry);
                self.remove(key);
                None
            }
        })
    }

    /// Set a cached entry.
    pub fn set(&self, key: String, data: Vec<u8>, ttl: Duration) {
        let size = data.len();

        // Check if we need to evict
        while self.current_size.load(std::sync::atomic::Ordering::Relaxed) + size > self.max_size {
            if !self.evict_oldest() {
                break;
            }
        }

        let entry = CachedData {
            data,
            created_at: Instant::now(),
            ttl,
            size,
        };

        if let Some(old) = self.entries.insert(key, entry) {
            self.current_size.fetch_sub(old.size, std::sync::atomic::Ordering::Relaxed);
        }
        self.current_size.fetch_add(size, std::sync::atomic::Ordering::Relaxed);
    }

    /// Remove an entry.
    pub fn remove(&self, key: &str) -> Option<CachedData> {
        self.entries.remove(key).map(|(_, entry)| {
            self.current_size.fetch_sub(entry.size, std::sync::atomic::Ordering::Relaxed);
            entry
        })
    }

    /// Clear all entries.
    pub fn clear(&self) {
        self.entries.clear();
        self.current_size.store(0, std::sync::atomic::Ordering::Relaxed);
    }

    /// Evict the oldest entry.
    fn evict_oldest(&self) -> bool {
        let mut oldest_key: Option<String> = None;
        let mut oldest_time = Instant::now();

        for entry in self.entries.iter() {
            if entry.created_at < oldest_time {
                oldest_time = entry.created_at;
                oldest_key = Some(entry.key().clone());
            }
        }

        if let Some(key) = oldest_key {
            self.remove(&key);
            true
        } else {
            false
        }
    }

    /// Get cache statistics.
    pub fn stats(&self) -> CacheStats {
        CacheStats {
            entries: self.entries.len(),
            size: self.current_size.load(std::sync::atomic::Ordering::Relaxed),
            max_size: self.max_size,
        }
    }
}

impl Default for DataLoaderCache {
    fn default() -> Self {
        Self::new()
    }
}

/// Cache statistics.
#[derive(Debug, Clone)]
pub struct CacheStats {
    /// Number of entries
    pub entries: usize,
    /// Current size in bytes
    pub size: usize,
    /// Maximum size in bytes
    pub max_size: usize,
}

// =============================================================================
// Result Types
// =============================================================================

/// Result type for data loader operations.
pub type DataLoaderResult<T> = Result<T, DataLoaderError>;

/// Error type for data loader operations.
#[derive(Debug, Clone)]
pub enum DataLoaderError {
    /// Data not found
    NotFound(String),
    /// Network error
    Network(String),
    /// Serialization error
    Serialization(String),
    /// Timeout
    Timeout,
    /// Validation error
    Validation(String),
    /// Permission denied
    PermissionDenied(String),
    /// Internal error
    Internal(String),
}

impl std::fmt::Display for DataLoaderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DataLoaderError::NotFound(msg) => write!(f, "Data not found: {}", msg),
            DataLoaderError::Network(msg) => write!(f, "Network error: {}", msg),
            DataLoaderError::Serialization(msg) => write!(f, "Serialization error: {}", msg),
            DataLoaderError::Timeout => write!(f, "Request timeout"),
            DataLoaderError::Validation(msg) => write!(f, "Validation error: {}", msg),
            DataLoaderError::PermissionDenied(msg) => write!(f, "Permission denied: {}", msg),
            DataLoaderError::Internal(msg) => write!(f, "Internal error: {}", msg),
        }
    }
}

impl std::error::Error for DataLoaderError {}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_data_loader_new() {
        let loader = DataLoader::new();
        assert!(loader.loaders().is_empty());
    }

    #[test]
    fn test_cache_set_get() {
        let cache = DataLoaderCache::new();
        let data = vec![1, 2, 3, 4, 5];
        let ttl = Duration::from_secs(60);

        cache.set("test_key".to_string(), data.clone(), ttl);

        let cached = cache.get("test_key");
        assert!(cached.is_some());
        assert_eq!(cached.unwrap().data, data);
    }

    #[test]
    fn test_cache_remove() {
        let cache = DataLoaderCache::new();
        let data = vec![1, 2, 3];
        let ttl = Duration::from_secs(60);

        cache.set("key".to_string(), data, ttl);
        assert!(cache.get("key").is_some());

        cache.remove("key");
        assert!(cache.get("key").is_none());
    }

    #[test]
    fn test_cache_clear() {
        let cache = DataLoaderCache::new();
        let ttl = Duration::from_secs(60);

        cache.set("key1".to_string(), vec![1], ttl);
        cache.set("key2".to_string(), vec![2], ttl);

        assert_eq!(cache.stats().entries, 2);

        cache.clear();
        assert_eq!(cache.stats().entries, 0);
    }

    #[test]
    fn test_loader_context_default() {
        let ctx = LoaderContext::default();
        assert_eq!(ctx.method, "GET");
        assert!(ctx.headers.is_empty());
    }

    #[test]
    fn test_data_loader_error_display() {
        assert_eq!(
            DataLoaderError::NotFound("user".to_string()).to_string(),
            "Data not found: user"
        );
        assert_eq!(DataLoaderError::Timeout.to_string(), "Request timeout");
    }

    #[test]
    fn test_file_to_page_path() {
        let loader = DataLoader::new();

        assert_eq!(loader.file_to_page_path(std::path::Path::new("index.pg")), "/");
        assert_eq!(loader.file_to_page_path(std::path::Path::new("about.pg")), "/about");
        assert_eq!(loader.file_to_page_path(std::path::Path::new("blog/posts.pg")), "/blog/posts");
    }
}
