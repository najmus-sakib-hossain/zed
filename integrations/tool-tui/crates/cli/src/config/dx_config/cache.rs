//! Configuration caching logic

use crate::utils::error::DxError;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use super::types::{CACHE_EXTENSION, CachedConfig, DxConfig};

/// Get the cache file path for a config file
pub(crate) fn cache_path(config_path: &Path) -> PathBuf {
    let mut cache_path = config_path.to_path_buf();
    let file_name = cache_path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "config".to_string());
    cache_path.set_file_name(format!(".{}{}", file_name, CACHE_EXTENSION));
    cache_path
}

/// Load configuration from cache if valid
pub(crate) fn load_from_cache(config_path: &Path) -> Option<DxConfig> {
    let cache_path = cache_path(config_path);

    if !cache_path.exists() {
        return None;
    }

    let source_mtime = fs::metadata(config_path)
        .and_then(|m| m.modified())
        .ok()?
        .duration_since(SystemTime::UNIX_EPOCH)
        .ok()?
        .as_secs();

    let cache_data = fs::read(&cache_path).ok()?;
    let cached: CachedConfig = serde_json::from_slice(&cache_data).ok()?;

    if cached.source_mtime >= source_mtime {
        Some(cached.config)
    } else {
        let _ = fs::remove_file(&cache_path);
        None
    }
}

/// Save configuration to cache
pub(crate) fn save_to_cache(config_path: &Path, config: &DxConfig) -> Result<(), DxError> {
    let cache_path = cache_path(config_path);

    let source_mtime = fs::metadata(config_path)
        .and_then(|m| m.modified())
        .map_err(|e| DxError::Io {
            message: e.to_string(),
        })?
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_err(|e| DxError::Io {
            message: e.to_string(),
        })?
        .as_secs();

    let cached = CachedConfig {
        config: config.clone(),
        source_mtime,
    };

    let cache_data = serde_json::to_vec(&cached).map_err(|e| DxError::Io {
        message: e.to_string(),
    })?;

    fs::write(&cache_path, cache_data).map_err(|e| DxError::Io {
        message: e.to_string(),
    })?;

    Ok(())
}

/// Invalidate the cache for a config file
pub fn invalidate_cache(config_path: &Path) {
    let cache_path = cache_path(config_path);
    let _ = fs::remove_file(cache_path);
}

impl DxConfig {
    /// Invalidate the cache for a config file
    pub fn invalidate_cache(config_path: &Path) {
        invalidate_cache(config_path);
    }
}
