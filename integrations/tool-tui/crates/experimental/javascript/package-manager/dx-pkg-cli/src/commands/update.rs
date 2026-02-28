//! Update command - Update packages to latest compatible versions
//!
//! Implements `dx update [package]` command:
//! - Check for newer compatible versions
//! - Update package.json
//! - Reinstall updated packages

use anyhow::{Context, Result};
use dx_pkg_extract::DirectExtractor;
use dx_pkg_npm::NpmClient;
use std::fs;

// Re-use PackageJson from add module
use super::add::PackageJson;

/// Parse semver constraint to get base version and constraint type
fn parse_constraint(constraint: &str) -> (char, &str) {
    let constraint = constraint.trim();
    if let Some(stripped) = constraint.strip_prefix('^') {
        ('^', stripped)
    } else if let Some(stripped) = constraint.strip_prefix('~') {
        ('~', stripped)
    } else if let Some(stripped) = constraint.strip_prefix(">=") {
        ('>', stripped)
    } else {
        ('=', constraint)
    }
}

/// Check if a version satisfies a constraint (simplified)
fn version_satisfies(version: &str, constraint: &str) -> bool {
    let (constraint_type, base_version) = parse_constraint(constraint);

    // Parse versions into parts
    let parse_version = |v: &str| -> (u32, u32, u32) {
        let parts: Vec<&str> = v.split('.').collect();
        let major = parts.first().and_then(|s| s.parse().ok()).unwrap_or(0);
        let minor = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(0);
        let patch = parts.get(2).and_then(|s| s.split('-').next()?.parse().ok()).unwrap_or(0);
        (major, minor, patch)
    };

    let (v_major, v_minor, v_patch) = parse_version(version);
    let (c_major, c_minor, c_patch) = parse_version(base_version);

    match constraint_type {
        '^' => {
            // Caret: allows changes that do not modify the left-most non-zero digit
            if c_major > 0 {
                v_major == c_major
                    && (v_minor > c_minor || (v_minor == c_minor && v_patch >= c_patch))
            } else if c_minor > 0 {
                v_major == 0 && v_minor == c_minor && v_patch >= c_patch
            } else {
                v_major == 0 && v_minor == 0 && v_patch == c_patch
            }
        }
        '~' => {
            // Tilde: allows patch-level changes
            v_major == c_major && v_minor == c_minor && v_patch >= c_patch
        }
        '>' => {
            // Greater than or equal
            (v_major, v_minor, v_patch) >= (c_major, c_minor, c_patch)
        }
        _ => {
            // Exact match
            version == base_version
        }
    }
}

/// Find the latest version that satisfies a constraint
async fn find_latest_compatible(
    client: &NpmClient,
    name: &str,
    constraint: &str,
) -> Result<Option<String>> {
    let metadata = client
        .get_abbreviated(name)
        .await
        .with_context(|| format!("Failed to fetch metadata for {}", name))?;

    // Get all versions and filter by constraint
    let mut compatible_versions: Vec<&String> =
        metadata.versions.keys().filter(|v| version_satisfies(v, constraint)).collect();

    // Sort versions (simple string sort works for semver in most cases)
    compatible_versions.sort_by(|a, b| {
        let parse = |v: &str| -> (u32, u32, u32) {
            let parts: Vec<&str> = v.split('.').collect();
            let major = parts.first().and_then(|s| s.parse().ok()).unwrap_or(0);
            let minor = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(0);
            let patch = parts.get(2).and_then(|s| s.split('-').next()?.parse().ok()).unwrap_or(0);
            (major, minor, patch)
        };
        parse(a).cmp(&parse(b))
    });

    Ok(compatible_versions.last().map(|s| (*s).clone()))
}

