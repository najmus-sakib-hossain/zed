//! PyPI JSON API client for downloading packages
//!
//! Provides a high-level interface for querying PyPI package metadata
//! and downloading packages with proper error handling.
//!
//! ## Wheel Selection
//!
//! This module implements wheel selection per PEP 425 (Compatibility Tags for Built Distributions).
//! When downloading a package, the client will:
//!
//! 1. Parse wheel filename tags (python version, ABI, platform)
//! 2. Match against the current platform environment
//! 3. Select the best compatible wheel based on specificity score
//! 4. Fall back to sdist if no compatible wheel is found
//!
//! Wheels are preferred over sdist because they:
//! - Don't require compilation
//! - Install faster
//! - Are more reliable (pre-built binaries)

use std::path::PathBuf;
use std::time::Duration;

use dx_py_core::wheel::{PlatformEnvironment, WheelTag};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::{Error, Result};

/// Default PyPI base URL
pub const PYPI_BASE_URL: &str = "https://pypi.org";

/// PyPI JSON API response for package metadata
///
/// This is the top-level response from the PyPI JSON API endpoints:
/// - `https://pypi.org/pypi/{name}/json` - Latest version
/// - `https://pypi.org/pypi/{name}/{version}/json` - Specific version
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PackageMetadata {
    /// Package information
    pub info: PackageInfoDetails,
    /// Available releases (version -> files)
    pub releases: HashMap<String, Vec<ReleaseFileInfo>>,
    /// URLs for the latest version
    pub urls: Vec<ReleaseFileInfo>,
}

/// Detailed package metadata from PyPI JSON API
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PackageInfoDetails {
    /// Package name
    pub name: String,
    /// Latest version
    pub version: String,
    /// Package summary/description
    pub summary: Option<String>,
    /// Author name
    pub author: Option<String>,
    /// Author email
    pub author_email: Option<String>,
    /// License
    pub license: Option<String>,
    /// Project homepage
    pub home_page: Option<String>,
    /// Required Python version
    pub requires_python: Option<String>,
    /// Package dependencies (PEP 508 format)
    pub requires_dist: Option<Vec<String>>,
    /// Project URLs
    pub project_urls: Option<HashMap<String, String>>,
    /// Package classifiers
    pub classifiers: Option<Vec<String>>,
    /// Package description
    pub description: Option<String>,
    /// Description content type
    pub description_content_type: Option<String>,
    /// Package keywords
    pub keywords: Option<String>,
    /// Maintainer name
    pub maintainer: Option<String>,
    /// Maintainer email
    pub maintainer_email: Option<String>,
    /// Platform
    pub platform: Option<String>,
}

/// Release file information from PyPI
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ReleaseFileInfo {
    /// Filename
    pub filename: String,
    /// Download URL
    pub url: String,
    /// File size in bytes
    pub size: u64,
    /// Package type (sdist, bdist_wheel, etc.)
    pub packagetype: String,
    /// Python version requirement
    pub python_version: Option<String>,
    /// File digests for integrity verification
    pub digests: FileDigestsInfo,
    /// Whether this requires Python
    pub requires_python: Option<String>,
    /// Upload time
    pub upload_time: Option<String>,
    /// Upload time ISO format
    pub upload_time_iso_8601: Option<String>,
    /// Whether the file has been yanked
    #[serde(default)]
    pub yanked: bool,
    /// Reason for yanking
    pub yanked_reason: Option<String>,
}

/// File digests for integrity verification
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FileDigestsInfo {
    /// MD5 hash (legacy)
    pub md5: Option<String>,
    /// SHA256 hash
    pub sha256: String,
    /// Blake2b-256 hash (newer)
    pub blake2b_256: Option<String>,
}


