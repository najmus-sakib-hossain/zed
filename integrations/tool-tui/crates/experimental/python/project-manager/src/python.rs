//! Python version management
//!
//! Provides functionality for discovering, installing, and managing Python versions.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::{Error, Result};

/// Information about a Python installation
#[derive(Debug, Clone)]
pub struct PythonInstall {
    /// Path to the Python executable
    pub path: PathBuf,
    /// Python version string (e.g., "3.12.0")
    pub version: String,
    /// Whether this is a system Python
    pub is_system: bool,
    /// Whether this is managed by dx-py
    pub is_managed: bool,
}

impl PythonInstall {
    /// Create a new Python installation info
    pub fn new(path: PathBuf, version: String) -> Self {
        Self {
            path,
            version,
            is_system: false,
            is_managed: false,
        }
    }

    /// Mark as system Python
    pub fn system(mut self) -> Self {
        self.is_system = true;
        self
    }

    /// Mark as managed by dx-py
    pub fn managed(mut self) -> Self {
        self.is_managed = true;
        self
    }
}

/// Python version manager
///
/// Handles discovery, installation, and management of Python versions.
pub struct PythonManager {
    /// Directory where managed Python versions are installed
    install_dir: PathBuf,
    /// URL for pre-built Python binaries
    #[allow(dead_code)]
    builds_url: String,
    /// Cache of discovered Python installations
    discovered: HashMap<String, PythonInstall>,
}

impl PythonManager {
    /// Create a new Python manager with default settings
    pub fn new() -> Self {
        let install_dir = dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("dx-py")
            .join("python");

        Self {
            install_dir,
            builds_url: "https://github.com/indygreg/python-build-standalone/releases/download"
                .to_string(),
            discovered: HashMap::new(),
        }
    }

    /// Create a Python manager with a custom install directory
    pub fn with_install_dir(install_dir: PathBuf) -> Self {
        Self {
            install_dir,
            builds_url: "https://github.com/indygreg/python-build-standalone/releases/download"
                .to_string(),
            discovered: HashMap::new(),
        }
    }

    /// Get the install directory
    pub fn install_dir(&self) -> &Path {
        &self.install_dir
    }

    /// Discover system Python installations
    #[allow(clippy::map_entry)]
    pub fn discover(&mut self) -> Vec<PythonInstall> {
        let mut found = Vec::new();

        // Check common locations based on platform
        #[cfg(unix)]
        let paths = vec![
            "/usr/bin/python3",
            "/usr/local/bin/python3",
            "/opt/homebrew/bin/python3",
        ];

        #[cfg(windows)]
        let paths = vec![
            "C:\\Python312\\python.exe",
            "C:\\Python311\\python.exe",
            "C:\\Python310\\python.exe",
            "C:\\Python39\\python.exe",
        ];

        for path_str in paths {
            let path = PathBuf::from(path_str);
            if path.exists() {
                if let Ok(version) = self.get_version(&path) {
                    let install = PythonInstall::new(path.clone(), version.clone()).system();
                    self.discovered.insert(version, install.clone());
                    found.push(install);
                }
            }
        }

        // Check PATH
        if let Ok(path_var) = std::env::var("PATH") {
            #[cfg(unix)]
            let separator = ':';
            #[cfg(windows)]
            let separator = ';';

            for dir in path_var.split(separator) {
                #[cfg(unix)]
                let python_path = PathBuf::from(dir).join("python3");
                #[cfg(windows)]
                let python_path = PathBuf::from(dir).join("python.exe");

                if python_path.exists() {
                    if let Ok(version) = self.get_version(&python_path) {
                        if !self.discovered.contains_key(&version) {
                            let install = PythonInstall::new(python_path, version.clone()).system();
                            self.discovered.insert(version, install.clone());
                            found.push(install);
                        }
                    }
                }
            }
        }

        // Check pyenv
        if let Ok(pyenv_root) = std::env::var("PYENV_ROOT") {
            let versions_dir = PathBuf::from(&pyenv_root).join("versions");
            if versions_dir.exists() {
                if let Ok(entries) = std::fs::read_dir(&versions_dir) {
                    for entry in entries.flatten() {
                        let version_dir = entry.path();
                        #[cfg(unix)]
                        let python_path = version_dir.join("bin").join("python");
                        #[cfg(windows)]
                        let python_path = version_dir.join("python.exe");

                        if python_path.exists() {
                            if let Ok(version) = self.get_version(&python_path) {
                                if !self.discovered.contains_key(&version) {
                                    let install = PythonInstall::new(python_path, version.clone());
                                    self.discovered.insert(version, install.clone());
                                    found.push(install);
                                }
                            }
                        }
                    }
                }
            }
        }

        // Check managed installations
        if self.install_dir.exists() {
            if let Ok(entries) = std::fs::read_dir(&self.install_dir) {
                for entry in entries.flatten() {
                    let version_dir = entry.path();
                    #[cfg(unix)]
                    let python_path = version_dir.join("bin").join("python3");
                    #[cfg(windows)]
                    let python_path = version_dir.join("python.exe");

                    if python_path.exists() {
                        if let Ok(version) = self.get_version(&python_path) {
                            if !self.discovered.contains_key(&version) {
                                let install =
                                    PythonInstall::new(python_path, version.clone()).managed();
                                self.discovered.insert(version, install.clone());
                                found.push(install);
                            }
                        }
                    }
                }
            }
        }

        found
    }

