//! DX Tool Cache Manager
//!
//! Manages the `.dx` folder structure for all DX tools, providing:
//! - Per-tool cache directories
//! - Warm start capabilities (10x faster than cold builds)
//! - R2 cloud sync for shared caches
//! - Blake3-based content hashing

use anyhow::{Context, Result};
use blake3::Hasher;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::SystemTime;

use crate::storage::blob::Blob;
use crate::storage::r2::{R2Config, R2Storage};

/// DX Tool identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DxToolId {
    /// Global cache
    Cache,
    /// Build orchestrator
    Forge,
    /// JS/TS bundler
    Bundler,
    /// npm packages (node_modules)
    NodeModules,
    /// Test runner
    Test,
    /// Binary CSS
    Style,
    /// SVG icons
    Icon,
    /// Fonts
    Font,
    /// Media (images/videos)
    Media,
    /// Internationalization
    I18n,
    /// UI components
    Ui,
    /// Serializer cache
    Serializer,
    /// Code generator
    Generator,
    /// AI task system
    Driven,
    /// Monorepo workspace
    Workspace,
    /// WWW framework (HTIP)
    Www,
}

impl DxToolId {
    /// Get folder name for this tool
    pub fn folder_name(&self) -> &'static str {
        match self {
            DxToolId::Cache => "cache",
            DxToolId::Forge => "forge",
            DxToolId::Bundler => "bundler",
            DxToolId::NodeModules => "node_modules",
            DxToolId::Test => "test",
            DxToolId::Style => "style",
            DxToolId::Icon => "icon",
            DxToolId::Font => "font",
            DxToolId::Media => "media",
            DxToolId::I18n => "i18n",
            DxToolId::Ui => "ui",
            DxToolId::Serializer => "serializer",
            DxToolId::Generator => "generator",
            DxToolId::Driven => "driven",
            DxToolId::Workspace => "workspace",
            DxToolId::Www => "www",
        }
    }

    /// Get all tool IDs
    pub fn all() -> &'static [DxToolId] {
        &[
            DxToolId::Cache,
            DxToolId::Forge,
            DxToolId::Bundler,
            DxToolId::NodeModules,
            DxToolId::Test,
            DxToolId::Style,
            DxToolId::Icon,
            DxToolId::Font,
            DxToolId::Media,
            DxToolId::I18n,
            DxToolId::Ui,
            DxToolId::Serializer,
            DxToolId::Generator,
            DxToolId::Driven,
            DxToolId::Workspace,
            DxToolId::Www,
        ]
    }
}

/// Cache entry metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry {
    /// Content hash (Blake3)
    pub hash: String,
    /// Original file path
    pub source_path: PathBuf,
    /// Cached file path
    pub cached_path: PathBuf,
    /// Creation timestamp
    pub created_at: u64,
    /// Last accessed timestamp
    pub last_accessed: u64,
    /// Size in bytes
    pub size: u64,
    /// Whether synced to R2
    pub synced_to_r2: bool,
}

/// Tool cache statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStats {
    pub tool: String,
    pub entries: usize,
    pub total_size: u64,
    pub hits: u64,
    pub misses: u64,
    pub hit_rate: f64,
    pub warm_start_time_ms: u64,
    pub cold_start_time_ms: u64,
    pub speedup: f64,
}

/// DX Tool Cache Manager
///
/// Manages the `.dx` folder structure and provides caching for all DX tools.
pub struct DxToolCacheManager {
    /// Root .dx directory
    dx_root: PathBuf,
    /// Per-tool cache directories
    tool_dirs: HashMap<DxToolId, PathBuf>,
    /// Cache index (in-memory)
    cache_index: Arc<RwLock<HashMap<String, CacheEntry>>>,
    /// Per-tool statistics
    stats: Arc<RwLock<HashMap<DxToolId, CacheStats>>>,
    /// R2 config (optional)
    r2_config: Option<R2Config>,
}

