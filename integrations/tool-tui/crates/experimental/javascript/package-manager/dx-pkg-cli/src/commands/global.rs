//! Global package management
//!
//! Implements global package installation and listing:
//! - `dx add -g <package>` - Install package globally
//! - `dx list -g` - List globally installed packages
//!
//! Global packages are installed to:
//! - Linux/macOS: ~/.dx/global/node_modules
//! - Windows: %LOCALAPPDATA%\dx\global\node_modules
//!
//! Binaries are symlinked to:
//! - Linux/macOS: ~/.dx/bin
//! - Windows: %LOCALAPPDATA%\dx\bin

use anyhow::{Context, Result};
use dx_pkg_extract::DirectExtractor;
use dx_pkg_npm::NpmClient;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

/// Get the global DX directory
/// - Linux/macOS: ~/.dx
/// - Windows: %LOCALAPPDATA%\dx
pub fn get_global_dx_dir() -> Result<PathBuf> {
    #[cfg(windows)]
    {
        dirs::data_local_dir()
            .map(|p| p.join("dx"))
            .context("Could not determine local data directory")
    }
    #[cfg(not(windows))]
    {
        dirs::home_dir()
            .map(|p| p.join(".dx"))
            .context("Could not determine home directory")
    }
}

/// Get the global node_modules directory
pub fn get_global_node_modules() -> Result<PathBuf> {
    Ok(get_global_dx_dir()?.join("global").join("node_modules"))
}

/// Get the global bin directory
pub fn get_global_bin_dir() -> Result<PathBuf> {
    Ok(get_global_dx_dir()?.join("bin"))
}

/// Simplified package.json structure for global packages
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GlobalPackageJson {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub bin: Option<BinField>,
    #[serde(flatten)]
    pub other: HashMap<String, serde_json::Value>,
}

/// Bin field can be a string or a map
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum BinField {
    Single(String),
    Multiple(HashMap<String, String>),
}

impl BinField {
    /// Get all bin entries as (name, path) pairs
    pub fn entries(&self, package_name: &str) -> Vec<(String, String)> {
        match self {
            BinField::Single(path) => {
                // Use package name as bin name
                let bin_name = package_name.split('/').next_back().unwrap_or(package_name);
                vec![(bin_name.to_string(), path.clone())]
            }
            BinField::Multiple(map) => map.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
        }
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
        if let Some(latest) = metadata.dist_tags.get("latest") {
            return Ok(latest.clone());
        }
        let mut versions: Vec<&String> = metadata.versions.keys().collect();
        versions.sort();
        if let Some(v) = versions.last() {
            return Ok((*v).clone());
        }
        anyhow::bail!("No versions found for {}", name);
    }

    if metadata.versions.contains_key(version_spec) {
        return Ok(version_spec.to_string());
    }

    if let Some(latest) = metadata.dist_tags.get("latest") {
        return Ok(latest.clone());
    }

    anyhow::bail!("Could not resolve version {} for {}", version_spec, name);
}

/// Create a symlink (or junction on Windows)
fn create_bin_link(source: &Path, target: &Path) -> Result<()> {
    // Remove existing link/file if present
    if target.exists() || target.is_symlink() {
        if target.is_dir() {
            fs::remove_dir_all(target)?;
        } else {
            fs::remove_file(target)?;
        }
    }

    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(source, target).with_context(|| {
            format!("Failed to create symlink: {} -> {}", target.display(), source.display())
        })?;
    }

    #[cfg(windows)]
    {
        // On Windows, create a .cmd wrapper script instead of symlink
        // This avoids needing admin privileges
        let cmd_target = target.with_extension("cmd");
        let script = format!(
            "@echo off\r\nnode \"{}\" %*\r\n",
            source.display().to_string().replace('/', "\\")
        );
        fs::write(&cmd_target, script)
            .with_context(|| format!("Failed to create bin wrapper: {}", cmd_target.display()))?;
    }

    Ok(())
}

