//! dx run <script> - Execute scripts from package.json
//!
//! Features:
//! - Run scripts defined in package.json
//! - Support pre/post hooks (pretest, posttest, etc.)
//! - Pass arguments to scripts
//! - Workspace support with --filter

use anyhow::{bail, Context, Result};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// Run a script from package.json
pub async fn run(script: &str, args: &[String], filter: Option<&str>, verbose: bool) -> Result<()> {
    let cwd = std::env::current_dir()?;

    // Check if we're in a workspace and filter is specified
    if let Some(filter_pattern) = filter {
        return run_in_workspace(script, args, filter_pattern, verbose).await;
    }

    // Find package.json
    let pkg_json_path = find_package_json(&cwd)?;
    let pkg_json = read_package_json(&pkg_json_path)?;

    // Get scripts
    let scripts = pkg_json
        .get("scripts")
        .and_then(|s| s.as_object())
        .map(|s| {
            s.iter()
                .map(|(k, v)| (k.clone(), v.as_str().unwrap_or("").to_string()))
                .collect::<HashMap<_, _>>()
        })
        .unwrap_or_default();

    // Check if script exists
    let script_cmd = scripts
        .get(script)
        .ok_or_else(|| anyhow::anyhow!("Script '{}' not found in package.json", script))?;

    if verbose {
        println!("📜 Running script: {}", script);
    }

    // Run pre-hook if exists
    let pre_script = format!("pre{}", script);
    if let Some(pre_cmd) = scripts.get(&pre_script) {
        if verbose {
            println!("  Running pre-hook: {}", pre_script);
        }
        run_script_command(pre_cmd, &[], pkg_json_path.parent().unwrap(), verbose)?;
    }

    // Run main script
    run_script_command(script_cmd, args, pkg_json_path.parent().unwrap(), verbose)?;

    // Run post-hook if exists
    let post_script = format!("post{}", script);
    if let Some(post_cmd) = scripts.get(&post_script) {
        if verbose {
            println!("  Running post-hook: {}", post_script);
        }
        run_script_command(post_cmd, &[], pkg_json_path.parent().unwrap(), verbose)?;
    }

    Ok(())
}

/// Run a script in workspace packages matching the filter
async fn run_in_workspace(
    script: &str,
    args: &[String],
    filter: &str,
    verbose: bool,
) -> Result<()> {
    use dx_pkg_workspace::{Workspace, WorkspaceFilter};

    let cwd = std::env::current_dir()?;

    let workspace =
        Workspace::detect(&cwd)?.ok_or_else(|| anyhow::anyhow!("Not in a workspace"))?;

    let filter = WorkspaceFilter::new(vec![filter.to_string()]);
    let packages = workspace.filter_packages(&filter);

    if packages.is_empty() {
        bail!("No packages match filter: {}", filter.patterns.join(", "));
    }

    println!("🏃 Running '{}' in {} packages", script, packages.len());

    for pkg in packages {
        println!("\n📦 {}", pkg.name);

        if let Some(cmd) = pkg.package_json.scripts.get(script) {
            run_script_command(cmd, args, &pkg.path, verbose)?;
        } else if verbose {
            println!("  ⚠️  Script '{}' not found, skipping", script);
        }
    }

    Ok(())
}

/// Execute a script command
fn run_script_command(cmd: &str, args: &[String], cwd: &Path, verbose: bool) -> Result<()> {
    // Build PATH with node_modules/.bin
    let mut path = std::env::var("PATH").unwrap_or_default();
    let bin_path = cwd.join("node_modules").join(".bin");

    #[cfg(windows)]
    {
        path = format!("{};{}", bin_path.display(), path);
    }
    #[cfg(not(windows))]
    {
        path = format!("{}:{}", bin_path.display(), path);
    }

    // Build full command with args
    let full_cmd = if args.is_empty() {
        cmd.to_string()
    } else {
        format!("{} {}", cmd, args.join(" "))
    };

    if verbose {
        println!("  $ {}", full_cmd);
    }

    // Execute command
    let status = if cfg!(windows) {
        Command::new("cmd")
            .args(["/C", &full_cmd])
            .current_dir(cwd)
            .env("PATH", &path)
            .env("npm_lifecycle_event", cmd)
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()
    } else {
        Command::new("sh")
            .args(["-c", &full_cmd])
            .current_dir(cwd)
            .env("PATH", &path)
            .env("npm_lifecycle_event", cmd)
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()
    };

    let status = status.context("Failed to execute script")?;

    if !status.success() {
        bail!("Script '{}' exited with code {}", cmd, status.code().unwrap_or(-1));
    }

    Ok(())
}

/// List available scripts
#[allow(dead_code)]
pub async fn list_scripts(verbose: bool) -> Result<()> {
    let cwd = std::env::current_dir()?;
    let pkg_json_path = find_package_json(&cwd)?;
    let pkg_json = read_package_json(&pkg_json_path)?;

    let scripts = pkg_json.get("scripts").and_then(|s| s.as_object());

    match scripts {
        Some(scripts) if !scripts.is_empty() => {
            println!("📜 Available scripts:");
            for (name, cmd) in scripts {
                if verbose {
                    println!("  {} → {}", name, cmd.as_str().unwrap_or(""));
                } else {
                    println!("  {}", name);
                }
            }
        }
        _ => {
            println!("No scripts defined in package.json");
        }
    }

    Ok(())
}

/// Find package.json in current or parent directories
fn find_package_json(start: &Path) -> Result<PathBuf> {
    let mut current = start.to_path_buf();

    loop {
        let pkg_json = current.join("package.json");
        if pkg_json.exists() {
            return Ok(pkg_json);
        }

        if !current.pop() {
            bail!("No package.json found in current directory or any parent");
        }
    }
}

/// Read and parse package.json
fn read_package_json(path: &Path) -> Result<serde_json::Value> {
    let content = std::fs::read_to_string(path).context("Failed to read package.json")?;
    serde_json::from_str(&content).context("Failed to parse package.json")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_find_package_json() {
        let temp = tempdir().unwrap();
        let pkg_json = temp.path().join("package.json");
        fs::write(&pkg_json, r#"{"name":"test"}"#).unwrap();

        let found = find_package_json(temp.path()).unwrap();
        assert_eq!(found, pkg_json);
    }

    #[test]
    fn test_find_package_json_not_found() {
        let temp = tempdir().unwrap();
        let result = find_package_json(temp.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_read_package_json() {
        let temp = tempdir().unwrap();
        let pkg_json = temp.path().join("package.json");
        fs::write(&pkg_json, r#"{"name":"test","scripts":{"build":"echo build"}}"#).unwrap();

        let parsed = read_package_json(&pkg_json).unwrap();
        assert_eq!(parsed["name"], "test");
        assert_eq!(parsed["scripts"]["build"], "echo build");
    }
}
