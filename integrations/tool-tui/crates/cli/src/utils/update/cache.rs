//! Update cache for storing version check results
//!
//! Caches update check results for 24 hours to avoid excessive API calls.
//! Feature: cli-production-ready, Task 6.2-6.3, 6.5
//! Validates: Requirements 9.5

use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::utils::error::DxError;

/// Default cache TTL: 24 hours
const CACHE_TTL_SECS: u64 = 24 * 60 * 60;

/// Cached update check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateCache {
    /// Unix timestamp when the check was performed
    pub checked_at: u64,
    /// Latest version found (if any)
    pub latest_version: Option<String>,
    /// Whether an update is available
    pub update_available: bool,
    /// Release notes summary (if update available)
    pub release_notes: Option<String>,
    /// Current version at time of check
    pub current_version: String,
}

impl UpdateCache {
    /// Create a new cache entry for "up to date"
    pub fn up_to_date(current_version: &str) -> Self {
        Self {
            checked_at: current_timestamp(),
            latest_version: None,
            update_available: false,
            release_notes: None,
            current_version: current_version.to_string(),
        }
    }

    /// Create a new cache entry for "update available"
    pub fn update_available(
        current_version: &str,
        latest_version: &str,
        release_notes: Option<String>,
    ) -> Self {
        Self {
            checked_at: current_timestamp(),
            latest_version: Some(latest_version.to_string()),
            update_available: true,
            release_notes,
            current_version: current_version.to_string(),
        }
    }

    /// Check if the cache is still valid (within TTL)
    pub fn is_valid(&self) -> bool {
        let now = current_timestamp();
        now.saturating_sub(self.checked_at) < CACHE_TTL_SECS
    }

    /// Get age of the cache in seconds
    pub fn age_secs(&self) -> u64 {
        current_timestamp().saturating_sub(self.checked_at)
    }
}

/// Result of an update check
#[derive(Debug, Clone)]
pub enum UpdateResult {
    /// Current version is up to date
    UpToDate,
    /// A new version is available
    UpdateAvailable {
        /// New version number
        new_version: String,
        /// Release notes summary
        release_notes: Option<String>,
    },
}

/// Manager for update check caching
pub struct UpdateCacheManager {
    /// Path to the cache file
    cache_path: PathBuf,
    /// Cache TTL in seconds
    ttl_secs: u64,
}

impl UpdateCacheManager {
    /// Create a new cache manager with default cache location
    pub fn new() -> Self {
        let cache_dir = dirs::cache_dir().unwrap_or_else(|| PathBuf::from(".")).join("dx");

        Self {
            cache_path: cache_dir.join("update_cache.json"),
            ttl_secs: CACHE_TTL_SECS,
        }
    }

    /// Create a cache manager with custom path (for testing)
    pub fn with_path(cache_path: PathBuf) -> Self {
        Self {
            cache_path,
            ttl_secs: CACHE_TTL_SECS,
        }
    }

    /// Set custom TTL (for testing)
    pub fn with_ttl(mut self, ttl_secs: u64) -> Self {
        self.ttl_secs = ttl_secs;
        self
    }

    /// Read the cached update check result
    pub fn read(&self) -> Result<Option<UpdateCache>, DxError> {
        if !self.cache_path.exists() {
            return Ok(None);
        }

        let content = std::fs::read_to_string(&self.cache_path).map_err(|e| DxError::Io {
            message: format!("Failed to read update cache: {}", e),
        })?;

        let cache: UpdateCache = serde_json::from_str(&content).map_err(|e| {
            // Corrupt cache - just return None so we re-check
            tracing::warn!("Update cache corrupted, will re-check: {}", e);
            DxError::Io {
                message: format!("Cache parse error: {}", e),
            }
        })?;

        // Check if cache is expired based on our TTL
        if cache.age_secs() >= self.ttl_secs {
            tracing::debug!("Update cache expired (age: {}s)", cache.age_secs());
            return Ok(None);
        }

        Ok(Some(cache))
    }

    /// Write a new cache entry
    pub fn write(&self, cache: &UpdateCache) -> Result<(), DxError> {
        // Ensure cache directory exists
        if let Some(parent) = self.cache_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| DxError::Io {
                message: format!("Failed to create cache directory: {}", e),
            })?;
        }

        let content = serde_json::to_string_pretty(cache).map_err(|e| DxError::Io {
            message: format!("Failed to serialize cache: {}", e),
        })?;

        std::fs::write(&self.cache_path, content).map_err(|e| DxError::Io {
            message: format!("Failed to write cache: {}", e),
        })?;

        tracing::debug!("Update cache written to {:?}", self.cache_path);
        Ok(())
    }

    /// Clear the cache
    pub fn clear(&self) -> Result<(), DxError> {
        if self.cache_path.exists() {
            std::fs::remove_file(&self.cache_path).map_err(|e| DxError::Io {
                message: format!("Failed to clear cache: {}", e),
            })?;
        }
        Ok(())
    }

    /// Get the cache file path
    pub fn path(&self) -> &PathBuf {
        &self.cache_path
    }
}

impl Default for UpdateCacheManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Get current Unix timestamp
fn current_timestamp() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or(Duration::ZERO).as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_update_cache_up_to_date() {
        let cache = UpdateCache::up_to_date("1.0.0");
        assert!(!cache.update_available);
        assert_eq!(cache.current_version, "1.0.0");
        assert!(cache.latest_version.is_none());
    }

    #[test]
    fn test_update_cache_update_available() {
        let cache = UpdateCache::update_available("1.0.0", "1.1.0", Some("Bug fixes".to_string()));
        assert!(cache.update_available);
        assert_eq!(cache.current_version, "1.0.0");
        assert_eq!(cache.latest_version, Some("1.1.0".to_string()));
        assert_eq!(cache.release_notes, Some("Bug fixes".to_string()));
    }

    #[test]
    fn test_cache_validity() {
        let cache = UpdateCache::up_to_date("1.0.0");
        assert!(cache.is_valid()); // Just created, should be valid

        // Age should be very small
        assert!(cache.age_secs() < 5);
    }

    #[test]
    fn test_cache_manager_read_write() {
        let dir = tempdir().unwrap();
        let cache_path = dir.path().join("test_cache.json");
        let manager = UpdateCacheManager::with_path(cache_path);

        // Initially empty
        assert!(manager.read().unwrap().is_none());

        // Write cache
        let cache = UpdateCache::up_to_date("1.0.0");
        manager.write(&cache).unwrap();

        // Read back
        let read_cache = manager.read().unwrap().unwrap();
        assert_eq!(read_cache.current_version, "1.0.0");
        assert!(!read_cache.update_available);
    }

    #[test]
    fn test_cache_manager_clear() {
        let dir = tempdir().unwrap();
        let cache_path = dir.path().join("test_cache.json");
        let manager = UpdateCacheManager::with_path(cache_path);

        // Write cache
        let cache = UpdateCache::up_to_date("1.0.0");
        manager.write(&cache).unwrap();
        assert!(manager.read().unwrap().is_some());

        // Clear
        manager.clear().unwrap();
        assert!(manager.read().unwrap().is_none());
    }

    #[test]
    fn test_cache_expiry() {
        let dir = tempdir().unwrap();
        let cache_path = dir.path().join("test_cache.json");

        // Use very short TTL
        let manager = UpdateCacheManager::with_path(cache_path).with_ttl(0);

        // Write cache
        let cache = UpdateCache::up_to_date("1.0.0");
        manager.write(&cache).unwrap();

        // Should be expired immediately with 0 TTL
        assert!(manager.read().unwrap().is_none());
    }
}
