//! PyPI registry client
//!
//! Provides HTTP client for fetching package metadata and downloading packages
//! from PyPI and compatible registries.

pub mod private;

pub use private::{
    CredentialProvider, EnvironmentCredentialProvider, NetrcCredentialProvider, RegistryConfig,
    RegistryCredentials, RegistryManager, SslConfig,
};

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::{Error, Result};

/// PyPI JSON API response for package metadata
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PyPiPackageInfo {
    /// Package information
    pub info: PackageInfo,
    /// Available releases (version -> files)
    #[serde(default)]
    pub releases: HashMap<String, Vec<ReleaseFile>>,
    /// URLs for the latest version
    #[serde(default)]
    pub urls: Vec<ReleaseFile>,
}

/// Package metadata from PyPI
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PackageInfo {
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
    /// Package dependencies
    pub requires_dist: Option<Vec<String>>,
}

/// Release file information
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ReleaseFile {
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
    /// SHA256 digest
    pub digests: FileDigests,
    /// Whether this requires Python
    pub requires_python: Option<String>,
}

/// File digests for integrity verification
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FileDigests {
    /// MD5 hash (legacy)
    pub md5: Option<String>,
    /// SHA256 hash
    pub sha256: String,
}

/// PyPI client for fetching package metadata and downloading packages
pub struct PyPiClient {
    /// HTTP client
    client: reqwest::blocking::Client,
    /// Base URL for PyPI API
    base_url: String,
}

impl Default for PyPiClient {
    fn default() -> Self {
        Self::new()
    }
}

impl PyPiClient {
    /// Create a new PyPI client with default settings
    pub fn new() -> Self {
        Self::with_base_url("https://pypi.org")
    }

    /// Create a new PyPI client with a custom base URL
    pub fn with_base_url(base_url: &str) -> Self {
        let client = reqwest::blocking::Client::builder()
            .user_agent("dx-py-package-manager/0.1.0")
            .build()
            .unwrap_or_else(|_| reqwest::blocking::Client::new());

        Self {
            client,
            base_url: base_url.trim_end_matches('/').to_string(),
        }
    }

    /// Get package metadata from PyPI
    pub fn get_package(&self, name: &str) -> Result<PyPiPackageInfo> {
        let url = format!("{}/pypi/{}/json", self.base_url, name);

        let response = self
            .client
            .get(&url)
            .send()
            .map_err(|e| Error::Cache(format!("Network error: {}", e)))?;

        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(Error::InvalidPackageName(format!("Package not found: {}", name)));
        }

        if !response.status().is_success() {
            return Err(Error::Cache(format!("HTTP error: {}", response.status())));
        }

