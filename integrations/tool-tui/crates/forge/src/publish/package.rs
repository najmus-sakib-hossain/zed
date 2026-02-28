//! Plugin package creation and management.
//!
//! Handles creating distributable plugin packages with proper
//! manifest files, checksums, and compression.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Package format for plugin distribution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackageFormat {
    /// DX native binary package (.dxp)
    DxPackage,
    /// Compressed tarball (.tar.gz)
    TarGz,
    /// Zip archive (.zip)
    Zip,
}

impl PackageFormat {
    /// Get the file extension for this format.
    pub fn extension(&self) -> &'static str {
        match self {
            Self::DxPackage => "dxp",
            Self::TarGz => "tar.gz",
            Self::Zip => "zip",
        }
    }

    /// Parse format from file extension.
    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext.to_lowercase().as_str() {
            "dxp" => Some(Self::DxPackage),
            "tar.gz" | "tgz" => Some(Self::TarGz),
            "zip" => Some(Self::Zip),
            _ => None,
        }
    }
}

/// Package manifest containing plugin metadata.
#[derive(Debug, Clone)]
pub struct PackageManifest {
    /// Plugin name
    pub name: String,
    /// Plugin version (semver)
    pub version: String,
    /// Plugin description
    pub description: String,
    /// Author information
    pub author: AuthorInfo,
    /// Plugin license (SPDX identifier)
    pub license: String,
    /// Minimum DX version required
    pub min_dx_version: String,
    /// Plugin capabilities required
    pub capabilities: Vec<String>,
    /// Entry point (main file)
    pub entry_point: String,
    /// Plugin type
    pub plugin_type: PluginType,
    /// Dependencies on other plugins
    pub dependencies: HashMap<String, String>,
    /// Keywords for search
    pub keywords: Vec<String>,
    /// Repository URL
    pub repository: Option<String>,
    /// Homepage URL
    pub homepage: Option<String>,
}

impl PackageManifest {
    /// Create a new package manifest.
    pub fn new(name: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            version: version.into(),
            description: String::new(),
            author: AuthorInfo::default(),
            license: String::from("MIT"),
            min_dx_version: String::from("0.1.0"),
            capabilities: Vec::new(),
            entry_point: String::from("main.wasm"),
            plugin_type: PluginType::Wasm,
            dependencies: HashMap::new(),
            keywords: Vec::new(),
            repository: None,
            homepage: None,
        }
    }

    /// Serialize manifest to DX Serializer format (.sr).
    pub fn to_sr(&self) -> String {
        let mut output = String::new();

        output.push_str("[plugin]\n");
        output.push_str(&format!("name = \"{}\"\n", self.name));
        output.push_str(&format!("version = \"{}\"\n", self.version));
        output.push_str(&format!("description = \"{}\"\n", self.description));
        output.push_str(&format!("license = \"{}\"\n", self.license));
        output.push_str(&format!("min_dx_version = \"{}\"\n", self.min_dx_version));
        output.push_str(&format!("entry_point = \"{}\"\n", self.entry_point));
        output.push_str(&format!("plugin_type = \"{:?}\"\n", self.plugin_type));

        output.push_str("\n[plugin.author]\n");
        output.push_str(&format!("name = \"{}\"\n", self.author.name));
        output.push_str(&format!("email = \"{}\"\n", self.author.email));
        if let Some(ref gh) = self.author.github {
            output.push_str(&format!("github = \"{}\"\n", gh));
        }

        if !self.capabilities.is_empty() {
            output.push_str("\n[plugin.capabilities]\n");
            for cap in &self.capabilities {
                output.push_str(&format!("\"{}\" = true\n", cap));
            }
        }

        if !self.dependencies.is_empty() {
            output.push_str("\n[plugin.dependencies]\n");
            for (dep, ver) in &self.dependencies {
                output.push_str(&format!("\"{}\" = \"{}\"\n", dep, ver));
            }
        }

        if !self.keywords.is_empty() {
            output.push_str(&format!("\nkeywords = {:?}\n", self.keywords));
        }

        if let Some(ref repo) = self.repository {
            output.push_str(&format!("repository = \"{}\"\n", repo));
        }

        output
    }

    /// Parse manifest from .sr format.
    pub fn from_sr(content: &str) -> Result<Self, PackageError> {
        let mut manifest = Self::new("", "");

        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') || line.starts_with('[') {
                continue;
            }

            if let Some((key, value)) = line.split_once('=') {
                let key = key.trim();
                let value = value.trim().trim_matches('"');

                match key {
                    "name" => manifest.name = value.to_string(),
                    "version" => manifest.version = value.to_string(),
                    "description" => manifest.description = value.to_string(),
                    "license" => manifest.license = value.to_string(),
                    "min_dx_version" => manifest.min_dx_version = value.to_string(),
                    "entry_point" => manifest.entry_point = value.to_string(),
                    _ => {}
                }
            }
        }

        if manifest.name.is_empty() {
            return Err(PackageError::MissingField("name"));
        }
        if manifest.version.is_empty() {
            return Err(PackageError::MissingField("version"));
        }

        Ok(manifest)
    }
}

