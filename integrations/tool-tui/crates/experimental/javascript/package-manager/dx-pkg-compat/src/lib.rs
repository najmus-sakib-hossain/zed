//! dx-pkg-compat: npm Compatibility Layer
//!
//! Bridges npm ecosystem to DX binary format:
//! - package.json parsing
//! - npm registry proxy
//! - Semver resolution

use dx_pkg_core::{version::Version, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// npm package.json structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageJson {
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub dependencies: HashMap<String, String>,
    #[serde(default, rename = "devDependencies")]
    pub dev_dependencies: HashMap<String, String>,
    #[serde(default, rename = "peerDependencies")]
    pub peer_dependencies: HashMap<String, String>,
    #[serde(default)]
    pub scripts: HashMap<String, String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub author: Option<String>,
    #[serde(default)]
    pub license: Option<String>,
}

impl PackageJson {
    /// Read package.json from file
    pub fn read(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path)?;
        let pkg: PackageJson = serde_json::from_str(&content).map_err(|e| {
            dx_pkg_core::Error::parse_with_location(
                format!("Invalid package.json: {}", e),
                path,
                0,
                0,
            )
        })?;
        Ok(pkg)
    }

    /// Parse version string to DX Version
    pub fn parse_version(&self) -> Result<Version> {
        parse_npm_version(&self.version)
    }

    /// Get all dependencies (production + dev)
    pub fn all_dependencies(&self) -> HashMap<String, String> {
        let mut all = self.dependencies.clone();
        all.extend(self.dev_dependencies.clone());
        all
    }
}

/// Parse npm version string (supports semver)
pub fn parse_npm_version(version_str: &str) -> Result<Version> {
    // Remove common prefixes
    let clean = version_str
        .trim_start_matches('^')
        .trim_start_matches('~')
        .trim_start_matches('v')
        .trim();

    // Parse x.y.z
    let parts: Vec<&str> = clean.split('.').collect();
    if parts.len() < 3 {
        return Err(dx_pkg_core::Error::invalid_version(version_str));
    }

    let major = parts[0].parse().map_err(|_| dx_pkg_core::Error::invalid_version(version_str))?;
    let minor = parts[1].parse().map_err(|_| dx_pkg_core::Error::invalid_version(version_str))?;
    let patch = parts[2]
        .split('-')
        .next()
        .unwrap_or("0")
        .parse()
        .map_err(|_| dx_pkg_core::Error::invalid_version(version_str))?;

    Ok(Version::new(major, minor, patch))
}

/// npm registry proxy
pub struct NpmRegistry {
    base_url: String,
}

impl NpmRegistry {
    /// Create new npm registry proxy
    pub fn new() -> Self {
        Self {
            base_url: "https://registry.npmjs.org".to_string(),
        }
    }

    /// Get package metadata URL
    pub fn package_url(&self, name: &str) -> String {
        format!("{}/{}", self.base_url, name)
    }

    /// Get tarball URL
    pub fn tarball_url(&self, name: &str, version: &str) -> String {
        format!("{}/{}/-/{}-{}.tgz", self.base_url, name, name, version)
    }
}

impl Default for NpmRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Convert package.json to DX format
pub struct PackageConverter;

impl PackageConverter {
    /// Convert npm package to DX binary format
    pub fn convert_to_dx(pkg: &PackageJson) -> Result<DxPackageMetadata> {
        Ok(DxPackageMetadata {
            name: pkg.name.clone(),
            version: pkg.parse_version()?,
            dependencies: pkg.dependencies.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
        })
    }
}

/// DX package metadata
#[derive(Debug, Clone)]
pub struct DxPackageMetadata {
    pub name: String,
    pub version: Version,
    pub dependencies: Vec<(String, String)>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_npm_version() {
        assert_eq!(parse_npm_version("1.2.3").unwrap(), Version::new(1, 2, 3));
        assert_eq!(parse_npm_version("^1.2.3").unwrap(), Version::new(1, 2, 3));
        assert_eq!(parse_npm_version("~1.2.3").unwrap(), Version::new(1, 2, 3));
        assert_eq!(parse_npm_version("v1.2.3").unwrap(), Version::new(1, 2, 3));
    }

    #[test]
    fn test_npm_registry() {
        let registry = NpmRegistry::new();

        assert_eq!(registry.package_url("react"), "https://registry.npmjs.org/react");
        assert_eq!(
            registry.tarball_url("react", "18.0.0"),
            "https://registry.npmjs.org/react/-/react-18.0.0.tgz"
        );
    }

    #[test]
    fn test_package_json_parsing() {
        let json = r#"{
            "name": "test-package",
            "version": "1.0.0",
            "dependencies": {
                "react": "^18.0.0"
            }
        }"#;

        let pkg: PackageJson = serde_json::from_str(json).unwrap();
        assert_eq!(pkg.name, "test-package");
        assert_eq!(pkg.dependencies.get("react"), Some(&"^18.0.0".to_string()));
    }
}
