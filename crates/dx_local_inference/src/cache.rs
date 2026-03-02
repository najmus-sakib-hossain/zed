//! Model cache manager — download, verify, store, and clean GGUF models.

use crate::gguf::{GgufModelInfo, ModelPurpose};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Manages the local model cache directory.
pub struct ModelCache {
    cache_dir: PathBuf,
    manifest: CacheManifest,
}

/// Tracks all cached models and their state.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct CacheManifest {
    /// Model ID → cached model info
    pub models: HashMap<String, CachedModel>,
    /// Total cache size in bytes
    pub total_size: u64,
    /// Maximum cache size in bytes (configurable)
    pub max_size: u64,
}

/// A model stored in the cache.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedModel {
    pub model_id: String,
    pub path: PathBuf,
    pub size_bytes: u64,
    pub sha256: Option<String>,
    pub last_used: u64,
    pub download_complete: bool,
    pub purpose: ModelPurpose,
}

impl ModelCache {
    /// Create a new model cache at the given directory.
    pub fn new(cache_dir: PathBuf) -> Result<Self> {
        std::fs::create_dir_all(&cache_dir)?;

        let manifest_path = cache_dir.join("manifest.json");
        let manifest = if manifest_path.exists() {
            let data = std::fs::read_to_string(&manifest_path)?;
            serde_json::from_str(&data).unwrap_or_default()
        } else {
            CacheManifest {
                max_size: 50 * 1024 * 1024 * 1024, // 50 GB default
                ..Default::default()
            }
        };

        Ok(Self { cache_dir, manifest })
    }

    /// Get the default cache directory (~/.dx/models/).
    pub fn default_dir() -> PathBuf {
        let home = std::env::var("DX_HOME")
            .or_else(|_| std::env::var("HOME"))
            .or_else(|_| std::env::var("USERPROFILE"))
            .unwrap_or_else(|_| ".".to_string());
        PathBuf::from(home).join(".dx").join("models")
    }

    /// Check if a model is cached and complete.
    pub fn is_cached(&self, model_id: &str) -> bool {
        self.manifest
            .models
            .get(model_id)
            .map(|m| m.download_complete)
            .unwrap_or(false)
    }

    /// Get the local path for a cached model.
    pub fn model_path(&self, model_id: &str) -> Option<PathBuf> {
        self.manifest
            .models
            .get(model_id)
            .filter(|m| m.download_complete)
            .map(|m| m.path.clone())
    }

    /// Register a new model in the cache.
    pub fn register(&mut self, info: &GgufModelInfo, path: PathBuf, sha256: Option<String>) -> Result<()> {
        let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);

        let cached = CachedModel {
            model_id: info.model_id.clone(),
            path,
            size_bytes: size,
            sha256,
            last_used: current_timestamp(),
            download_complete: true,
            purpose: info.purpose,
        };

        self.manifest.total_size += size;
        self.manifest.models.insert(info.model_id.clone(), cached);
        self.save_manifest()?;
        Ok(())
    }

    /// Mark a model as recently used.
    pub fn touch(&mut self, model_id: &str) {
        if let Some(model) = self.manifest.models.get_mut(model_id) {
            model.last_used = current_timestamp();
        }
    }

    /// Remove a specific model from the cache.
    pub fn remove(&mut self, model_id: &str) -> Result<()> {
        if let Some(model) = self.manifest.models.remove(model_id) {
            self.manifest.total_size = self.manifest.total_size.saturating_sub(model.size_bytes);
            if model.path.exists() {
                std::fs::remove_file(&model.path)?;
            }
            self.save_manifest()?;
        }
        Ok(())
    }

    /// Evict least-recently-used models until under the size limit.
    pub fn evict_to_fit(&mut self, needed_bytes: u64) -> Result<Vec<String>> {
        let mut evicted = Vec::new();
        let target_size = self.manifest.max_size.saturating_sub(needed_bytes);

        while self.manifest.total_size > target_size {
            // Find LRU model
            let lru_id = self
                .manifest
                .models
                .iter()
                .min_by_key(|(_, m)| m.last_used)
                .map(|(id, _)| id.clone());

            match lru_id {
                Some(id) => {
                    evicted.push(id.clone());
                    self.remove(&id)?;
                }
                None => break,
            }
        }

        Ok(evicted)
    }

    /// List all cached models.
    pub fn list(&self) -> Vec<&CachedModel> {
        self.manifest.models.values().collect()
    }

    /// Get total cache size in bytes.
    pub fn total_size(&self) -> u64 {
        self.manifest.total_size
    }

    /// Get available space before hitting limit.
    pub fn available_space(&self) -> u64 {
        self.manifest.max_size.saturating_sub(self.manifest.total_size)
    }

    /// File path for a model download destination.
    pub fn download_path(&self, model_id: &str, filename: &str) -> PathBuf {
        self.cache_dir.join(model_id).join(filename)
    }

    fn save_manifest(&self) -> Result<()> {
        let manifest_path = self.cache_dir.join("manifest.json");
        let data = serde_json::to_string_pretty(&self.manifest)?;
        std::fs::write(manifest_path, data)?;
        Ok(())
    }
}

fn current_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