impl DxToolCacheManager {
    /// Create new cache manager for a project
    pub fn new(project_root: &Path) -> Result<Self> {
        let dx_root = project_root.join(".dx");
        std::fs::create_dir_all(&dx_root)?;

        // Create all tool directories
        let mut tool_dirs = HashMap::new();
        for tool_id in DxToolId::all() {
            let tool_dir = dx_root.join(tool_id.folder_name());
            std::fs::create_dir_all(&tool_dir)?;
            tool_dirs.insert(*tool_id, tool_dir);
        }

        // Load cache index if exists
        let index_path = dx_root.join("cache").join("index.json");
        let cache_index = if index_path.exists() {
            let content = std::fs::read_to_string(&index_path)?;
            serde_json::from_str(&content).unwrap_or_default()
        } else {
            HashMap::new()
        };

        // Load R2 config from environment (optional)
        let r2_config = R2Config::from_env().ok();

        Ok(Self {
            dx_root,
            tool_dirs,
            cache_index: Arc::new(RwLock::new(cache_index)),
            stats: Arc::new(RwLock::new(HashMap::new())),
            r2_config,
        })
    }

    /// Get the .dx root directory
    pub fn dx_root(&self) -> &Path {
        &self.dx_root
    }

    /// Get tool cache directory
    pub fn tool_dir(&self, tool: DxToolId) -> Option<&Path> {
        self.tool_dirs.get(&tool).map(|p| p.as_path())
    }

    /// Compute content hash using Blake3
    pub fn hash_content(content: &[u8]) -> String {
        let mut hasher = Hasher::new();
        hasher.update(content);
        hasher.finalize().to_hex().to_string()
    }

    /// Compute file hash
    pub fn hash_file(path: &Path) -> Result<String> {
        let content = std::fs::read(path)?;
        Ok(Self::hash_content(&content))
    }

    /// Check if content is cached
    pub fn is_cached(&self, hash: &str) -> bool {
        self.cache_index.read().contains_key(hash)
    }

    /// Get cached content path
    pub fn get_cached_path(&self, tool: DxToolId, hash: &str) -> Option<PathBuf> {
        let key = format!("{}:{}", tool.folder_name(), hash);
        self.cache_index.read().get(&key).map(|e| e.cached_path.clone())
    }

    /// Store content in cache
    pub fn cache_content(
        &self,
        tool: DxToolId,
        source_path: &Path,
        content: &[u8],
    ) -> Result<CacheEntry> {
        let hash = Self::hash_content(content);
        let tool_dir = self.tool_dirs.get(&tool).context("Tool not found")?;

        // Store in hash-based directory structure (first 2 chars as dir)
        let cache_subdir = tool_dir.join(&hash[..2]);
        std::fs::create_dir_all(&cache_subdir)?;

        let cached_path = cache_subdir.join(&hash[2..]);
        std::fs::write(&cached_path, content)?;

        let now = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs();

        let entry = CacheEntry {
            hash: hash.clone(),
            source_path: source_path.to_path_buf(),
            cached_path: cached_path.clone(),
            created_at: now,
            last_accessed: now,
            size: content.len() as u64,
            synced_to_r2: false,
        };

        let key = format!("{}:{}", tool.folder_name(), hash);
        self.cache_index.write().insert(key, entry.clone());
        self.save_index()?;

        Ok(entry)
    }

    /// Get cached content
    pub fn get_cached_content(&self, tool: DxToolId, hash: &str) -> Result<Option<Vec<u8>>> {
        let key = format!("{}:{}", tool.folder_name(), hash);

        let mut index = self.cache_index.write();
        if let Some(entry) = index.get_mut(&key) {
            // Update access time
            entry.last_accessed =
                SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs();

            // Update hit stats
            let mut stats = self.stats.write();
            if let Some(s) = stats.get_mut(&tool) {
                s.hits += 1;
                s.hit_rate = s.hits as f64 / (s.hits + s.misses) as f64;
            }

            let content = std::fs::read(&entry.cached_path)?;
            Ok(Some(content))
        } else {
            // Update miss stats
            let mut stats = self.stats.write();
            if let Some(s) = stats.get_mut(&tool) {
                s.misses += 1;
                s.hit_rate = s.hits as f64 / (s.hits + s.misses) as f64;
            }
            Ok(None)
        }
    }