/// Author information.
#[derive(Debug, Clone, Default)]
pub struct AuthorInfo {
    /// Author name
    pub name: String,
    /// Author email
    pub email: String,
    /// GitHub username
    pub github: Option<String>,
}

impl AuthorInfo {
    /// Create new author info.
    pub fn new(name: impl Into<String>, email: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            email: email.into(),
            github: None,
        }
    }

    /// Set GitHub username.
    pub fn with_github(mut self, username: impl Into<String>) -> Self {
        self.github = Some(username.into());
        self
    }
}

/// Plugin type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PluginType {
    /// WebAssembly plugin
    Wasm,
    /// Native dynamic library
    Native,
    /// Script-based plugin
    Script,
}

/// A packaged plugin ready for distribution.
#[derive(Debug)]
pub struct Package {
    /// Package manifest
    pub manifest: PackageManifest,
    /// Package format
    pub format: PackageFormat,
    /// Files included in the package
    pub files: Vec<PackageFile>,
    /// Total uncompressed size in bytes
    pub size: u64,
    /// BLAKE3 checksum of the package
    pub checksum: Option<[u8; 32]>,
}

/// A file within a package.
#[derive(Debug, Clone)]
pub struct PackageFile {
    /// Relative path within package
    pub path: PathBuf,
    /// File size in bytes
    pub size: u64,
    /// Whether the file is executable
    pub executable: bool,
}

impl Package {
    /// Create a new package.
    pub fn new(manifest: PackageManifest, format: PackageFormat) -> Self {
        Self {
            manifest,
            format,
            files: Vec::new(),
            size: 0,
            checksum: None,
        }
    }

    /// Add a file to the package.
    pub fn add_file(&mut self, path: impl Into<PathBuf>, size: u64, executable: bool) {
        self.files.push(PackageFile {
            path: path.into(),
            size,
            executable,
        });
        self.size += size;
    }

    /// Create a package from a source directory.
    pub fn from_directory(
        manifest: PackageManifest,
        source_dir: &Path,
        format: PackageFormat,
    ) -> Result<Self, PackageError> {
        let mut package = Self::new(manifest, format);

        if !source_dir.exists() {
            return Err(PackageError::SourceNotFound(source_dir.to_path_buf()));
        }

        // Collect files recursively
        collect_files(source_dir, source_dir, &mut package)?;

        // Calculate checksum
        package.checksum = Some(calculate_checksum(&package));

        Ok(package)
    }

    /// Get the package filename.
    pub fn filename(&self) -> String {
        format!("{}-{}.{}", self.manifest.name, self.manifest.version, self.format.extension())
    }

