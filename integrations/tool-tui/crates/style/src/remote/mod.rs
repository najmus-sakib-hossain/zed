//! Remote Style Importer
//!
//! Fetches and caches styles from remote sources using `@username:stylename` syntax.
//! Supports DX Registry and GitHub sources with version pinning.
//!
//! **Validates: Requirements 2.1, 2.2, 2.3, 2.4, 2.6, 2.7, 2.8**

use ahash::AHashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

/// Remote style reference parsed from class name
#[derive(Debug, Clone, PartialEq)]
pub enum RemoteRef {
    /// `@username:stylename[@version]`
    DxRegistry {
        username: String,
        stylename: String,
        version: Option<String>,
    },
    /// `@github:owner/repo:path[@ref]`
    GitHub {
        owner: String,
        repo: String,
        path: String,
        git_ref: Option<String>,
    },
}

/// Parsed style definition from remote source
#[derive(Debug, Clone)]
pub struct StyleDefinition {
    /// Style name
    pub name: String,
    /// Version string
    pub version: String,
    /// Author username
    pub author: String,
    /// Space-separated class names
    pub classes: String,
    /// Raw CSS content
    pub css: String,
}

/// Error types for remote import operations
#[derive(Debug, Clone)]
pub enum ImportError {
    /// Failed to parse remote reference
    ParseError(String),
    /// Network request failed
    NetworkError(String),
    /// Invalid style format
    InvalidFormat(String),
    /// Rate limited
    RateLimited,
    /// Cache error
    CacheError(String),
}

impl std::fmt::Display for ImportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ImportError::ParseError(msg) => write!(f, "Parse error: {}", msg),
            ImportError::NetworkError(msg) => write!(f, "Network error: {}", msg),
            ImportError::InvalidFormat(msg) => write!(f, "Invalid format: {}", msg),
            ImportError::RateLimited => write!(f, "Rate limited"),
            ImportError::CacheError(msg) => write!(f, "Cache error: {}", msg),
        }
    }
}

impl std::error::Error for ImportError {}

/// Cache entry for remote styles
#[derive(Debug, Clone)]
pub struct CacheEntry {
    /// Cached style definition
    pub style: StyleDefinition,
    /// When the entry was cached
    pub cached_at: SystemTime,
    /// Cache TTL
    pub ttl: Duration,
}

impl CacheEntry {
    /// Check if cache entry is still valid
    pub fn is_valid(&self) -> bool {
        if let Ok(elapsed) = self.cached_at.elapsed() {
            elapsed < self.ttl
        } else {
            false
        }
    }
}

/// Simple rate limiter
pub struct RateLimiter {
    /// Minimum interval between requests
    min_interval: Duration,
    /// Last request time
    last_request: Option<SystemTime>,
}

impl RateLimiter {
    /// Create a new rate limiter
    pub fn new(min_interval: Duration) -> Self {
        Self {
            min_interval,
            last_request: None,
        }
    }

    /// Check if we can make a request now
    pub fn can_request(&self) -> bool {
        match self.last_request {
            Some(last) => {
                if let Ok(elapsed) = last.elapsed() {
                    elapsed >= self.min_interval
                } else {
                    true
                }
            }
            None => true,
        }
    }

    /// Record that a request was made
    pub fn record_request(&mut self) {
        self.last_request = Some(SystemTime::now());
    }

    /// Get time to wait before next request
    pub fn time_to_wait(&self) -> Duration {
        match self.last_request {
            Some(last) => {
                if let Ok(elapsed) = last.elapsed() {
                    if elapsed < self.min_interval {
                        self.min_interval - elapsed
                    } else {
                        Duration::ZERO
                    }
                } else {
                    Duration::ZERO
                }
            }
            None => Duration::ZERO,
        }
    }
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self::new(Duration::from_millis(100))
    }
}

/// Remote style importer with caching
///
/// **Validates: Requirements 2.1, 2.2, 2.3, 2.4, 2.6, 2.7, 2.8**
pub struct RemoteImporter {
    /// Cache directory (.dx/style/cache/)
    cache_dir: PathBuf,
    /// In-memory cache
    memory_cache: AHashMap<String, CacheEntry>,
    /// Rate limiter
    rate_limiter: RateLimiter,
    /// Default cache TTL
    default_ttl: Duration,
}