/// PyPI JSON API client for downloading packages
///
/// This client provides methods to:
/// - Query package metadata from PyPI
/// - Get specific version information
/// - List available versions
/// - Get package dependencies
pub struct PyPiDownloader {
    /// HTTP client
    client: Client,
    /// Base URL for PyPI API
    base_url: String,
    /// Cache directory for downloaded packages
    cache_dir: Option<PathBuf>,
    /// Request timeout
    timeout: Duration,
}

impl Default for PyPiDownloader {
    fn default() -> Self {
        Self::new()
    }
}

impl PyPiDownloader {
    /// Create a new PyPI downloader with default settings
    pub fn new() -> Self {
        Self::with_base_url(PYPI_BASE_URL)
    }

    /// Create a new PyPI downloader with a custom base URL
    pub fn with_base_url(base_url: &str) -> Self {
        let client = Client::builder()
            .user_agent("dx-py/0.1.0")
            .timeout(Duration::from_secs(300))
            .connect_timeout(Duration::from_secs(30))
            .pool_max_idle_per_host(10)
            .build()
            .unwrap_or_else(|_| Client::new());

        Self {
            client,
            base_url: base_url.trim_end_matches('/').to_string(),
            cache_dir: None,
            timeout: Duration::from_secs(300),
        }
    }

    /// Set the cache directory for downloaded packages
    pub fn with_cache_dir(mut self, cache_dir: PathBuf) -> Self {
        self.cache_dir = Some(cache_dir);
        self
    }

    /// Set the request timeout
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Get the base URL
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// Get the cache directory
    pub fn cache_dir(&self) -> Option<&PathBuf> {
        self.cache_dir.as_ref()
    }

    /// Get the timeout
    pub fn timeout(&self) -> Duration {
        self.timeout
    }


    /// Get package metadata from PyPI (latest version)
    ///
    /// Queries the PyPI JSON API at `https://pypi.org/pypi/{name}/json`
    /// and returns the full package metadata including all releases.
    pub async fn get_package_metadata(&self, name: &str) -> Result<PackageMetadata> {
        let url = format!("{}/pypi/{}/json", self.base_url, name);
        self.fetch_metadata(&url, name, None).await
    }

    /// Get package metadata for a specific version
    ///
    /// Queries the PyPI JSON API at `https://pypi.org/pypi/{name}/{version}/json`
    /// and returns the package metadata for that specific version.
    pub async fn get_package_version(
        &self,
        name: &str,
        version: &str,
    ) -> Result<PackageMetadata> {
        let url = format!("{}/pypi/{}/{}/json", self.base_url, name, version);
        self.fetch_metadata(&url, name, Some(version)).await
    }

    /// Internal method to fetch and parse metadata from a URL
    async fn fetch_metadata(
        &self,
        url: &str,
        name: &str,
        version: Option<&str>,
    ) -> Result<PackageMetadata> {
        let response = self
            .client
            .get(url)
            .timeout(self.timeout)
            .send()
            .await
            .map_err(|e| Error::Network(format!("Failed to fetch package '{}': {}", name, e)))?;

        match response.status() {
            status if status == reqwest::StatusCode::NOT_FOUND => {
                let msg = match version {
                    Some(v) => format!("{}=={}", name, v),
                    None => name.to_string(),
                };
                Err(Error::PackageNotFound(msg))
            }
            status if !status.is_success() => Err(Error::Network(format!(
                "HTTP {} when fetching package '{}'",
                status, name
            ))),
            _ => response.json().await.map_err(|e| {
                Error::Network(format!(
                    "Failed to parse JSON response for package '{}': {}",
                    name, e
                ))
            }),
        }
    }


    /// Get all available versions for a package
    ///
    /// Returns a sorted list of all versions available on PyPI.
    pub async fn get_available_versions(&self, name: &str) -> Result<Vec<String>> {
        let metadata = self.get_package_metadata(name).await?;
        let mut versions: Vec<String> = metadata.releases.keys().cloned().collect();
        versions.sort();
        Ok(versions)
    }