    /// Save cache index to disk
    fn save_index(&self) -> Result<()> {
        let index_path = self.dx_root.join("cache").join("index.json");
        let content = serde_json::to_string_pretty(&*self.cache_index.read())?;
        std::fs::write(&index_path, content)?;
        Ok(())
    }

    /// Initialize warm start for a tool
    ///
    /// Loads the cache index and prepares for fast builds
    pub fn warm_start(&self, tool: DxToolId) -> Result<WarmStartResult> {
        let start = std::time::Instant::now();

        let _tool_dir = self.tool_dirs.get(&tool).context("Tool not found")?;

        // Count cached entries for this tool
        let prefix = format!("{}:", tool.folder_name());
        let entries: Vec<_> = self
            .cache_index
            .read()
            .iter()
            .filter(|(k, _)| k.starts_with(&prefix))
            .map(|(_, v)| v.clone())
            .collect();

        let total_size: u64 = entries.iter().map(|e| e.size).sum();
        let duration = start.elapsed();

        Ok(WarmStartResult {
            tool: tool.folder_name().to_string(),
            cached_entries: entries.len(),
            total_size,
            load_time_ms: duration.as_millis() as u64,
            ready: !entries.is_empty(),
        })
    }

    /// Get statistics for a tool
    pub fn get_stats(&self, tool: DxToolId) -> Option<CacheStats> {
        self.stats.read().get(&tool).cloned()
    }

    /// Clear cache for a specific tool
    pub fn clear_tool_cache(&self, tool: DxToolId) -> Result<()> {
        let tool_dir = self.tool_dirs.get(&tool).context("Tool not found")?;

        // Remove all files in tool directory
        if tool_dir.exists() {
            std::fs::remove_dir_all(tool_dir)?;
            std::fs::create_dir_all(tool_dir)?;
        }

        // Remove from index
        let prefix = format!("{}:", tool.folder_name());
        self.cache_index.write().retain(|k, _| !k.starts_with(&prefix));
        self.save_index()?;

        Ok(())
    }

    /// Clear all caches
    pub fn clear_all(&self) -> Result<()> {
        for tool_id in DxToolId::all() {
            self.clear_tool_cache(*tool_id)?;
        }
        Ok(())
    }

    /// Get R2 bucket name (if configured)
    pub fn r2_bucket(&self) -> Option<&str> {
        self.r2_config.as_ref().map(|c| c.bucket_name.as_str())
    }

    /// Check if R2 is configured
    pub fn is_r2_configured(&self) -> bool {
        self.r2_config.is_some()
    }

