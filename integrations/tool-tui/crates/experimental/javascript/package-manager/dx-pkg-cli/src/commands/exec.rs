//! dx exec <command> - Execute commands with node_modules/.bin in PATH
//!
//! Similar to npx but for locally installed packages.

use anyhow::{bail, Context, Result};
use std::process::{Command, Stdio};

/// Execute a command with node_modules/.bin in PATH
pub async fn run(command: &str, args: &[String], verbose: bool) -> Result<()> {
    let cwd = std::env::current_dir()?;

    // Build PATH with node_modules/.bin
    let mut path = std::env::var("PATH").unwrap_or_default();
    let bin_path = cwd.join("node_modules").join(".bin");

    if !bin_path.exists() && verbose {
        println!("⚠️  node_modules/.bin not found, running command without local binaries");
    }

    #[cfg(windows)]
    {
        path = format!("{};{}", bin_path.display(), path);
    }
    #[cfg(not(windows))]
    {
        path = format!("{}:{}", bin_path.display(), path);
    }

    if verbose {
        println!("🔧 Executing: {} {}", command, args.join(" "));
        println!("   PATH includes: {}", bin_path.display());
    }

    // Execute command
    let status = Command::new(command)
        .args(args)
        .current_dir(&cwd)
        .env("PATH", &path)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .context(format!("Failed to execute command: {}", command))?;

    if !status.success() {
        bail!("Command '{}' exited with code {}", command, status.code().unwrap_or(-1));
    }

    Ok(())
}

/// List available binaries in node_modules/.bin
#[allow(dead_code)]
pub async fn list_binaries(verbose: bool) -> Result<()> {
    let cwd = std::env::current_dir()?;
    let bin_path = cwd.join("node_modules").join(".bin");

    if !bin_path.exists() {
        println!("No binaries found (node_modules/.bin does not exist)");
        println!("Run 'dx install' first to install dependencies.");
        return Ok(());
    }

    println!("📦 Available binaries:");

    let entries = std::fs::read_dir(&bin_path)?;
    let mut binaries: Vec<String> = Vec::new();

    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();

        // Skip .cmd files on Windows (they're duplicates)
        #[cfg(windows)]
        if name.ends_with(".cmd") || name.ends_with(".ps1") {
            continue;
        }

        binaries.push(name);
    }

    binaries.sort();

    for binary in binaries {
        if verbose {
            let full_path = bin_path.join(&binary);
            println!("  {} → {}", binary, full_path.display());
        } else {
            println!("  {}", binary);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_list_binaries_no_node_modules() {
        // This test just verifies the function doesn't panic
        let temp = tempdir().unwrap();
        std::env::set_current_dir(temp.path()).unwrap();

        // Should not panic, just print a message
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(list_binaries(false));
        assert!(result.is_ok());
    }
}
