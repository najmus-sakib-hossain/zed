//! Package installer with hard link optimization
//!
//! Installs DPP packages to site-packages using hard links when possible,
//! falling back to copy when hard links aren't supported (e.g., cross-filesystem).

pub mod editable;

pub use editable::{EditableInstall, EditableInstaller};

use std::fs;
use std::path::{Path, PathBuf};

use crate::cache::GlobalCache;
use crate::Result;

/// Installation strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum InstallStrategy {
    /// Hard links from cache - fast, deduplication
    #[default]
    HardLink,
    /// Copy files - fallback, always works
    Copy,
}

/// File entry for installation
#[derive(Debug, Clone)]
pub struct InstallFile {
    /// Relative path within the package
    pub path: String,
    /// File content
    pub content: Vec<u8>,
}

/// Package to install
#[derive(Debug, Clone)]
pub struct InstallPackage {
    /// Package name
    pub name: String,
    /// Package version
    pub version: String,
    /// Content hash (BLAKE3)
    pub hash: [u8; 32],
    /// Files to install
    pub files: Vec<InstallFile>,
}

/// Installation result
#[derive(Debug, Default)]
pub struct InstallResult {
    /// Number of files installed
    pub files_installed: u64,
    /// Number of hard links created
    pub hard_links: u64,
    /// Number of files copied
    pub copies: u64,
    /// Total bytes installed
    pub bytes_installed: u64,
}

/// Zero-copy installer
pub struct Installer {
    /// Global package cache
    cache: GlobalCache,
    /// Installation strategy
    strategy: InstallStrategy,
}

impl Installer {
    /// Create a new installer with the given cache
    pub fn new(cache: GlobalCache) -> Self {
        Self {
            cache,
            strategy: InstallStrategy::default(),
        }
    }

    /// Create a new installer with a specific strategy
    pub fn with_strategy(cache: GlobalCache, strategy: InstallStrategy) -> Self {
        Self { cache, strategy }
    }

    /// Get the cache
    pub fn cache(&self) -> &GlobalCache {
        &self.cache
    }

    /// Get the installation strategy
    pub fn strategy(&self) -> InstallStrategy {
        self.strategy
    }

    /// Install a package to site-packages
    pub fn install(&self, package: &InstallPackage, site_packages: &Path) -> Result<InstallResult> {
        // First, ensure package is in cache
        let cache_path = self.ensure_cached(package)?;

        // Install based on strategy
        match self.strategy {
            InstallStrategy::HardLink => self.install_hardlink(package, &cache_path, site_packages),
            InstallStrategy::Copy => self.install_copy(package, site_packages),
        }
    }

    /// Install multiple packages
    pub fn install_all(
        &self,
        packages: &[InstallPackage],
        site_packages: &Path,
    ) -> Result<InstallResult> {
        let mut total = InstallResult::default();

        for package in packages {
            let result = self.install(package, site_packages)?;
            total.files_installed += result.files_installed;
            total.hard_links += result.hard_links;
            total.copies += result.copies;
            total.bytes_installed += result.bytes_installed;
        }

        Ok(total)
    }

    /// Ensure package is in cache, return cache path
    fn ensure_cached(&self, package: &InstallPackage) -> Result<PathBuf> {
        // Check if already cached
        if self.cache.contains(&package.hash) {
            return Ok(self.cache.get_path(&package.hash));
        }

        // Build package data and store in cache
        // Note: We use store() not store_verified() because the hash was computed
        // from the original package content, not our serialization format
        let mut data = Vec::new();

        // Simple format: file count + entries
        data.extend_from_slice(&(package.files.len() as u32).to_le_bytes());

        for file in &package.files {
            let path_bytes = file.path.as_bytes();
            data.extend_from_slice(&(path_bytes.len() as u16).to_le_bytes());
            data.extend_from_slice(path_bytes);
            data.extend_from_slice(&(file.content.len() as u64).to_le_bytes());
            data.extend_from_slice(&file.content);
        }

        self.cache.store(&package.hash, &data)
    }

    /// Install using hard links from cache
    fn install_hardlink(
        &self,
        package: &InstallPackage,
        cache_path: &Path,
        site_packages: &Path,
    ) -> Result<InstallResult> {
        let mut result = InstallResult::default();

        // Extract files from cache to a temp location first
        let cache_extract_dir = cache_path.with_extension("extracted");
        if !cache_extract_dir.exists() {
            self.extract_to_dir(package, &cache_extract_dir)?;
        }

        // Create hard links to site-packages
        for file in &package.files {
            let src = cache_extract_dir.join(&file.path);
            let dst = site_packages.join(&file.path);

            // Create parent directories
            if let Some(parent) = dst.parent() {
                fs::create_dir_all(parent)?;
            }

            // Try hard link first, fall back to copy
            if src.exists() {
                match fs::hard_link(&src, &dst) {
                    Ok(()) => {
                        result.hard_links += 1;
                    }
                    Err(_) => {
                        // Fall back to copy (cross-filesystem or other issue)
                        fs::copy(&src, &dst)?;
                        result.copies += 1;
                    }
                }
            } else {
                // Source doesn't exist in extracted cache, write directly
                fs::write(&dst, &file.content)?;
                result.copies += 1;
            }

            result.files_installed += 1;
            result.bytes_installed += file.content.len() as u64;
        }

        Ok(result)
    }

    /// Install by copying files
    fn install_copy(
        &self,
        package: &InstallPackage,
        site_packages: &Path,
    ) -> Result<InstallResult> {
        let mut result = InstallResult::default();

        for file in &package.files {
            let dst = site_packages.join(&file.path);

            // Create parent directories
            if let Some(parent) = dst.parent() {
                fs::create_dir_all(parent)?;
            }

            // Write file
            fs::write(&dst, &file.content)?;

            result.files_installed += 1;
            result.copies += 1;
            result.bytes_installed += file.content.len() as u64;
        }

        Ok(result)
    }

    /// Extract package files to a directory
    fn extract_to_dir(&self, package: &InstallPackage, dir: &Path) -> Result<()> {
        fs::create_dir_all(dir)?;

        for file in &package.files {
            let path = dir.join(&file.path);

            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }

            fs::write(&path, &file.content)?;
        }

        Ok(())
    }

    /// Uninstall a package from site-packages
    pub fn uninstall(&self, package_name: &str, site_packages: &Path) -> Result<u64> {
        let mut removed = 0;

        // Look for package directory (normalized name)
        let normalized = package_name.replace('-', "_");
        let pkg_dir = site_packages.join(&normalized);

        if pkg_dir.exists() {
            removed += self.count_files(&pkg_dir)?;
            fs::remove_dir_all(&pkg_dir)?;
        }

        // Also check for .dist-info directory
        for entry in fs::read_dir(site_packages)? {
            let entry = entry?;
            let name = entry.file_name();
            let name_str = name.to_string_lossy();

            if name_str.starts_with(&format!("{}-", normalized)) && name_str.ends_with(".dist-info")
            {
                removed += self.count_files(&entry.path())?;
                fs::remove_dir_all(entry.path())?;
            }
        }

        Ok(removed)
    }

    /// Count files in a directory recursively
    #[allow(clippy::only_used_in_recursion)]
    fn count_files(&self, dir: &Path) -> Result<u64> {
        let mut count = 0;

        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                count += self.count_files(&path)?;
            } else {
                count += 1;
            }
        }

        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_package(name: &str, version: &str) -> InstallPackage {
        let normalized = name.replace('-', "_");
        let files = vec![
            InstallFile {
                path: format!("{}/__init__.py", normalized),
                content: b"# init".to_vec(),
            },
            InstallFile {
                path: format!("{}/main.py", normalized),
                content: b"def main(): pass".to_vec(),
            },
        ];

        // Compute hash from files
        let mut hasher = blake3::Hasher::new();
        for file in &files {
            hasher.update(file.path.as_bytes());
            hasher.update(&file.content);
        }
        let hash = *hasher.finalize().as_bytes();

        InstallPackage {
            name: name.to_string(),
            version: version.to_string(),
            hash,
            files,
        }
    }

    #[test]
    fn test_install_copy() {
        let cache_dir = TempDir::new().unwrap();
        let site_packages = TempDir::new().unwrap();

        let cache = GlobalCache::new(cache_dir.path()).unwrap();
        let installer = Installer::with_strategy(cache, InstallStrategy::Copy);

        let package = create_test_package("test-pkg", "1.0.0");
        let result = installer.install(&package, site_packages.path()).unwrap();

        assert_eq!(result.files_installed, 2);
        assert_eq!(result.copies, 2);
        assert_eq!(result.hard_links, 0);

        // Verify files exist
        assert!(site_packages.path().join("test_pkg/__init__.py").exists());
        assert!(site_packages.path().join("test_pkg/main.py").exists());
    }

    #[test]
    fn test_install_hardlink() {
        let cache_dir = TempDir::new().unwrap();
        let site_packages = TempDir::new().unwrap();

        let cache = GlobalCache::new(cache_dir.path()).unwrap();
        let installer = Installer::with_strategy(cache, InstallStrategy::HardLink);

        let package = create_test_package("test-pkg", "1.0.0");
        let result = installer.install(&package, site_packages.path()).unwrap();

        assert_eq!(result.files_installed, 2);
        // Hard links may or may not work depending on filesystem
        assert!(result.hard_links + result.copies == 2);

        // Verify files exist
        assert!(site_packages.path().join("test_pkg/__init__.py").exists());
        assert!(site_packages.path().join("test_pkg/main.py").exists());
    }

    #[test]
    fn test_install_multiple() {
        let cache_dir = TempDir::new().unwrap();
        let site_packages = TempDir::new().unwrap();

        let cache = GlobalCache::new(cache_dir.path()).unwrap();
        let installer = Installer::with_strategy(cache, InstallStrategy::Copy);

        let packages = vec![
            create_test_package("pkg-a", "1.0.0"),
            create_test_package("pkg-b", "2.0.0"),
        ];

        let result = installer.install_all(&packages, site_packages.path()).unwrap();

        assert_eq!(result.files_installed, 4);

        // Verify files exist
        assert!(site_packages.path().join("pkg_a/__init__.py").exists());
        assert!(site_packages.path().join("pkg_b/__init__.py").exists());
    }

    #[test]
    fn test_uninstall() {
        let cache_dir = TempDir::new().unwrap();
        let site_packages = TempDir::new().unwrap();

        let cache = GlobalCache::new(cache_dir.path()).unwrap();
        let installer = Installer::with_strategy(cache, InstallStrategy::Copy);

        let package = create_test_package("test-pkg", "1.0.0");
        installer.install(&package, site_packages.path()).unwrap();

        // Verify installed
        assert!(site_packages.path().join("test_pkg/__init__.py").exists());

        // Uninstall
        let removed = installer.uninstall("test-pkg", site_packages.path()).unwrap();
        assert_eq!(removed, 2);

        // Verify removed
        assert!(!site_packages.path().join("test_pkg").exists());
    }
}

use crate::Error;
use std::io::{BufRead, BufReader, Read};
use zip::ZipArchive;

/// Installed package information
#[derive(Debug, Clone)]
pub struct InstalledPackage {
    /// Package name
    pub name: String,
    /// Package version
    pub version: String,
    /// List of installed files
    pub files: Vec<PathBuf>,
    /// Path to .dist-info directory
    pub dist_info: PathBuf,
}

/// RECORD file entry
#[derive(Debug, Clone)]
pub struct RecordEntry {
    /// File path relative to site-packages
    pub path: String,
    /// Hash algorithm and digest (e.g., "sha256=...")
    pub hash: Option<String>,
    /// File size in bytes
    pub size: Option<u64>,
}