        response.json().map_err(|e| Error::Cache(format!("JSON parse error: {}", e)))
    }

    /// Get package metadata for a specific version
    pub fn get_package_version(&self, name: &str, version: &str) -> Result<PyPiPackageInfo> {
        let url = format!("{}/pypi/{}/{}/json", self.base_url, name, version);

        let response = self
            .client
            .get(&url)
            .send()
            .map_err(|e| Error::Cache(format!("Network error: {}", e)))?;

        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(Error::InvalidPackageName(format!(
                "Package version not found: {}=={}",
                name, version
            )));
        }

        if !response.status().is_success() {
            return Err(Error::Cache(format!("HTTP error: {}", response.status())));
        }

        response.json().map_err(|e| Error::Cache(format!("JSON parse error: {}", e)))
    }

    /// Get all available versions for a package
    pub fn get_versions(&self, name: &str) -> Result<Vec<String>> {
        let info = self.get_package(name)?;
        let mut versions: Vec<String> = info.releases.keys().cloned().collect();
        versions.sort();
        Ok(versions)
    }

    /// Get dependencies for a package version
    pub fn get_dependencies(&self, name: &str, version: &str) -> Result<Vec<String>> {
        let info = self.get_package_version(name, version)?;
        Ok(info.info.requires_dist.unwrap_or_default())
    }

    /// Find the best wheel file for the current platform
    pub fn find_wheel(&self, name: &str, version: &str) -> Result<Option<ReleaseFile>> {
        let info = self.get_package_version(name, version)?;

        // Get files for this version
        let files = info.releases.get(version).cloned().unwrap_or_default();

        // Prefer wheels over sdist
        // Priority: platform-specific wheel > universal wheel > sdist
        let mut best_wheel: Option<ReleaseFile> = None;

        for file in files {
            if file.packagetype == "bdist_wheel" {
                // Check if this is a better match
                let dominated = best_wheel.as_ref().is_some_and(|best| {
                    // Prefer more specific wheels
                    file.filename.contains("any") && !best.filename.contains("any")
                });

                if !dominated {
                    best_wheel = Some(file);
                }
            }
        }

        Ok(best_wheel)
    }

    /// Download a file and verify its integrity
    pub fn download(&self, url: &str, expected_sha256: &str) -> Result<Vec<u8>> {
        let response = self
            .client
            .get(url)
            .send()
            .map_err(|e| Error::Cache(format!("Download error: {}", e)))?;

        if !response.status().is_success() {
            return Err(Error::Cache(format!("Download failed: {}", response.status())));
        }

        let data = response
            .bytes()
            .map_err(|e| Error::Cache(format!("Read error: {}", e)))?
            .to_vec();

        // Verify SHA256
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(&data);
        let computed = hex::encode(hasher.finalize());

        if computed != expected_sha256 {
            return Err(Error::Cache(format!(
                "SHA256 mismatch: expected {}, got {}",
                expected_sha256, computed
            )));
        }

        Ok(data)
    }

    /// Download a wheel file for a package version
    pub fn download_wheel(&self, name: &str, version: &str) -> Result<Vec<u8>> {
        let wheel = self
            .find_wheel(name, version)?
            .ok_or_else(|| Error::Cache(format!("No wheel found for {}=={}", name, version)))?;

        self.download(&wheel.url, &wheel.digests.sha256)
    }
}

/// Parsed dependency specification (PEP 508)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DependencySpec {
    /// Package name
    pub name: String,
    /// Version constraint (e.g., ">=1.0,<2.0")
    pub version_constraint: Option<String>,
    /// Extras (e.g., ["dev", "test"])
    pub extras: Vec<String>,
    /// Environment markers (e.g., "python_version >= '3.8'")
    pub markers: Option<String>,
    /// URL dependency (package @ url)
    pub url: Option<String>,
    /// Path dependency (package @ file://path)
    pub path: Option<String>,
}

impl std::fmt::Display for DependencySpec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)?;

        if !self.extras.is_empty() {
            write!(f, "[{}]", self.extras.join(","))?;
        }

        if let Some(ref url) = self.url {
            write!(f, " @ {}", url)?;
        } else if let Some(ref path) = self.path {
            write!(f, " @ file://{}", path)?;
        } else if let Some(ref constraint) = self.version_constraint {
            write!(f, "{}", constraint)?;
        }

        if let Some(ref markers) = self.markers {
            write!(f, "; {}", markers)?;
        }

        Ok(())
    }
}