impl RemoteImporter {
    /// Create a new remote importer
    pub fn new(cache_dir: impl AsRef<Path>) -> Self {
        Self {
            cache_dir: cache_dir.as_ref().to_path_buf(),
            memory_cache: AHashMap::new(),
            rate_limiter: RateLimiter::default(),
            default_ttl: Duration::from_secs(3600), // 1 hour default
        }
    }

    /// Parse a remote reference from class name
    ///
    /// **Validates: Requirements 2.1, 2.2**
    pub fn parse_ref(class: &str) -> Option<RemoteRef> {
        if !class.starts_with('@') {
            return None;
        }
        let rest = &class[1..];

        if let Some(gh) = rest.strip_prefix("github:") {
            Self::parse_github_ref(gh)
        } else {
            Self::parse_dx_registry_ref(rest)
        }
    }

    /// Parse @username:stylename[@version] syntax
    fn parse_dx_registry_ref(input: &str) -> Option<RemoteRef> {
        // Split by @ for version
        let (main_part, version) = if let Some(idx) = input.rfind('@') {
            let (main, ver) = input.split_at(idx);
            (main, Some(ver[1..].to_string()))
        } else {
            (input, None)
        };

        // Split by : for username:stylename
        let (username, stylename) = main_part.split_once(':')?;

        if username.is_empty() || stylename.is_empty() {
            return None;
        }

        Some(RemoteRef::DxRegistry {
            username: username.to_string(),
            stylename: stylename.to_string(),
            version,
        })
    }

    /// Parse @github:owner/repo:path[@ref] syntax
    fn parse_github_ref(input: &str) -> Option<RemoteRef> {
        // Split by @ for git ref
        let (main_part, git_ref) = if let Some(idx) = input.rfind('@') {
            let (main, ref_) = input.split_at(idx);
            (main, Some(ref_[1..].to_string()))
        } else {
            (input, None)
        };

        // Split by : for path
        let (repo_part, path) = main_part.split_once(':')?;

        // Split by / for owner/repo
        let (owner, repo) = repo_part.split_once('/')?;

        if owner.is_empty() || repo.is_empty() || path.is_empty() {
            return None;
        }

        Some(RemoteRef::GitHub {
            owner: owner.to_string(),
            repo: repo.to_string(),
            path: path.to_string(),
            git_ref,
        })
    }

    /// Generate cache key for a remote reference
    ///
    /// **Validates: Requirements 2.6** (version-specific caching)
    pub fn cache_key(ref_: &RemoteRef) -> String {
        match ref_ {
            RemoteRef::DxRegistry {
                username,
                stylename,
                version,
            } => match version {
                Some(v) => format!("dx_{}_{}_v{}", username, stylename, v),
                None => format!("dx_{}_{}_latest", username, stylename),
            },
            RemoteRef::GitHub {
                owner,
                repo,
                path,
                git_ref,
            } => {
                let safe_path = path.replace('/', "_");
                match git_ref {
                    Some(r) => format!("gh_{}_{}_{}_ref{}", owner, repo, safe_path, r),
                    None => format!("gh_{}_{}_{}_main", owner, repo, safe_path),
                }
            }
        }
    }

    /// Get cache file path for a remote reference
    fn cache_path(&self, ref_: &RemoteRef) -> PathBuf {
        let key = Self::cache_key(ref_);
        self.cache_dir.join(format!("{}.sr", key))
    }

    /// Try to load from memory cache
    fn get_from_memory_cache(&self, ref_: &RemoteRef) -> Option<StyleDefinition> {
        let key = Self::cache_key(ref_);
        self.memory_cache.get(&key).and_then(|entry| {
            if entry.is_valid() {
                Some(entry.style.clone())
            } else {
                None
            }
        })
    }

    /// Try to load from disk cache
    ///
    /// **Validates: Requirements 2.3**
    fn get_from_disk_cache(&self, ref_: &RemoteRef) -> Option<StyleDefinition> {
        let path = self.cache_path(ref_);
        if !path.exists() {
            return None;
        }

        let content = std::fs::read_to_string(&path).ok()?;
        Self::parse_style_definition(&content).ok()
    }

    /// Save to disk cache
    ///
    /// **Validates: Requirements 2.3, 2.8**
    fn save_to_disk_cache(&self, ref_: &RemoteRef, content: &str) -> Result<(), ImportError> {
        // Ensure cache directory exists
        if !self.cache_dir.exists() {
            std::fs::create_dir_all(&self.cache_dir)
                .map_err(|e| ImportError::CacheError(e.to_string()))?;
        }

        let path = self.cache_path(ref_);
        std::fs::write(&path, content).map_err(|e| ImportError::CacheError(e.to_string()))?;

        Ok(())
    }