    /// Sync tool cache to R2
    ///
    /// Uploads all unsynced cache entries for the specified tool to R2.
    /// Uses content-hash deduplication to skip already-uploaded entries.
    pub async fn sync_to_r2(&self, tool: DxToolId) -> Result<SyncResult> {
        let r2_config = self.r2_config.as_ref()
            .ok_or_else(|| anyhow::anyhow!("R2 not configured. Set R2_ACCOUNT_ID, R2_BUCKET_NAME, R2_ACCESS_KEY_ID, and R2_SECRET_ACCESS_KEY environment variables."))?;

        let storage =
            R2Storage::new(r2_config.clone()).context("Failed to create R2 storage client")?;

        let prefix = format!("{}:", tool.folder_name());
        let entries: Vec<CacheEntry> = self
            .cache_index
            .read()
            .iter()
            .filter(|(k, v)| k.starts_with(&prefix) && !v.synced_to_r2)
            .map(|(_, v)| v.clone())
            .collect();

        if entries.is_empty() {
            return Ok(SyncResult {
                uploaded: 0,
                skipped: 0,
                failed: 0,
            });
        }

        let mut uploaded = 0;
        let mut skipped = 0;
        let mut failed = 0;

        for entry in entries {
            // Read cached content
            let content = match std::fs::read(&entry.cached_path) {
                Ok(c) => c,
                Err(e) => {
                    tracing::warn!("Failed to read cache entry {}: {}", entry.hash, e);
                    failed += 1;
                    continue;
                }
            };

            // Create blob and check if it exists in R2
            let blob = Blob::from_content(&entry.hash, content);
            let hash = blob.hash();

            match storage.blob_exists(hash).await {
                Ok(true) => {
                    // Already in R2, mark as synced
                    skipped += 1;
                    let key = format!("{}:{}", tool.folder_name(), entry.hash);
                    if let Some(e) = self.cache_index.write().get_mut(&key) {
                        e.synced_to_r2 = true;
                    }
                }
                Ok(false) => {
                    // Upload to R2
                    match storage.upload_blob(&blob).await {
                        Ok(_) => {
                            uploaded += 1;
                            let key = format!("{}:{}", tool.folder_name(), entry.hash);
                            if let Some(e) = self.cache_index.write().get_mut(&key) {
                                e.synced_to_r2 = true;
                            }
                        }
                        Err(e) => {
                            tracing::warn!("Failed to upload blob {}: {}", hash, e);
                            failed += 1;
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!("Failed to check blob existence {}: {}", hash, e);
                    failed += 1;
                }
            }
        }

        // Save updated index
        if let Err(e) = self.save_index() {
            tracing::warn!("Failed to save cache index after R2 sync: {}", e);
        }

        Ok(SyncResult {
            uploaded,
            skipped,
            failed,
        })
    }

    /// Pull tool cache from R2
    ///
    /// Downloads cache entries from R2 that are not present locally.
    /// Verifies integrity using content hashes.
    pub async fn pull_from_r2(&self, tool: DxToolId) -> Result<SyncResult> {
        let r2_config = self.r2_config.as_ref()
            .ok_or_else(|| anyhow::anyhow!("R2 not configured. Set R2_ACCOUNT_ID, R2_BUCKET_NAME, R2_ACCESS_KEY_ID, and R2_SECRET_ACCESS_KEY environment variables."))?;

        let storage =
            R2Storage::new(r2_config.clone()).context("Failed to create R2 storage client")?;

        // List blobs in R2 for this tool
        let prefix = format!("blobs/{}/", tool.folder_name());
        let remote_hashes =
            storage.list_blobs(Some(&prefix)).await.context("Failed to list R2 blobs")?;

        if remote_hashes.is_empty() {
            return Ok(SyncResult {
                uploaded: 0,
                skipped: 0,
                failed: 0,
            });
        }

        let tool_dir = self
            .tool_dirs
            .get(&tool)
            .ok_or_else(|| anyhow::anyhow!("Tool directory not found for {:?}", tool))?;

        let mut uploaded = 0; // "uploaded" here means downloaded/added locally
        let mut skipped = 0;
        let mut failed = 0;

        for hash in remote_hashes {
            let key = format!("{}:{}", tool.folder_name(), hash);

            // Check if we already have this entry
            if self.cache_index.read().contains_key(&key) {
                skipped += 1;
                continue;
            }

            // Download from R2
            match storage.download_blob(&hash).await {
                Ok(blob) => {
                    let content = match blob.to_binary() {
                        Ok(c) => c,
                        Err(e) => {
                            tracing::warn!("Failed to serialize blob {}: {}", hash, e);
                            failed += 1;
                            continue;
                        }
                    };

                    // Verify hash
                    let computed_hash = Self::hash_content(&content);
                    if computed_hash != hash {
                        tracing::warn!(
                            "Hash mismatch for blob {}: expected {}, got {}",
                            hash,
                            hash,
                            computed_hash
                        );
                        failed += 1;
                        continue;
                    }

                    // Store locally
                    let cache_subdir = tool_dir.join(&hash[..2.min(hash.len())]);
                    if let Err(e) = std::fs::create_dir_all(&cache_subdir) {
                        tracing::warn!("Failed to create cache subdir: {}", e);
                        failed += 1;
                        continue;
                    }

                    let cached_path = cache_subdir.join(&hash[2.min(hash.len())..]);
                    if let Err(e) = std::fs::write(&cached_path, &content) {
                        tracing::warn!("Failed to write cache entry: {}", e);
                        failed += 1;
                        continue;
                    }

                    let now = SystemTime::now()
                        .duration_since(SystemTime::UNIX_EPOCH)
                        .map(|d| d.as_secs())
                        .unwrap_or(0);

                    let entry = CacheEntry {
                        hash: hash.clone(),
                        source_path: PathBuf::new(), // Unknown source for pulled entries
                        cached_path,
                        created_at: now,
                        last_accessed: now,
                        size: content.len() as u64,
                        synced_to_r2: true,
                    };

                    self.cache_index.write().insert(key, entry);
                    uploaded += 1;
                }
                Err(e) => {
                    tracing::warn!("Failed to download blob {}: {}", hash, e);
                    failed += 1;
                }
            }
        }

        // Save updated index
        if let Err(e) = self.save_index() {
            tracing::warn!("Failed to save cache index after R2 pull: {}", e);
        }

        Ok(SyncResult {
            uploaded,
            skipped,
            failed,
        })
    }
}

/// Warm start result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WarmStartResult {
    pub tool: String,
    pub cached_entries: usize,
    pub total_size: u64,
    pub load_time_ms: u64,
    pub ready: bool,
}

/// R2 sync result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncResult {
    pub uploaded: usize,
    pub skipped: usize,
    pub failed: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_create_dx_folder_structure() {
        let temp_dir = TempDir::new().unwrap();
        let manager = DxToolCacheManager::new(temp_dir.path()).unwrap();

        // Check all tool directories were created
        for tool_id in DxToolId::all() {
            let tool_dir = manager.tool_dir(*tool_id);
            assert!(tool_dir.is_some());
            assert!(tool_dir.unwrap().exists());
        }
    }

