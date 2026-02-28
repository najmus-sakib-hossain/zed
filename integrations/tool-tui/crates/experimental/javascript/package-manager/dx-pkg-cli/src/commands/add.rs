//! Add command - Add a package to dependencies
//!
//! Implements `dx add <package>` command:
//! - Parse package@version spec
//! - Resolve version from npm registry
//! - Add to package.json
//! - Fetch and install package

use anyhow::{Context, Result};
use dx_pkg_extract::DirectExtractor;
use dx_pkg_npm::NpmClient;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Simplified package.json structure for reading/writing
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PackageJson {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub dependencies: HashMap<String, String>,
    #[serde(default)]
    pub dev_dependencies: HashMap<String, String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub peer_dependencies: Option<HashMap<String, String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub optional_dependencies: Option<HashMap<String, String>>,
    #[serde(flatten)]
    pub other: HashMap<String, serde_json::Value>,
}

impl PackageJson {
    /// Load package.json from path
    pub fn load(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read {}", path.display()))?;
        let pkg: PackageJson = serde_json::from_str(&content)
            .with_context(|| format!("Failed to parse {}", path.display()))?;
        Ok(pkg)
    }

    /// Save package.json to path
    pub fn save(&self, path: &Path) -> Result<()> {
        let content =
            serde_json::to_string_pretty(self).context("Failed to serialize package.json")?;
        fs::write(path, content).with_context(|| format!("Failed to write {}", path.display()))?;
        Ok(())
    }
}

/// Parse package spec like "lodash", "lodash@4.17.21", "lodash@^4.17.0"
fn parse_package_spec(spec: &str) -> (String, String) {
    // Handle scoped packages like @types/node@1.0.0
    if let Some(rest) = spec.strip_prefix('@') {
        // Find the second @ (version separator)
        if let Some(at_pos) = rest.find('@') {
            let name = spec[..at_pos + 1].to_string();
            let version = spec[at_pos + 2..].to_string();
            return (name, version);
        }
        // No version specified for scoped package
        return (spec.to_string(), "latest".to_string());
    }

    // Regular package
    if let Some(at_pos) = spec.rfind('@') {
        if at_pos > 0 {
            let name = spec[..at_pos].to_string();
            let version = spec[at_pos + 1..].to_string();
            return (name, version);
        }
    }

    (spec.to_string(), "latest".to_string())
}

/// Resolve "latest" or version constraint to actual version
async fn resolve_version(client: &NpmClient, name: &str, version_spec: &str) -> Result<String> {
    let metadata = client
        .get_abbreviated(name)
        .await
        .with_context(|| format!("Failed to fetch metadata for {}", name))?;

    if version_spec == "latest" {
        // Get the latest tag
        if let Some(latest) = metadata.dist_tags.get("latest") {
            return Ok(latest.clone());
        }
        // Fallback to highest version
        let mut versions: Vec<&String> = metadata.versions.keys().collect();
        versions.sort();
        if let Some(v) = versions.last() {
            return Ok((*v).clone());
        }
        anyhow::bail!("No versions found for {}", name);
    }

    // For specific version or constraint, try to find matching version
    // For now, if it's an exact version, use it; otherwise use latest
    if metadata.versions.contains_key(version_spec) {
        return Ok(version_spec.to_string());
    }

    // For constraints like ^4.17.0, find the latest matching version
    // Simplified: just use the latest version for now
    if let Some(latest) = metadata.dist_tags.get("latest") {
        return Ok(latest.clone());
    }

    anyhow::bail!("Could not resolve version {} for {}", version_spec, name);
}

/// Run the add command
pub async fn run(package: &str, dev: bool, verbose: bool) -> Result<()> {
    let cwd = std::env::current_dir()?;
    let package_json_path = cwd.join("package.json");

    // Parse package spec (name@version)
    let (name, version_spec) = parse_package_spec(package);

    if verbose {
        println!(
            "📦 Adding {} to {}",
            name,
            if dev {
                "devDependencies"
            } else {
                "dependencies"
            }
        );
    }

    // Check if package.json exists
    if !package_json_path.exists() {
        anyhow::bail!("package.json not found in current directory. Run 'npm init' first.");
    }

    // Load existing package.json
    let mut pkg_json = PackageJson::load(&package_json_path)?;

    // Create npm client and resolve version
    let client = NpmClient::new().context("Failed to create npm client")?;

    if verbose {
        println!("🔍 Resolving version for {}@{}", name, version_spec);
    }

    let resolved_version = resolve_version(&client, &name, &version_spec).await?;

    if verbose {
        println!("📋 Resolved {} to version {}", name, resolved_version);
    }

    // Add to package.json with caret prefix for semver compatibility
    let version_entry = format!("^{}", resolved_version);

    if dev {
        pkg_json.dev_dependencies.insert(name.clone(), version_entry.clone());
    } else {
        pkg_json.dependencies.insert(name.clone(), version_entry.clone());
    }

    // Save package.json
    pkg_json.save(&package_json_path)?;

    if verbose {
        println!("📝 Updated package.json");
    }

    // Fetch and install the package
    let metadata = client.get_abbreviated(&name).await?;
    let version_info = metadata
        .versions
        .get(&resolved_version)
        .with_context(|| format!("Version {} not found for {}", resolved_version, name))?;

    if verbose {
        println!("⬇️  Downloading {}@{}", name, resolved_version);
    }

    // Download tarball
    let tarball_data = client
        .download_tarball(&version_info.dist.tarball)
        .await
        .with_context(|| format!("Failed to download {}", name))?;

    // Create node_modules directory
    let node_modules = cwd.join("node_modules");
    fs::create_dir_all(&node_modules)?;

    // Create temp file for tarball
    let temp_dir = tempfile::tempdir()?;
    let tarball_path = temp_dir.path().join(format!("{}.tgz", name.replace('/', "_")));
    fs::write(&tarball_path, &tarball_data)?;

    // Extract to node_modules
    let target_dir = node_modules.join(&name);
    if target_dir.exists() {
        fs::remove_dir_all(&target_dir)?;
    }
    fs::create_dir_all(&target_dir)?;

    if verbose {
        println!("📂 Extracting to {}", target_dir.display());
    }

    DirectExtractor::extract(&tarball_path, &target_dir)
        .with_context(|| format!("Failed to extract {}", name))?;

    println!("✅ Added {} v{}", name, resolved_version);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_package_spec_simple() {
        let (name, version) = parse_package_spec("lodash");
        assert_eq!(name, "lodash");
        assert_eq!(version, "latest");
    }

    #[test]
    fn test_parse_package_spec_with_version() {
        let (name, version) = parse_package_spec("lodash@4.17.21");
        assert_eq!(name, "lodash");
        assert_eq!(version, "4.17.21");
    }

    #[test]
    fn test_parse_package_spec_with_constraint() {
        let (name, version) = parse_package_spec("lodash@^4.17.0");
        assert_eq!(name, "lodash");
        assert_eq!(version, "^4.17.0");
    }

    #[test]
    fn test_parse_package_spec_scoped() {
        let (name, version) = parse_package_spec("@types/node");
        assert_eq!(name, "@types/node");
        assert_eq!(version, "latest");
    }

    #[test]
    fn test_parse_package_spec_scoped_with_version() {
        let (name, version) = parse_package_spec("@types/node@18.0.0");
        assert_eq!(name, "@types/node");
        assert_eq!(version, "18.0.0");
    }
}