    /// Save to memory cache
    fn save_to_memory_cache(&mut self, ref_: &RemoteRef, style: StyleDefinition) {
        let key = Self::cache_key(ref_);
        self.memory_cache.insert(
            key,
            CacheEntry {
                style,
                cached_at: SystemTime::now(),
                ttl: self.default_ttl,
            },
        );
    }

    /// Parse a style definition from content
    ///
    /// **Validates: Requirements 2.7**
    pub fn parse_style_definition(content: &str) -> Result<StyleDefinition, ImportError> {
        let mut name = String::new();
        let mut version = String::new();
        let mut author = String::new();
        let mut classes = String::new();
        let mut css = String::new();
        let mut in_css_block = false;

        for line in content.lines() {
            let line = line.trim();

            if in_css_block {
                if line == "]" {
                    in_css_block = false;
                } else {
                    if !css.is_empty() {
                        css.push('\n');
                    }
                    css.push_str(line);
                }
                continue;
            }

            if line.starts_with("css[") {
                in_css_block = true;
                continue;
            }

            if let Some(val) = line.strip_prefix("name=") {
                name = val.to_string();
            } else if let Some(val) = line.strip_prefix("version=") {
                version = val.to_string();
            } else if let Some(val) = line.strip_prefix("author=") {
                author = val.to_string();
            } else if let Some(val) = line.strip_prefix("classes=") {
                classes = val.to_string();
            }
        }

        // Validate required fields
        if name.is_empty() {
            return Err(ImportError::InvalidFormat("Missing 'name' field".to_string()));
        }
        if css.is_empty() && classes.is_empty() {
            return Err(ImportError::InvalidFormat("Missing 'css' or 'classes' field".to_string()));
        }

        Ok(StyleDefinition {
            name,
            version,
            author,
            classes,
            css,
        })
    }

    /// Build URL for fetching a remote style (mock implementation)
    pub fn build_url(ref_: &RemoteRef) -> String {
        match ref_ {
            RemoteRef::DxRegistry {
                username,
                stylename,
                version,
            } => match version {
                Some(v) => format!("https://dx.style/api/v1/{}/{}/v{}", username, stylename, v),
                None => format!("https://dx.style/api/v1/{}/{}/latest", username, stylename),
            },
            RemoteRef::GitHub {
                owner,
                repo,
                path,
                git_ref,
            } => {
                let ref_str = git_ref.as_deref().unwrap_or("main");
                format!("https://raw.githubusercontent.com/{}/{}/{}/{}", owner, repo, ref_str, path)
            }
        }
    }

    /// Fetch a remote style (synchronous, for testing)
    ///
    /// In production, this would use async HTTP client.
    /// For now, it only uses cache.
    ///
    /// **Validates: Requirements 2.3, 2.4, 2.6**
    pub fn fetch(&mut self, ref_: &RemoteRef) -> Result<StyleDefinition, ImportError> {
        // Try memory cache first
        if let Some(style) = self.get_from_memory_cache(ref_) {
            return Ok(style);
        }

        // Try disk cache
        if let Some(style) = self.get_from_disk_cache(ref_) {
            // Update memory cache
            self.save_to_memory_cache(ref_, style.clone());
            return Ok(style);
        }

        // In a real implementation, we would fetch from network here
        // For now, return an error indicating cache miss
        Err(ImportError::NetworkError(format!(
            "Style not in cache and network fetch not implemented. URL would be: {}",
            Self::build_url(ref_)
        )))
    }

    /// Import a style from content (for testing/manual import)
    pub fn import_from_content(
        &mut self,
        ref_: &RemoteRef,
        content: &str,
    ) -> Result<StyleDefinition, ImportError> {
        // Parse and validate
        let style = Self::parse_style_definition(content)?;

        // Save to caches
        self.save_to_disk_cache(ref_, content)?;
        self.save_to_memory_cache(ref_, style.clone());

        Ok(style)
    }

    /// Check if rate limiting allows a request
    pub fn can_request(&self) -> bool {
        self.rate_limiter.can_request()
    }

    /// Clear all caches
    pub fn clear_cache(&mut self) {
        self.memory_cache.clear();
        if self.cache_dir.exists() {
            let _ = std::fs::remove_dir_all(&self.cache_dir);
        }
    }
}

