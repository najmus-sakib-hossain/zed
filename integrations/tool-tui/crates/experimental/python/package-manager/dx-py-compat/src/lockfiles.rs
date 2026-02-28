//! Lock File Compatibility
//!
//! Implements parsers and serializers for various Python lock file formats:
//! - uv.lock (TOML format)
//! - poetry.lock (TOML format)
//! - Pipfile.lock (JSON format)
//! - requirements.txt (plain text)
//!
//! Requirements: 2.4.1-2.4.7

use std::path::Path;

use dx_py_core::{Error, Result};

/// A locked package in the lock file
#[derive(Debug, Clone, PartialEq)]
pub struct LockedPackage {
    /// Package name (normalized)
    pub name: String,
    /// Exact version
    pub version: String,
    /// Source URL or registry
    pub source: Option<PackageSource>,
    /// Dependencies of this package
    pub dependencies: Vec<LockedDependency>,
    /// Optional extras that were resolved
    pub extras: Vec<String>,
    /// Environment markers
    pub markers: Option<String>,
    /// File hashes for verification
    pub hashes: Vec<String>,
}

/// Source of a locked package
#[derive(Debug, Clone, PartialEq)]
pub enum PackageSource {
    /// PyPI or other registry
    Registry { url: String },
    /// Git repository
    Git { url: String, rev: Option<String> },
    /// Local path
    Path { path: String },
    /// Direct URL
    Url { url: String },
}

/// A dependency reference in a locked package
#[derive(Debug, Clone, PartialEq)]
pub struct LockedDependency {
    /// Package name
    pub name: String,
    /// Version constraint (for reference)
    pub version: Option<String>,
    /// Environment markers
    pub markers: Option<String>,
    /// Required extras
    pub extras: Vec<String>,
}

/// Lock file metadata
#[derive(Debug, Clone, Default)]
pub struct LockMetadata {
    /// Lock file format version
    pub version: Option<u32>,
    /// Python version constraint
    pub requires_python: Option<String>,
    /// Hash of the input requirements
    pub content_hash: Option<String>,
    /// Tool that created the lock file
    pub created_by: Option<String>,
}

/// A complete lock file
#[derive(Debug, Clone)]
pub struct LockFile {
    /// Locked packages
    pub packages: Vec<LockedPackage>,
    /// Metadata about the lock file
    pub metadata: LockMetadata,
}

impl LockFile {
    /// Create a new empty lock file
    pub fn new() -> Self {
        Self {
            packages: Vec::new(),
            metadata: LockMetadata::default(),
        }
    }

    /// Create a lock file with packages
    pub fn with_packages(packages: Vec<LockedPackage>) -> Self {
        Self {
            packages,
            metadata: LockMetadata::default(),
        }
    }

    /// Get a package by name
    pub fn get_package(&self, name: &str) -> Option<&LockedPackage> {
        let normalized = name.replace('-', "_").to_lowercase();
        self.packages
            .iter()
            .find(|p| p.name.replace('-', "_").to_lowercase() == normalized)
    }

    /// Check if a package is locked
    pub fn contains(&self, name: &str) -> bool {
        self.get_package(name).is_some()
    }
}

impl Default for LockFile {
    fn default() -> Self {
        Self::new()
    }
}

/// Trait for lock file format implementations
pub trait LockFileFormat {
    /// Parse a lock file from string content
    fn parse(content: &str) -> Result<LockFile>;

    /// Serialize a lock file to string
    fn serialize(lock: &LockFile) -> Result<String>;

    /// Get the file extension for this format
    fn extension() -> &'static str;
}

// ============================================================================
// uv.lock Format (TOML)
// ============================================================================

/// uv.lock format parser and serializer
pub struct UvLockFormat;

impl LockFileFormat for UvLockFormat {
    fn parse(content: &str) -> Result<LockFile> {
        let toml: toml::Value = toml::from_str(content)
            .map_err(|e| Error::Cache(format!("Failed to parse uv.lock: {}", e)))?;

        let mut packages = Vec::new();
        let mut metadata = LockMetadata::default();

        // Parse version
        if let Some(version) = toml.get("version").and_then(|v| v.as_integer()) {
            metadata.version = Some(version as u32);
        }

        // Parse requires-python
        if let Some(requires) = toml.get("requires-python").and_then(|v| v.as_str()) {
            metadata.requires_python = Some(requires.to_string());
        }

        // Parse packages
        if let Some(pkg_array) = toml.get("package").and_then(|v| v.as_array()) {
            for pkg_value in pkg_array {
                if let Some(pkg) = parse_uv_package(pkg_value) {
                    packages.push(pkg);
                }
            }
        }

        Ok(LockFile { packages, metadata })
    }

