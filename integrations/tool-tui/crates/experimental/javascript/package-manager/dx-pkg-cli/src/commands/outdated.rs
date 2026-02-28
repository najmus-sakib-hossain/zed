//! Outdated command - Show packages with newer versions available
//!
//! Implements `dx outdated` command:
//! - Compare installed vs latest versions
//! - Show wanted (constraint-compatible) vs latest versions

use anyhow::{Context, Result};
use dx_pkg_npm::NpmClient;
use serde::Deserialize;
use std::fs;
use std::path::Path;

// Re-use PackageJson from add module
use super::add::PackageJson;

/// Simplified package.json for reading installed package info
#[derive(Debug, Deserialize)]
struct InstalledPackageJson {
    #[serde(default)]
    version: String,
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

/// Parse semver constraint to get base version
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
            if c_major > 0 {
                v_major == c_major
                    && (v_minor > c_minor || (v_minor == c_minor && v_patch >= c_patch))
            } else if c_minor > 0 {
                v_major == 0 && v_minor == c_minor && v_patch >= c_patch
            } else {
                v_major == 0 && v_minor == 0 && v_patch == c_patch
            }
        }
        '~' => v_major == c_major && v_minor == c_minor && v_patch >= c_patch,
        '>' => (v_major, v_minor, v_patch) >= (c_major, c_minor, c_patch),
        _ => version == base_version,
    }
}

/// Find the latest version that satisfies a constraint
fn find_wanted_version(versions: &[String], constraint: &str) -> Option<String> {
    let mut compatible: Vec<&String> =
        versions.iter().filter(|v| version_satisfies(v, constraint)).collect();

    compatible.sort_by(|a, b| {
        let parse = |v: &str| -> (u32, u32, u32) {
            let parts: Vec<&str> = v.split('.').collect();
            let major = parts.first().and_then(|s| s.parse().ok()).unwrap_or(0);
            let minor = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(0);
            let patch = parts.get(2).and_then(|s| s.split('-').next()?.parse().ok()).unwrap_or(0);
            (major, minor, patch)
        };
        parse(a).cmp(&parse(b))
    });

    compatible.last().map(|s| (*s).clone())
}

/// Outdated package info
struct OutdatedInfo {
    name: String,
    current: String,
    wanted: String,
    latest: String,
    dep_type: String,
}

/// Run the outdated command
pub async fn run(verbose: bool) -> Result<()> {
    let cwd = std::env::current_dir()?;
    let package_json_path = cwd.join("package.json");
    let node_modules = cwd.join("node_modules");

    // Check if package.json exists
    if !package_json_path.exists() {
        anyhow::bail!("package.json not found in current directory");
    }

    // Load package.json
    let pkg_json = PackageJson::load(&package_json_path)?;

    // Create npm client
    let client = NpmClient::new().context("Failed to create npm client")?;

    // Collect all dependencies
    let mut all_deps: Vec<(String, String, String)> = Vec::new();

    for (name, version) in &pkg_json.dependencies {
        all_deps.push((name.clone(), version.clone(), "dependencies".to_string()));
    }

    for (name, version) in &pkg_json.dev_dependencies {
        all_deps.push((name.clone(), version.clone(), "devDependencies".to_string()));
    }

    if all_deps.is_empty() {
        println!("No dependencies found");
        return Ok(());
    }

    println!("Checking {} packages for updates...", all_deps.len());

    let mut outdated: Vec<OutdatedInfo> = Vec::new();

    for (name, constraint, dep_type) in all_deps {
        if verbose {
            println!("  Checking {}...", name);
        }

        // Get installed version
        let current = match get_installed_version(&node_modules, &name) {
            Some(v) => v,
            None => {
                if verbose {
                    println!("  ⚠️  {} not installed", name);
                }
                continue;
            }
        };

        // Fetch metadata from npm
        let metadata = match client.get_abbreviated(&name).await {
            Ok(m) => m,
            Err(e) => {
                if verbose {
                    println!("  ⚠️  Failed to fetch {}: {}", name, e);
                }
                continue;
            }
        };

        // Get latest version
        let latest = metadata.dist_tags.get("latest").cloned().unwrap_or_else(|| current.clone());

        // Get wanted version (latest that satisfies constraint)
        let versions: Vec<String> = metadata.versions.keys().cloned().collect();
        let wanted = find_wanted_version(&versions, &constraint).unwrap_or_else(|| current.clone());

        // Check if outdated
        if current != wanted || current != latest {
            outdated.push(OutdatedInfo {
                name,
                current,
                wanted,
                latest,
                dep_type,
            });
        }
    }

    if outdated.is_empty() {
        println!("\n✅ All packages are up to date!");
        return Ok(());
    }

    // Print table header
    println!();
    println!(
        "{:<30} {:<15} {:<15} {:<15} {:<15}",
        "Package", "Current", "Wanted", "Latest", "Location"
    );
    println!("{}", "-".repeat(90));

    // Print outdated packages
    for info in &outdated {
        let current_color = if info.current != info.wanted {
            "⚠️ "
        } else {
            ""
        };
        let latest_color = if info.wanted != info.latest {
            "🔴"
        } else {
            ""
        };

        println!(
            "{:<30} {:<15} {:<15} {:<15} {}",
            info.name,
            format!("{}{}", current_color, info.current),
            info.wanted,
            format!("{}{}", latest_color, info.latest),
            info.dep_type
        );
    }

    println!();
    println!("Found {} outdated package(s)", outdated.len());
    println!();
    println!("Run `dx update` to update to wanted versions");
    println!("Run `dx update <package>` to update a specific package");

    Ok(())
}