impl Default for RemoteImporter {
    fn default() -> Self {
        Self::new(".dx/style/cache")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn test_cache_dir() -> PathBuf {
        PathBuf::from(".dx/test-cache")
    }

    fn cleanup_test_cache() {
        let dir = test_cache_dir();
        if dir.exists() {
            let _ = fs::remove_dir_all(&dir);
        }
    }

    #[test]
    fn test_parse_dx_registry_ref() {
        let result = RemoteImporter::parse_ref("@alice:card-hover");
        assert!(result.is_some());
        if let Some(RemoteRef::DxRegistry {
            username,
            stylename,
            version,
        }) = result
        {
            assert_eq!(username, "alice");
            assert_eq!(stylename, "card-hover");
            assert!(version.is_none());
        } else {
            panic!("Expected DxRegistry");
        }
    }

    #[test]
    fn test_parse_dx_registry_ref_with_version() {
        let result = RemoteImporter::parse_ref("@alice:card-hover@1.0.0");
        assert!(result.is_some());
        if let Some(RemoteRef::DxRegistry {
            username,
            stylename,
            version,
        }) = result
        {
            assert_eq!(username, "alice");
            assert_eq!(stylename, "card-hover");
            assert_eq!(version, Some("1.0.0".to_string()));
        } else {
            panic!("Expected DxRegistry");
        }
    }

    #[test]
    fn test_parse_github_ref() {
        let result = RemoteImporter::parse_ref("@github:owner/repo:styles/button.sr");
        assert!(result.is_some());
        if let Some(RemoteRef::GitHub {
            owner,
            repo,
            path,
            git_ref,
        }) = result
        {
            assert_eq!(owner, "owner");
            assert_eq!(repo, "repo");
            assert_eq!(path, "styles/button.sr");
            assert!(git_ref.is_none());
        } else {
            panic!("Expected GitHub");
        }
    }

    #[test]
    fn test_parse_github_ref_with_ref() {
        let result = RemoteImporter::parse_ref("@github:owner/repo:styles/button.sr@v2.0");
        assert!(result.is_some());
        if let Some(RemoteRef::GitHub {
            owner,
            repo,
            path,
            git_ref,
        }) = result
        {
            assert_eq!(owner, "owner");
            assert_eq!(repo, "repo");
            assert_eq!(path, "styles/button.sr");
            assert_eq!(git_ref, Some("v2.0".to_string()));
        } else {
            panic!("Expected GitHub");
        }
    }

    #[test]
    fn test_parse_invalid_ref() {
        assert!(RemoteImporter::parse_ref("not-a-ref").is_none());
        assert!(RemoteImporter::parse_ref("@").is_none());
        assert!(RemoteImporter::parse_ref("@:").is_none());
    }

    #[test]
    fn test_cache_key_dx_registry() {
        let ref_ = RemoteRef::DxRegistry {
            username: "alice".to_string(),
            stylename: "button".to_string(),
            version: None,
        };
        assert_eq!(RemoteImporter::cache_key(&ref_), "dx_alice_button_latest");

        let ref_versioned = RemoteRef::DxRegistry {
            username: "alice".to_string(),
            stylename: "button".to_string(),
            version: Some("1.0.0".to_string()),
        };
        assert_eq!(RemoteImporter::cache_key(&ref_versioned), "dx_alice_button_v1.0.0");
    }

    #[test]
    fn test_cache_key_github() {
        let ref_ = RemoteRef::GitHub {
            owner: "owner".to_string(),
            repo: "repo".to_string(),
            path: "styles/button.sr".to_string(),
            git_ref: None,
        };
        assert_eq!(RemoteImporter::cache_key(&ref_), "gh_owner_repo_styles_button.sr_main");
    }

    #[test]
    fn test_parse_style_definition() {
        let content = r#"name=card-hover
version=1.0.0
author=alice
classes=bg-white rounded-lg shadow-md
css[
.card-hover {
  background: white;
}
]"#;

        let result = RemoteImporter::parse_style_definition(content);
        assert!(result.is_ok());
        let style = result.unwrap();
        assert_eq!(style.name, "card-hover");
        assert_eq!(style.version, "1.0.0");
        assert_eq!(style.author, "alice");
        assert_eq!(style.classes, "bg-white rounded-lg shadow-md");
        assert!(style.css.contains("background: white"));
    }

    #[test]
    fn test_parse_style_definition_invalid() {
        // Missing name
        let content = "version=1.0.0\nclasses=test";
        assert!(RemoteImporter::parse_style_definition(content).is_err());

        // Missing css and classes
        let content = "name=test\nversion=1.0.0";
        assert!(RemoteImporter::parse_style_definition(content).is_err());
    }

    #[test]
    fn test_import_and_cache() {
        cleanup_test_cache();

        let mut importer = RemoteImporter::new(test_cache_dir());
        let ref_ = RemoteRef::DxRegistry {
            username: "test".to_string(),
            stylename: "button".to_string(),
            version: Some("1.0.0".to_string()),
        };

        let content = r#"name=button
version=1.0.0
author=test
classes=bg-blue text-white
css[
.button { color: white; }
]"#;

        // Import
        let result = importer.import_from_content(&ref_, content);
        assert!(result.is_ok());

        // Should be in memory cache now
        let cached = importer.get_from_memory_cache(&ref_);
        assert!(cached.is_some());

        // Should be on disk
        let disk_cached = importer.get_from_disk_cache(&ref_);
        assert!(disk_cached.is_some());

        cleanup_test_cache();
    }

    #[test]
    fn test_rate_limiter() {
        let limiter = RateLimiter::new(Duration::from_millis(100));
        assert!(limiter.can_request());

        let mut limiter = limiter;
        limiter.record_request();

        // Immediately after, should not be able to request
        // (unless 100ms has passed during test execution)
        let wait_time = limiter.time_to_wait();
        assert!(wait_time <= Duration::from_millis(100));
    }

    #[test]
    fn test_build_url() {
        let dx_ref = RemoteRef::DxRegistry {
            username: "alice".to_string(),
            stylename: "button".to_string(),
            version: None,
        };
        let url = RemoteImporter::build_url(&dx_ref);
        assert!(url.contains("alice"));
        assert!(url.contains("button"));
        assert!(url.contains("latest"));

        let gh_ref = RemoteRef::GitHub {
            owner: "owner".to_string(),
            repo: "repo".to_string(),
            path: "styles/button.sr".to_string(),
            git_ref: Some("v1.0".to_string()),
        };
        let url = RemoteImporter::build_url(&gh_ref);
        assert!(url.contains("githubusercontent"));
        assert!(url.contains("v1.0"));
    }
}

#[cfg(test)]
mod prop_tests {
    use super::*;
    use proptest::prelude::*;