    /// Get the version of a Python executable
    pub fn get_version(&self, python_path: &Path) -> Result<String> {
        let output = Command::new(python_path)
            .args(["--version"])
            .output()
            .map_err(|e| Error::PythonNotFound(format!("Failed to run Python: {}", e)))?;

        if !output.status.success() {
            return Err(Error::PythonNotFound(format!(
                "Python returned error: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }

        let version_str = String::from_utf8_lossy(&output.stdout);
        // Parse "Python 3.12.0" -> "3.12.0"
        let version = version_str
            .trim()
            .strip_prefix("Python ")
            .unwrap_or(version_str.trim())
            .to_string();

        Ok(version)
    }

    /// Find a Python installation matching the version constraint
    pub fn find(&self, version_constraint: &str) -> Option<&PythonInstall> {
        // Simple matching for now - exact version or prefix match
        let constraint = version_constraint.trim();

        // Try exact match first
        if let Some(install) = self.discovered.get(constraint) {
            return Some(install);
        }

        // Try prefix match (e.g., "3.12" matches "3.12.0")
        for (version, install) in &self.discovered {
            if version.starts_with(constraint) {
                return Some(install);
            }
        }

        None
    }

    /// Get the path where a version would be installed
    pub fn version_path(&self, version: &str) -> PathBuf {
        self.install_dir.join(version)
    }

    /// Check if a version is installed
    pub fn is_installed(&self, version: &str) -> bool {
        let version_dir = self.version_path(version);
        #[cfg(unix)]
        let python_path = version_dir.join("bin").join("python3");
        #[cfg(windows)]
        let python_path = version_dir.join("python.exe");

        python_path.exists()
    }

    /// Get the Python executable path for a version
    pub fn python_path(&self, version: &str) -> PathBuf {
        let version_dir = self.version_path(version);
        #[cfg(unix)]
        {
            version_dir.join("bin").join("python3")
        }
        #[cfg(windows)]
        {
            version_dir.join("python.exe")
        }
    }

    /// Pin a Python version for a project
    pub fn pin(&self, project_dir: &Path, version: &str) -> Result<()> {
        let pin_file = project_dir.join(".python-version");
        std::fs::write(&pin_file, format!("{}\n", version)).map_err(Error::Io)?;
        Ok(())
    }

    /// Read the pinned Python version for a project
    pub fn read_pin(&self, project_dir: &Path) -> Result<Option<String>> {
        let pin_file = project_dir.join(".python-version");
        if !pin_file.exists() {
            return Ok(None);
        }

        let content = std::fs::read_to_string(&pin_file).map_err(Error::Io)?;
        Ok(Some(content.trim().to_string()))
    }

    /// List all discovered Python installations
    pub fn list(&self) -> Vec<&PythonInstall> {
        self.discovered.values().collect()
    }
}

impl Default for PythonManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_python_manager_new() {
        let manager = PythonManager::new();
        assert!(manager.install_dir().to_string_lossy().contains("dx-py"));
    }

    #[test]
    fn test_python_manager_with_install_dir() {
        let temp_dir = TempDir::new().unwrap();
        let manager = PythonManager::with_install_dir(temp_dir.path().to_path_buf());
        assert_eq!(manager.install_dir(), temp_dir.path());
    }

    #[test]
    fn test_version_path() {
        let temp_dir = TempDir::new().unwrap();
        let manager = PythonManager::with_install_dir(temp_dir.path().to_path_buf());
        let path = manager.version_path("3.12.0");
        assert!(path.ends_with("3.12.0"));
    }

    #[test]
    fn test_pin_and_read() {
        let temp_dir = TempDir::new().unwrap();
        let manager = PythonManager::new();

        manager.pin(temp_dir.path(), "3.12.0").unwrap();
        let pinned = manager.read_pin(temp_dir.path()).unwrap();
        assert_eq!(pinned, Some("3.12.0".to_string()));
    }

    #[test]
    fn test_read_pin_nonexistent() {
        let temp_dir = TempDir::new().unwrap();
        let manager = PythonManager::new();

        let pinned = manager.read_pin(temp_dir.path()).unwrap();
        assert_eq!(pinned, None);
    }
}

use serde::Deserialize;

/// Python release information from python-build-standalone
#[derive(Debug, Clone, Deserialize)]
pub struct PythonRelease {
    /// Python version
    pub version: String,
    /// Download URL
    pub url: String,
    /// SHA256 hash
    pub sha256: String,
    /// Platform (windows, macos, linux)
    pub platform: String,
    /// Architecture (x86_64, aarch64)
    pub arch: String,
}

/// Real Python manager with download support
pub struct RealPythonManager {
    /// Base Python manager
    manager: PythonManager,
    /// HTTP client for downloads
    client: reqwest::blocking::Client,
}

impl RealPythonManager {
    /// Create a new real Python manager
    pub fn new() -> Self {
        Self {
            manager: PythonManager::new(),
            client: reqwest::blocking::Client::builder()
                .user_agent("dx-py/0.1.0")
                .timeout(std::time::Duration::from_secs(600))
                .build()
                .expect("Failed to create HTTP client"),
        }
    }

    /// Create a real Python manager with a custom install directory
    pub fn with_install_dir(install_dir: PathBuf) -> Self {
        Self {
            manager: PythonManager::with_install_dir(install_dir),
            client: reqwest::blocking::Client::builder()
                .user_agent("dx-py/0.1.0")
                .timeout(std::time::Duration::from_secs(600))
                .build()
                .expect("Failed to create HTTP client"),
        }
    }

    /// Get the install directory
    pub fn install_dir(&self) -> &Path {
        self.manager.install_dir()
    }

    /// List available Python versions from python-build-standalone
    pub fn list_available(&self) -> Result<Vec<PythonRelease>> {
        // Get releases from GitHub API
        let url =
            "https://api.github.com/repos/indygreg/python-build-standalone/releases?per_page=10";

        let response = self
            .client
            .get(url)
            .header("Accept", "application/vnd.github.v3+json")
            .send()
            .map_err(|e| Error::PythonNotFound(format!("Failed to fetch releases: {}", e)))?;

        if !response.status().is_success() {
            return Err(Error::PythonNotFound(format!("GitHub API error: {}", response.status())));
        }

        let releases: Vec<GitHubRelease> = response
            .json()
            .map_err(|e| Error::PythonNotFound(format!("Failed to parse releases: {}", e)))?;

        let mut python_releases = Vec::new();
        let (platform, arch) = self.detect_platform();

        for release in releases {
            for asset in release.assets {
                if let Some(pr) = self.parse_asset(&asset, &platform, &arch) {
                    python_releases.push(pr);
                }
            }
        }

        Ok(python_releases)
    }

    /// Detect current platform and architecture
    fn detect_platform(&self) -> (String, String) {
        #[cfg(target_os = "windows")]
        let platform = "windows".to_string();
        #[cfg(target_os = "macos")]
        let platform = "apple-darwin".to_string();
        #[cfg(target_os = "linux")]
        let platform = "unknown-linux-gnu".to_string();
        #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
        let platform = "unknown".to_string();

        #[cfg(target_arch = "x86_64")]
        let arch = "x86_64".to_string();
        #[cfg(target_arch = "aarch64")]
        let arch = "aarch64".to_string();
        #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
        let arch = "unknown".to_string();

        (platform, arch)
    }

    /// Parse a GitHub asset into a PythonRelease
    fn parse_asset(
        &self,
        asset: &GitHubAsset,
        platform: &str,
        arch: &str,
    ) -> Option<PythonRelease> {
        let name = &asset.name;

        // Filter for install_only builds for our platform
        if !name.contains("install_only") {
            return None;
        }
        if !name.contains(platform) {
            return None;
        }
        if !name.contains(arch) {
            return None;
        }

        // Extract version from filename
        // Format: cpython-3.12.0+20231002-x86_64-unknown-linux-gnu-install_only.tar.gz
        let version = name.strip_prefix("cpython-")?.split('+').next()?.to_string();

        Some(PythonRelease {
            version,
            url: asset.browser_download_url.clone(),
            sha256: String::new(), // Would need to fetch from checksum file
            platform: platform.to_string(),
            arch: arch.to_string(),
        })
    }

    /// Install a Python version
    pub fn install(&self, version: &str) -> Result<PythonInstall> {
        // Check if already installed
        if self.manager.is_installed(version) {
            let path = self.manager.python_path(version);
            return Ok(PythonInstall::new(path, version.to_string()).managed());
        }

        // Find the release for this version
        let releases = self.list_available()?;
        let release = releases
            .iter()
            .find(|r| r.version == version || r.version.starts_with(version))
            .ok_or_else(|| Error::PythonNotFound(format!("Version {} not found", version)))?;

        // Download the archive
        let data = self.download(&release.url)?;

        // Verify SHA256 if available
        if !release.sha256.is_empty() {
            self.verify_sha256(&data, &release.sha256)?;
        }

        // Extract to install directory
        let install_path = self.manager.version_path(&release.version);
        self.extract(&data, &install_path, &release.url)?;

        let python_path = self.manager.python_path(&release.version);
        Ok(PythonInstall::new(python_path, release.version.clone()).managed())
    }

    /// Download a file
    fn download(&self, url: &str) -> Result<Vec<u8>> {
        let response = self
            .client
            .get(url)
            .send()
            .map_err(|e| Error::PythonNotFound(format!("Download failed: {}", e)))?;

        if !response.status().is_success() {
            return Err(Error::PythonNotFound(format!("Download failed: {}", response.status())));
        }

        let data = response
            .bytes()
            .map_err(|e| Error::PythonNotFound(format!("Failed to read response: {}", e)))?
            .to_vec();

        Ok(data)
    }

    /// Verify SHA256 hash
    fn verify_sha256(&self, data: &[u8], expected: &str) -> Result<()> {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(data);
        let computed = hex::encode(hasher.finalize());

        if computed != expected.to_lowercase() {
            return Err(Error::PythonNotFound(format!(
                "SHA256 mismatch: expected {}, got {}",
                expected, computed
            )));
        }

        Ok(())
    }

    /// Extract archive to destination
    fn extract(&self, data: &[u8], dest: &Path, url: &str) -> Result<()> {
        std::fs::create_dir_all(dest)?;

        if url.ends_with(".tar.gz") || url.ends_with(".tgz") {
            self.extract_tar_gz(data, dest)?;
        } else if url.ends_with(".zip") {
            self.extract_zip(data, dest)?;
        } else {
            return Err(Error::PythonNotFound(format!("Unknown archive format: {}", url)));
        }

        Ok(())
    }

    /// Extract tar.gz archive
    fn extract_tar_gz(&self, data: &[u8], dest: &Path) -> Result<()> {
        use flate2::read::GzDecoder;
        use tar::Archive;

        let decoder = GzDecoder::new(std::io::Cursor::new(data));
        let mut archive = Archive::new(decoder);

        // Extract, stripping the first component (usually "python")
        for entry in archive
            .entries()
            .map_err(|e| Error::PythonNotFound(format!("Failed to read archive: {}", e)))?
        {
            let mut entry =
                entry.map_err(|e| Error::PythonNotFound(format!("Failed to read entry: {}", e)))?;
            let path = entry
                .path()
                .map_err(|e| Error::PythonNotFound(format!("Invalid path: {}", e)))?;

            // Strip first component
            let stripped: PathBuf = path.components().skip(1).collect();
            if stripped.as_os_str().is_empty() {
                continue;
            }

            let dest_path = dest.join(&stripped);

            if entry.header().entry_type().is_dir() {
                std::fs::create_dir_all(&dest_path)?;
            } else {
                if let Some(parent) = dest_path.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                entry
                    .unpack(&dest_path)
                    .map_err(|e| Error::PythonNotFound(format!("Failed to extract: {}", e)))?;
            }
        }

        Ok(())
    }

    /// Extract zip archive
    fn extract_zip(&self, data: &[u8], dest: &Path) -> Result<()> {
        use std::io::Read;
        use zip::ZipArchive;

        let cursor = std::io::Cursor::new(data);
        let mut archive = ZipArchive::new(cursor)
            .map_err(|e| Error::PythonNotFound(format!("Failed to open zip: {}", e)))?;

        for i in 0..archive.len() {
            let mut file = archive
                .by_index(i)
                .map_err(|e| Error::PythonNotFound(format!("Failed to read entry: {}", e)))?;

            let path = file.name().to_string();

            // Strip first component
            let stripped: PathBuf = PathBuf::from(&path).components().skip(1).collect();
            if stripped.as_os_str().is_empty() {
                continue;
            }

            let dest_path = dest.join(&stripped);

            if file.is_dir() {
                std::fs::create_dir_all(&dest_path)?;
            } else {
                if let Some(parent) = dest_path.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                let mut content = Vec::new();
                file.read_to_end(&mut content)
                    .map_err(|e| Error::PythonNotFound(format!("Failed to read file: {}", e)))?;
                std::fs::write(&dest_path, &content)?;
            }
        }

        Ok(())
    }

    /// Check if a version is installed
    pub fn is_installed(&self, version: &str) -> bool {
        self.manager.is_installed(version)
    }

    /// Discover Python installations
    pub fn discover(&mut self) -> Vec<PythonInstall> {
        self.manager.discover()
    }

    /// Find a Python installation
    pub fn find(&self, version_constraint: &str) -> Option<&PythonInstall> {
        self.manager.find(version_constraint)
    }
}

impl Default for RealPythonManager {
    fn default() -> Self {
        Self::new()
    }
}

/// GitHub release response
#[derive(Debug, Deserialize)]
struct GitHubRelease {
    #[allow(dead_code)]
    tag_name: String,
    assets: Vec<GitHubAsset>,
}

/// GitHub asset response
#[derive(Debug, Deserialize)]
struct GitHubAsset {
    name: String,
    browser_download_url: String,
}
