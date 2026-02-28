//! Extension discovery across search paths
//!
//! Handles locating C extension files (.pyd/.so) in the filesystem.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::abi::AbiVersion;
use crate::error::{ExtensionError, ExtensionResult};

/// Extension discovery service
#[derive(Debug, Clone)]
pub struct ExtensionDiscovery {
    /// Search paths for extensions
    search_paths: Vec<PathBuf>,
    /// ABI version for extension suffix matching
    abi_version: AbiVersion,
}

impl ExtensionDiscovery {
    /// Create a new extension discovery service
    pub fn new(abi_version: AbiVersion) -> Self {
        Self {
            search_paths: Vec::new(),
            abi_version,
        }
    }

    /// Add a search path
    pub fn add_search_path<P: AsRef<Path>>(&mut self, path: P) {
        let path = path.as_ref().to_path_buf();
        if !self.search_paths.contains(&path) {
            self.search_paths.push(path);
        }
    }

    /// Add multiple search paths
    pub fn add_search_paths<I, P>(&mut self, paths: I)
    where
        I: IntoIterator<Item = P>,
        P: AsRef<Path>,
    {
        for path in paths {
            self.add_search_path(path);
        }
    }

    /// Get the current search paths
    pub fn search_paths(&self) -> &[PathBuf] {
        &self.search_paths
    }

    /// Find an extension by module name
    ///
    /// Module names can be:
    /// - Simple: `numpy`
    /// - Dotted: `numpy.core._multiarray_umath`
    pub fn find_extension(&self, module_name: &str) -> ExtensionResult<PathBuf> {
        let possible_filenames = self.generate_possible_filenames(module_name);

        for search_path in &self.search_paths {
            for filename in &possible_filenames {
                let full_path = search_path.join(filename);
                if full_path.exists() && full_path.is_file() {
                    return Ok(full_path);
                }
            }

            // Also check for package structure (e.g., numpy/core/_multiarray_umath.so)
            if module_name.contains('.') {
                let parts: Vec<&str> = module_name.split('.').collect();
                let mut package_path = search_path.clone();
                for part in &parts[..parts.len() - 1] {
                    package_path = package_path.join(part);
                }

                let module_part = parts.last().unwrap();
                for suffix in self.extension_suffixes() {
                    let filename = format!("{}{}", module_part, suffix);
                    let full_path = package_path.join(&filename);
                    if full_path.exists() && full_path.is_file() {
                        return Ok(full_path);
                    }
                }
            }
        }

        Err(ExtensionError::NotFound {
            name: module_name.to_string(),
            searched_paths: self.search_paths.clone(),
        })
    }

    /// Find all extensions in search paths
    pub fn find_all_extensions(&self) -> Vec<DiscoveredExtension> {
        let mut extensions = Vec::new();
        let mut seen = HashSet::new();

        for search_path in &self.search_paths {
            self.scan_directory(search_path, &mut extensions, &mut seen);
        }

        extensions
    }

    /// Generate possible filenames for a module
    fn generate_possible_filenames(&self, module_name: &str) -> Vec<String> {
        let base_name = module_name.replace('.', "/");
        let simple_name = module_name.split('.').next_back().unwrap_or(module_name);

        let mut filenames = Vec::new();

        for suffix in self.extension_suffixes() {
            // Simple name with suffix
            filenames.push(format!("{}{}", simple_name, suffix));
            // Full path with suffix
            filenames.push(format!("{}{}", base_name, suffix));
        }

        filenames
    }

    /// Get platform-specific extension suffixes
    fn extension_suffixes(&self) -> Vec<String> {
        let mut suffixes = Vec::new();

        // Add ABI-specific suffix
        suffixes.push(self.abi_version.extension_suffix());

        // Add generic suffixes as fallback
        #[cfg(target_os = "windows")]
        {
            suffixes.push(".pyd".to_string());
        }
        #[cfg(not(target_os = "windows"))]
        {
            suffixes.push(".so".to_string());
        }

        suffixes
    }

