//! Bun installer and version manager

use anyhow::{Context, Result};
use std::process::Command;

/// Check if Bun is installed and get version
pub fn check_bun() -> Result<String> {
    let output = Command::new("bun")
        .arg("--version")
        .output()
        .context("Failed to execute bun --version")?;

    if !output.status.success() {
        anyhow::bail!("Bun is not installed or not in PATH");
    }

    let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Ok(version)
}

/// Install Bun (platform-specific)
pub async fn install_bun() -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        install_bun_windows().await
    }

    #[cfg(target_os = "macos")]
    {
        install_bun_unix().await
    }

    #[cfg(target_os = "linux")]
    {
        install_bun_unix().await
    }
}

#[cfg(unix)]
async fn install_bun_unix() -> Result<()> {
    use tokio::process::Command as TokioCommand;

    println!("Installing Bun...");

    let status = TokioCommand::new("curl")
        .args(["-fsSL", "https://bun.sh/install"])
        .stdout(std::process::Stdio::piped())
        .spawn()?
        .wait()
        .await?;

    if !status.success() {
        anyhow::bail!("Failed to install Bun");
    }

    println!("Bun installed successfully!");
    Ok(())
}

#[cfg(target_os = "windows")]
async fn install_bun_windows() -> Result<()> {
    use tokio::process::Command as TokioCommand;

    println!("Installing Bun via PowerShell...");

    let status = TokioCommand::new("powershell")
        .args(["-c", "irm bun.sh/install.ps1 | iex"])
        .spawn()?
        .wait()
        .await?;

    if !status.success() {
        anyhow::bail!("Failed to install Bun");
    }

    println!("Bun installed successfully!");
    Ok(())
}

/// Ensure Bun is installed, install if missing
pub async fn ensure_bun() -> Result<String> {
    match check_bun() {
        Ok(version) => {
            println!("Bun {} detected", version);
            Ok(version)
        }
        Err(_) => {
            println!("Bun not found. Installing...");
            install_bun().await?;
            check_bun()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_check_bun() {
        let result = check_bun();
        println!("Bun check result: {:?}", result);
    }

    #[tokio::test]
    async fn test_ensure_bun() {
        let result = ensure_bun().await;
        assert!(result.is_ok());
    }
}