impl DependencySpec {
    /// Parse a PEP 508 dependency string
    pub fn parse(spec: &str) -> Result<Self> {
        let spec = spec.trim();

        if spec.is_empty() {
            return Err(Error::InvalidPackageName("Empty dependency spec".to_string()));
        }

        // Check for markers (after ';')
        let (spec_part, markers) = if let Some(idx) = spec.find(';') {
            (spec[..idx].trim(), Some(spec[idx + 1..].trim().to_string()))
        } else {
            (spec, None)
        };

        // Check for URL dependency (@ url)
        let (name_extras_version, url, path) = if let Some(at_idx) = spec_part.find(" @ ") {
            let url_part = spec_part[at_idx + 3..].trim();
            let name_part = spec_part[..at_idx].trim();

            if url_part.starts_with("file://") {
                // Path dependency
                let path_str = url_part.strip_prefix("file://").unwrap_or(url_part);
                (name_part.to_string(), None, Some(path_str.to_string()))
            } else {
                // URL dependency
                (name_part.to_string(), Some(url_part.to_string()), None)
            }
        } else {
            (spec_part.to_string(), None, None)
        };

        // Check for extras (in brackets)
        let (name_and_version, extras) = if let Some(start) = name_extras_version.find('[') {
            if let Some(end) = name_extras_version.find(']') {
                let extras_str = &name_extras_version[start + 1..end];
                let extras: Vec<String> = extras_str
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
                // Combine name part with version part (after ']')
                let name_part = &name_extras_version[..start];
                let version_part = &name_extras_version[end + 1..];
                (format!("{}{}", name_part, version_part), extras)
            } else {
                (name_extras_version, Vec::new())
            }
        } else {
            (name_extras_version, Vec::new())
        };

        // Check for version constraint (only if no URL/path)
        let (name, version_constraint) = if url.is_none() && path.is_none() {
            if let Some(idx) = name_and_version.find(['>', '<', '=', '!', '~']) {
                (
                    name_and_version[..idx].trim().to_string(),
                    Some(name_and_version[idx..].trim().to_string()),
                )
            } else {
                (name_and_version.trim().to_string(), None)
            }
        } else {
            (name_and_version.trim().to_string(), None)
        };

        if name.is_empty() {
            return Err(Error::InvalidPackageName("Empty package name".to_string()));
        }

        // Normalize package name (PEP 503)
        let normalized_name = name.to_lowercase().replace(['-', '.'], "_");

        Ok(Self {
            name: normalized_name,
            version_constraint,
            extras,
            markers,
            url,
            path,
        })
    }

    /// Check if this is a URL dependency
    pub fn is_url_dependency(&self) -> bool {
        self.url.is_some()
    }

    /// Check if this is a path dependency
    pub fn is_path_dependency(&self) -> bool {
        self.path.is_some()
    }

    /// Check if this has version constraints
    pub fn has_version_constraint(&self) -> bool {
        self.version_constraint.is_some()
    }

    /// Get the normalized package name
    pub fn normalized_name(&self) -> &str {
        &self.name
    }
}

/// Async PyPI client for fetching package metadata and downloading packages
pub struct AsyncPyPiClient {
    /// HTTP client
    client: reqwest::Client,
    /// Base URL for PyPI API
    base_url: String,
    /// Extra index URLs
    extra_indexes: Vec<String>,
}

impl Default for AsyncPyPiClient {
    fn default() -> Self {
        Self::new()
    }
}

impl AsyncPyPiClient {
    /// Create a new async PyPI client with default settings
    pub fn new() -> Self {
        Self::with_base_url("https://pypi.org")
    }

    /// Create a new async PyPI client with a custom base URL
    pub fn with_base_url(base_url: &str) -> Self {
        let client = reqwest::Client::builder()
            .user_agent("dx-py/0.1.0")
            .timeout(std::time::Duration::from_secs(300))
            .connect_timeout(std::time::Duration::from_secs(30))
            .pool_max_idle_per_host(10)
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        Self {
            client,
            base_url: base_url.trim_end_matches('/').to_string(),
            extra_indexes: Vec::new(),
        }
    }

    /// Add extra index URLs
    pub fn with_extra_indexes(mut self, indexes: Vec<String>) -> Self {
        self.extra_indexes = indexes;
        self
    }

    /// Get the base URL
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// Get extra index URLs
    pub fn extra_indexes(&self) -> &[String] {
        &self.extra_indexes
    }