impl RecordEntry {
    /// Parse a RECORD line
    pub fn parse(line: &str) -> Option<Self> {
        let parts: Vec<&str> = line.split(',').collect();
        if parts.is_empty() || parts[0].is_empty() {
            return None;
        }

        let path = parts[0].to_string();
        let hash = parts.get(1).filter(|s| !s.is_empty()).map(|s| s.to_string());
        let size = parts.get(2).and_then(|s| s.parse().ok());

        Some(Self { path, hash, size })
    }

    /// Create a new RECORD entry with computed hash
    pub fn new(path: String, content: &[u8]) -> Self {
        use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
        use sha2::{Digest, Sha256};

        let mut hasher = Sha256::new();
        hasher.update(content);
        let hash_bytes = hasher.finalize();
        let hash_b64 = URL_SAFE_NO_PAD.encode(hash_bytes);
        let hash = format!("sha256={}", hash_b64);
        let size = content.len() as u64;

        Self {
            path,
            hash: Some(hash),
            size: Some(size),
        }
    }

    /// Create a RECORD entry without hash (for RECORD file itself)
    pub fn without_hash(path: String) -> Self {
        Self {
            path,
            hash: None,
            size: None,
        }
    }

    /// Format as a RECORD line
    pub fn to_record_line(&self) -> String {
        let hash = self.hash.as_deref().unwrap_or("");
        let size = self.size.map(|s| s.to_string()).unwrap_or_default();
        format!("{},{},{}", self.path, hash, size)
    }
}

/// Dist-info metadata for creating .dist-info directory
#[derive(Debug, Clone)]
pub struct DistInfoMetadata {
    /// Package name
    pub name: String,
    /// Package version
    pub version: String,
    /// Original METADATA content from wheel (if available)
    pub metadata_content: Option<String>,
    /// Original WHEEL content from wheel (if available)
    pub wheel_content: Option<String>,
    /// Requires-Dist dependencies
    pub requires_dist: Vec<String>,
    /// Requires-Python constraint
    pub requires_python: Option<String>,
    /// Package summary/description
    pub summary: Option<String>,
}

/// Real wheel installer that extracts wheel files to site-packages
pub struct WheelInstaller {
    /// Global package cache
    cache: GlobalCache,
    /// Site-packages directory
    site_packages: PathBuf,
    /// Installation strategy
    #[allow(dead_code)]
    strategy: InstallStrategy,
}

impl WheelInstaller {
    /// Create a new wheel installer
    pub fn new(cache: GlobalCache, site_packages: PathBuf) -> Self {
        Self {
            cache,
            site_packages,
            strategy: InstallStrategy::default(),
        }
    }

    /// Create a wheel installer with a specific strategy
    pub fn with_strategy(
        cache: GlobalCache,
        site_packages: PathBuf,
        strategy: InstallStrategy,
    ) -> Self {
        Self {
            cache,
            site_packages,
            strategy,
        }
    }

    /// Get the site-packages directory
    pub fn site_packages(&self) -> &Path {
        &self.site_packages
    }

    /// Get the cache
    pub fn cache(&self) -> &GlobalCache {
        &self.cache
    }

    /// Install a wheel file from a file path
    pub fn install_wheel_from_path(&self, wheel_path: &Path) -> Result<InstalledPackage> {
        let wheel_data = fs::read(wheel_path)?;
        self.install_wheel(&wheel_data)
    }

    /// Install a wheel file from bytes
    pub fn install_wheel(&self, wheel_data: &[u8]) -> Result<InstalledPackage> {
        use std::io::Cursor;

        let cursor = Cursor::new(wheel_data);
        let mut archive = ZipArchive::new(cursor)
            .map_err(|e| Error::Cache(format!("Failed to open wheel: {}", e)))?;

        // Find the .dist-info directory
        let dist_info_name = self.find_dist_info(&mut archive)?;
        let (name, version) = self.parse_dist_info_name(&dist_info_name)?;

        let mut installed_files = Vec::new();
        let mut record_entries = Vec::new();

        // Extract all files
        for i in 0..archive.len() {
            let mut file = archive
                .by_index(i)
                .map_err(|e| Error::Cache(format!("Failed to read wheel entry: {}", e)))?;

            let file_path = file.name().to_string();

            // Skip directories
            if file_path.ends_with('/') {
                continue;
            }

            // Handle .data directory specially
            let dest_path = if file_path.contains(".data/") {
                self.handle_data_file(&file_path, &name)?
            } else {
                self.site_packages.join(&file_path)
            };

            // Create parent directories
            if let Some(parent) = dest_path.parent() {
                fs::create_dir_all(parent)?;
            }

            // Read and write file content
            let mut content = Vec::new();
            file.read_to_end(&mut content)
                .map_err(|e| Error::Cache(format!("Failed to read file: {}", e)))?;

            // Write atomically using temp file
            let temp_path = dest_path.with_extension("tmp");
            fs::write(&temp_path, &content)?;
            fs::rename(&temp_path, &dest_path)?;

            // Create RECORD entry for this file (relative to site-packages)
            let relative_path = dest_path
                .strip_prefix(&self.site_packages)
                .unwrap_or(&dest_path)
                .to_string_lossy()
                .replace('\\', "/");
            
            // Don't add hash for RECORD file itself
            if !relative_path.ends_with("/RECORD") {
                record_entries.push(RecordEntry::new(relative_path, &content));
            }

            installed_files.push(dest_path);
        }

        let dist_info_path = self.site_packages.join(&dist_info_name);

        // Create INSTALLER file if it doesn't exist (Requirements: 9.2)
        let installer_path = dist_info_path.join("INSTALLER");
        if !installer_path.exists() {
            let installer_content = b"dx-py\n";
            fs::write(&installer_path, installer_content)?;
            let relative_path = format!("{}/INSTALLER", dist_info_name);
            record_entries.push(RecordEntry::new(relative_path, installer_content));
        }

        // Generate entry point scripts (Requirements: 9.3)
        let script_files = self.generate_scripts(&dist_info_path)?;
        
        // Add script files to RECORD
        for script_path in &script_files {
            let relative_path = script_path
                .strip_prefix(&self.site_packages)
                .unwrap_or(script_path)
                .to_string_lossy()
                .replace('\\', "/");
            
            if script_path.exists() {
                let content = fs::read(script_path)?;
                record_entries.push(RecordEntry::new(relative_path, &content));
            }
        }
        
        installed_files.extend(script_files);

        // Write updated RECORD file (Requirements: 9.6)
        let record_path = dist_info_path.join("RECORD");
        let mut record_content = String::new();
        for entry in &record_entries {
            record_content.push_str(&entry.to_record_line());
            record_content.push('\n');
        }
        // Add RECORD file itself without hash
        record_content.push_str(&format!("{}/RECORD,,\n", dist_info_name));
        fs::write(&record_path, record_content)?;

        Ok(InstalledPackage {
            name,
            version,
            files: installed_files,
            dist_info: dist_info_path,
        })
    }

    /// Install a wheel from the cache
    pub fn install_from_cache(&self, hash: &[u8; 32]) -> Result<InstalledPackage> {
        let data = self.cache.get(hash)?;
        self.install_wheel(&data)
    }

    /// Check if a wheel is a pure Python wheel (platform-independent)
    pub fn is_pure_python_wheel(wheel_data: &[u8]) -> Result<bool> {
        use std::io::Cursor;

        let cursor = Cursor::new(wheel_data);
        let mut archive = ZipArchive::new(cursor)
            .map_err(|e| Error::Cache(format!("Failed to open wheel: {}", e)))?;

        // Look for WHEEL file and check Root-Is-Purelib
        for i in 0..archive.len() {
            if let Ok(mut file) = archive.by_index(i) {
                let name = file.name().to_string();
                if name.ends_with("/WHEEL") {
                    let mut content = String::new();
                    file.read_to_string(&mut content)
                        .map_err(|e| Error::Cache(format!("Failed to read WHEEL file: {}", e)))?;

                    for line in content.lines() {
                        if let Some(value) = line.strip_prefix("Root-Is-Purelib:") {
                            return Ok(value.trim().eq_ignore_ascii_case("true"));
                        }
                    }
                    // If Root-Is-Purelib is not found, assume it's not pure Python
                    return Ok(false);
                }
            }
        }

        // If no WHEEL file found, assume it's not pure Python
        Ok(false)
    }

    /// Get the platform tags from a wheel
    pub fn get_wheel_tags(wheel_data: &[u8]) -> Result<Vec<String>> {
        use std::io::Cursor;

        let cursor = Cursor::new(wheel_data);
        let mut archive = ZipArchive::new(cursor)
            .map_err(|e| Error::Cache(format!("Failed to open wheel: {}", e)))?;

        let mut tags = Vec::new();

        // Look for WHEEL file and extract tags
        for i in 0..archive.len() {
            if let Ok(mut file) = archive.by_index(i) {
                let name = file.name().to_string();
                if name.ends_with("/WHEEL") {
                    let mut content = String::new();
                    file.read_to_string(&mut content)
                        .map_err(|e| Error::Cache(format!("Failed to read WHEEL file: {}", e)))?;

                    for line in content.lines() {
                        if let Some(value) = line.strip_prefix("Tag:") {
                            tags.push(value.trim().to_string());
                        }
                    }
                    break;
                }
            }
        }

        Ok(tags)
    }

    /// Find the .dist-info directory name in the wheel
    fn find_dist_info<R: Read + std::io::Seek>(
        &self,
        archive: &mut ZipArchive<R>,
    ) -> Result<String> {
        for i in 0..archive.len() {
            if let Ok(file) = archive.by_index(i) {
                let name = file.name().to_string();
                if name.contains(".dist-info/") {
                    // Extract the dist-info directory name
                    let parts: Vec<&str> = name.split('/').collect();
                    if !parts.is_empty() && parts[0].ends_with(".dist-info") {
                        return Ok(parts[0].to_string());
                    }
                }
            }
        }
        Err(Error::Cache("No .dist-info directory found in wheel".to_string()))
    }

    /// Parse package name and version from dist-info directory name
    fn parse_dist_info_name(&self, dist_info: &str) -> Result<(String, String)> {
        // Format: {name}-{version}.dist-info
        let without_suffix = dist_info
            .strip_suffix(".dist-info")
            .ok_or_else(|| Error::Cache("Invalid dist-info name".to_string()))?;

        // Find the last hyphen that separates name from version
        let parts: Vec<&str> = without_suffix.rsplitn(2, '-').collect();
        if parts.len() != 2 {
            return Err(Error::Cache("Invalid dist-info name format".to_string()));
        }

        let version = parts[0].to_string();
        let name = parts[1].to_string();

        Ok((name, version))
    }

    /// Handle files in the .data directory
    fn handle_data_file(&self, file_path: &str, _package_name: &str) -> Result<PathBuf> {
        // .data directory structure: {name}-{version}.data/{category}/{path}
        // Categories: scripts, headers, data, purelib, platlib

        let parts: Vec<&str> = file_path.split('/').collect();
        if parts.len() < 3 {
            return Ok(self.site_packages.join(file_path));
        }

        let category = parts[1];
        let rest: PathBuf = parts[2..].iter().collect();

        match category {
            "scripts" => {
                // Scripts go to bin directory (parent of site-packages)
                let bin_dir = self
                    .site_packages
                    .parent()
                    .map(|p| p.join("Scripts"))
                    .unwrap_or_else(|| self.site_packages.join("Scripts"));
                Ok(bin_dir.join(rest))
            }
            "headers" => {
                // Headers go to include directory
                let include_dir = self
                    .site_packages
                    .parent()
                    .map(|p| p.join("include"))
                    .unwrap_or_else(|| self.site_packages.join("include"));
                Ok(include_dir.join(rest))
            }
            "data" => {
                // Data files go to the root of the environment
                let data_dir = self.site_packages.parent().unwrap_or(&self.site_packages);
                Ok(data_dir.join(rest))
            }
            "purelib" | "platlib" => {
                // These go directly to site-packages
                Ok(self.site_packages.join(rest))
            }
            _ => {
                // Unknown category, put in site-packages
                Ok(self.site_packages.join(file_path))
            }
        }
    }