/// Install a package globally
pub async fn install_global(packages: &[String], verbose: bool) -> Result<()> {
    let global_node_modules = get_global_node_modules()?;
    let global_bin = get_global_bin_dir()?;

    // Ensure directories exist
    fs::create_dir_all(&global_node_modules)?;
    fs::create_dir_all(&global_bin)?;

    let client = NpmClient::new().context("Failed to create npm client")?;

    for package in packages {
        let (name, version_spec) = parse_package_spec(package);

        if verbose {
            println!("📦 Installing {} globally...", name);
        }

        // Resolve version
        let resolved_version = resolve_version(&client, &name, &version_spec).await?;

        if verbose {
            println!("📋 Resolved {} to version {}", name, resolved_version);
        }

        // Fetch metadata and download
        let metadata = client.get_abbreviated(&name).await?;
        let version_info = metadata
            .versions
            .get(&resolved_version)
            .with_context(|| format!("Version {} not found for {}", resolved_version, name))?;

        if verbose {
            println!("⬇️  Downloading {}@{}", name, resolved_version);
        }

        let tarball_data = client
            .download_tarball(&version_info.dist.tarball)
            .await
            .with_context(|| format!("Failed to download {}", name))?;

        // Extract to global node_modules
        let target_dir = global_node_modules.join(&name);
        if target_dir.exists() {
            fs::remove_dir_all(&target_dir)?;
        }
        fs::create_dir_all(&target_dir)?;

        // Create temp file for tarball
        let temp_dir = tempfile::tempdir()?;
        let tarball_path = temp_dir.path().join(format!("{}.tgz", name.replace('/', "_")));
        fs::write(&tarball_path, &tarball_data)?;

        if verbose {
            println!("📂 Extracting to {}", target_dir.display());
        }

        DirectExtractor::extract(&tarball_path, &target_dir)
            .with_context(|| format!("Failed to extract {}", name))?;

        // Read package.json to find bin entries
        let pkg_json_path = target_dir.join("package.json");
        if pkg_json_path.exists() {
            let content = fs::read_to_string(&pkg_json_path)?;
            let pkg: GlobalPackageJson = serde_json::from_str(&content)?;

            if let Some(bin) = &pkg.bin {
                let entries = bin.entries(&name);
                for (bin_name, bin_path) in entries {
                    let source = target_dir.join(&bin_path);
                    let target = global_bin.join(&bin_name);

                    if source.exists() {
                        // Make the script executable on Unix
                        #[cfg(unix)]
                        {
                            use std::os::unix::fs::PermissionsExt;
                            let mut perms = fs::metadata(&source)?.permissions();
                            perms.set_mode(0o755);
                            fs::set_permissions(&source, perms)?;
                        }

                        create_bin_link(&source, &target)?;

                        if verbose {
                            println!("🔗 Linked {} -> {}", bin_name, source.display());
                        }
                    }
                }
            }
        }

        println!("✅ Installed {} v{} globally", name, resolved_version);
    }

    // Print PATH hint
    println!();
    println!("💡 Make sure {} is in your PATH", global_bin.display());
    #[cfg(unix)]
    println!(
        "   Add to ~/.bashrc or ~/.zshrc: export PATH=\"{}:$PATH\"",
        global_bin.display()
    );
    #[cfg(windows)]
    println!("   Add to your PATH environment variable: {}", global_bin.display());

    Ok(())
}

/// Installed global package info
#[derive(Debug)]
pub struct GlobalPackageInfo {
    pub name: String,
    pub version: String,
    pub binaries: Vec<String>,
}