/// Run the update command
pub async fn run(package: Option<&str>, verbose: bool) -> Result<()> {
    let cwd = std::env::current_dir()?;
    let package_json_path = cwd.join("package.json");
    let node_modules = cwd.join("node_modules");

    // Check if package.json exists
    if !package_json_path.exists() {
        anyhow::bail!("package.json not found in current directory");
    }

    // Load package.json
    let mut pkg_json = PackageJson::load(&package_json_path)?;

    // Create npm client
    let client = NpmClient::new().context("Failed to create npm client")?;

    // Collect packages to update
    let packages_to_update: Vec<(String, String, bool)> = if let Some(pkg_name) = package {
        // Update specific package
        if let Some(constraint) = pkg_json.dependencies.get(pkg_name) {
            vec![(pkg_name.to_string(), constraint.clone(), false)]
        } else if let Some(constraint) = pkg_json.dev_dependencies.get(pkg_name) {
            vec![(pkg_name.to_string(), constraint.clone(), true)]
        } else {
            anyhow::bail!("Package '{}' not found in dependencies", pkg_name);
        }
    } else {
        // Update all packages
        let mut all: Vec<(String, String, bool)> = pkg_json
            .dependencies
            .iter()
            .map(|(k, v)| (k.clone(), v.clone(), false))
            .collect();
        all.extend(pkg_json.dev_dependencies.iter().map(|(k, v)| (k.clone(), v.clone(), true)));
        all
    };

    if packages_to_update.is_empty() {
        println!("No packages to update");
        return Ok(());
    }

    println!("🔍 Checking for updates...");

    let mut updated_count = 0;

    for (name, constraint, is_dev) in packages_to_update {
        if verbose {
            println!("  Checking {}...", name);
        }

        // Find latest compatible version
        let latest = match find_latest_compatible(&client, &name, &constraint).await {
            Ok(Some(v)) => v,
            Ok(None) => {
                if verbose {
                    println!("  ⚠️  No compatible version found for {}", name);
                }
                continue;
            }
            Err(e) => {
                if verbose {
                    println!("  ⚠️  Failed to check {}: {}", name, e);
                }
                continue;
            }
        };

        // Check if update is needed
        let (_, current_version) = parse_constraint(&constraint);
        if current_version == latest {
            if verbose {
                println!("  ✓ {} is up to date ({})", name, latest);
            }
            continue;
        }

        println!("  📦 Updating {} {} → {}", name, current_version, latest);

        // Update package.json
        let new_constraint = format!("^{}", latest);
        if is_dev {
            pkg_json.dev_dependencies.insert(name.clone(), new_constraint);
        } else {
            pkg_json.dependencies.insert(name.clone(), new_constraint);
        }

        // Download and install new version
        let metadata = client.get_abbreviated(&name).await?;
        if let Some(version_info) = metadata.versions.get(&latest) {
            let tarball_data = client
                .download_tarball(&version_info.dist.tarball)
                .await
                .with_context(|| format!("Failed to download {}", name))?;

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

            DirectExtractor::extract(&tarball_path, &target_dir)
                .with_context(|| format!("Failed to extract {}", name))?;
        }

        updated_count += 1;
    }

    // Save package.json
    pkg_json.save(&package_json_path)?;

    if updated_count > 0 {
        println!("✅ Updated {} package(s)", updated_count);
    } else {
        println!("✅ All packages are up to date");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_constraint() {
        assert_eq!(parse_constraint("^4.17.21"), ('^', "4.17.21"));
        assert_eq!(parse_constraint("~1.2.3"), ('~', "1.2.3"));
        assert_eq!(parse_constraint(">=2.0.0"), ('>', "2.0.0"));
        assert_eq!(parse_constraint("1.0.0"), ('=', "1.0.0"));
    }

    #[test]
    fn test_version_satisfies_caret() {
        // ^4.17.0 should allow 4.17.x and 4.18.x but not 5.x
        assert!(version_satisfies("4.17.21", "^4.17.0"));
        assert!(version_satisfies("4.18.0", "^4.17.0"));
        assert!(!version_satisfies("5.0.0", "^4.17.0"));
        assert!(!version_satisfies("4.16.0", "^4.17.0"));
    }

    #[test]
    fn test_version_satisfies_tilde() {
        // ~1.2.3 should allow 1.2.x but not 1.3.x
        assert!(version_satisfies("1.2.3", "~1.2.3"));
        assert!(version_satisfies("1.2.5", "~1.2.3"));
        assert!(!version_satisfies("1.3.0", "~1.2.3"));
    }
}