    /// Get package metadata from PyPI
    pub async fn get_package(&self, name: &str) -> Result<PyPiPackageInfo> {
        let url = format!("{}/pypi/{}/json", self.base_url, name);

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| Error::Network(format!("Network error: {}", e)))?;

        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(Error::PackageNotFound(name.to_string()));
        }

        if !response.status().is_success() {
            return Err(Error::Network(format!("HTTP error: {}", response.status())));
        }

        response
            .json()
            .await
            .map_err(|e| Error::Network(format!("JSON parse error: {}", e)))
    }

    /// Get package metadata for a specific version
    pub async fn get_package_version(&self, name: &str, version: &str) -> Result<PyPiPackageInfo> {
        let url = format!("{}/pypi/{}/{}/json", self.base_url, name, version);

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| Error::Network(format!("Network error: {}", e)))?;

        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(Error::PackageNotFound(format!("{}=={}", name, version)));
        }

        if !response.status().is_success() {
            return Err(Error::Network(format!("HTTP error: {}", response.status())));
        }

        response
            .json()
            .await
            .map_err(|e| Error::Network(format!("JSON parse error: {}", e)))
    }

    /// Get all available versions for a package
    pub async fn get_versions(&self, name: &str) -> Result<Vec<String>> {
        let info = self.get_package(name).await?;
        let mut versions: Vec<String> = info.releases.keys().cloned().collect();
        versions.sort();
        Ok(versions)
    }

    /// Get dependencies for a package version
    pub async fn get_dependencies(&self, name: &str, version: &str) -> Result<Vec<DependencySpec>> {
        let info = self.get_package_version(name, version).await?;
        let deps = info.info.requires_dist.unwrap_or_default();

        deps.iter().map(|s| DependencySpec::parse(s)).collect()
    }

    /// Find the best wheel file for the given platform environment
    pub async fn find_best_wheel(
        &self,
        name: &str,
        version: &str,
        env: &dx_py_core::wheel::PlatformEnvironment,
    ) -> Result<Option<ReleaseFile>> {
        let info = self.get_package_version(name, version).await?;
        let files = info.releases.get(version).cloned().unwrap_or_default();

        let mut best_wheel: Option<(ReleaseFile, u32)> = None;

        for file in files {
            if file.packagetype != "bdist_wheel" {
                continue;
            }

            // Parse wheel tag
            if let Ok(tag) = dx_py_core::wheel::WheelTag::parse(&file.filename) {
                if tag.is_compatible(env) {
                    let score = tag.specificity_score(env);
                    if best_wheel.as_ref().is_none_or(|(_, best_score)| score > *best_score) {
                        best_wheel = Some((file, score));
                    }
                }
            }
        }

        Ok(best_wheel.map(|(file, _)| file))
    }

    /// Find any compatible wheel or fall back to sdist
    pub async fn find_distribution(
        &self,
        name: &str,
        version: &str,
        env: &dx_py_core::wheel::PlatformEnvironment,
    ) -> Result<Option<ReleaseFile>> {
        // First try to find a compatible wheel
        if let Some(wheel) = self.find_best_wheel(name, version, env).await? {
            return Ok(Some(wheel));
        }

        // Fall back to sdist
        let info = self.get_package_version(name, version).await?;
        let files = info.releases.get(version).cloned().unwrap_or_default();

        for file in files {
            if file.packagetype == "sdist" {
                return Ok(Some(file));
            }
        }

        Ok(None)
    }

    /// Download a file and verify its integrity
    pub async fn download(&self, url: &str, expected_sha256: &str) -> Result<Vec<u8>> {
        let response = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|e| Error::Network(format!("Download error: {}", e)))?;

        if !response.status().is_success() {
            return Err(Error::Network(format!("Download failed: {}", response.status())));
        }

        let data = response
            .bytes()
            .await
            .map_err(|e| Error::Network(format!("Read error: {}", e)))?
            .to_vec();

        // Verify SHA256
        crate::verify_sha256(&data, expected_sha256)?;

        Ok(data)
    }
}