    /// Get the latest version of a package
    ///
    /// Returns the latest version as reported by PyPI's `info.version` field.
    pub async fn get_latest_version(&self, name: &str) -> Result<String> {
        let metadata = self.get_package_metadata(name).await?;
        Ok(metadata.info.version)
    }

    /// Get dependencies for a package version
    ///
    /// Returns the list of dependencies in PEP 508 format.
    pub async fn get_dependencies(&self, name: &str, version: &str) -> Result<Vec<String>> {
        let metadata = self.get_package_version(name, version).await?;
        Ok(metadata.info.requires_dist.unwrap_or_default())
    }

    /// Get release files for a specific version
    ///
    /// Returns all available distribution files (wheels, sdist) for a version.
    pub async fn get_release_files(
        &self,
        name: &str,
        version: &str,
    ) -> Result<Vec<ReleaseFileInfo>> {
        let metadata = self.get_package_version(name, version).await?;
        Ok(metadata
            .releases
            .get(version)
            .cloned()
            .unwrap_or_default())
    }

    /// Find wheel files for a specific version
    ///
    /// Returns only wheel distribution files, filtering out sdist and other formats.
    pub async fn find_wheels(&self, name: &str, version: &str) -> Result<Vec<ReleaseFileInfo>> {
        let files = self.get_release_files(name, version).await?;
        Ok(files
            .into_iter()
            .filter(|f| f.packagetype == "bdist_wheel" && !f.yanked)
            .collect())
    }

    /// Find sdist (source distribution) for a specific version
    ///
    /// Returns the source distribution file if available.
    pub async fn find_sdist(&self, name: &str, version: &str) -> Result<Option<ReleaseFileInfo>> {
        let files = self.get_release_files(name, version).await?;
        Ok(files
            .into_iter()
            .find(|f| f.packagetype == "sdist" && !f.yanked))
    }

    /// Select the best wheel for the given platform environment
    ///
    /// This method implements wheel selection per PEP 425:
    /// 1. Parses wheel filename tags (python version, ABI, platform)
    /// 2. Filters to only compatible wheels
    /// 3. Selects the wheel with the highest specificity score
    ///
    /// Returns `None` if no compatible wheel is found.
    ///
    /// # Arguments
    /// * `name` - Package name
    /// * `version` - Package version
    /// * `env` - Platform environment to match against
    ///
    /// # Example
    /// ```ignore
    /// let env = PlatformEnvironment::detect();
    /// let wheel = downloader.select_wheel("numpy", "1.26.0", &env).await?;
    /// ```
    pub async fn select_wheel(
        &self,
        name: &str,
        version: &str,
        env: &PlatformEnvironment,
    ) -> Result<Option<ReleaseFileInfo>> {
        let files = self.get_release_files(name, version).await?;

        let mut best_wheel: Option<(ReleaseFileInfo, u32)> = None;

        for file in files {
            // Skip non-wheel files and yanked files
            if file.packagetype != "bdist_wheel" || file.yanked {
                continue;
            }

            // Parse wheel tag from filename
            match WheelTag::parse(&file.filename) {
                Ok(tag) => {
                    // Check if wheel is compatible with the environment
                    if tag.is_compatible(env) {
                        let score = tag.specificity_score(env);
                        // Keep the wheel with the highest specificity score
                        if best_wheel.as_ref().is_none_or(|(_, best_score)| score > *best_score) {
                            best_wheel = Some((file, score));
                        }
                    }
                }
                Err(_) => {
                    // Skip wheels with unparseable filenames
                    continue;
                }
            }
        }

        Ok(best_wheel.map(|(file, _)| file))
    }