    #[test]
    fn test_cache_content() {
        let temp_dir = TempDir::new().unwrap();
        let manager = DxToolCacheManager::new(temp_dir.path()).unwrap();

        let content = b"test content";
        let source = temp_dir.path().join("test.txt");

        let entry = manager.cache_content(DxToolId::Bundler, &source, content).unwrap();
        assert!(!entry.hash.is_empty());
        assert!(entry.cached_path.exists());

        // Retrieve cached content
        let cached = manager.get_cached_content(DxToolId::Bundler, &entry.hash).unwrap();
        assert_eq!(cached, Some(content.to_vec()));
    }

    #[test]
    fn test_warm_start() {
        let temp_dir = TempDir::new().unwrap();
        let manager = DxToolCacheManager::new(temp_dir.path()).unwrap();

        // Add some cached content
        for i in 0..5 {
            let content = format!("content {}", i);
            let source = temp_dir.path().join(format!("file{}.txt", i));
            manager.cache_content(DxToolId::Style, &source, content.as_bytes()).unwrap();
        }

        let result = manager.warm_start(DxToolId::Style).unwrap();
        assert_eq!(result.cached_entries, 5);
        assert!(result.ready);
    }

    #[test]
    fn test_clear_cache() {
        let temp_dir = TempDir::new().unwrap();
        let manager = DxToolCacheManager::new(temp_dir.path()).unwrap();

        let content = b"test";
        let source = temp_dir.path().join("test.txt");
        manager.cache_content(DxToolId::Font, &source, content).unwrap();

        manager.clear_tool_cache(DxToolId::Font).unwrap();

        let result = manager.warm_start(DxToolId::Font).unwrap();
        assert_eq!(result.cached_entries, 0);
    }
}