impl Clone for AsyncPyPiClient {
    fn clone(&self) -> Self {
        Self {
            client: self.client.clone(),
            base_url: self.base_url.clone(),
            extra_indexes: self.extra_indexes.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dependency_spec_parse_simple() {
        let spec = DependencySpec::parse("requests").unwrap();
        assert_eq!(spec.name, "requests");
        assert!(spec.version_constraint.is_none());
        assert!(spec.extras.is_empty());
        assert!(spec.markers.is_none());
        assert!(spec.url.is_none());
        assert!(spec.path.is_none());
    }

    #[test]
    fn test_dependency_spec_parse_with_version() {
        let spec = DependencySpec::parse("requests>=2.0").unwrap();
        assert_eq!(spec.name, "requests");
        assert_eq!(spec.version_constraint, Some(">=2.0".to_string()));
    }

    #[test]
    fn test_dependency_spec_parse_with_extras() {
        let spec = DependencySpec::parse("requests[security,socks]>=2.0").unwrap();
        assert_eq!(spec.name, "requests");
        assert_eq!(spec.extras, vec!["security", "socks"]);
        assert_eq!(spec.version_constraint, Some(">=2.0".to_string()));
    }

    #[test]
    fn test_dependency_spec_parse_with_markers() {
        let spec = DependencySpec::parse("requests>=2.0; python_version >= '3.8'").unwrap();
        assert_eq!(spec.name, "requests");
        assert_eq!(spec.version_constraint, Some(">=2.0".to_string()));
        assert_eq!(spec.markers, Some("python_version >= '3.8'".to_string()));
    }

    #[test]
    fn test_dependency_spec_parse_complex() {
        let spec =
            DependencySpec::parse("urllib3[brotli,socks]>=1.21.1,<3; python_version >= '3.7'")
                .unwrap();
        assert_eq!(spec.name, "urllib3");
        assert_eq!(spec.extras, vec!["brotli", "socks"]);
        assert_eq!(spec.version_constraint, Some(">=1.21.1,<3".to_string()));
        assert_eq!(spec.markers, Some("python_version >= '3.7'".to_string()));
    }

    #[test]
    fn test_dependency_spec_parse_url() {
        let spec =
            DependencySpec::parse("mypackage @ https://example.com/mypackage-1.0.0.whl").unwrap();
        assert_eq!(spec.name, "mypackage");
        assert_eq!(spec.url, Some("https://example.com/mypackage-1.0.0.whl".to_string()));
        assert!(spec.version_constraint.is_none());
        assert!(spec.is_url_dependency());
    }

    #[test]
    fn test_dependency_spec_parse_path() {
        let spec = DependencySpec::parse("mypackage @ file:///path/to/package").unwrap();
        assert_eq!(spec.name, "mypackage");
        assert_eq!(spec.path, Some("/path/to/package".to_string()));
        assert!(spec.version_constraint.is_none());
        assert!(spec.is_path_dependency());
    }

    #[test]
    fn test_dependency_spec_parse_url_with_extras() {
        let spec = DependencySpec::parse("mypackage[dev] @ https://example.com/pkg.whl").unwrap();
        assert_eq!(spec.name, "mypackage");
        assert_eq!(spec.extras, vec!["dev"]);
        assert_eq!(spec.url, Some("https://example.com/pkg.whl".to_string()));
    }

    #[test]
    fn test_dependency_spec_parse_url_with_markers() {
        let spec = DependencySpec::parse(
            "mypackage @ https://example.com/pkg.whl; python_version >= '3.8'",
        )
        .unwrap();
        assert_eq!(spec.name, "mypackage");
        assert_eq!(spec.url, Some("https://example.com/pkg.whl".to_string()));
        assert_eq!(spec.markers, Some("python_version >= '3.8'".to_string()));
    }

    #[test]
    fn test_dependency_spec_name_normalization() {
        // Hyphens should be normalized to underscores
        let spec = DependencySpec::parse("my-package>=1.0").unwrap();
        assert_eq!(spec.name, "my_package");

        // Dots should be normalized to underscores
        let spec = DependencySpec::parse("my.package>=1.0").unwrap();
        assert_eq!(spec.name, "my_package");

        // Mixed case should be lowercased
        let spec = DependencySpec::parse("MyPackage>=1.0").unwrap();
        assert_eq!(spec.name, "mypackage");
    }

    #[test]
    fn test_dependency_spec_display_roundtrip() {
        let cases = vec![
            "requests",
            "requests>=2.0",
            "requests[security]>=2.0",
            "requests>=2.0; python_version >= '3.8'",
        ];

        for case in cases {
            let spec = DependencySpec::parse(case).unwrap();
            let formatted = spec.to_string();
            let reparsed = DependencySpec::parse(&formatted).unwrap();
            assert_eq!(spec.name, reparsed.name, "name mismatch for {}", case);
            assert_eq!(spec.extras, reparsed.extras, "extras mismatch for {}", case);
            assert_eq!(
                spec.version_constraint, reparsed.version_constraint,
                "version mismatch for {}",
                case
            );
        }
    }
}