    /// Select the best distribution (wheel or sdist) for the given platform
    ///
    /// This method implements the wheel preference logic per Requirement 8.5:
    /// "WHEN a wheel is available for the current platform, THE Package_Manager
    /// SHALL prefer it over sdist"
    ///
    /// Selection priority:
    /// 1. Platform-specific wheel (highest specificity score)
    /// 2. Universal wheel (py3-none-any)
    /// 3. Source distribution (sdist) as fallback
    ///
    /// # Arguments
    /// * `name` - Package name
    /// * `version` - Package version
    /// * `env` - Platform environment to match against
    ///
    /// # Returns
    /// * `Ok(Some(file))` - Best distribution found
    /// * `Ok(None)` - No distribution available
    /// * `Err(_)` - Network or API error
    ///
    /// # Example
    /// ```ignore
    /// let env = PlatformEnvironment::detect();
    /// let dist = downloader.select_distribution("requests", "2.31.0", &env).await?;
    /// match dist {
    ///     Some(file) if file.packagetype == "bdist_wheel" => {
    ///         println!("Using wheel: {}", file.filename);
    ///     }
    ///     Some(file) => {
    ///         println!("Falling back to sdist: {}", file.filename);
    ///     }
    ///     None => {
    ///         println!("No distribution available");
    ///     }
    /// }
    /// ```
    pub async fn select_distribution(
        &self,
        name: &str,
        version: &str,
        env: &PlatformEnvironment,
    ) -> Result<Option<ReleaseFileInfo>> {
        // First, try to find a compatible wheel (preferred)
        if let Some(wheel) = self.select_wheel(name, version, env).await? {
            return Ok(Some(wheel));
        }

        // Fall back to sdist if no compatible wheel found
        self.find_sdist(name, version).await
    }

    /// Get all compatible wheels for a package version, sorted by specificity
    ///
    /// Returns all wheels that are compatible with the given environment,
    /// sorted from most specific (best match) to least specific.
    ///
    /// This is useful when you want to see all available options or
    /// implement custom selection logic.
    ///
    /// # Arguments
    /// * `name` - Package name
    /// * `version` - Package version
    /// * `env` - Platform environment to match against
    pub async fn get_compatible_wheels(
        &self,
        name: &str,
        version: &str,
        env: &PlatformEnvironment,
    ) -> Result<Vec<(ReleaseFileInfo, u32)>> {
        let files = self.get_release_files(name, version).await?;

        let mut compatible: Vec<(ReleaseFileInfo, u32)> = files
            .into_iter()
            .filter(|f| f.packagetype == "bdist_wheel" && !f.yanked)
            .filter_map(|file| {
                WheelTag::parse(&file.filename).ok().and_then(|tag| {
                    if tag.is_compatible(env) {
                        Some((file, tag.specificity_score(env)))
                    } else {
                        None
                    }
                })
            })
            .collect();

        // Sort by specificity score (highest first)
        compatible.sort_by(|a, b| b.1.cmp(&a.1));

        Ok(compatible)
    }

    /// Check if a wheel filename is compatible with the given environment
    ///
    /// This is a convenience method for checking compatibility without
    /// fetching from PyPI.
    ///
    /// # Arguments
    /// * `filename` - Wheel filename (e.g., "numpy-1.26.0-cp312-cp312-manylinux_2_17_x86_64.whl")
    /// * `env` - Platform environment to match against
    ///
    /// # Returns
    /// * `Ok(true)` - Wheel is compatible
    /// * `Ok(false)` - Wheel is not compatible
    /// * `Err(_)` - Invalid wheel filename
    pub fn is_wheel_compatible(filename: &str, env: &PlatformEnvironment) -> Result<bool> {
        let tag = WheelTag::parse(filename)
            .map_err(|e| Error::InvalidPackageName(format!("Invalid wheel filename: {}", e)))?;
        Ok(tag.is_compatible(env))
    }

    /// Parse wheel tags from a filename
    ///
    /// Extracts the python, ABI, and platform tags from a wheel filename.
    ///
    /// # Arguments
    /// * `filename` - Wheel filename
    ///
    /// # Returns
    /// * `Ok(WheelTag)` - Parsed wheel tag
    /// * `Err(_)` - Invalid wheel filename
    pub fn parse_wheel_tags(filename: &str) -> Result<WheelTag> {
        WheelTag::parse(filename)
            .map_err(|e| Error::InvalidPackageName(format!("Invalid wheel filename: {}", e)))
    }


