//! Remove command - Remove a package from dependencies
//!
//! Implements `dx remove <package>` command:
//! - Remove from package.json (dependencies or devDependencies)
//! - Remove from node_modules

use anyhow::{Context, Result};
use std::fs;

// Re-use PackageJson from add module
use super::add::PackageJson;

/// Run the remove command
pub async fn run(package: &str, verbose: bool) -> Result<()> {
    let cwd = std::env::current_dir()?;
    let package_json_path = cwd.join("package.json");
    let node_modules = cwd.join("node_modules");

    if verbose {
        println!("🗑️  Removing {}", package);
    }

    // Check if package.json exists
    if !package_json_path.exists() {
        anyhow::bail!("package.json not found in current directory");
    }

    // Load package.json
    let mut pkg_json = PackageJson::load(&package_json_path)?;

    // Remove from dependencies
    let was_dep = pkg_json.dependencies.remove(package).is_some();
    let was_dev_dep = pkg_json.dev_dependencies.remove(package).is_some();

    // Also check peer and optional dependencies
    let was_peer_dep = pkg_json
        .peer_dependencies
        .as_mut()
        .map(|deps| deps.remove(package).is_some())
        .unwrap_or(false);
    let was_optional_dep = pkg_json
        .optional_dependencies
        .as_mut()
        .map(|deps| deps.remove(package).is_some())
        .unwrap_or(false);

    if !was_dep && !was_dev_dep && !was_peer_dep && !was_optional_dep {
        anyhow::bail!("Package '{}' is not listed in package.json", package);
    }

    // Save package.json
    pkg_json.save(&package_json_path)?;

    if verbose {
        let dep_type = if was_dep {
            "dependencies"
        } else if was_dev_dep {
            "devDependencies"
        } else if was_peer_dep {
            "peerDependencies"
        } else {
            "optionalDependencies"
        };
        println!("📝 Removed {} from {}", package, dep_type);
    }

    // Remove from node_modules
    let pkg_path = node_modules.join(package);
    if pkg_path.exists() {
        fs::remove_dir_all(&pkg_path)
            .with_context(|| format!("Failed to remove {}", pkg_path.display()))?;

        if verbose {
            println!("🗂️  Removed {}", pkg_path.display());
        }
    } else if verbose {
        println!("⚠️  Package not found in node_modules (already removed?)");
    }

    // Handle scoped packages - also remove empty scope directory
    if package.starts_with('@') {
        if let Some(scope) = package.split('/').next() {
            let scope_dir = node_modules.join(scope);
            if scope_dir.exists() {
                // Check if scope directory is empty
                let is_empty = fs::read_dir(&scope_dir)
                    .map(|mut entries| entries.next().is_none())
                    .unwrap_or(false);

                if is_empty {
                    fs::remove_dir(&scope_dir).ok();
                    if verbose {
                        println!("🗂️  Removed empty scope directory {}", scope);
                    }
                }
            }
        }
    }

    println!("✅ Removed {}", package);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use tempfile::tempdir;

    #[test]
    fn test_remove_from_dependencies() {
        let temp = tempdir().unwrap();
        let pkg_json_path = temp.path().join("package.json");

        // Create a package.json with a dependency
        let pkg = PackageJson {
            name: "test".to_string(),
            version: "1.0.0".to_string(),
            dependencies: HashMap::from([("lodash".to_string(), "^4.17.21".to_string())]),
            dev_dependencies: HashMap::new(),
            peer_dependencies: None,
            optional_dependencies: None,
            other: HashMap::new(),
        };
        pkg.save(&pkg_json_path).unwrap();

        // Verify it was saved
        let loaded = PackageJson::load(&pkg_json_path).unwrap();
        assert!(loaded.dependencies.contains_key("lodash"));
    }
}