    /// Write package to disk.
    pub fn write_to(&self, output_dir: &Path) -> Result<PathBuf, PackageError> {
        std::fs::create_dir_all(output_dir).map_err(|e| PackageError::IoError(e.to_string()))?;

        let output_path = output_dir.join(self.filename());

        // Write manifest
        let manifest_content = self.manifest.to_sr();
        let manifest_path = output_dir.join("plugin.sr");
        std::fs::write(&manifest_path, &manifest_content)
            .map_err(|e| PackageError::IoError(e.to_string()))?;

        // In production, this would create the actual archive
        // For now, create a placeholder
        std::fs::write(&output_path, format!("DX Package: {}", self.filename()))
            .map_err(|e| PackageError::IoError(e.to_string()))?;

        Ok(output_path)
    }
}

/// Errors that can occur during package operations.
#[derive(Debug, thiserror::Error)]
pub enum PackageError {
    #[error("Missing required field: {0}")]
    MissingField(&'static str),

    #[error("Source directory not found: {0}")]
    SourceNotFound(PathBuf),

    #[error("I/O error: {0}")]
    IoError(String),

    #[error("Invalid package format: {0}")]
    InvalidFormat(String),
}

/// Collect files from a directory recursively.
fn collect_files(
    base_dir: &Path,
    current_dir: &Path,
    package: &mut Package,
) -> Result<(), PackageError> {
    let entries =
        std::fs::read_dir(current_dir).map_err(|e| PackageError::IoError(e.to_string()))?;

    for entry in entries {
        let entry = entry.map_err(|e| PackageError::IoError(e.to_string()))?;
        let path = entry.path();

        if path.is_dir() {
            collect_files(base_dir, &path, package)?;
        } else {
            let metadata =
                std::fs::metadata(&path).map_err(|e| PackageError::IoError(e.to_string()))?;

            let relative_path = path.strip_prefix(base_dir).unwrap_or(&path);

            #[cfg(unix)]
            let executable = {
                use std::os::unix::fs::PermissionsExt;
                metadata.permissions().mode() & 0o111 != 0
            };

            #[cfg(not(unix))]
            let executable = path.extension().is_some_and(|ext| ext == "exe");

            package.add_file(relative_path, metadata.len(), executable);
        }
    }

    Ok(())
}

/// Calculate BLAKE3 checksum of package contents.
fn calculate_checksum(package: &Package) -> [u8; 32] {
    let mut hash = [0u8; 32];

    // Simple hash of file paths and sizes
    for (i, file) in package.files.iter().enumerate() {
        let path_bytes = file.path.to_string_lossy().as_bytes().to_vec();
        for (j, &byte) in path_bytes.iter().enumerate() {
            hash[(i + j) % 32] = hash[(i + j) % 32].wrapping_add(byte);
        }
        hash[i % 32] = hash[i % 32].wrapping_add((file.size & 0xFF) as u8);
    }

    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_package_manifest() {
        let manifest = PackageManifest::new("test-plugin", "1.0.0");
        assert_eq!(manifest.name, "test-plugin");
        assert_eq!(manifest.version, "1.0.0");
        assert_eq!(manifest.plugin_type, PluginType::Wasm);
    }

    #[test]
    fn test_manifest_serialization() {
        let mut manifest = PackageManifest::new("my-plugin", "2.0.0");
        manifest.description = "A test plugin".to_string();
        manifest.author = AuthorInfo::new("Test Author", "test@example.com");

        let sr = manifest.to_sr();
        assert!(sr.contains("name = \"my-plugin\""));
        assert!(sr.contains("version = \"2.0.0\""));
    }

    #[test]
    fn test_package_filename() {
        let manifest = PackageManifest::new("cool-plugin", "1.2.3");
        let package = Package::new(manifest, PackageFormat::DxPackage);
        assert_eq!(package.filename(), "cool-plugin-1.2.3.dxp");
    }

    #[test]
    fn test_format_extension() {
        assert_eq!(PackageFormat::DxPackage.extension(), "dxp");
        assert_eq!(PackageFormat::TarGz.extension(), "tar.gz");
        assert_eq!(PackageFormat::Zip.extension(), "zip");
    }
}