/// List globally installed packages
pub async fn list_global(verbose: bool) -> Result<()> {
    let global_node_modules = get_global_node_modules()?;
    let global_bin = get_global_bin_dir()?;

    if !global_node_modules.exists() {
        println!("No global packages installed.");
        println!();
        println!("Install packages globally with: dx add -g <package>");
        return Ok(());
    }

    let mut packages: Vec<GlobalPackageInfo> = Vec::new();

    // Read all directories in global node_modules
    for entry in fs::read_dir(&global_node_modules)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            let name = entry.file_name().to_string_lossy().to_string();

            // Handle scoped packages (@scope/package)
            if name.starts_with('@') {
                // This is a scope directory, look inside
                for sub_entry in fs::read_dir(&path)? {
                    let sub_entry = sub_entry?;
                    let sub_path = sub_entry.path();

                    if sub_path.is_dir() {
                        let sub_name = sub_entry.file_name().to_string_lossy().to_string();
                        let full_name = format!("{}/{}", name, sub_name);

                        if let Some(info) = read_package_info(&sub_path, &full_name) {
                            packages.push(info);
                        }
                    }
                }
            } else if let Some(info) = read_package_info(&path, &name) {
                packages.push(info);
            }
        }
    }

    if packages.is_empty() {
        println!("No global packages installed.");
        println!();
        println!("Install packages globally with: dx add -g <package>");
        return Ok(());
    }

    // Sort by name
    packages.sort_by(|a, b| a.name.cmp(&b.name));

    println!("Global packages ({}):", global_node_modules.display());
    println!();

    for (i, pkg) in packages.iter().enumerate() {
        let is_last = i == packages.len() - 1;
        let prefix = if is_last { "└──" } else { "├──" };

        println!("{} {}@{}", prefix, pkg.name, pkg.version);

        if verbose && !pkg.binaries.is_empty() {
            let sub_prefix = if is_last { "    " } else { "│   " };
            for bin in &pkg.binaries {
                println!("{}└── bin: {}", sub_prefix, bin);
            }
        }
    }

    println!();
    println!("{} packages installed globally", packages.len());

    if verbose {
        println!();
        println!("Binaries directory: {}", global_bin.display());
    }

    Ok(())
}

/// Read package info from a directory
fn read_package_info(path: &Path, name: &str) -> Option<GlobalPackageInfo> {
    let pkg_json_path = path.join("package.json");

    if !pkg_json_path.exists() {
        return None;
    }

    let content = fs::read_to_string(&pkg_json_path).ok()?;
    let pkg: GlobalPackageJson = serde_json::from_str(&content).ok()?;

    let binaries = pkg
        .bin
        .map(|b| b.entries(name).into_iter().map(|(n, _)| n).collect())
        .unwrap_or_default();

    Some(GlobalPackageInfo {
        name: pkg.name.clone(),
        version: pkg.version.clone(),
        binaries,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_package_spec_simple() {
        let (name, version) = parse_package_spec("typescript");
        assert_eq!(name, "typescript");
        assert_eq!(version, "latest");
    }

    #[test]
    fn test_parse_package_spec_with_version() {
        let (name, version) = parse_package_spec("typescript@5.0.0");
        assert_eq!(name, "typescript");
        assert_eq!(version, "5.0.0");
    }

    #[test]
    fn test_parse_package_spec_scoped() {
        let (name, version) = parse_package_spec("@angular/cli");
        assert_eq!(name, "@angular/cli");
        assert_eq!(version, "latest");
    }

    #[test]
    fn test_parse_package_spec_scoped_with_version() {
        let (name, version) = parse_package_spec("@angular/cli@17.0.0");
        assert_eq!(name, "@angular/cli");
        assert_eq!(version, "17.0.0");
    }

    #[test]
    fn test_bin_field_single() {
        let bin = BinField::Single("./bin/cli.js".to_string());
        let entries = bin.entries("my-package");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].0, "my-package");
        assert_eq!(entries[0].1, "./bin/cli.js");
    }

    #[test]
    fn test_bin_field_multiple() {
        let mut map = HashMap::new();
        map.insert("cmd1".to_string(), "./bin/cmd1.js".to_string());
        map.insert("cmd2".to_string(), "./bin/cmd2.js".to_string());
        let bin = BinField::Multiple(map);
        let entries = bin.entries("my-package");
        assert_eq!(entries.len(), 2);
    }

    #[test]
    fn test_get_global_dirs() {
        // These should not panic
        let dx_dir = get_global_dx_dir();
        assert!(dx_dir.is_ok());

        let node_modules = get_global_node_modules();
        assert!(node_modules.is_ok());

        let bin_dir = get_global_bin_dir();
        assert!(bin_dir.is_ok());
    }
}
