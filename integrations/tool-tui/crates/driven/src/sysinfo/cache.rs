//! System information caching with TTL support

use super::SystemInfo;
use std::time::{Duration, Instant};

/// Cache entry with timestamp
#[derive(Debug, Clone)]
pub struct CacheEntry {
    /// Cached system information
    pub info: SystemInfo,
    /// When the cache was populated
    pub cached_at: Instant,
}

/// System information cache
#[derive(Debug, Default)]
pub struct SystemInfoCache {
    entry: Option<CacheEntry>,
}

impl SystemInfoCache {
    /// Create a new empty cache
    pub fn new() -> Self {
        Self { entry: None }
    }

    /// Get cached value if within TTL
    pub fn get_if_valid(&self, ttl: Duration) -> Option<&SystemInfo> {
        self.entry.as_ref().and_then(|entry| {
            if entry.cached_at.elapsed() < ttl {
                Some(&entry.info)
            } else {
                None
            }
        })
    }

    /// Set cached value
    pub fn set(&mut self, info: SystemInfo) {
        self.entry = Some(CacheEntry {
            info,
            cached_at: Instant::now(),
        });
    }

    /// Invalidate the cache
    pub fn invalidate(&mut self) {
        self.entry = None;
    }

    /// Check if cache is valid
    pub fn is_valid(&self, ttl: Duration) -> bool {
        self.entry.as_ref().map(|e| e.cached_at.elapsed() < ttl).unwrap_or(false)
    }

    /// Get the age of the cache
    pub fn age(&self) -> Option<Duration> {
        self.entry.as_ref().map(|e| e.cached_at.elapsed())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::sleep;

    fn create_test_info() -> SystemInfo {
        SystemInfo {
            os: super::super::OsInfo {
                name: "test".to_string(),
                version: "1.0".to_string(),
                arch: "x86_64".to_string(),
                family: "unix".to_string(),
            },
            shell: super::super::ShellInfo {
                name: "bash".to_string(),
                version: Some("5.0".to_string()),
                path: std::path::PathBuf::from("/bin/bash"),
            },
            languages: vec![],
            package_managers: vec![],
            project: None,
            git: None,
            build_tools: vec![],
            test_frameworks: vec![],
            collected_at: std::time::SystemTime::now(),
        }
    }

    #[test]
    fn test_cache_empty() {
        let cache = SystemInfoCache::new();
        assert!(cache.get_if_valid(Duration::from_secs(60)).is_none());
        assert!(!cache.is_valid(Duration::from_secs(60)));
    }

    #[test]
    fn test_cache_set_get() {
        let mut cache = SystemInfoCache::new();
        let info = create_test_info();

        cache.set(info);

        assert!(cache.get_if_valid(Duration::from_secs(60)).is_some());
        assert!(cache.is_valid(Duration::from_secs(60)));
    }

    #[test]
    fn test_cache_invalidate() {
        let mut cache = SystemInfoCache::new();
        cache.set(create_test_info());

        cache.invalidate();

        assert!(cache.get_if_valid(Duration::from_secs(60)).is_none());
    }

    #[test]
    fn test_cache_ttl_expiry() {
        let mut cache = SystemInfoCache::new();
        cache.set(create_test_info());

        // Very short TTL
        sleep(Duration::from_millis(10));

        assert!(cache.get_if_valid(Duration::from_millis(1)).is_none());
        assert!(!cache.is_valid(Duration::from_millis(1)));
    }
}