    /// Check if a package exists on PyPI
    pub async fn package_exists(&self, name: &str) -> Result<bool> {
        match self.get_package_metadata(name).await {
            Ok(_) => Ok(true),
            Err(Error::PackageNotFound(_)) => Ok(false),
            Err(e) => Err(e),
        }
    }

    /// Check if a specific version exists on PyPI
    pub async fn version_exists(&self, name: &str, version: &str) -> Result<bool> {
        match self.get_package_version(name, version).await {
            Ok(_) => Ok(true),
            Err(Error::PackageNotFound(_)) => Ok(false),
            Err(e) => Err(e),
        }
    }
}

impl Clone for PyPiDownloader {
    fn clone(&self) -> Self {
        Self {
            client: self.client.clone(),
            base_url: self.base_url.clone(),
            cache_dir: self.cache_dir.clone(),
            timeout: self.timeout,
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use dx_py_core::wheel::PlatformEnvironment;

    #[test]
    fn test_pypi_downloader_defaults() {
        let downloader = PyPiDownloader::new();
        assert_eq!(downloader.base_url(), PYPI_BASE_URL);
        assert!(downloader.cache_dir().is_none());
        assert_eq!(downloader.timeout(), Duration::from_secs(300));
    }

    #[test]
    fn test_pypi_downloader_custom_base_url() {
        let downloader = PyPiDownloader::with_base_url("https://test.pypi.org");
        assert_eq!(downloader.base_url(), "https://test.pypi.org");
    }

    #[test]
    fn test_pypi_downloader_base_url_trailing_slash() {
        let downloader = PyPiDownloader::with_base_url("https://pypi.org/");
        assert_eq!(downloader.base_url(), "https://pypi.org");
    }

    #[test]
    fn test_pypi_downloader_with_cache_dir() {
        let cache_dir = PathBuf::from("/tmp/cache");
        let downloader = PyPiDownloader::new().with_cache_dir(cache_dir.clone());
        assert_eq!(downloader.cache_dir(), Some(&cache_dir));
    }

    #[test]
    fn test_pypi_downloader_with_timeout() {
        let timeout = Duration::from_secs(60);
        let downloader = PyPiDownloader::new().with_timeout(timeout);
        assert_eq!(downloader.timeout(), timeout);
    }

    #[test]
    fn test_pypi_downloader_clone() {
        let cache_dir = PathBuf::from("/tmp/cache");
        let downloader = PyPiDownloader::with_base_url("https://test.pypi.org")
            .with_cache_dir(cache_dir.clone())
            .with_timeout(Duration::from_secs(60));

        let cloned = downloader.clone();
        assert_eq!(cloned.base_url(), "https://test.pypi.org");
        assert_eq!(cloned.cache_dir(), Some(&cache_dir));
        assert_eq!(cloned.timeout(), Duration::from_secs(60));
    }


    #[test]
    fn test_package_metadata_deserialize() {
        let json = r#"{
            "info": {
                "name": "requests",
                "version": "2.31.0",
                "summary": "Python HTTP for Humans.",
                "author": "Kenneth Reitz",
                "author_email": "me@kennethreitz.org",
                "license": "Apache 2.0",
                "home_page": "https://requests.readthedocs.io",
                "requires_python": ">=3.7",
                "requires_dist": [
                    "charset-normalizer<4,>=2",
                    "idna<4,>=2.5",
                    "urllib3<3,>=1.21.1",
                    "certifi>=2017.4.17"
                ]
            },
            "releases": {
                "2.31.0": [
                    {
                        "filename": "requests-2.31.0-py3-none-any.whl",
                        "url": "https://files.pythonhosted.org/packages/requests-2.31.0-py3-none-any.whl",
                        "size": 62574,
                        "packagetype": "bdist_wheel",
                        "python_version": "py3",
                        "digests": {
                            "sha256": "58cd2187c01e70e6e26505bca751777aa9f2ee0b7f4300988b709f44e013003f"
                        },
                        "requires_python": ">=3.7"
                    }
                ]
            },
            "urls": []
        }"#;

        let metadata: PackageMetadata = serde_json::from_str(json).unwrap();
        assert_eq!(metadata.info.name, "requests");
        assert_eq!(metadata.info.version, "2.31.0");
        assert_eq!(metadata.info.summary, Some("Python HTTP for Humans.".to_string()));
        assert_eq!(metadata.info.requires_python, Some(">=3.7".to_string()));

        let deps = metadata.info.requires_dist.unwrap();
        assert_eq!(deps.len(), 4);
        assert!(deps.contains(&"charset-normalizer<4,>=2".to_string()));

        let releases = metadata.releases.get("2.31.0").unwrap();
        assert_eq!(releases.len(), 1);
        assert_eq!(releases[0].filename, "requests-2.31.0-py3-none-any.whl");
        assert_eq!(releases[0].packagetype, "bdist_wheel");
    }


    #[test]
    fn test_release_file_deserialize_with_yanked() {
        let json = r#"{
            "filename": "test-1.0.0.tar.gz",
            "url": "https://example.com/test-1.0.0.tar.gz",
            "size": 1234,
            "packagetype": "sdist",
            "digests": {
                "sha256": "abc123"
            },
            "yanked": true,
            "yanked_reason": "Security vulnerability"
        }"#;

        let file: ReleaseFileInfo = serde_json::from_str(json).unwrap();
        assert!(file.yanked);
        assert_eq!(file.yanked_reason, Some("Security vulnerability".to_string()));
    }