    fn serialize(lock: &LockFile) -> Result<String> {
        let mut output = String::new();

        // Write version
        output.push_str(&format!("version = {}\n", lock.metadata.version.unwrap_or(1)));

        // Write requires-python if present
        if let Some(ref requires) = lock.metadata.requires_python {
            output.push_str(&format!("requires-python = \"{}\"\n", requires));
        }

        output.push('\n');

        // Write packages
        for pkg in &lock.packages {
            output.push_str("[[package]]\n");
            output.push_str(&format!("name = \"{}\"\n", pkg.name));
            output.push_str(&format!("version = \"{}\"\n", pkg.version));

            // Write source if present
            if let Some(ref source) = pkg.source {
                match source {
                    PackageSource::Registry { url } => {
                        output.push_str(&format!("source = {{ registry = \"{}\" }}\n", url));
                    }
                    PackageSource::Git { url, rev } => {
                        if let Some(ref r) = rev {
                            output.push_str(&format!(
                                "source = {{ git = \"{}\", rev = \"{}\" }}\n",
                                url, r
                            ));
                        } else {
                            output.push_str(&format!("source = {{ git = \"{}\" }}\n", url));
                        }
                    }
                    PackageSource::Path { path } => {
                        output.push_str(&format!("source = {{ path = \"{}\" }}\n", path));
                    }
                    PackageSource::Url { url } => {
                        output.push_str(&format!("source = {{ url = \"{}\" }}\n", url));
                    }
                }
            }

            // Write dependencies
            if !pkg.dependencies.is_empty() {
                output.push_str("dependencies = [\n");
                for dep in &pkg.dependencies {
                    if let Some(ref markers) = dep.markers {
                        output.push_str(&format!(
                            "    {{ name = \"{}\", marker = \"{}\" }},\n",
                            dep.name, markers
                        ));
                    } else {
                        output.push_str(&format!("    {{ name = \"{}\" }},\n", dep.name));
                    }
                }
                output.push_str("]\n");
            }

            output.push('\n');
        }

        Ok(output)
    }

    fn extension() -> &'static str {
        "lock"
    }
}