    /// Scan a directory for extensions
    fn scan_directory(
        &self,
        dir: &Path,
        extensions: &mut Vec<DiscoveredExtension>,
        seen: &mut HashSet<PathBuf>,
    ) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };

        for entry in entries.flatten() {
            let path = entry.path();

            if path.is_dir() {
                // Recursively scan subdirectories
                self.scan_directory(&path, extensions, seen);
            } else if self.is_extension_file(&path) && seen.insert(path.clone()) {
                if let Some(ext) = self.parse_extension_info(&path) {
                    extensions.push(ext);
                }
            }
        }
    }

    /// Check if a file is a C extension
    fn is_extension_file(&self, path: &Path) -> bool {
        let Some(filename) = path.file_name().and_then(|s| s.to_str()) else {
            return false;
        };

        let lower = filename.to_lowercase();

        #[cfg(target_os = "windows")]
        {
            lower.ends_with(".pyd")
        }
        #[cfg(not(target_os = "windows"))]
        {
            lower.ends_with(".so") && (lower.contains("cpython") || lower.contains("cp3"))
        }
    }

    /// Parse extension information from a file path
    fn parse_extension_info(&self, path: &Path) -> Option<DiscoveredExtension> {
        let filename = path.file_name()?.to_str()?;
        let abi_version = AbiVersion::from_filename(filename);

        // Extract module name from filename
        let module_name = self.extract_module_name(filename)?;

        Some(DiscoveredExtension {
            path: path.to_path_buf(),
            module_name,
            abi_version,
        })
    }

    /// Extract module name from extension filename
    fn extract_module_name(&self, filename: &str) -> Option<String> {
        // Remove extension suffix
        let name = filename.strip_suffix(".pyd").or_else(|| filename.strip_suffix(".so"))?;

        // Remove ABI tag (e.g., .cpython-311-x86_64-linux-gnu)
        let name = if let Some(pos) = name.find(".cp") {
            &name[..pos]
        } else if let Some(pos) = name.find(".cpython") {
            &name[..pos]
        } else {
            name
        };

        Some(name.to_string())
    }
}

impl Default for ExtensionDiscovery {
    fn default() -> Self {
        Self::new(AbiVersion::dx_py_abi())
    }
}

/// Information about a discovered extension
#[derive(Debug, Clone)]
pub struct DiscoveredExtension {
    /// Path to the extension file
    pub path: PathBuf,
    /// Module name
    pub module_name: String,
    /// Detected ABI version (if parseable from filename)
    pub abi_version: Option<AbiVersion>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extension_suffixes_windows() {
        let discovery = ExtensionDiscovery::new(AbiVersion::new(3, 11, 0));
        let suffixes = discovery.extension_suffixes();

        #[cfg(target_os = "windows")]
        {
            assert!(suffixes.iter().any(|s| s.ends_with(".pyd")));
        }
        #[cfg(not(target_os = "windows"))]
        {
            assert!(suffixes.iter().any(|s| s.ends_with(".so")));
        }
    }

    #[test]
    fn test_generate_possible_filenames() {
        let discovery = ExtensionDiscovery::new(AbiVersion::new(3, 11, 0));
        let filenames = discovery.generate_possible_filenames("numpy.core._multiarray_umath");

        assert!(!filenames.is_empty());
        assert!(filenames.iter().any(|f| f.contains("_multiarray_umath")));
    }

    #[test]
    fn test_extract_module_name() {
        let discovery = ExtensionDiscovery::default();

        #[cfg(target_os = "windows")]
        {
            let name = discovery.extract_module_name("_multiarray_umath.cp311-win_amd64.pyd");
            assert_eq!(name, Some("_multiarray_umath".to_string()));
        }
        #[cfg(target_os = "linux")]
        {
            let name =
                discovery.extract_module_name("_multiarray_umath.cpython-311-x86_64-linux-gnu.so");
            assert_eq!(name, Some("_multiarray_umath".to_string()));
        }
    }

    #[test]
    fn test_add_search_paths() {
        let mut discovery = ExtensionDiscovery::default();
        discovery.add_search_path("/usr/lib/python3.11");
        discovery.add_search_path("/usr/local/lib/python3.11");

        assert_eq!(discovery.search_paths().len(), 2);

        // Adding duplicate should not increase count
        discovery.add_search_path("/usr/lib/python3.11");
        assert_eq!(discovery.search_paths().len(), 2);
    }
}
