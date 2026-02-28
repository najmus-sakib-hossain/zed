//! dx dlx <package> - Download and execute a package without installing
//!
//! Similar to npx for running packages without adding them to dependencies.

use anyhow::{bail, Context, Result};
use std::path::PathBuf;
use std::process::{Command, Stdio};

/// Download and execute a package
pub async fn run(package: &str, args: &[String], verbose: bool) -> Result<()> {
    // Parse package name and version
    let (pkg_name, pkg_version) = parse_package_spec(package);

    if verbose {
        println!(
            "📦 Downloading {} (version: {})",
            pkg_name,
            pkg_version.as_deref().unwrap_or("latest")
        );
    }

    // Create temporary directory for the package
    let temp_dir = get_dlx_cache_dir()?;
    let pkg_dir = temp_dir.join(format!(
        "{}@{}",
        pkg_name.replace('/', "_"),
        pkg_version.as_deref().unwrap_or("latest")
    ));

    // Check if already cached
    let needs_install = !pkg_dir.join("node_modules").exists();

    if needs_install {
        if verbose {
            println!("  Installing to cache: {}", pkg_dir.display());
        }

        std::fs::create_dir_all(&pkg_dir)?;

        // Create minimal package.json
        let pkg_json = format!(
            r#"{{"name":"dx-dlx-temp","version":"1.0.0","dependencies":{{"{}":"{}"}}}}"#,
            pkg_name,
            pkg_version.as_deref().unwrap_or("*")
        );
        std::fs::write(pkg_dir.join("package.json"), pkg_json)?;

        // Run npm install (or dx install when available)
        let install_status = Command::new("npm")
            .args(["install", "--prefer-offline", "--no-audit", "--no-fund"])
            .current_dir(&pkg_dir)
            .stdout(if verbose {
                Stdio::inherit()
            } else {
                Stdio::null()
            })
            .stderr(if verbose {
                Stdio::inherit()
            } else {
                Stdio::null()
            })
            .status()
            .context("Failed to install package. Make sure npm is available.")?;

        if !install_status.success() {
            bail!("Failed to install package: {}", package);
        }
    } else if verbose {
        println!("  Using cached version");
    }

    // Find the binary to execute
    let bin_name = get_bin_name(&pkg_name);
    let bin_path = pkg_dir.join("node_modules").join(".bin").join(&bin_name);

    #[cfg(windows)]
    let bin_path = if bin_path.exists() {
        bin_path
    } else {
        pkg_dir.join("node_modules").join(".bin").join(format!("{}.cmd", bin_name))
    };

    if !bin_path.exists() {
        // Try to find any binary in the package
        let bin_dir = pkg_dir.join("node_modules").join(".bin");
        if bin_dir.exists() {
            let entries: Vec<_> = std::fs::read_dir(&bin_dir)?
                .filter_map(|e| e.ok())
                .map(|e| e.file_name().to_string_lossy().to_string())
                .filter(|n| !n.ends_with(".cmd") && !n.ends_with(".ps1"))
                .collect();

            if entries.is_empty() {
                bail!("Package '{}' does not provide any executables", package);
            } else {
                bail!(
                    "Binary '{}' not found. Available binaries: {}",
                    bin_name,
                    entries.join(", ")
                );
            }
        } else {
            bail!("Package '{}' does not provide any executables", package);
        }
    }

    if verbose {
        println!("🚀 Running: {} {}", bin_path.display(), args.join(" "));
    }

    // Execute the binary
    let status = Command::new(&bin_path)
        .args(args)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .context(format!("Failed to execute: {}", bin_path.display()))?;

    if !status.success() {
        std::process::exit(status.code().unwrap_or(1));
    }

    Ok(())
}

/// Parse package specification (name@version)
fn parse_package_spec(spec: &str) -> (String, Option<String>) {
    // Handle scoped packages (@scope/name@version)
    if let Some(stripped) = spec.strip_prefix('@') {
        // Find the second @ (version separator)
        if let Some(at_pos) = stripped.find('@') {
            let name = format!("@{}", &stripped[..at_pos]);
            let version = stripped[at_pos + 1..].to_string();
            return (name, Some(version));
        }
        return (spec.to_string(), None);
    }

    // Regular package (name@version)
    if let Some(at_pos) = spec.find('@') {
        let name = spec[..at_pos].to_string();
        let version = spec[at_pos + 1..].to_string();
        (name, Some(version))
    } else {
        (spec.to_string(), None)
    }
}

/// Get the binary name from package name
fn get_bin_name(pkg_name: &str) -> String {
    // For scoped packages, use the part after /
    if pkg_name.starts_with('@') {
        if let Some(slash_pos) = pkg_name.find('/') {
            return pkg_name[slash_pos + 1..].to_string();
        }
    }
    pkg_name.to_string()
}

/// Get the dlx cache directory
fn get_dlx_cache_dir() -> Result<PathBuf> {
    let cache_dir = dirs::cache_dir().unwrap_or_else(|| PathBuf::from(".")).join("dx").join("dlx");

    std::fs::create_dir_all(&cache_dir)?;
    Ok(cache_dir)
}

/// Clear the dlx cache
#[allow(dead_code)]
pub async fn clear_cache(verbose: bool) -> Result<()> {
    let cache_dir = get_dlx_cache_dir()?;

    if cache_dir.exists() {
        if verbose {
            println!("🗑️  Clearing dlx cache: {}", cache_dir.display());
        }
        std::fs::remove_dir_all(&cache_dir)?;
        println!("✅ Cache cleared");
    } else {
        println!("Cache is already empty");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_package_spec_simple() {
        let (name, version) = parse_package_spec("typescript");
        assert_eq!(name, "typescript");
        assert_eq!(version, None);
    }

    #[test]
    fn test_parse_package_spec_with_version() {
        let (name, version) = parse_package_spec("typescript@5.0.0");
        assert_eq!(name, "typescript");
        assert_eq!(version, Some("5.0.0".to_string()));
    }

    #[test]
    fn test_parse_package_spec_scoped() {
        let (name, version) = parse_package_spec("@types/node");
        assert_eq!(name, "@types/node");
        assert_eq!(version, None);
    }

    #[test]
    fn test_parse_package_spec_scoped_with_version() {
        let (name, version) = parse_package_spec("@types/node@18.0.0");
        assert_eq!(name, "@types/node");
        assert_eq!(version, Some("18.0.0".to_string()));
    }

    #[test]
    fn test_get_bin_name_simple() {
        assert_eq!(get_bin_name("typescript"), "typescript");
    }

    #[test]
    fn test_get_bin_name_scoped() {
        assert_eq!(get_bin_name("@angular/cli"), "cli");
    }
}