fn parse_uv_package(value: &toml::Value) -> Option<LockedPackage> {
    let name = value.get("name")?.as_str()?.to_string();
    let version = value.get("version")?.as_str()?.to_string();

    let source = value.get("source").and_then(|s| {
        if let Some(registry) = s.get("registry").and_then(|r| r.as_str()) {
            Some(PackageSource::Registry {
                url: registry.to_string(),
            })
        } else if let Some(git) = s.get("git").and_then(|g| g.as_str()) {
            let rev = s.get("rev").and_then(|r| r.as_str()).map(|r| r.to_string());
            Some(PackageSource::Git {
                url: git.to_string(),
                rev,
            })
        } else if let Some(path) = s.get("path").and_then(|p| p.as_str()) {
            Some(PackageSource::Path {
                path: path.to_string(),
            })
        } else {
            s.get("url").and_then(|u| u.as_str()).map(|url| PackageSource::Url {
                url: url.to_string(),
            })
        }
    });

    let dependencies = value
        .get("dependencies")
        .and_then(|d| d.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|d| {
                    let name = d.get("name")?.as_str()?.to_string();
                    let markers = d.get("marker").and_then(|m| m.as_str()).map(|m| m.to_string());
                    let extras = d
                        .get("extras")
                        .and_then(|e| e.as_array())
                        .map(|arr| {
                            arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect()
                        })
                        .unwrap_or_default();
                    Some(LockedDependency {
                        name,
                        version: None,
                        markers,
                        extras,
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    let extras = value
        .get("extras")
        .and_then(|e| e.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
        .unwrap_or_default();

    let markers = value.get("marker").and_then(|m| m.as_str()).map(|m| m.to_string());

    let hashes = value
        .get("hashes")
        .and_then(|h| h.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
        .unwrap_or_default();

    Some(LockedPackage {
        name,
        version,
        source,
        dependencies,
        extras,
        markers,
        hashes,
    })
}

// ============================================================================
// poetry.lock Format (TOML)
// ============================================================================

/// poetry.lock format parser and serializer
pub struct PoetryLockFormat;

impl LockFileFormat for PoetryLockFormat {
    fn parse(content: &str) -> Result<LockFile> {
        let toml: toml::Value = toml::from_str(content)
            .map_err(|e| Error::Cache(format!("Failed to parse poetry.lock: {}", e)))?;

        let mut packages = Vec::new();
        let mut metadata = LockMetadata::default();

        // Parse metadata
        if let Some(meta) = toml.get("metadata") {
            if let Some(hash) = meta.get("content-hash").and_then(|v| v.as_str()) {
                metadata.content_hash = Some(hash.to_string());
            }
            if let Some(python) = meta.get("python-versions").and_then(|v| v.as_str()) {
                metadata.requires_python = Some(python.to_string());
            }
        }

        // Parse packages
        if let Some(pkg_array) = toml.get("package").and_then(|v| v.as_array()) {
            for pkg_value in pkg_array {
                if let Some(pkg) = parse_poetry_package(pkg_value) {
                    packages.push(pkg);
                }
            }
        }

        Ok(LockFile { packages, metadata })
    }

    fn serialize(lock: &LockFile) -> Result<String> {
        let mut output = String::new();

        // Write packages
        for pkg in &lock.packages {
            output.push_str("[[package]]\n");
            output.push_str(&format!("name = \"{}\"\n", pkg.name));
            output.push_str(&format!("version = \"{}\"\n", pkg.version));
            output.push_str("description = \"\"\n");
            output.push_str("category = \"main\"\n");
            output.push_str("optional = false\n");
            output.push_str("python-versions = \"*\"\n");

            // Write dependencies
            if !pkg.dependencies.is_empty() {
                output.push_str("\n[package.dependencies]\n");
                for dep in &pkg.dependencies {
                    if let Some(ref ver) = dep.version {
                        output.push_str(&format!("{} = \"{}\"\n", dep.name, ver));
                    } else {
                        output.push_str(&format!("{} = \"*\"\n", dep.name));
                    }
                }
            }

            output.push('\n');
        }

        // Write metadata
        output.push_str("[metadata]\n");
        output.push_str("lock-version = \"1.1\"\n");
        if let Some(ref python) = lock.metadata.requires_python {
            output.push_str(&format!("python-versions = \"{}\"\n", python));
        }
        if let Some(ref hash) = lock.metadata.content_hash {
            output.push_str(&format!("content-hash = \"{}\"\n", hash));
        }

        Ok(output)
    }

    fn extension() -> &'static str {
        "lock"
    }
}

fn parse_poetry_package(value: &toml::Value) -> Option<LockedPackage> {
    let name = value.get("name")?.as_str()?.to_string();
    let version = value.get("version")?.as_str()?.to_string();

    // Parse source
    let source = value.get("source").and_then(|s| {
        let source_type = s.get("type").and_then(|t| t.as_str())?;
        match source_type {
            "git" => {
                let url = s.get("url").and_then(|u| u.as_str())?.to_string();
                let rev = s.get("reference").and_then(|r| r.as_str()).map(|r| r.to_string());
                Some(PackageSource::Git { url, rev })
            }
            "url" => {
                let url = s.get("url").and_then(|u| u.as_str())?.to_string();
                Some(PackageSource::Url { url })
            }
            "directory" | "file" => {
                let path = s.get("url").and_then(|u| u.as_str())?.to_string();
                Some(PackageSource::Path { path })
            }
            _ => None,
        }
    });

    // Parse dependencies
    let dependencies = value
        .get("dependencies")
        .and_then(|d| d.as_table())
        .map(|table| {
            table
                .iter()
                .map(|(name, spec)| {
                    let version = match spec {
                        toml::Value::String(s) => Some(s.clone()),
                        toml::Value::Table(t) => {
                            t.get("version").and_then(|v| v.as_str()).map(|s| s.to_string())
                        }
                        _ => None,
                    };
                    let markers = match spec {
                        toml::Value::Table(t) => {
                            t.get("markers").and_then(|m| m.as_str()).map(|s| s.to_string())
                        }
                        _ => None,
                    };
                    let extras = match spec {
                        toml::Value::Table(t) => t
                            .get("extras")
                            .and_then(|e| e.as_array())
                            .map(|arr| {
                                arr.iter()
                                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                                    .collect()
                            })
                            .unwrap_or_default(),
                        _ => Vec::new(),
                    };
                    LockedDependency {
                        name: name.clone(),
                        version,
                        markers,
                        extras,
                    }
                })
                .collect()
        })
        .unwrap_or_default();

    // Parse extras
    let extras = value
        .get("extras")
        .and_then(|e| e.as_table())
        .map(|table| table.keys().cloned().collect())
        .unwrap_or_default();

    // Parse markers
    let markers = value.get("markers").and_then(|m| m.as_str()).map(|m| m.to_string());

    Some(LockedPackage {
        name,
        version,
        source,
        dependencies,
        extras,
        markers,
        hashes: Vec::new(),
    })
}

// ============================================================================
// Pipfile.lock Format (JSON)
// ============================================================================

/// Pipfile.lock format parser and serializer
pub struct PipfileLockFormat;

impl LockFileFormat for PipfileLockFormat {
    fn parse(content: &str) -> Result<LockFile> {
        let json: serde_json::Value = serde_json::from_str(content)
            .map_err(|e| Error::Cache(format!("Failed to parse Pipfile.lock: {}", e)))?;

        let mut packages = Vec::new();
        let mut metadata = LockMetadata::default();

        // Parse _meta section
        if let Some(meta) = json.get("_meta") {
            if let Some(hash) =
                meta.get("hash").and_then(|h| h.get("sha256")).and_then(|s| s.as_str())
            {
                metadata.content_hash = Some(hash.to_string());
            }
            if let Some(requires) = meta
                .get("requires")
                .and_then(|r| r.get("python_version"))
                .and_then(|p| p.as_str())
            {
                metadata.requires_python = Some(format!(">={}", requires));
            }
        }

        // Parse default packages
        if let Some(default) = json.get("default").and_then(|d| d.as_object()) {
            for (name, spec) in default {
                if let Some(pkg) = parse_pipfile_package(name, spec) {
                    packages.push(pkg);
                }
            }
        }

        // Parse develop packages
        if let Some(develop) = json.get("develop").and_then(|d| d.as_object()) {
            for (name, spec) in develop {
                if let Some(mut pkg) = parse_pipfile_package(name, spec) {
                    pkg.extras.push("dev".to_string());
                    packages.push(pkg);
                }
            }
        }

        Ok(LockFile { packages, metadata })
    }

    fn serialize(lock: &LockFile) -> Result<String> {
        let mut json = serde_json::Map::new();

        // Write _meta
        let mut meta = serde_json::Map::new();
        let mut hash_obj = serde_json::Map::new();
        hash_obj.insert(
            "sha256".to_string(),
            serde_json::Value::String(lock.metadata.content_hash.clone().unwrap_or_default()),
        );
        meta.insert("hash".to_string(), serde_json::Value::Object(hash_obj));

        let mut requires = serde_json::Map::new();
        if let Some(ref python) = lock.metadata.requires_python {
            // Extract version from constraint like ">=3.8"
            let version = python.trim_start_matches(|c: char| !c.is_ascii_digit());
            requires.insert(
                "python_version".to_string(),
                serde_json::Value::String(version.to_string()),
            );
        }
        meta.insert("requires".to_string(), serde_json::Value::Object(requires));

        let mut sources = Vec::new();
        let mut source = serde_json::Map::new();
        source.insert("name".to_string(), serde_json::Value::String("pypi".to_string()));
        source.insert(
            "url".to_string(),
            serde_json::Value::String("https://pypi.org/simple".to_string()),
        );
        source.insert("verify_ssl".to_string(), serde_json::Value::Bool(true));
        sources.push(serde_json::Value::Object(source));
        meta.insert("sources".to_string(), serde_json::Value::Array(sources));

        json.insert("_meta".to_string(), serde_json::Value::Object(meta));

        // Write default packages
        let mut default = serde_json::Map::new();
        for pkg in &lock.packages {
            if !pkg.extras.contains(&"dev".to_string()) {
                let mut pkg_obj = serde_json::Map::new();
                pkg_obj.insert(
                    "version".to_string(),
                    serde_json::Value::String(format!("=={}", pkg.version)),
                );
                if !pkg.hashes.is_empty() {
                    let hashes: Vec<_> =
                        pkg.hashes.iter().map(|h| serde_json::Value::String(h.clone())).collect();
                    pkg_obj.insert("hashes".to_string(), serde_json::Value::Array(hashes));
                }
                if let Some(ref markers) = pkg.markers {
                    pkg_obj
                        .insert("markers".to_string(), serde_json::Value::String(markers.clone()));
                }
                default.insert(pkg.name.clone(), serde_json::Value::Object(pkg_obj));
            }
        }
        json.insert("default".to_string(), serde_json::Value::Object(default));

        // Write develop packages
        let mut develop = serde_json::Map::new();
        for pkg in &lock.packages {
            if pkg.extras.contains(&"dev".to_string()) {
                let mut pkg_obj = serde_json::Map::new();
                pkg_obj.insert(
                    "version".to_string(),
                    serde_json::Value::String(format!("=={}", pkg.version)),
                );
                develop.insert(pkg.name.clone(), serde_json::Value::Object(pkg_obj));
            }
        }
        json.insert("develop".to_string(), serde_json::Value::Object(develop));

        serde_json::to_string_pretty(&serde_json::Value::Object(json))
            .map_err(|e| Error::Cache(format!("Failed to serialize Pipfile.lock: {}", e)))
    }

    fn extension() -> &'static str {
        "lock"
    }
}

fn parse_pipfile_package(name: &str, spec: &serde_json::Value) -> Option<LockedPackage> {
    let version = spec
        .get("version")
        .and_then(|v| v.as_str())
        .map(|v| v.trim_start_matches("==").to_string())?;

    let source = spec
        .get("git")
        .and_then(|g| g.as_str())
        .map(|url| {
            let rev = spec.get("ref").and_then(|r| r.as_str()).map(|r| r.to_string());
            PackageSource::Git {
                url: url.to_string(),
                rev,
            }
        })
        .or_else(|| {
            spec.get("path").and_then(|p| p.as_str()).map(|path| PackageSource::Path {
                path: path.to_string(),
            })
        });

    let markers = spec.get("markers").and_then(|m| m.as_str()).map(|m| m.to_string());

    let hashes = spec
        .get("hashes")
        .and_then(|h| h.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
        .unwrap_or_default();

    Some(LockedPackage {
        name: name.to_string(),
        version,
        source,
        dependencies: Vec::new(), // Pipfile.lock doesn't store dependencies
        extras: Vec::new(),
        markers,
        hashes,
    })
}

// ============================================================================
// requirements.txt Format (Plain Text)
// ============================================================================

/// requirements.txt format parser and serializer
pub struct RequirementsTxtFormat;

impl LockFileFormat for RequirementsTxtFormat {
    fn parse(content: &str) -> Result<LockFile> {
        let mut packages = Vec::new();

        for line in content.lines() {
            let line = line.trim();

            // Skip empty lines and comments
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            // Skip options like -i, --index-url, etc.
            if line.starts_with('-') {
                continue;
            }

            if let Some(pkg) = parse_requirement_line(line) {
                packages.push(pkg);
            }
        }

        Ok(LockFile {
            packages,
            metadata: LockMetadata::default(),
        })
    }

    fn serialize(lock: &LockFile) -> Result<String> {
        let mut output = String::new();
        output.push_str("# Generated by dx-py\n");

        for pkg in &lock.packages {
            // Write package with pinned version
            output.push_str(&format!("{}=={}", pkg.name, pkg.version));

            // Add markers if present
            if let Some(ref markers) = pkg.markers {
                output.push_str(&format!(" ; {}", markers));
            }

            // Add hashes if present
            for hash in &pkg.hashes {
                output.push_str(&format!(" \\\n    --hash={}", hash));
            }

            output.push('\n');
        }

        Ok(output)
    }

    fn extension() -> &'static str {
        "txt"
    }
}

fn parse_requirement_line(line: &str) -> Option<LockedPackage> {
    // Handle lines with markers: package==1.0.0 ; python_version >= "3.8"
    let (spec, markers) = if let Some(idx) = line.find(';') {
        let (s, m) = line.split_at(idx);
        (s.trim(), Some(m[1..].trim().to_string()))
    } else {
        (line, None)
    };

    // Handle lines with hashes: package==1.0.0 --hash=sha256:...
    let spec = spec.split("--hash").next()?.trim();

    // Parse name and version
    // Formats: name==version, name>=version, name~=version, name[extras]==version
    let (name, version, extras) = parse_requirement_spec(spec)?;

    Some(LockedPackage {
        name,
        version,
        source: None,
        dependencies: Vec::new(),
        extras,
        markers,
        hashes: Vec::new(),
    })
}

fn parse_requirement_spec(spec: &str) -> Option<(String, String, Vec<String>)> {
    // Handle extras: name[extra1,extra2]==version
    let (name_part, rest) = if let Some(bracket_idx) = spec.find('[') {
        let close_idx = spec.find(']')?;
        let name = spec[..bracket_idx].to_string();
        let extras_str = &spec[bracket_idx + 1..close_idx];
        let extras: Vec<String> = extras_str.split(',').map(|s| s.trim().to_string()).collect();
        let rest = &spec[close_idx + 1..];
        (name, (rest, extras))
    } else {
        // Find version specifier
        let version_start = spec.find(['=', '>', '<', '~', '!'])?;
        let name = spec[..version_start].to_string();
        let rest = &spec[version_start..];
        (name, (rest, Vec::new()))
    };

    let (version_spec, extras) = rest;

    // Extract version from specifier
    let version = version_spec.trim_start_matches(['=', '>', '<', '~', '!']).trim().to_string();

    if name_part.is_empty() || version.is_empty() {
        return None;
    }

    Some((name_part, version, extras))
}

// ============================================================================
// Lock File Detection and Loading
// ============================================================================

/// Detect and load a lock file from a directory
pub fn load_lock_file(dir: &Path) -> Result<Option<LockFile>> {
    // Try uv.lock first
    let uv_lock = dir.join("uv.lock");
    if uv_lock.exists() {
        let content = std::fs::read_to_string(&uv_lock)?;
        return Ok(Some(UvLockFormat::parse(&content)?));
    }

    // Try poetry.lock
    let poetry_lock = dir.join("poetry.lock");
    if poetry_lock.exists() {
        let content = std::fs::read_to_string(&poetry_lock)?;
        return Ok(Some(PoetryLockFormat::parse(&content)?));
    }

    // Try Pipfile.lock
    let pipfile_lock = dir.join("Pipfile.lock");
    if pipfile_lock.exists() {
        let content = std::fs::read_to_string(&pipfile_lock)?;
        return Ok(Some(PipfileLockFormat::parse(&content)?));
    }

    // Try requirements.txt (less preferred as it's not a true lock file)
    let requirements = dir.join("requirements.txt");
    if requirements.exists() {
        let content = std::fs::read_to_string(&requirements)?;
        return Ok(Some(RequirementsTxtFormat::parse(&content)?));
    }

    Ok(None)
}

/// Save a lock file to a directory in the specified format
pub fn save_lock_file<F: LockFileFormat>(
    lock: &LockFile,
    dir: &Path,
    filename: &str,
) -> Result<()> {
    let content = F::serialize(lock)?;
    let path = dir.join(filename);
    std::fs::write(&path, content)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_uv_lock_parse() {
        let content = r#"
version = 1
requires-python = ">=3.8"

[[package]]
name = "requests"
version = "2.31.0"
source = { registry = "https://pypi.org/simple" }
dependencies = [
    { name = "urllib3" },
    { name = "certifi" },
]

[[package]]
name = "urllib3"
version = "2.0.0"

[[package]]
name = "certifi"
version = "2023.7.22"
"#;

        let lock = UvLockFormat::parse(content).unwrap();
        assert_eq!(lock.packages.len(), 3);
        assert_eq!(lock.metadata.version, Some(1));
        assert_eq!(lock.metadata.requires_python, Some(">=3.8".to_string()));

        let requests = lock.get_package("requests").unwrap();
        assert_eq!(requests.version, "2.31.0");
        assert_eq!(requests.dependencies.len(), 2);
    }

    #[test]
    fn test_uv_lock_roundtrip() {
        let lock = LockFile {
            packages: vec![LockedPackage {
                name: "test-pkg".to_string(),
                version: "1.0.0".to_string(),
                source: Some(PackageSource::Registry {
                    url: "https://pypi.org/simple".to_string(),
                }),
                dependencies: vec![LockedDependency {
                    name: "dep-a".to_string(),
                    version: None,
                    markers: None,
                    extras: Vec::new(),
                }],
                extras: Vec::new(),
                markers: None,
                hashes: Vec::new(),
            }],
            metadata: LockMetadata {
                version: Some(1),
                requires_python: Some(">=3.8".to_string()),
                ..Default::default()
            },
        };

        let serialized = UvLockFormat::serialize(&lock).unwrap();
        let parsed = UvLockFormat::parse(&serialized).unwrap();

        assert_eq!(parsed.packages.len(), 1);
        assert_eq!(parsed.packages[0].name, "test-pkg");
        assert_eq!(parsed.packages[0].version, "1.0.0");
    }

    #[test]
    fn test_poetry_lock_parse() {
        let content = r#"
[[package]]
name = "requests"
version = "2.31.0"
description = "Python HTTP library"
category = "main"
optional = false
python-versions = ">=3.7"

[package.dependencies]
urllib3 = ">=1.21.1,<3"
certifi = ">=2017.4.17"

[metadata]
lock-version = "1.1"
python-versions = "^3.8"
content-hash = "abc123"
"#;

        let lock = PoetryLockFormat::parse(content).unwrap();
        assert_eq!(lock.packages.len(), 1);
        assert_eq!(lock.metadata.content_hash, Some("abc123".to_string()));

        let requests = lock.get_package("requests").unwrap();
        assert_eq!(requests.version, "2.31.0");
        assert_eq!(requests.dependencies.len(), 2);
    }

    #[test]
    fn test_pipfile_lock_parse() {
        let content = r#"
{
    "_meta": {
        "hash": {
            "sha256": "abc123"
        },
        "requires": {
            "python_version": "3.8"
        },
        "sources": []
    },
    "default": {
        "requests": {
            "version": "==2.31.0",
            "hashes": ["sha256:abc"]
        }
    },
    "develop": {
        "pytest": {
            "version": "==7.0.0"
        }
    }
}
"#;

        let lock = PipfileLockFormat::parse(content).unwrap();
        assert_eq!(lock.packages.len(), 2);
        assert_eq!(lock.metadata.content_hash, Some("abc123".to_string()));

        let requests = lock.get_package("requests").unwrap();
        assert_eq!(requests.version, "2.31.0");
        assert_eq!(requests.hashes.len(), 1);
    }

    #[test]
    fn test_requirements_txt_parse() {
        let content = r#"
# This is a comment
requests==2.31.0
urllib3>=2.0.0
certifi==2023.7.22 ; python_version >= "3.8"
flask[async]==2.3.0
"#;

        let lock = RequirementsTxtFormat::parse(content).unwrap();
        assert_eq!(lock.packages.len(), 4);

        let requests = lock.get_package("requests").unwrap();
        assert_eq!(requests.version, "2.31.0");

        let certifi = lock.get_package("certifi").unwrap();
        assert!(certifi.markers.is_some());

        let flask = lock.get_package("flask").unwrap();
        assert!(flask.extras.contains(&"async".to_string()));
    }

    #[test]
    fn test_requirements_txt_roundtrip() {
        let lock = LockFile {
            packages: vec![LockedPackage {
                name: "requests".to_string(),
                version: "2.31.0".to_string(),
                source: None,
                dependencies: Vec::new(),
                extras: Vec::new(),
                markers: Some("python_version >= \"3.8\"".to_string()),
                hashes: vec!["sha256:abc123".to_string()],
            }],
            metadata: LockMetadata::default(),
        };

        let serialized = RequirementsTxtFormat::serialize(&lock).unwrap();
        assert!(serialized.contains("requests==2.31.0"));
        assert!(serialized.contains("python_version"));
        assert!(serialized.contains("--hash=sha256:abc123"));
    }

    #[test]
    fn test_lock_file_contains() {
        let lock = LockFile {
            packages: vec![LockedPackage {
                name: "test-pkg".to_string(),
                version: "1.0.0".to_string(),
                source: None,
                dependencies: Vec::new(),
                extras: Vec::new(),
                markers: None,
                hashes: Vec::new(),
            }],
            metadata: LockMetadata::default(),
        };

        assert!(lock.contains("test-pkg"));
        assert!(lock.contains("test_pkg")); // Normalized
        assert!(lock.contains("TEST-PKG")); // Case insensitive
        assert!(!lock.contains("other-pkg"));
    }
}
