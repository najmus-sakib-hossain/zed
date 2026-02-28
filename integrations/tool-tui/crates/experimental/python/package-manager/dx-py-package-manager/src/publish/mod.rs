//! PyPI Publish System
//!
//! Implements package upload to PyPI and compatible registries using
//! the PyPI upload API (multipart form upload).

use std::path::Path;

use reqwest::blocking::{multipart, Client};
use sha2::{Digest, Sha256};

use crate::{Error, Result};

/// Default PyPI upload URL
pub const DEFAULT_REPOSITORY_URL: &str = "https://upload.pypi.org/legacy/";

/// TestPyPI upload URL
pub const TEST_PYPI_URL: &str = "https://test.pypi.org/legacy/";

/// PyPI Publish Client
///
/// Handles uploading packages to PyPI or compatible registries.
pub struct PublishClient {
    /// HTTP client
    client: Client,
    /// Repository URL
    repository_url: String,
}

impl PublishClient {
    /// Create a new publish client for the default PyPI
    pub fn new() -> Self {
        Self::with_repository(DEFAULT_REPOSITORY_URL)
    }

    /// Create a new publish client for TestPyPI
    pub fn test_pypi() -> Self {
        Self::with_repository(TEST_PYPI_URL)
    }

    /// Create a new publish client for a custom repository
    pub fn with_repository(url: &str) -> Self {
        let client = Client::builder()
            .user_agent(format!("dx-py/{}", env!("CARGO_PKG_VERSION")))
            .build()
            .unwrap_or_else(|_| Client::new());

        Self {
            client,
            repository_url: url.to_string(),
        }
    }

    /// Get the repository URL
    pub fn repository_url(&self) -> &str {
        &self.repository_url
    }

    /// Upload a package file to the repository
    ///
    /// # Arguments
    /// * `file_path` - Path to the wheel or sdist file
    /// * `token` - API token for authentication (use "__token__" as username)
    pub fn upload(&self, file_path: &Path, token: &str) -> Result<UploadResult> {
        if !file_path.exists() {
            return Err(Error::Cache(format!("File not found: {}", file_path.display())));
        }

        let filename = file_path
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| Error::Cache("Invalid filename".to_string()))?;

        // Read file content
        let content = std::fs::read(file_path)?;

        // Compute hashes
        let md5_hash = format!("{:x}", md5::compute(&content));
        let sha256_hash = {
            let mut hasher = Sha256::new();
            hasher.update(&content);
            format!("{:x}", hasher.finalize())
        };

        // Determine file type
        let filetype = if filename.ends_with(".whl") {
            "bdist_wheel"
        } else if filename.ends_with(".tar.gz") {
            "sdist"
        } else {
            return Err(Error::Cache(format!("Unsupported file type: {}", filename)));
        };

        // Parse package info from filename
        let (name, version) = parse_package_info(filename)?;

        // Build multipart form
        let form = multipart::Form::new()
            .text(":action", "file_upload")
            .text("protocol_version", "1")
            .text("name", name.clone())
            .text("version", version.clone())
            .text("filetype", filetype.to_string())
            .text("md5_digest", md5_hash)
            .text("sha256_digest", sha256_hash.clone())
            .part(
                "content",
                multipart::Part::bytes(content)
                    .file_name(filename.to_string())
                    .mime_str("application/octet-stream")
                    .map_err(|e| Error::Cache(format!("Failed to set MIME type: {}", e)))?,
            );

        // Send request with basic auth
        let response = self
            .client
            .post(&self.repository_url)
            .basic_auth("__token__", Some(token))
            .multipart(form)
            .send()
            .map_err(|e| Error::Cache(format!("Upload failed: {}", e)))?;

        let status = response.status();
        let body = response.text().unwrap_or_else(|_| "No response body".to_string());

        if status.is_success() {
            Ok(UploadResult {
                filename: filename.to_string(),
                name,
                version,
                sha256: sha256_hash,
                repository: self.repository_url.clone(),
            })
        } else {
            Err(Error::Cache(format!("Upload failed with status {}: {}", status, body)))
        }
    }

    /// Upload multiple package files
    pub fn upload_all(&self, files: &[&Path], token: &str) -> Result<Vec<UploadResult>> {
        let mut results = Vec::new();
        for file in files {
            results.push(self.upload(file, token)?);
        }
        Ok(results)
    }
}

impl Default for PublishClient {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of a successful upload
#[derive(Debug, Clone)]
pub struct UploadResult {
    /// Uploaded filename
    pub filename: String,
    /// Package name
    pub name: String,
    /// Package version
    pub version: String,
    /// SHA256 hash of the uploaded file
    pub sha256: String,
    /// Repository URL
    pub repository: String,
}

/// Parse package name and version from filename
fn parse_package_info(filename: &str) -> Result<(String, String)> {
    // Wheel format: {name}-{version}(-{build})?-{python}-{abi}-{platform}.whl
    // Sdist format: {name}-{version}.tar.gz

    if filename.ends_with(".whl") {
        // Parse wheel filename
        let parts: Vec<&str> = filename.trim_end_matches(".whl").split('-').collect();
        if parts.len() >= 5 {
            let name = parts[0].replace('_', "-");
            let version = parts[1].to_string();
            return Ok((name, version));
        }
    } else if filename.ends_with(".tar.gz") {
        // Parse sdist filename
        let base = filename.trim_end_matches(".tar.gz");
        if let Some(idx) = base.rfind('-') {
            let name = base[..idx].to_string();
            let version = base[idx + 1..].to_string();
            return Ok((name, version));
        }
    }

    Err(Error::Cache(format!(
        "Could not parse package info from filename: {}",
        filename
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_wheel_filename() {
        let (name, version) = parse_package_info("requests-2.31.0-py3-none-any.whl").unwrap();
        assert_eq!(name, "requests");
        assert_eq!(version, "2.31.0");
    }

    #[test]
    fn test_parse_wheel_with_underscore() {
        let (name, version) = parse_package_info("my_package-1.0.0-py3-none-any.whl").unwrap();
        assert_eq!(name, "my-package");
        assert_eq!(version, "1.0.0");
    }

    #[test]
    fn test_parse_sdist_filename() {
        let (name, version) = parse_package_info("requests-2.31.0.tar.gz").unwrap();
        assert_eq!(name, "requests");
        assert_eq!(version, "2.31.0");
    }

    #[test]
    fn test_parse_sdist_with_hyphen() {
        let (name, version) = parse_package_info("my-package-1.0.0.tar.gz").unwrap();
        assert_eq!(name, "my-package");
        assert_eq!(version, "1.0.0");
    }
}