    #[test]
    fn test_release_file_deserialize_without_yanked() {
        let json = r#"{
            "filename": "test-1.0.0.tar.gz",
            "url": "https://example.com/test-1.0.0.tar.gz",
            "size": 1234,
            "packagetype": "sdist",
            "digests": {
                "sha256": "abc123"
            }
        }"#;

        let file: ReleaseFileInfo = serde_json::from_str(json).unwrap();
        assert!(!file.yanked);
        assert!(file.yanked_reason.is_none());
    }

    #[test]
    fn test_file_digests_deserialize() {
        let json = r#"{
            "md5": "abc123",
            "sha256": "def456",
            "blake2b_256": "ghi789"
        }"#;

        let digests: FileDigestsInfo = serde_json::from_str(json).unwrap();
        assert_eq!(digests.md5, Some("abc123".to_string()));
        assert_eq!(digests.sha256, "def456");
        assert_eq!(digests.blake2b_256, Some("ghi789".to_string()));
    }

    #[test]
    fn test_file_digests_deserialize_minimal() {
        let json = r#"{
            "sha256": "def456"
        }"#;

        let digests: FileDigestsInfo = serde_json::from_str(json).unwrap();
        assert!(digests.md5.is_none());
        assert_eq!(digests.sha256, "def456");
        assert!(digests.blake2b_256.is_none());
    }

    #[test]
    fn test_parse_wheel_tags_simple() {
        let tag = PyPiDownloader::parse_wheel_tags("requests-2.31.0-py3-none-any.whl").unwrap();
        assert_eq!(tag.name, "requests");
        assert_eq!(tag.version, "2.31.0");
        assert_eq!(tag.python_tags, vec!["py3"]);
        assert_eq!(tag.abi_tags, vec!["none"]);
        assert_eq!(tag.platform_tags, vec!["any"]);
    }

    #[test]
    fn test_parse_wheel_tags_cpython() {
        let tag = PyPiDownloader::parse_wheel_tags(
            "numpy-1.26.0-cp312-cp312-manylinux_2_17_x86_64.whl",
        )
        .unwrap();
        assert_eq!(tag.name, "numpy");
        assert_eq!(tag.version, "1.26.0");
        assert_eq!(tag.python_tags, vec!["cp312"]);
        assert_eq!(tag.abi_tags, vec!["cp312"]);
        assert_eq!(tag.platform_tags, vec!["manylinux_2_17_x86_64"]);
    }

    #[test]
    fn test_parse_wheel_tags_windows() {
        let tag =
            PyPiDownloader::parse_wheel_tags("pywin32-306-cp312-cp312-win_amd64.whl").unwrap();
        assert_eq!(tag.name, "pywin32");
        assert_eq!(tag.platform_tags, vec!["win_amd64"]);
    }

    #[test]
    fn test_parse_wheel_tags_macos() {
        let tag = PyPiDownloader::parse_wheel_tags(
            "cryptography-41.0.0-cp312-cp312-macosx_10_12_x86_64.whl",
        )
        .unwrap();
        assert_eq!(tag.name, "cryptography");
        assert_eq!(tag.platform_tags, vec!["macosx_10_12_x86_64"]);
    }

    #[test]
    fn test_parse_wheel_tags_multi_python() {
        let tag = PyPiDownloader::parse_wheel_tags("six-1.16.0-py2.py3-none-any.whl").unwrap();
        assert_eq!(tag.python_tags, vec!["py2", "py3"]);
    }

    #[test]
    fn test_parse_wheel_tags_invalid() {
        let result = PyPiDownloader::parse_wheel_tags("not-a-wheel.tar.gz");
        assert!(result.is_err());
    }

    #[test]
    fn test_is_wheel_compatible_pure_python() {
        let env = PlatformEnvironment::detect().with_python(3, 12);
        // Pure Python wheels should be compatible everywhere
        assert!(PyPiDownloader::is_wheel_compatible("requests-2.31.0-py3-none-any.whl", &env).unwrap());
    }

    #[test]
    fn test_is_wheel_compatible_py2_py3() {
        let env = PlatformEnvironment::detect().with_python(3, 12);
        // py2.py3 wheels should be compatible with Python 3
        assert!(PyPiDownloader::is_wheel_compatible("six-1.16.0-py2.py3-none-any.whl", &env).unwrap());
    }

    #[test]
    fn test_is_wheel_compatible_wrong_python_version() {
        let env = PlatformEnvironment::detect().with_python(3, 10);
        // cp312 wheel should not be compatible with Python 3.10
        let result = PyPiDownloader::is_wheel_compatible(
            "numpy-1.26.0-cp312-cp312-manylinux_2_17_x86_64.whl",
            &env,
        )
        .unwrap();
        // This depends on the platform, but the Python version check should fail
        // for cp312 on Python 3.10
        assert!(!result);
    }

    #[test]
    fn test_wheel_specificity_scoring() {
        let env = PlatformEnvironment::detect().with_python(3, 12);

        let pure_tag = PyPiDownloader::parse_wheel_tags("pkg-1.0.0-py3-none-any.whl").unwrap();
        let specific_tag = PyPiDownloader::parse_wheel_tags(
            "pkg-1.0.0-cp312-cp312-manylinux_2_17_x86_64.whl",
        )
        .unwrap();

        // Platform-specific wheel should have higher specificity score
        let pure_score = pure_tag.specificity_score(&env);
        let specific_score = specific_tag.specificity_score(&env);

        // The specific wheel should score higher (more specific = preferred)
        assert!(
            specific_score > pure_score,
            "Expected specific wheel score ({}) > pure wheel score ({})",
            specific_score,
            pure_score
        );
    }

    #[test]
    fn test_wheel_abi3_compatibility() {
        let env = PlatformEnvironment::detect().with_python(3, 12);
        // abi3 wheels should be compatible with any Python 3.x
        let result = PyPiDownloader::is_wheel_compatible(
            "cffi-1.16.0-cp312-abi3-manylinux_2_17_x86_64.whl",
            &env,
        );
        // Result depends on platform, but parsing should succeed
        assert!(result.is_ok());
    }
}