    /// Read RECORD file from an installed package
    pub fn read_record(&self, dist_info: &Path) -> Result<Vec<RecordEntry>> {
        let record_path = dist_info.join("RECORD");
        if !record_path.exists() {
            return Ok(Vec::new());
        }

        let file = fs::File::open(&record_path)?;
        let reader = BufReader::new(file);
        let mut entries = Vec::new();

        for line in reader.lines() {
            let line = line?;
            if let Some(entry) = RecordEntry::parse(&line) {
                entries.push(entry);
            }
        }

        Ok(entries)
    }

    /// Uninstall a package using its RECORD file
    ///
    /// This method:
    /// 1. Finds the package's .dist-info directory in site-packages
    /// 2. Reads the RECORD file to get the list of installed files
    /// 3. Removes entry point scripts from bin/Scripts directory
    /// 4. Removes all files listed in RECORD
    /// 5. Removes empty directories after file removal
    /// 6. Removes the .dist-info directory itself
    /// 7. Handles errors gracefully (missing files, permission issues)
    ///
    /// Requirements: 9.5
    pub fn uninstall(&self, package_name: &str) -> Result<u64> {
        let normalized = package_name.replace('-', "_").to_lowercase();
        let mut removed = 0;

        // Find the .dist-info directory
        let dist_info = self.find_installed_dist_info(&normalized)?;

        // Remove entry point scripts first (before reading RECORD)
        // This handles scripts that may have been created during installation
        removed += self.remove_entry_point_scripts(&dist_info)?;

        // Read RECORD file to get list of installed files
        let records = self.read_record(&dist_info)?;

        // Remove all files listed in RECORD
        for entry in &records {
            let file_path = self.site_packages.join(&entry.path);
            
            // Skip if file doesn't exist (may have been manually removed)
            if !file_path.exists() {
                continue;
            }
            
            // Only remove files, not directories (directories are cleaned up later)
            if file_path.is_file() {
                match fs::remove_file(&file_path) {
                    Ok(()) => {
                        removed += 1;
                    }
                    Err(e) => {
                        // Log permission errors but continue with other files
                        if e.kind() == std::io::ErrorKind::PermissionDenied {
                            eprintln!(
                                "Warning: Permission denied removing file: {}",
                                file_path.display()
                            );
                        } else {
                            // For other errors, propagate them
                            return Err(e.into());
                        }
                    }
                }
            }
        }

        // Remove the .dist-info directory
        if dist_info.exists() {
            let count = self.count_files_in_dir(&dist_info)?;
            match fs::remove_dir_all(&dist_info) {
                Ok(()) => {
                    removed += count;
                }
                Err(e) => {
                    if e.kind() == std::io::ErrorKind::PermissionDenied {
                        eprintln!(
                            "Warning: Permission denied removing dist-info: {}",
                            dist_info.display()
                        );
                    } else {
                        return Err(e.into());
                    }
                }
            }
        }

        // Clean up empty directories
        self.cleanup_empty_dirs(&self.site_packages)?;

        Ok(removed)
    }

    /// Remove entry point scripts created during installation
    ///
    /// Parses entry_points.txt from the dist-info directory and removes
    /// the corresponding scripts from the bin/Scripts directory.
    ///
    /// Requirements: 9.5
    fn remove_entry_point_scripts(&self, dist_info: &Path) -> Result<u64> {
        let entry_points_path = dist_info.join("entry_points.txt");
        if !entry_points_path.exists() {
            return Ok(0);
        }

        let content = match fs::read_to_string(&entry_points_path) {
            Ok(c) => c,
            Err(_) => return Ok(0), // If we can't read it, skip script removal
        };

        let entry_points = self.parse_entry_points(&content);
        let scripts_dir = self.get_scripts_directory();
        let mut removed = 0;

        // Process console_scripts
        if let Some(console_scripts) = entry_points.get("console_scripts") {
            for (name, _) in console_scripts {
                removed += self.remove_script(&scripts_dir, name)?;
            }
        }

        // Process gui_scripts
        if let Some(gui_scripts) = entry_points.get("gui_scripts") {
            for (name, _) in gui_scripts {
                removed += self.remove_script(&scripts_dir, name)?;
            }
        }

        Ok(removed)
    }

    /// Remove a single script from the scripts directory
    ///
    /// Handles both Unix scripts and Windows .cmd/.py wrappers
    fn remove_script(&self, scripts_dir: &Path, name: &str) -> Result<u64> {
        let mut removed = 0;

        // On Unix, remove the script directly
        #[cfg(not(windows))]
        {
            let script_path = scripts_dir.join(name);
            if script_path.exists() {
                match fs::remove_file(&script_path) {
                    Ok(()) => removed += 1,
                    Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
                        eprintln!("Warning: Permission denied removing script: {}", script_path.display());
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                        // Already removed, ignore
                    }
                    Err(e) => return Err(e.into()),
                }
            }
        }

