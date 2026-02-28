//! List command - Display installed packages
//!
//! Implements `dx list` command:
//! - Display installed packages with versions
//! - Show dependency tree (optional)

use anyhow::Result;
use serde::Deserialize;
use std::fs;
use std::path::Path;

// Re-use PackageJson from add module
use super::add::PackageJson;

/// Simplified package.json for reading installed package info
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct InstalledPackageJson {
    #[serde(default)]
    name: String,
    #[serde(default)]
    version: String,
    #[serde(default)]
    description: Option<String>,
}

/// Get installed version from node_modules
fn get_installed_version(node_modules: &Path, package_name: &str) -> Option<String> {
    let pkg_json_path = node_modules.join(package_name).join("package.json");
    if pkg_json_path.exists() {
        if let Ok(content) = fs::read_to_string(&pkg_json_path) {
            if let Ok(pkg) = serde_json::from_str::<InstalledPackageJson>(&content) {
                return Some(pkg.version);
            }
        }
    }
    None
}

/// Run the list command
pub async fn run(depth: usize, verbose: bool) -> Result<()> {
    let cwd = std::env::current_dir()?;
    let package_json_path = cwd.join("package.json");
    let node_modules = cwd.join("node_modules");

    // Check if package.json exists
    if !package_json_path.exists() {
        anyhow::bail!("package.json not found in current directory");
    }

    // Load package.json
    let pkg_json = PackageJson::load(&package_json_path)?;

    // Print project info
    println!("{}@{}", pkg_json.name, pkg_json.version);
    println!("{}", cwd.display());

    // Collect all dependencies
    let mut all_deps: Vec<(&str, &String, &str)> = Vec::new();

    for (name, version) in &pkg_json.dependencies {
        all_deps.push((name, version, ""));
    }

    for (name, version) in &pkg_json.dev_dependencies {
        all_deps.push((name, version, "dev"));
    }

    if let Some(ref peer_deps) = pkg_json.peer_dependencies {
        for (name, version) in peer_deps {
            all_deps.push((name, version, "peer"));
        }
    }

    if let Some(ref optional_deps) = pkg_json.optional_dependencies {
        for (name, version) in optional_deps {
            all_deps.push((name, version, "optional"));
        }
    }

    // Sort by name
    all_deps.sort_by(|a, b| a.0.cmp(b.0));

    if all_deps.is_empty() {
        println!("\n(no dependencies)");
        return Ok(());
    }

    println!();

    // Print dependencies
    for (i, (name, constraint, dep_type)) in all_deps.iter().enumerate() {
        let is_last = i == all_deps.len() - 1;
        let prefix = if is_last { "└──" } else { "├──" };

        // Get installed version
        let installed = get_installed_version(&node_modules, name);

        let version_display = match &installed {
            Some(v) => v.to_string(),
            None => format!("{} (not installed)", constraint),
        };

        let type_suffix = if dep_type.is_empty() {
            String::new()
        } else {
            format!(" [{}]", dep_type)
        };

        println!("{} {}@{}{}", prefix, name, version_display, type_suffix);

        // If verbose and depth > 0, show sub-dependencies
        if verbose && depth > 0 && installed.is_some() {
            let sub_pkg_path = node_modules.join(name).join("package.json");
            if sub_pkg_path.exists() {
                if let Ok(content) = fs::read_to_string(&sub_pkg_path) {
                    if let Ok(sub_pkg) = serde_json::from_str::<PackageJson>(&content) {
                        let sub_prefix = if is_last { "    " } else { "│   " };
                        for (sub_name, sub_version) in &sub_pkg.dependencies {
                            let sub_installed = get_installed_version(&node_modules, sub_name);
                            let sub_version_display = match &sub_installed {
                                Some(v) => v.to_string(),
                                None => sub_version.to_string(),
                            };
                            println!("{}├── {}@{}", sub_prefix, sub_name, sub_version_display);
                        }
                    }
                }
            }
        }
    }

    // Print summary
    let installed_count = all_deps
        .iter()
        .filter(|(name, _, _)| get_installed_version(&node_modules, name).is_some())
        .count();

    println!();
    println!("{} packages ({} installed)", all_deps.len(), installed_count);

    Ok(())
}