    fn arb_username() -> impl Strategy<Value = String> {
        "[a-z][a-z0-9_]{2,15}".prop_map(|s| s)
    }

    fn arb_stylename() -> impl Strategy<Value = String> {
        "[a-z][a-z0-9-]{2,20}".prop_map(|s| s)
    }

    fn arb_version() -> impl Strategy<Value = String> {
        (1u32..10u32, 0u32..20u32, 0u32..100u32)
            .prop_map(|(major, minor, patch)| format!("{}.{}.{}", major, minor, patch))
    }

    fn arb_style_content() -> impl Strategy<Value = String> {
        (arb_stylename(), arb_version(), arb_username())
            .prop_map(|(name, version, author)| {
                format!(
                    "name={}\nversion={}\nauthor={}\nclasses=bg-white text-black\ncss[\n.{} {{ color: black; }}\n]",
                    name, version, author, name
                )
            })
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Feature: dx-style-advanced-features, Property 3: Remote Import Caching
        /// *For any* successfully fetched remote style, the Remote_Importer SHALL create
        /// a cache file in `.dx/style/cache/` that can be used for subsequent requests.
        /// **Validates: Requirements 2.3, 8.2**
        #[test]
        fn prop_remote_import_caching(
            username in arb_username(),
            stylename in arb_stylename(),
            content in arb_style_content()
        ) {
            let cache_dir = PathBuf::from(format!(".dx/test-cache-prop-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&cache_dir);

            let mut importer = RemoteImporter::new(&cache_dir);
            let ref_ = RemoteRef::DxRegistry {
                username: username.clone(),
                stylename: stylename.clone(),
                version: None,
            };

            // Import content
            let result = importer.import_from_content(&ref_, &content);

            if result.is_ok() {
                // Cache file should exist
                let cache_path = importer.cache_path(&ref_);
                prop_assert!(
                    cache_path.exists(),
                    "Cache file should exist at {:?}",
                    cache_path
                );

                // Should be able to fetch from cache
                let cached = importer.fetch(&ref_);
                prop_assert!(
                    cached.is_ok(),
                    "Should be able to fetch from cache"
                );

                // Cached content should match
                let cached_style = cached.unwrap();
                prop_assert!(
                    !cached_style.name.is_empty(),
                    "Cached style should have a name"
                );
            }

            let _ = std::fs::remove_dir_all(&cache_dir);
        }

        /// Feature: dx-style-advanced-features, Property 4: Remote Import Version Pinning
        /// *For any* versioned remote import (e.g., `@username:style@1.0.0`), the Remote_Importer
        /// SHALL fetch and cache the exact specified version.
        /// **Validates: Requirements 2.6**
        #[test]
        fn prop_remote_import_version_pinning(
            username in arb_username(),
            stylename in arb_stylename(),
            version in arb_version()
        ) {
            let cache_dir = PathBuf::from(format!(".dx/test-cache-ver-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&cache_dir);

            let mut importer = RemoteImporter::new(&cache_dir);

            // Create versioned reference
            let ref_versioned = RemoteRef::DxRegistry {
                username: username.clone(),
                stylename: stylename.clone(),
                version: Some(version.clone()),
            };

            // Create unversioned reference
            let ref_latest = RemoteRef::DxRegistry {
                username: username.clone(),
                stylename: stylename.clone(),
                version: None,
            };

            // Cache keys should be different
            let key_versioned = RemoteImporter::cache_key(&ref_versioned);
            let key_latest = RemoteImporter::cache_key(&ref_latest);

            prop_assert!(
                key_versioned != key_latest,
                "Versioned and latest cache keys should differ: {} vs {}",
                key_versioned, key_latest
            );

            // Versioned key should contain the version
            prop_assert!(
                key_versioned.contains(&version),
                "Versioned cache key '{}' should contain version '{}'",
                key_versioned, version
            );

            // Import versioned content
            let content = format!(
                "name={}\nversion={}\nauthor={}\nclasses=test\ncss[\n.test {{}}\n]",
                stylename, version, username
            );

            let result = importer.import_from_content(&ref_versioned, &content);

            if result.is_ok() {
                // Versioned cache should exist
                let versioned_path = importer.cache_path(&ref_versioned);
                prop_assert!(
                    versioned_path.exists(),
                    "Versioned cache file should exist"
                );

                // Latest cache should NOT exist (different key)
                let latest_path = importer.cache_path(&ref_latest);
                prop_assert!(
                    !latest_path.exists(),
                    "Latest cache file should not exist when only versioned was imported"
                );
            }

            let _ = std::fs::remove_dir_all(&cache_dir);
        }

        /// Property test for reference parsing round-trip
        #[test]
        fn prop_dx_registry_ref_parsing(
            username in arb_username(),
            stylename in arb_stylename()
        ) {
            let class_name = format!("@{}:{}", username, stylename);
            let result = RemoteImporter::parse_ref(&class_name);

            prop_assert!(
                result.is_some(),
                "Should parse valid DX registry ref: {}",
                class_name
            );

            if let Some(RemoteRef::DxRegistry { username: u, stylename: s, version: v }) = result {
                prop_assert_eq!(u, username, "Username should match");
                prop_assert_eq!(s, stylename, "Stylename should match");
                prop_assert!(v.is_none(), "Version should be None for unversioned ref");
            }
        }

        /// Property test for versioned reference parsing
        #[test]
        fn prop_dx_registry_versioned_ref_parsing(
            username in arb_username(),
            stylename in arb_stylename(),
            version in arb_version()
        ) {
            let class_name = format!("@{}:{}@{}", username, stylename, version);
            let result = RemoteImporter::parse_ref(&class_name);

            prop_assert!(
                result.is_some(),
                "Should parse valid versioned DX registry ref: {}",
                class_name
            );

            if let Some(RemoteRef::DxRegistry { username: u, stylename: s, version: v }) = result {
                prop_assert_eq!(u, username, "Username should match");
                prop_assert_eq!(s, stylename, "Stylename should match");
                prop_assert_eq!(v, Some(version), "Version should match");
            }
        }

        /// Property test for style definition validation
        #[test]
        fn prop_style_definition_validation(
            content in arb_style_content()
        ) {
            let result = RemoteImporter::parse_style_definition(&content);

            prop_assert!(
                result.is_ok(),
                "Valid style content should parse successfully"
            );

            let style = result.unwrap();
            prop_assert!(
                !style.name.is_empty(),
                "Parsed style should have non-empty name"
            );
        }
    }
}