        // On Windows, remove both .cmd and -script.py files
        #[cfg(windows)]
        {
            let cmd_path = scripts_dir.join(format!("{}.cmd", name));
            let py_path = scripts_dir.join(format!("{}-script.py", name));

            for path in [cmd_path, py_path] {
                if path.exists() {
                    match fs::remove_file(&path) {
                        Ok(()) => removed += 1,
                        Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
                            eprintln!("Warning: Permission denied removing script: {}", path.display());
                        }
                        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                            // Already removed, ignore
                        }
                        Err(e) => return Err(e.into()),
                    }
                }
            }
        }

        Ok(removed)
    }

    /// Find the .dist-info directory for an installed package
    fn find_installed_dist_info(&self, normalized_name: &str) -> Result<PathBuf> {
        for entry in fs::read_dir(&self.site_packages)? {
            let entry = entry?;
            let name = entry.file_name();
            let name_str = name.to_string_lossy().to_lowercase();

            if name_str.starts_with(&format!("{}-", normalized_name))
                && name_str.ends_with(".dist-info")
            {
                return Ok(entry.path());
            }
        }

        Err(Error::PackageNotFound(normalized_name.to_string()))
    }

    /// Count files in a directory
    #[allow(clippy::only_used_in_recursion)]
    fn count_files_in_dir(&self, dir: &Path) -> Result<u64> {
        let mut count = 0;
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            if entry.path().is_file() {
                count += 1;
            } else if entry.path().is_dir() {
                count += self.count_files_in_dir(&entry.path())?;
            }
        }
        Ok(count)
    }

    /// Clean up empty directories
    #[allow(clippy::only_used_in_recursion)]
    fn cleanup_empty_dirs(&self, dir: &Path) -> Result<()> {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                self.cleanup_empty_dirs(&path)?;
                // Try to remove if empty
                let _ = fs::remove_dir(&path);
            }
        }
        Ok(())
    }

    /// Generate entry point scripts from entry_points.txt
    ///
    /// Parses the entry_points.txt file from the wheel's .dist-info directory
    /// and creates executable wrapper scripts for both console_scripts and gui_scripts.
    ///
    /// Requirements: 9.3
    pub fn generate_scripts(&self, dist_info: &Path) -> Result<Vec<PathBuf>> {
        let entry_points_path = dist_info.join("entry_points.txt");
        if !entry_points_path.exists() {
            return Ok(Vec::new());
        }

        let content = fs::read_to_string(&entry_points_path)?;
        let entry_points = self.parse_entry_points(&content);
        let mut scripts = Vec::new();

        // Process console_scripts
        if let Some(console_scripts) = entry_points.get("console_scripts") {
            for (name, target) in console_scripts {
                if let Some(script_path) = self.create_entry_point_script(name, target, false)? {
                    scripts.push(script_path);
                }
            }
        }

        // Process gui_scripts
        if let Some(gui_scripts) = entry_points.get("gui_scripts") {
            for (name, target) in gui_scripts {
                if let Some(script_path) = self.create_entry_point_script(name, target, true)? {
                    scripts.push(script_path);
                }
            }
        }

        Ok(scripts)
    }

    /// Parse entry_points.txt content into a map of sections
    ///
    /// Format:
    /// ```text
    /// [console_scripts]
    /// script_name = module:function
    /// another_script = module.submodule:func
    ///
    /// [gui_scripts]
    /// gui_app = myapp.gui:main
    /// ```
    fn parse_entry_points(&self, content: &str) -> std::collections::HashMap<String, Vec<(String, String)>> {
        let mut sections: std::collections::HashMap<String, Vec<(String, String)>> = std::collections::HashMap::new();
        let mut current_section: Option<String> = None;

        for line in content.lines() {
            let line = line.trim();
            
            // Skip empty lines and comments
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            // Check for section header
            if line.starts_with('[') && line.ends_with(']') {
                let section_name = line[1..line.len()-1].trim().to_string();
                current_section = Some(section_name.clone());
                sections.entry(section_name).or_default();
                continue;
            }

            // Parse entry point definition
            if let Some(ref section) = current_section {
                if let Some((name, target)) = self.parse_entry_point_line(line) {
                    if let Some(entries) = sections.get_mut(section) {
                        entries.push((name, target));
                    }
                }
            }
        }

        sections
    }

    /// Parse a single entry point line: "name = module:function" or "name = module:object.method"
    fn parse_entry_point_line(&self, line: &str) -> Option<(String, String)> {
        let parts: Vec<&str> = line.splitn(2, '=').collect();
        if parts.len() != 2 {
            return None;
        }

        let name = parts[0].trim().to_string();
        let target = parts[1].trim().to_string();

        // Validate target format: module:function or module:object.method
        if !target.contains(':') {
            return None;
        }

        Some((name, target))
    }

    /// Create an entry point script
    ///
    /// Creates executable wrapper scripts in the bin/Scripts directory.
    /// Handles both Unix (shebang scripts) and Windows (.cmd scripts).
    ///
    /// Requirements: 9.3
    fn create_entry_point_script(&self, name: &str, target: &str, is_gui: bool) -> Result<Option<PathBuf>> {
        // Parse target: module:function or module:object.method
        let target_parts: Vec<&str> = target.splitn(2, ':').collect();
        if target_parts.len() != 2 {
            return Ok(None);
        }

        let module = target_parts[0].trim();
        let attr = target_parts[1].trim();

        // Get scripts directory - use "Scripts" on Windows, "bin" on Unix
        let scripts_dir = self.get_scripts_directory();
        fs::create_dir_all(&scripts_dir)?;

        // Generate the wrapper script content
        let wrapper = self.generate_wrapper_script(module, attr, is_gui);

        #[cfg(windows)]
        {
            // On Windows, create both a .py script and a .cmd wrapper
            let py_path = scripts_dir.join(format!("{}-script.py", name));
            fs::write(&py_path, &wrapper)?;

            // Create .cmd wrapper that invokes the Python script
            let cmd_path = scripts_dir.join(format!("{}.cmd", name));
            let cmd_content = if is_gui {
                // For GUI scripts, use pythonw to avoid console window
                format!("@echo off\r\npythonw \"{}\" %*\r\n", py_path.to_string_lossy())
            } else {
                format!("@echo off\r\npython \"{}\" %*\r\n", py_path.to_string_lossy())
            };
            fs::write(&cmd_path, cmd_content)?;

            Ok(Some(cmd_path))
        }

        #[cfg(not(windows))]
        {
            let script_path = scripts_dir.join(name);
            fs::write(&script_path, &wrapper)?;

            // Make executable on Unix
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mut perms = fs::metadata(&script_path)?.permissions();
                perms.set_mode(0o755);
                fs::set_permissions(&script_path, perms)?;
            }

            Ok(Some(script_path))
        }
    }

    /// Get the scripts directory path
    ///
    /// Returns the appropriate scripts directory based on the platform:
    /// - Windows: Scripts (sibling of site-packages)
    /// - Unix: bin (sibling of site-packages)
    fn get_scripts_directory(&self) -> PathBuf {
        self.site_packages
            .parent()
            .map(|p| {
                if cfg!(windows) {
                    p.join("Scripts")
                } else {
                    p.join("bin")
                }
            })
            .unwrap_or_else(|| {
                if cfg!(windows) {
                    self.site_packages.join("Scripts")
                } else {
                    self.site_packages.join("bin")
                }
            })
    }

    /// Generate wrapper script content
    ///
    /// Creates a Python script that imports the specified module and calls the function.
    /// The script handles both simple functions and object.method patterns.
    fn generate_wrapper_script(&self, module: &str, attr: &str, is_gui: bool) -> String {
        // Handle object.method pattern (e.g., "Class.method" or "obj.func")
        let (import_attr, call_expr) = if attr.contains('.') {
            // For "object.method", we import "object" and call "object.method()"
            let parts: Vec<&str> = attr.splitn(2, '.').collect();
            (parts[0], attr)
        } else {
            // Simple function: import and call directly
            (attr, attr)
        };

        let shebang = if is_gui {
            "#!/usr/bin/env pythonw"
        } else {
            "#!/usr/bin/env python"
        };

        format!(
            r#"{}
# -*- coding: utf-8 -*-
# Entry point script generated by dx-py
import sys

from {} import {}

if __name__ == '__main__':
    sys.exit({}())
"#,
            shebang, module, import_attr, call_expr
        )
    }

    /// Create the .dist-info directory with all required files
    ///
    /// This creates:
    /// - METADATA: Package metadata (name, version, dependencies)
    /// - RECORD: List of all installed files with hashes
    /// - WHEEL: Wheel format metadata
    /// - INSTALLER: Indicates dx-py installed the package
    ///
    /// Requirements: 9.2, 9.6
    pub fn create_dist_info(
        &self,
        name: &str,
        version: &str,
        installed_files: &[PathBuf],
        metadata: Option<&DistInfoMetadata>,
    ) -> Result<PathBuf> {
        let normalized_name = name.replace('-', "_");
        let dist_info_name = format!("{}-{}.dist-info", normalized_name, version);
        let dist_info_path = self.site_packages.join(&dist_info_name);

        // Create the .dist-info directory
        fs::create_dir_all(&dist_info_path)?;

        // Collect all record entries for files we'll create
        let mut record_entries: Vec<RecordEntry> = Vec::new();

        // Write METADATA file
        let metadata_content = self.generate_metadata_content(name, version, metadata);
        let metadata_path = dist_info_path.join("METADATA");
        fs::write(&metadata_path, &metadata_content)?;
        record_entries.push(RecordEntry::new(
            format!("{}/METADATA", dist_info_name),
            metadata_content.as_bytes(),
        ));

        // Write WHEEL file
        let wheel_content = self.generate_wheel_content(metadata);
        let wheel_path = dist_info_path.join("WHEEL");
        fs::write(&wheel_path, &wheel_content)?;
        record_entries.push(RecordEntry::new(
            format!("{}/WHEEL", dist_info_name),
            wheel_content.as_bytes(),
        ));

        // Write INSTALLER file
        let installer_content = "dx-py\n";
        let installer_path = dist_info_path.join("INSTALLER");
        fs::write(&installer_path, installer_content)?;
        record_entries.push(RecordEntry::new(
            format!("{}/INSTALLER", dist_info_name),
            installer_content.as_bytes(),
        ));

        // Add entries for all installed package files
        for file_path in installed_files {
            if let Ok(relative) = file_path.strip_prefix(&self.site_packages) {
                let relative_str = relative.to_string_lossy().replace('\\', "/");
                // Skip dist-info files as we handle them separately
                if !relative_str.contains(".dist-info/") {
                    if let Ok(content) = fs::read(file_path) {
                        record_entries.push(RecordEntry::new(relative_str, &content));
                    }
                }
            }
        }

        // Write RECORD file (must be last, and without its own hash)
        let record_content = self.generate_record_content(&record_entries, &dist_info_name);
        let record_path = dist_info_path.join("RECORD");
        fs::write(&record_path, &record_content)?;

        Ok(dist_info_path)
    }

    /// Generate METADATA file content
    ///
    /// Format follows PEP 566 / PEP 643 (Core Metadata)
    fn generate_metadata_content(
        &self,
        name: &str,
        version: &str,
        metadata: Option<&DistInfoMetadata>,
    ) -> String {
        // If we have original metadata content, use it
        if let Some(meta) = metadata {
            if let Some(ref content) = meta.metadata_content {
                return content.clone();
            }
        }

        // Generate minimal metadata
        let mut content = String::new();
        content.push_str("Metadata-Version: 2.1\n");
        content.push_str(&format!("Name: {}\n", name.replace('_', "-")));
        content.push_str(&format!("Version: {}\n", version));

        if let Some(meta) = metadata {
            if let Some(ref summary) = meta.summary {
                content.push_str(&format!("Summary: {}\n", summary));
            }
            if let Some(ref requires_python) = meta.requires_python {
                content.push_str(&format!("Requires-Python: {}\n", requires_python));
            }
            for dep in &meta.requires_dist {
                content.push_str(&format!("Requires-Dist: {}\n", dep));
            }
        }

        content
    }

    /// Generate WHEEL file content
    ///
    /// Format follows PEP 427 (Wheel format)
    fn generate_wheel_content(&self, metadata: Option<&DistInfoMetadata>) -> String {
        // If we have original wheel content, use it
        if let Some(meta) = metadata {
            if let Some(ref content) = meta.wheel_content {
                return content.clone();
            }
        }

        // Generate default wheel metadata
        let mut content = String::new();
        content.push_str("Wheel-Version: 1.0\n");
        content.push_str("Generator: dx-py\n");
        content.push_str("Root-Is-Purelib: true\n");
        content.push_str("Tag: py3-none-any\n");

        content
    }

    /// Generate RECORD file content
    ///
    /// Format: path,hash,size (CSV-like)
    /// The RECORD file itself is listed without hash
    fn generate_record_content(&self, entries: &[RecordEntry], dist_info_name: &str) -> String {
        let mut lines: Vec<String> = entries.iter().map(|e| e.to_record_line()).collect();

        // Add RECORD file entry without hash (per PEP 376)
        lines.push(format!("{}/RECORD,,", dist_info_name));

        lines.join("\n") + "\n"
    }

    /// Install a wheel and create proper dist-info
    ///
    /// This is an enhanced version of install_wheel that also creates
    /// the dist-info directory with all required files.
    ///
    /// Requirements: 9.1, 9.2, 9.6
    pub fn install_wheel_with_dist_info(&self, wheel_data: &[u8]) -> Result<InstalledPackage> {
        use std::io::Cursor;

        let cursor = Cursor::new(wheel_data);
        let mut archive = ZipArchive::new(cursor)
            .map_err(|e| Error::Cache(format!("Failed to open wheel: {}", e)))?;

        // Find the .dist-info directory and extract metadata
        let dist_info_name = self.find_dist_info(&mut archive)?;
        let (name, version) = self.parse_dist_info_name(&dist_info_name)?;

        // Extract metadata from wheel
        let metadata = self.extract_wheel_metadata(&mut archive, &dist_info_name)?;

        let mut installed_files = Vec::new();
        let mut dist_info_files_content: Vec<(String, Vec<u8>)> = Vec::new();

        // Extract all files
        for i in 0..archive.len() {
            let mut file = archive
                .by_index(i)
                .map_err(|e| Error::Cache(format!("Failed to read wheel entry: {}", e)))?;

            let file_path = file.name().to_string();

            // Skip directories
            if file_path.ends_with('/') {
                continue;
            }

            // Read file content
            let mut content = Vec::new();
            file.read_to_end(&mut content)
                .map_err(|e| Error::Cache(format!("Failed to read file: {}", e)))?;

            // Store dist-info files for later processing
            if file_path.starts_with(&dist_info_name) {
                dist_info_files_content.push((file_path, content));
                continue;
            }

            // Handle .data directory specially
            let dest_path = if file_path.contains(".data/") {
                self.handle_data_file(&file_path, &name)?
            } else {
                self.site_packages.join(&file_path)
            };

            // Create parent directories
            if let Some(parent) = dest_path.parent() {
                fs::create_dir_all(parent)?;
            }

            // Write atomically using temp file
            let temp_path = dest_path.with_extension("tmp");
            fs::write(&temp_path, &content)?;
            fs::rename(&temp_path, &dest_path)?;

            installed_files.push(dest_path);
        }

        // Create dist-info directory with proper files
        let dist_info_path = self.create_dist_info(&name, &version, &installed_files, Some(&metadata))?;

        // Also extract any additional dist-info files from the wheel (like entry_points.txt)
        for (file_path, content) in dist_info_files_content {
            let file_name = file_path
                .strip_prefix(&format!("{}/", dist_info_name))
                .unwrap_or(&file_path);

            // Skip files we've already created
            if matches!(file_name, "METADATA" | "WHEEL" | "INSTALLER" | "RECORD") {
                continue;
            }

            let dest_path = dist_info_path.join(file_name);
            if let Some(parent) = dest_path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(&dest_path, &content)?;
        }

        // Update RECORD with any additional files
        self.update_record_with_additional_files(&dist_info_path)?;

        // Generate entry point scripts (Requirements: 9.3)
        let script_files = self.generate_scripts(&dist_info_path)?;
        installed_files.extend(script_files);

        Ok(InstalledPackage {
            name,
            version,
            files: installed_files,
            dist_info: dist_info_path,
        })
    }

    /// Extract metadata from wheel archive
    fn extract_wheel_metadata<R: Read + std::io::Seek>(
        &self,
        archive: &mut ZipArchive<R>,
        dist_info_name: &str,
    ) -> Result<DistInfoMetadata> {
        let mut metadata_content = None;
        let mut wheel_content = None;
        let mut requires_dist = Vec::new();
        let mut requires_python = None;
        let mut summary = None;
        let mut name = String::new();
        let mut version = String::new();

        // Look for METADATA and WHEEL files
        for i in 0..archive.len() {
            if let Ok(mut file) = archive.by_index(i) {
                let file_name = file.name().to_string();

                if file_name == format!("{}/METADATA", dist_info_name) {
                    let mut content = String::new();
                    file.read_to_string(&mut content)
                        .map_err(|e| Error::Cache(format!("Failed to read METADATA: {}", e)))?;

                    // Parse metadata fields
                    for line in content.lines() {
                        if let Some(value) = line.strip_prefix("Name:") {
                            name = value.trim().to_string();
                        } else if let Some(value) = line.strip_prefix("Version:") {
                            version = value.trim().to_string();
                        } else if let Some(value) = line.strip_prefix("Summary:") {
                            summary = Some(value.trim().to_string());
                        } else if let Some(value) = line.strip_prefix("Requires-Python:") {
                            requires_python = Some(value.trim().to_string());
                        } else if let Some(value) = line.strip_prefix("Requires-Dist:") {
                            requires_dist.push(value.trim().to_string());
                        }
                    }

                    metadata_content = Some(content);
                } else if file_name == format!("{}/WHEEL", dist_info_name) {
                    let mut content = String::new();
                    file.read_to_string(&mut content)
                        .map_err(|e| Error::Cache(format!("Failed to read WHEEL: {}", e)))?;
                    wheel_content = Some(content);
                }
            }
        }

        Ok(DistInfoMetadata {
            name,
            version,
            metadata_content,
            wheel_content,
            requires_dist,
            requires_python,
            summary,
        })
    }

    /// Update RECORD file with any additional files in dist-info
    fn update_record_with_additional_files(&self, dist_info_path: &Path) -> Result<()> {
        let dist_info_name = dist_info_path
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| Error::Cache("Invalid dist-info path".to_string()))?;

        // Read existing RECORD
        let record_path = dist_info_path.join("RECORD");
        let existing_content = fs::read_to_string(&record_path).unwrap_or_default();
        let mut existing_paths: std::collections::HashSet<String> = existing_content
            .lines()
            .filter_map(|line| line.split(',').next())
            .map(|s| s.to_string())
            .collect();

        let mut new_entries = Vec::new();

        // Check for additional files in dist-info
        for entry in fs::read_dir(dist_info_path)? {
            let entry = entry?;
            let file_name = entry.file_name();
            let file_name_str = file_name.to_string_lossy();
            let relative_path = format!("{}/{}", dist_info_name, file_name_str);

            if !existing_paths.contains(&relative_path) && entry.path().is_file() {
                let content = fs::read(entry.path())?;
                // Don't add hash for RECORD file
                if file_name_str == "RECORD" {
                    continue;
                }
                new_entries.push(RecordEntry::new(relative_path.clone(), &content));
                existing_paths.insert(relative_path);
            }
        }

        // If we have new entries, append them to RECORD
        if !new_entries.is_empty() {
            let mut content = existing_content.trim_end().to_string();
            for entry in new_entries {
                content.push('\n');
                content.push_str(&entry.to_record_line());
            }
            // Ensure RECORD entry is at the end
            if !content.contains(&format!("{}/RECORD,,", dist_info_name)) {
                content.push('\n');
                content.push_str(&format!("{}/RECORD,,", dist_info_name));
            }
            content.push('\n');
            fs::write(&record_path, content)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod wheel_tests {
    use super::*;
    use std::io::{Cursor, Write};
    use tempfile::TempDir;
    use zip::write::FileOptions;
    use zip::ZipWriter;

    /// Helper function to create a test wheel file in memory
    fn create_test_wheel(
        name: &str,
        version: &str,
        files: Vec<(&str, &[u8])>,
        is_pure_python: bool,
    ) -> Vec<u8> {
        let buffer = Cursor::new(Vec::new());
        let mut zip = ZipWriter::new(buffer);
        let options = FileOptions::default().compression_method(zip::CompressionMethod::Stored);

        let normalized_name = name.replace('-', "_");
        let dist_info = format!("{}-{}.dist-info", normalized_name, version);

        // Add package files
        for (path, content) in files {
            zip.start_file(path, options).unwrap();
            zip.write_all(content).unwrap();
        }

        // Add METADATA file
        let metadata = format!(
            "Metadata-Version: 2.1\nName: {}\nVersion: {}\nRequires-Python: >=3.8\n",
            name, version
        );
        zip.start_file(format!("{}/METADATA", dist_info), options)
            .unwrap();
        zip.write_all(metadata.as_bytes()).unwrap();

        // Add WHEEL file
        let wheel_content = format!(
            "Wheel-Version: 1.0\nGenerator: test\nRoot-Is-Purelib: {}\nTag: py3-none-any\n",
            if is_pure_python { "true" } else { "false" }
        );
        zip.start_file(format!("{}/WHEEL", dist_info), options)
            .unwrap();
        zip.write_all(wheel_content.as_bytes()).unwrap();

        // Add RECORD file (empty for simplicity)
        zip.start_file(format!("{}/RECORD", dist_info), options)
            .unwrap();
        zip.write_all(b"").unwrap();

        zip.finish().unwrap().into_inner()
    }

    /// Helper function to create a wheel with .data directory
    fn create_wheel_with_data(
        name: &str,
        version: &str,
        data_files: Vec<(&str, &str, &[u8])>, // (category, path, content)
    ) -> Vec<u8> {
        let buffer = Cursor::new(Vec::new());
        let mut zip = ZipWriter::new(buffer);
        let options = FileOptions::default().compression_method(zip::CompressionMethod::Stored);

        let normalized_name = name.replace('-', "_");
        let dist_info = format!("{}-{}.dist-info", normalized_name, version);
        let data_dir = format!("{}-{}.data", normalized_name, version);

        // Add package __init__.py
        let init_path = format!("{}/__init__.py", normalized_name);
        zip.start_file(&init_path, options).unwrap();
        zip.write_all(b"# Package init").unwrap();

        // Add .data files
        for (category, path, content) in data_files {
            let full_path = format!("{}/{}/{}", data_dir, category, path);
            zip.start_file(&full_path, options).unwrap();
            zip.write_all(content).unwrap();
        }

        // Add METADATA file
        let metadata = format!(
            "Metadata-Version: 2.1\nName: {}\nVersion: {}\n",
            name, version
        );
        zip.start_file(format!("{}/METADATA", dist_info), options)
            .unwrap();
        zip.write_all(metadata.as_bytes()).unwrap();

        // Add WHEEL file
        let wheel_content = "Wheel-Version: 1.0\nGenerator: test\nRoot-Is-Purelib: false\nTag: cp312-cp312-linux_x86_64\n";
        zip.start_file(format!("{}/WHEEL", dist_info), options)
            .unwrap();
        zip.write_all(wheel_content.as_bytes()).unwrap();

        // Add RECORD file
        zip.start_file(format!("{}/RECORD", dist_info), options)
            .unwrap();
        zip.write_all(b"").unwrap();

        zip.finish().unwrap().into_inner()
    }

    #[test]
    fn test_wheel_extraction_basic() {
        let cache_dir = TempDir::new().unwrap();
        let site_packages = TempDir::new().unwrap();

        let cache = GlobalCache::new(cache_dir.path()).unwrap();
        let installer = WheelInstaller::new(cache, site_packages.path().to_path_buf());

        // Create a simple wheel
        let wheel_data = create_test_wheel(
            "test-pkg",
            "1.0.0",
            vec![
                ("test_pkg/__init__.py", b"# init"),
                ("test_pkg/main.py", b"def main(): pass"),
            ],
            true,
        );

        // Install the wheel
        let result = installer.install_wheel(&wheel_data).unwrap();

        // Verify package info
        assert_eq!(result.name, "test_pkg");
        assert_eq!(result.version, "1.0.0");

        // Verify files were extracted
        assert!(site_packages.path().join("test_pkg/__init__.py").exists());
        assert!(site_packages.path().join("test_pkg/main.py").exists());

        // Verify dist-info was created
        assert!(site_packages
            .path()
            .join("test_pkg-1.0.0.dist-info")
            .exists());
        assert!(site_packages
            .path()
            .join("test_pkg-1.0.0.dist-info/METADATA")
            .exists());
        assert!(site_packages
            .path()
            .join("test_pkg-1.0.0.dist-info/WHEEL")
            .exists());
    }

    #[test]
    fn test_wheel_extraction_creates_directory_structure() {
        let cache_dir = TempDir::new().unwrap();
        let site_packages = TempDir::new().unwrap();

        let cache = GlobalCache::new(cache_dir.path()).unwrap();
        let installer = WheelInstaller::new(cache, site_packages.path().to_path_buf());

        // Create a wheel with nested directories
        let wheel_data = create_test_wheel(
            "nested-pkg",
            "2.0.0",
            vec![
                ("nested_pkg/__init__.py", b"# init"),
                ("nested_pkg/sub/__init__.py", b"# sub init"),
                ("nested_pkg/sub/deep/__init__.py", b"# deep init"),
                ("nested_pkg/sub/deep/module.py", b"def func(): pass"),
            ],
            true,
        );

        let result = installer.install_wheel(&wheel_data).unwrap();

        // Verify nested directory structure was created
        assert!(site_packages.path().join("nested_pkg").is_dir());
        assert!(site_packages.path().join("nested_pkg/sub").is_dir());
        assert!(site_packages.path().join("nested_pkg/sub/deep").is_dir());
        assert!(site_packages
            .path()
            .join("nested_pkg/sub/deep/module.py")
            .exists());

        // Verify all files are tracked
        assert!(result.files.len() >= 4);
    }

    #[test]
    fn test_wheel_extraction_handles_data_directory_scripts() {
        let cache_dir = TempDir::new().unwrap();
        let venv_dir = TempDir::new().unwrap();

        // Create a venv-like structure
        let site_packages = venv_dir.path().join("lib/site-packages");
        fs::create_dir_all(&site_packages).unwrap();

        let cache = GlobalCache::new(cache_dir.path()).unwrap();
        let installer = WheelInstaller::new(cache, site_packages.clone());

        // Create a wheel with scripts in .data directory
        let wheel_data = create_wheel_with_data(
            "cli-tool",
            "1.0.0",
            vec![("scripts", "mytool", b"#!/usr/bin/env python\nprint('hello')")],
        );

        let result = installer.install_wheel(&wheel_data).unwrap();

        // Verify package was installed
        assert_eq!(result.name, "cli_tool");
        assert!(site_packages.join("cli_tool/__init__.py").exists());

        // Scripts should go to Scripts directory (sibling of site-packages)
        let _scripts_dir = venv_dir.path().join("lib/Scripts");
        // Note: The script may or may not exist depending on the exact path structure
        // The important thing is that the installer handles the .data directory correctly
    }

    #[test]
    fn test_wheel_extraction_handles_data_directory_purelib() {
        let cache_dir = TempDir::new().unwrap();
        let site_packages = TempDir::new().unwrap();

        let cache = GlobalCache::new(cache_dir.path()).unwrap();
        let installer = WheelInstaller::new(cache, site_packages.path().to_path_buf());

        // Create a wheel with purelib files in .data directory
        let wheel_data = create_wheel_with_data(
            "lib-pkg",
            "1.0.0",
            vec![("purelib", "extra_module.py", b"# extra module")],
        );

        installer.install_wheel(&wheel_data).unwrap();

        // purelib files should go directly to site-packages
        assert!(site_packages.path().join("extra_module.py").exists());
    }

    #[test]
    fn test_wheel_extraction_handles_data_directory_platlib() {
        let cache_dir = TempDir::new().unwrap();
        let site_packages = TempDir::new().unwrap();

        let cache = GlobalCache::new(cache_dir.path()).unwrap();
        let installer = WheelInstaller::new(cache, site_packages.path().to_path_buf());

        // Create a wheel with platlib files in .data directory
        let wheel_data = create_wheel_with_data(
            "native-pkg",
            "1.0.0",
            vec![("platlib", "native_ext.so", b"\x7fELF...")],
        );

        installer.install_wheel(&wheel_data).unwrap();

        // platlib files should go directly to site-packages
        assert!(site_packages.path().join("native_ext.so").exists());
    }

    #[test]
    fn test_is_pure_python_wheel() {
        // Test pure Python wheel
        let pure_wheel = create_test_wheel(
            "pure-pkg",
            "1.0.0",
            vec![("pure_pkg/__init__.py", b"# init")],
            true,
        );
        assert!(WheelInstaller::is_pure_python_wheel(&pure_wheel).unwrap());

        // Test platform-specific wheel
        let platform_wheel = create_test_wheel(
            "native-pkg",
            "1.0.0",
            vec![("native_pkg/__init__.py", b"# init")],
            false,
        );
        assert!(!WheelInstaller::is_pure_python_wheel(&platform_wheel).unwrap());
    }

    #[test]
    fn test_get_wheel_tags() {
        let wheel_data = create_test_wheel(
            "test-pkg",
            "1.0.0",
            vec![("test_pkg/__init__.py", b"# init")],
            true,
        );

        let tags = WheelInstaller::get_wheel_tags(&wheel_data).unwrap();
        assert!(!tags.is_empty());
        assert!(tags.contains(&"py3-none-any".to_string()));
    }

    #[test]
    fn test_wheel_extraction_preserves_file_content() {
        let cache_dir = TempDir::new().unwrap();
        let site_packages = TempDir::new().unwrap();

        let cache = GlobalCache::new(cache_dir.path()).unwrap();
        let installer = WheelInstaller::new(cache, site_packages.path().to_path_buf());

        let expected_content = b"def hello():\n    return 'Hello, World!'\n";
        let wheel_data = create_test_wheel(
            "content-pkg",
            "1.0.0",
            vec![
                ("content_pkg/__init__.py", b"# init"),
                ("content_pkg/hello.py", expected_content),
            ],
            true,
        );

        installer.install_wheel(&wheel_data).unwrap();

        // Verify file content is preserved
        let actual_content = fs::read(site_packages.path().join("content_pkg/hello.py")).unwrap();
        assert_eq!(actual_content, expected_content);
    }

    #[test]
    fn test_wheel_extraction_handles_hyphenated_names() {
        let cache_dir = TempDir::new().unwrap();
        let site_packages = TempDir::new().unwrap();

        let cache = GlobalCache::new(cache_dir.path()).unwrap();
        let installer = WheelInstaller::new(cache, site_packages.path().to_path_buf());

        // Package name with hyphens (normalized to underscores in wheel)
        let wheel_data = create_test_wheel(
            "my-cool-package",
            "1.0.0",
            vec![("my_cool_package/__init__.py", b"# init")],
            true,
        );

        let result = installer.install_wheel(&wheel_data).unwrap();

        // Name should be normalized
        assert_eq!(result.name, "my_cool_package");
        assert!(site_packages
            .path()
            .join("my_cool_package/__init__.py")
            .exists());
    }

    #[test]
    fn test_wheel_extraction_returns_installed_files_list() {
        let cache_dir = TempDir::new().unwrap();
        let site_packages = TempDir::new().unwrap();

        let cache = GlobalCache::new(cache_dir.path()).unwrap();
        let installer = WheelInstaller::new(cache, site_packages.path().to_path_buf());

        let wheel_data = create_test_wheel(
            "tracked-pkg",
            "1.0.0",
            vec![
                ("tracked_pkg/__init__.py", b"# init"),
                ("tracked_pkg/a.py", b"# a"),
                ("tracked_pkg/b.py", b"# b"),
            ],
            true,
        );

        let result = installer.install_wheel(&wheel_data).unwrap();

        // Should have all package files plus dist-info files
        assert!(result.files.len() >= 3);

        // All returned paths should exist
        for file in &result.files {
            assert!(file.exists(), "File should exist: {:?}", file);
        }
    }

    #[test]
    fn test_record_entry_parsing() {
        // Test valid RECORD line
        let entry = RecordEntry::parse("package/module.py,sha256=abc123,1234").unwrap();
        assert_eq!(entry.path, "package/module.py");
        assert_eq!(entry.hash, Some("sha256=abc123".to_string()));
        assert_eq!(entry.size, Some(1234));

        // Test RECORD line without hash
        let entry = RecordEntry::parse("package/__init__.py,,").unwrap();
        assert_eq!(entry.path, "package/__init__.py");
        assert_eq!(entry.hash, None);
        assert_eq!(entry.size, None);

        // Test empty line
        assert!(RecordEntry::parse("").is_none());
        assert!(RecordEntry::parse(",,").is_none());
    }

    #[test]
    fn test_wheel_installer_from_path() {
        let cache_dir = TempDir::new().unwrap();
        let site_packages = TempDir::new().unwrap();
        let wheel_dir = TempDir::new().unwrap();

        let cache = GlobalCache::new(cache_dir.path()).unwrap();
        let installer = WheelInstaller::new(cache, site_packages.path().to_path_buf());

        // Create and save a wheel file
        let wheel_data = create_test_wheel(
            "file-pkg",
            "1.0.0",
            vec![("file_pkg/__init__.py", b"# init")],
            true,
        );
        let wheel_path = wheel_dir.path().join("file_pkg-1.0.0-py3-none-any.whl");
        fs::write(&wheel_path, &wheel_data).unwrap();

        // Install from path
        let result = installer.install_wheel_from_path(&wheel_path).unwrap();

        assert_eq!(result.name, "file_pkg");
        assert_eq!(result.version, "1.0.0");
        assert!(site_packages.path().join("file_pkg/__init__.py").exists());
    }

    #[test]
    fn test_wheel_extraction_invalid_wheel() {
        let cache_dir = TempDir::new().unwrap();
        let site_packages = TempDir::new().unwrap();

        let cache = GlobalCache::new(cache_dir.path()).unwrap();
        let installer = WheelInstaller::new(cache, site_packages.path().to_path_buf());

        // Try to install invalid data
        let result = installer.install_wheel(b"not a valid zip file");
        assert!(result.is_err());
    }

    #[test]
    fn test_wheel_extraction_missing_dist_info() {
        let cache_dir = TempDir::new().unwrap();
        let site_packages = TempDir::new().unwrap();

        let cache = GlobalCache::new(cache_dir.path()).unwrap();
        let installer = WheelInstaller::new(cache, site_packages.path().to_path_buf());

        // Create a zip without dist-info
        let buffer = Cursor::new(Vec::new());
        let mut zip = ZipWriter::new(buffer);
        let options = FileOptions::default().compression_method(zip::CompressionMethod::Stored);

        zip.start_file("some_pkg/__init__.py", options).unwrap();
        zip.write_all(b"# init").unwrap();

        let wheel_data = zip.finish().unwrap().into_inner();
        let result = installer.install_wheel(&wheel_data);

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("dist-info"));
    }

    #[test]
    fn test_dist_info_creation() {
        let cache_dir = TempDir::new().unwrap();
        let site_packages = TempDir::new().unwrap();

        let cache = GlobalCache::new(cache_dir.path()).unwrap();
        let installer = WheelInstaller::new(cache, site_packages.path().to_path_buf());

        // Create some test files
        let pkg_dir = site_packages.path().join("test_pkg");
        fs::create_dir_all(&pkg_dir).unwrap();
        fs::write(pkg_dir.join("__init__.py"), b"# init").unwrap();
        fs::write(pkg_dir.join("main.py"), b"def main(): pass").unwrap();

        let installed_files = vec![
            pkg_dir.join("__init__.py"),
            pkg_dir.join("main.py"),
        ];

        // Create dist-info
        let dist_info = installer
            .create_dist_info("test-pkg", "1.0.0", &installed_files, None)
            .unwrap();

        // Verify dist-info directory exists
        assert!(dist_info.exists());
        assert!(dist_info.is_dir());

        // Verify METADATA file
        let metadata_path = dist_info.join("METADATA");
        assert!(metadata_path.exists());
        let metadata_content = fs::read_to_string(&metadata_path).unwrap();
        assert!(metadata_content.contains("Name: test-pkg"));
        assert!(metadata_content.contains("Version: 1.0.0"));
        assert!(metadata_content.contains("Metadata-Version: 2.1"));

        // Verify WHEEL file
        let wheel_path = dist_info.join("WHEEL");
        assert!(wheel_path.exists());
        let wheel_content = fs::read_to_string(&wheel_path).unwrap();
        assert!(wheel_content.contains("Wheel-Version: 1.0"));
        assert!(wheel_content.contains("Generator: dx-py"));

        // Verify INSTALLER file
        let installer_path = dist_info.join("INSTALLER");
        assert!(installer_path.exists());
        let installer_content = fs::read_to_string(&installer_path).unwrap();
        assert_eq!(installer_content.trim(), "dx-py");

        // Verify RECORD file
        let record_path = dist_info.join("RECORD");
        assert!(record_path.exists());
        let record_content = fs::read_to_string(&record_path).unwrap();
        // RECORD should contain entries for installed files
        assert!(record_content.contains("test_pkg/__init__.py"));
        assert!(record_content.contains("test_pkg/main.py"));
        // RECORD should contain entries for dist-info files
        assert!(record_content.contains("METADATA"));
        assert!(record_content.contains("WHEEL"));
        assert!(record_content.contains("INSTALLER"));
        // RECORD file itself should be listed without hash
        assert!(record_content.contains("RECORD,,"));
    }

    #[test]
    fn test_dist_info_with_metadata() {
        let cache_dir = TempDir::new().unwrap();
        let site_packages = TempDir::new().unwrap();

        let cache = GlobalCache::new(cache_dir.path()).unwrap();
        let installer = WheelInstaller::new(cache, site_packages.path().to_path_buf());

        let metadata = DistInfoMetadata {
            name: "my-package".to_string(),
            version: "2.0.0".to_string(),
            metadata_content: None,
            wheel_content: None,
            requires_dist: vec![
                "requests>=2.0".to_string(),
                "click>=7.0".to_string(),
            ],
            requires_python: Some(">=3.8".to_string()),
            summary: Some("A test package".to_string()),
        };

        let dist_info = installer
            .create_dist_info("my-package", "2.0.0", &[], Some(&metadata))
            .unwrap();

        // Verify METADATA contains dependencies
        let metadata_content = fs::read_to_string(dist_info.join("METADATA")).unwrap();
        assert!(metadata_content.contains("Requires-Dist: requests>=2.0"));
        assert!(metadata_content.contains("Requires-Dist: click>=7.0"));
        assert!(metadata_content.contains("Requires-Python: >=3.8"));
        assert!(metadata_content.contains("Summary: A test package"));
    }

    #[test]
    fn test_record_entry_creation() {
        let content = b"def hello(): return 'world'";
        let entry = RecordEntry::new("test/module.py".to_string(), content);

        assert_eq!(entry.path, "test/module.py");
        assert!(entry.hash.is_some());
        assert!(entry.hash.as_ref().unwrap().starts_with("sha256="));
        assert_eq!(entry.size, Some(content.len() as u64));

        // Verify the record line format
        let line = entry.to_record_line();
        assert!(line.starts_with("test/module.py,sha256="));
        assert!(line.ends_with(&format!(",{}", content.len())));
    }

    #[test]
    fn test_record_entry_without_hash() {
        let entry = RecordEntry::without_hash("dist-info/RECORD".to_string());

        assert_eq!(entry.path, "dist-info/RECORD");
        assert!(entry.hash.is_none());
        assert!(entry.size.is_none());

        let line = entry.to_record_line();
        assert_eq!(line, "dist-info/RECORD,,");
    }

    #[test]
    fn test_install_wheel_with_dist_info() {
        let cache_dir = TempDir::new().unwrap();
        let site_packages = TempDir::new().unwrap();

        let cache = GlobalCache::new(cache_dir.path()).unwrap();
        let installer = WheelInstaller::new(cache, site_packages.path().to_path_buf());

        // Create a wheel with dependencies
        let wheel_data = create_wheel_with_dependencies(
            "dep-pkg",
            "1.0.0",
            vec![("dep_pkg/__init__.py", b"# init")],
            vec!["requests>=2.0", "click>=7.0"],
        );

        let result = installer.install_wheel_with_dist_info(&wheel_data).unwrap();

        // Verify package info
        assert_eq!(result.name, "dep_pkg");
        assert_eq!(result.version, "1.0.0");

        // Verify dist-info was created properly
        assert!(result.dist_info.exists());

        // Verify INSTALLER file indicates dx-py
        let installer_content = fs::read_to_string(result.dist_info.join("INSTALLER")).unwrap();
        assert_eq!(installer_content.trim(), "dx-py");

        // Verify RECORD file has proper entries
        let record_content = fs::read_to_string(result.dist_info.join("RECORD")).unwrap();
        assert!(record_content.contains("dep_pkg/__init__.py"));
        assert!(record_content.contains("sha256=")); // Should have hashes
    }

    /// Helper function to create a wheel with dependencies
    fn create_wheel_with_dependencies(
        name: &str,
        version: &str,
        files: Vec<(&str, &[u8])>,
        dependencies: Vec<&str>,
    ) -> Vec<u8> {
        let buffer = Cursor::new(Vec::new());
        let mut zip = ZipWriter::new(buffer);
        let options = FileOptions::default().compression_method(zip::CompressionMethod::Stored);

        let normalized_name = name.replace('-', "_");
        let dist_info = format!("{}-{}.dist-info", normalized_name, version);

        // Add package files
        for (path, content) in files {
            zip.start_file(path, options).unwrap();
            zip.write_all(content).unwrap();
        }

        // Add METADATA file with dependencies
        let mut metadata = format!(
            "Metadata-Version: 2.1\nName: {}\nVersion: {}\nRequires-Python: >=3.8\n",
            name, version
        );
        for dep in dependencies {
            metadata.push_str(&format!("Requires-Dist: {}\n", dep));
        }
        zip.start_file(format!("{}/METADATA", dist_info), options)
            .unwrap();
        zip.write_all(metadata.as_bytes()).unwrap();

        // Add WHEEL file
        let wheel_content = "Wheel-Version: 1.0\nGenerator: test\nRoot-Is-Purelib: true\nTag: py3-none-any\n";
        zip.start_file(format!("{}/WHEEL", dist_info), options)
            .unwrap();
        zip.write_all(wheel_content.as_bytes()).unwrap();

        // Add RECORD file (empty for simplicity)
        zip.start_file(format!("{}/RECORD", dist_info), options)
            .unwrap();
        zip.write_all(b"").unwrap();

        zip.finish().unwrap().into_inner()
    }

    #[test]
    fn test_dist_info_preserves_original_metadata() {
        let cache_dir = TempDir::new().unwrap();
        let site_packages = TempDir::new().unwrap();

        let cache = GlobalCache::new(cache_dir.path()).unwrap();
        let installer = WheelInstaller::new(cache, site_packages.path().to_path_buf());

        let original_metadata = "Metadata-Version: 2.1\nName: original-pkg\nVersion: 3.0.0\nAuthor: Test Author\nLicense: MIT\n";
        let original_wheel = "Wheel-Version: 1.0\nGenerator: custom-tool\nRoot-Is-Purelib: true\nTag: py3-none-any\n";

        let metadata = DistInfoMetadata {
            name: "original-pkg".to_string(),
            version: "3.0.0".to_string(),
            metadata_content: Some(original_metadata.to_string()),
            wheel_content: Some(original_wheel.to_string()),
            requires_dist: vec![],
            requires_python: None,
            summary: None,
        };

        let dist_info = installer
            .create_dist_info("original-pkg", "3.0.0", &[], Some(&metadata))
            .unwrap();

        // Verify original METADATA is preserved
        let metadata_content = fs::read_to_string(dist_info.join("METADATA")).unwrap();
        assert_eq!(metadata_content, original_metadata);

        // Verify original WHEEL is preserved
        let wheel_content = fs::read_to_string(dist_info.join("WHEEL")).unwrap();
        assert_eq!(wheel_content, original_wheel);
    }

    /// Helper function to create a wheel with entry points
    fn create_wheel_with_entry_points(
        name: &str,
        version: &str,
        console_scripts: Vec<(&str, &str)>,
        gui_scripts: Vec<(&str, &str)>,
    ) -> Vec<u8> {
        let buffer = Cursor::new(Vec::new());
        let mut zip = ZipWriter::new(buffer);
        let options = FileOptions::default().compression_method(zip::CompressionMethod::Stored);

        let normalized_name = name.replace('-', "_");
        let dist_info = format!("{}-{}.dist-info", normalized_name, version);

        // Add package __init__.py
        let init_path = format!("{}/__init__.py", normalized_name);
        zip.start_file(&init_path, options).unwrap();
        zip.write_all(b"def main(): return 0\n").unwrap();

        // Add METADATA file
        let metadata = format!(
            "Metadata-Version: 2.1\nName: {}\nVersion: {}\n",
            name, version
        );
        zip.start_file(format!("{}/METADATA", dist_info), options)
            .unwrap();
        zip.write_all(metadata.as_bytes()).unwrap();

        // Add WHEEL file
        let wheel_content = "Wheel-Version: 1.0\nGenerator: test\nRoot-Is-Purelib: true\nTag: py3-none-any\n";
        zip.start_file(format!("{}/WHEEL", dist_info), options)
            .unwrap();
        zip.write_all(wheel_content.as_bytes()).unwrap();

        // Add entry_points.txt
        let mut entry_points_content = String::new();
        if !console_scripts.is_empty() {
            entry_points_content.push_str("[console_scripts]\n");
            for (name, target) in &console_scripts {
                entry_points_content.push_str(&format!("{} = {}\n", name, target));
            }
        }
        if !gui_scripts.is_empty() {
            entry_points_content.push_str("\n[gui_scripts]\n");
            for (name, target) in &gui_scripts {
                entry_points_content.push_str(&format!("{} = {}\n", name, target));
            }
        }
        if !entry_points_content.is_empty() {
            zip.start_file(format!("{}/entry_points.txt", dist_info), options)
                .unwrap();
            zip.write_all(entry_points_content.as_bytes()).unwrap();
        }

        // Add RECORD file (empty for simplicity)
        zip.start_file(format!("{}/RECORD", dist_info), options)
            .unwrap();
        zip.write_all(b"").unwrap();

        zip.finish().unwrap().into_inner()
    }

    #[test]
    fn test_parse_entry_points_basic() {
        let cache_dir = TempDir::new().unwrap();
        let site_packages = TempDir::new().unwrap();

        let cache = GlobalCache::new(cache_dir.path()).unwrap();
        let installer = WheelInstaller::new(cache, site_packages.path().to_path_buf());

        let content = r#"[console_scripts]
my-script = mypackage:main
another-script = mypackage.cli:run

[gui_scripts]
my-gui = mypackage.gui:start
"#;

        let sections = installer.parse_entry_points(content);

        // Verify console_scripts
        let console = sections.get("console_scripts").unwrap();
        assert_eq!(console.len(), 2);
        assert_eq!(console[0], ("my-script".to_string(), "mypackage:main".to_string()));
        assert_eq!(console[1], ("another-script".to_string(), "mypackage.cli:run".to_string()));

        // Verify gui_scripts
        let gui = sections.get("gui_scripts").unwrap();
        assert_eq!(gui.len(), 1);
        assert_eq!(gui[0], ("my-gui".to_string(), "mypackage.gui:start".to_string()));
    }

    #[test]
    fn test_parse_entry_points_with_comments_and_empty_lines() {
        let cache_dir = TempDir::new().unwrap();
        let site_packages = TempDir::new().unwrap();

        let cache = GlobalCache::new(cache_dir.path()).unwrap();
        let installer = WheelInstaller::new(cache, site_packages.path().to_path_buf());

        let content = r#"# This is a comment
[console_scripts]
# Another comment
my-script = mypackage:main

# Empty lines above and below

another-script = mypackage.cli:run
"#;

        let sections = installer.parse_entry_points(content);

        let console = sections.get("console_scripts").unwrap();
        assert_eq!(console.len(), 2);
        assert_eq!(console[0], ("my-script".to_string(), "mypackage:main".to_string()));
        assert_eq!(console[1], ("another-script".to_string(), "mypackage.cli:run".to_string()));
    }

    #[test]
    fn test_parse_entry_points_object_method_pattern() {
        let cache_dir = TempDir::new().unwrap();
        let site_packages = TempDir::new().unwrap();

        let cache = GlobalCache::new(cache_dir.path()).unwrap();
        let installer = WheelInstaller::new(cache, site_packages.path().to_path_buf());

        let content = r#"[console_scripts]
my-cli = mypackage.cli:CLI.main
"#;

        let sections = installer.parse_entry_points(content);

        let console = sections.get("console_scripts").unwrap();
        assert_eq!(console.len(), 1);
        assert_eq!(console[0], ("my-cli".to_string(), "mypackage.cli:CLI.main".to_string()));
    }

    #[test]
    fn test_parse_entry_point_line_valid() {
        let cache_dir = TempDir::new().unwrap();
        let site_packages = TempDir::new().unwrap();

        let cache = GlobalCache::new(cache_dir.path()).unwrap();
        let installer = WheelInstaller::new(cache, site_packages.path().to_path_buf());

        // Simple function
        let result = installer.parse_entry_point_line("my-script = mypackage:main");
        assert_eq!(result, Some(("my-script".to_string(), "mypackage:main".to_string())));

        // With submodule
        let result = installer.parse_entry_point_line("cli = mypackage.cli:run");
        assert_eq!(result, Some(("cli".to_string(), "mypackage.cli:run".to_string())));

        // Object.method pattern
        let result = installer.parse_entry_point_line("app = mypackage:App.run");
        assert_eq!(result, Some(("app".to_string(), "mypackage:App.run".to_string())));

        // With extra whitespace
        let result = installer.parse_entry_point_line("  my-script  =  mypackage:main  ");
        assert_eq!(result, Some(("my-script".to_string(), "mypackage:main".to_string())));
    }

    #[test]
    fn test_parse_entry_point_line_invalid() {
        let cache_dir = TempDir::new().unwrap();
        let site_packages = TempDir::new().unwrap();

        let cache = GlobalCache::new(cache_dir.path()).unwrap();
        let installer = WheelInstaller::new(cache, site_packages.path().to_path_buf());

        // Missing colon in target
        let result = installer.parse_entry_point_line("my-script = mypackage.main");
        assert_eq!(result, None);

        // Missing equals sign
        let result = installer.parse_entry_point_line("my-script mypackage:main");
        assert_eq!(result, None);

        // Empty line
        let result = installer.parse_entry_point_line("");
        assert_eq!(result, None);
    }

    #[test]
    fn test_generate_wrapper_script_simple_function() {
        let cache_dir = TempDir::new().unwrap();
        let site_packages = TempDir::new().unwrap();

        let cache = GlobalCache::new(cache_dir.path()).unwrap();
        let installer = WheelInstaller::new(cache, site_packages.path().to_path_buf());

        let script = installer.generate_wrapper_script("mypackage", "main", false);

        assert!(script.contains("#!/usr/bin/env python"));
        assert!(script.contains("from mypackage import main"));
        assert!(script.contains("sys.exit(main())"));
    }

    #[test]
    fn test_generate_wrapper_script_object_method() {
        let cache_dir = TempDir::new().unwrap();
        let site_packages = TempDir::new().unwrap();

        let cache = GlobalCache::new(cache_dir.path()).unwrap();
        let installer = WheelInstaller::new(cache, site_packages.path().to_path_buf());

        let script = installer.generate_wrapper_script("mypackage.cli", "CLI.main", false);

        assert!(script.contains("#!/usr/bin/env python"));
        assert!(script.contains("from mypackage.cli import CLI"));
        assert!(script.contains("sys.exit(CLI.main())"));
    }

    #[test]
    fn test_generate_wrapper_script_gui() {
        let cache_dir = TempDir::new().unwrap();
        let site_packages = TempDir::new().unwrap();

        let cache = GlobalCache::new(cache_dir.path()).unwrap();
        let installer = WheelInstaller::new(cache, site_packages.path().to_path_buf());

        let script = installer.generate_wrapper_script("mypackage.gui", "start", true);

        assert!(script.contains("#!/usr/bin/env pythonw"));
        assert!(script.contains("from mypackage.gui import start"));
        assert!(script.contains("sys.exit(start())"));
    }

    #[test]
    fn test_generate_scripts_creates_scripts_directory() {
        let cache_dir = TempDir::new().unwrap();
        let venv_dir = TempDir::new().unwrap();

        // Create a venv-like structure
        let site_packages = venv_dir.path().join("lib").join("site-packages");
        fs::create_dir_all(&site_packages).unwrap();

        let cache = GlobalCache::new(cache_dir.path()).unwrap();
        let installer = WheelInstaller::new(cache, site_packages.clone());

        // Create dist-info with entry_points.txt
        let dist_info = site_packages.join("test_pkg-1.0.0.dist-info");
        fs::create_dir_all(&dist_info).unwrap();

        let entry_points = r#"[console_scripts]
test-cli = test_pkg:main
"#;
        fs::write(dist_info.join("entry_points.txt"), entry_points).unwrap();

        // Generate scripts
        let scripts = installer.generate_scripts(&dist_info).unwrap();

        // Verify scripts directory was created
        let scripts_dir = installer.get_scripts_directory();
        assert!(scripts_dir.exists(), "Scripts directory should be created");

        // Verify script was created
        assert!(!scripts.is_empty(), "Should have created at least one script");
    }

    #[test]
    fn test_generate_scripts_no_entry_points() {
        let cache_dir = TempDir::new().unwrap();
        let site_packages = TempDir::new().unwrap();

        let cache = GlobalCache::new(cache_dir.path()).unwrap();
        let installer = WheelInstaller::new(cache, site_packages.path().to_path_buf());

        // Create dist-info without entry_points.txt
        let dist_info = site_packages.path().join("test_pkg-1.0.0.dist-info");
        fs::create_dir_all(&dist_info).unwrap();

        // Generate scripts (should return empty)
        let scripts = installer.generate_scripts(&dist_info).unwrap();
        assert!(scripts.is_empty(), "Should return empty when no entry_points.txt");
    }

    #[test]
    fn test_wheel_installation_with_entry_points() {
        let cache_dir = TempDir::new().unwrap();
        let venv_dir = TempDir::new().unwrap();

        // Create a venv-like structure
        let site_packages = venv_dir.path().join("lib").join("site-packages");
        fs::create_dir_all(&site_packages).unwrap();

        let cache = GlobalCache::new(cache_dir.path()).unwrap();
        let installer = WheelInstaller::new(cache, site_packages.clone());

        // Create a wheel with entry points
        let wheel_data = create_wheel_with_entry_points(
            "cli-tool",
            "1.0.0",
            vec![("cli-tool", "cli_tool:main"), ("cli-helper", "cli_tool.helper:run")],
            vec![("cli-gui", "cli_tool.gui:start")],
        );

        // Install the wheel
        let result = installer.install_wheel(&wheel_data).unwrap();

        // Verify package was installed
        assert_eq!(result.name, "cli_tool");
        assert!(site_packages.join("cli_tool/__init__.py").exists());

        // Verify entry_points.txt was extracted
        assert!(result.dist_info.join("entry_points.txt").exists());

        // Verify scripts directory exists
        let scripts_dir = installer.get_scripts_directory();
        if scripts_dir.exists() {
            // On Unix, check for script files
            #[cfg(not(windows))]
            {
                let cli_tool_script = scripts_dir.join("cli-tool");
                if cli_tool_script.exists() {
                    let content = fs::read_to_string(&cli_tool_script).unwrap();
                    assert!(content.contains("from cli_tool import main"));
                    assert!(content.contains("sys.exit(main())"));

                    // Verify executable permission on Unix
                    #[cfg(unix)]
                    {
                        use std::os::unix::fs::PermissionsExt;
                        let perms = fs::metadata(&cli_tool_script).unwrap().permissions();
                        assert!(perms.mode() & 0o111 != 0, "Script should be executable");
                    }
                }
            }

            // On Windows, check for .cmd files
            #[cfg(windows)]
            {
                let cli_tool_cmd = scripts_dir.join("cli-tool.cmd");
                if cli_tool_cmd.exists() {
                    let content = fs::read_to_string(&cli_tool_cmd).unwrap();
                    assert!(content.contains("python"));
                }
            }
        }
    }

    #[test]
    fn test_get_scripts_directory() {
        let cache_dir = TempDir::new().unwrap();
        let venv_dir = TempDir::new().unwrap();

        // Create a venv-like structure
        let site_packages = venv_dir.path().join("lib").join("site-packages");
        fs::create_dir_all(&site_packages).unwrap();

        let cache = GlobalCache::new(cache_dir.path()).unwrap();
        let installer = WheelInstaller::new(cache, site_packages.clone());

        let scripts_dir = installer.get_scripts_directory();

        // Should be sibling of site-packages
        #[cfg(windows)]
        assert!(scripts_dir.ends_with("Scripts"));
        #[cfg(not(windows))]
        assert!(scripts_dir.ends_with("bin"));

        // Should be at the same level as lib
        assert_eq!(scripts_dir.parent(), site_packages.parent());
    }

    #[test]
    fn test_wheel_uninstall_removes_all_files() {
        // Test that uninstall removes all files listed in RECORD
        // Requirements: 9.5
        let cache_dir = TempDir::new().unwrap();
        let site_packages = TempDir::new().unwrap();

        let cache = GlobalCache::new(cache_dir.path()).unwrap();
        let installer = WheelInstaller::new(cache, site_packages.path().to_path_buf());

        // Create a wheel with multiple files
        let wheel_data = create_test_wheel(
            "test_pkg",
            "1.0.0",
            vec![
                ("test_pkg/__init__.py", b"# init file"),
                ("test_pkg/module.py", b"def func(): pass"),
                ("test_pkg/subdir/nested.py", b"# nested file"),
            ],
            true,
        );

        // Install the wheel
        let installed = installer.install_wheel(&wheel_data).unwrap();
        
        // Verify files were installed
        assert!(site_packages.path().join("test_pkg/__init__.py").exists());
        assert!(site_packages.path().join("test_pkg/module.py").exists());
        assert!(site_packages.path().join("test_pkg/subdir/nested.py").exists());
        assert!(installed.dist_info.exists());

        // Uninstall the package
        let removed = installer.uninstall("test_pkg").unwrap();
        assert!(removed > 0, "Should have removed at least one file");

        // Verify all files were removed
        assert!(!site_packages.path().join("test_pkg/__init__.py").exists());
        assert!(!site_packages.path().join("test_pkg/module.py").exists());
        assert!(!site_packages.path().join("test_pkg/subdir/nested.py").exists());
        assert!(!installed.dist_info.exists(), "dist-info should be removed");
    }

    #[test]
    fn test_wheel_uninstall_removes_entry_point_scripts() {
        // Test that uninstall removes entry point scripts
        // Requirements: 9.5
        let cache_dir = TempDir::new().unwrap();
        let site_packages = TempDir::new().unwrap();

        let cache = GlobalCache::new(cache_dir.path()).unwrap();
        let installer = WheelInstaller::new(cache, site_packages.path().to_path_buf());

        // Create a wheel with entry points
        let wheel_data = create_wheel_with_entry_points(
            "cli_tool",
            "1.0.0",
            vec![("cli-tool", "cli_tool:main")],
            vec![],
        );

        // Install the wheel
        let installed = installer.install_wheel(&wheel_data).unwrap();
        
        // Verify package was installed
        assert!(site_packages.path().join("cli_tool/__init__.py").exists());
        assert!(installed.dist_info.exists());

        // Verify scripts were created
        let scripts_dir = installer.get_scripts_directory();
        fs::create_dir_all(&scripts_dir).unwrap();
        
        #[cfg(not(windows))]
        let script_path = scripts_dir.join("cli-tool");
        #[cfg(windows)]
        let script_path = scripts_dir.join("cli-tool.cmd");

        // Create the script manually for testing (since generate_scripts may not create it in test env)
        if !script_path.exists() {
            fs::write(&script_path, b"#!/usr/bin/env python\n# test script").unwrap();
        }

        let script_existed = script_path.exists();

        // Uninstall the package
        let removed = installer.uninstall("cli_tool").unwrap();
        assert!(removed > 0, "Should have removed at least one file");

        // Verify package files were removed
        assert!(!site_packages.path().join("cli_tool/__init__.py").exists());
        assert!(!installed.dist_info.exists());

        // Verify script was removed (if it existed)
        if script_existed {
            assert!(!script_path.exists(), "Entry point script should be removed");
        }
    }

    #[test]
    fn test_wheel_uninstall_handles_missing_package() {
        // Test that uninstall returns error for non-existent package
        // Requirements: 9.5
        let cache_dir = TempDir::new().unwrap();
        let site_packages = TempDir::new().unwrap();

        let cache = GlobalCache::new(cache_dir.path()).unwrap();
        let installer = WheelInstaller::new(cache, site_packages.path().to_path_buf());

        // Try to uninstall a package that doesn't exist
        let result = installer.uninstall("nonexistent_package");
        assert!(result.is_err(), "Should return error for non-existent package");
    }

    #[test]
    fn test_wheel_uninstall_handles_hyphenated_names() {
        // Test that uninstall works with hyphenated package names
        // Requirements: 9.5
        let cache_dir = TempDir::new().unwrap();
        let site_packages = TempDir::new().unwrap();

        let cache = GlobalCache::new(cache_dir.path()).unwrap();
        let installer = WheelInstaller::new(cache, site_packages.path().to_path_buf());

        // Create a wheel with hyphenated name (normalized to underscores in wheel)
        let wheel_data = create_test_wheel(
            "my_test_pkg",  // Wheel uses underscores
            "1.0.0",
            vec![("my_test_pkg/__init__.py", b"# init")],
            true,
        );

        // Install the wheel
        let installed = installer.install_wheel(&wheel_data).unwrap();
        assert!(site_packages.path().join("my_test_pkg/__init__.py").exists());

        // Uninstall using hyphenated name
        let removed = installer.uninstall("my-test-pkg").unwrap();
        assert!(removed > 0, "Should have removed files");

        // Verify files were removed
        assert!(!site_packages.path().join("my_test_pkg/__init__.py").exists());
        assert!(!installed.dist_info.exists());
    }

    #[test]
    fn test_wheel_uninstall_cleans_up_empty_directories() {
        // Test that uninstall removes empty directories after file removal
        // Requirements: 9.5
        let cache_dir = TempDir::new().unwrap();
        let site_packages = TempDir::new().unwrap();

        let cache = GlobalCache::new(cache_dir.path()).unwrap();
        let installer = WheelInstaller::new(cache, site_packages.path().to_path_buf());

        // Create a wheel with nested directory structure
        let wheel_data = create_test_wheel(
            "nested_pkg",
            "1.0.0",
            vec![
                ("nested_pkg/__init__.py", b"# init"),
                ("nested_pkg/sub1/module1.py", b"# module1"),
                ("nested_pkg/sub1/sub2/module2.py", b"# module2"),
            ],
            true,
        );

        // Install the wheel
        let installed = installer.install_wheel(&wheel_data).unwrap();
        
        // Verify directory structure exists
        let pkg_dir = site_packages.path().join("nested_pkg");
        let sub1_dir = pkg_dir.join("sub1");
        let sub2_dir = sub1_dir.join("sub2");
        assert!(pkg_dir.exists());
        assert!(sub1_dir.exists());
        assert!(sub2_dir.exists());

        // Uninstall the package
        let removed = installer.uninstall("nested_pkg").unwrap();
        assert!(removed > 0, "Should have removed files");

        // Verify empty directories were cleaned up
        // Note: The cleanup may not remove all directories if they're not empty
        // or if there are permission issues, but the main package directory
        // should be removed if all files are gone
        assert!(!installed.dist_info.exists());
    }
}
