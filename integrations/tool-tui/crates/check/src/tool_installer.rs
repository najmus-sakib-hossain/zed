//! Automatic tool installation for external formatters and linters
//!
//! Downloads and installs required tools automatically when not found.

use std::path::PathBuf;
use std::process::Command;

/// Tool that can be automatically installed
#[derive(Debug, Clone)]
pub struct Tool {
    pub name: &'static str,
    pub check_command: &'static str,
    pub install_method: InstallMethod,
}

#[derive(Debug, Clone)]
pub enum InstallMethod {
    /// Install via cargo
    Cargo { package: &'static str },
    /// Install via pip/pipx
    Pip { package: &'static str },
    /// Download binary from URL
    Binary {
        url: &'static str,
        extract_path: &'static str,
    },
    /// System package manager
    System { instructions: &'static str },
}

impl Tool {
    /// Check if tool is installed
    pub fn is_installed(&self) -> bool {
        Command::new(self.check_command)
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    /// Install the tool automatically
    pub fn install(&self) -> Result<(), String> {
        tracing::info!("Installing {} automatically...", self.name);

        match &self.install_method {
            InstallMethod::Cargo { package } => self.install_via_cargo(package),
            InstallMethod::Pip { package } => self.install_via_pip(package),
            InstallMethod::Binary { url, extract_path } => self.install_binary(url, extract_path),
            InstallMethod::System { instructions } => {
                Err(format!("Please install {} manually:\n{}", self.name, instructions))
            }
        }
    }

    fn install_via_cargo(&self, package: &str) -> Result<(), String> {
        let output = Command::new("cargo")
            .args(["install", package])
            .output()
            .map_err(|e| format!("Failed to run cargo: {}", e))?;

        if output.status.success() {
            tracing::info!("Successfully installed {} via cargo", self.name);
            Ok(())
        } else {
            Err(format!(
                "Failed to install {}: {}",
                self.name,
                String::from_utf8_lossy(&output.stderr)
            ))
        }
    }

    fn install_via_pip(&self, package: &str) -> Result<(), String> {
        // Try pipx first (better for CLI tools), fall back to pip
        let pipx_result = Command::new("pipx").args(["install", package]).output();

        if let Ok(output) = pipx_result {
            if output.status.success() {
                tracing::info!("Successfully installed {} via pipx", self.name);
                return Ok(());
            }
        }

        // Fall back to pip
        let output = Command::new("pip")
            .args(["install", "--user", package])
            .output()
            .map_err(|e| format!("Failed to run pip: {}", e))?;

        if output.status.success() {
            tracing::info!("Successfully installed {} via pip", self.name);
            Ok(())
        } else {
            Err(format!(
                "Failed to install {}: {}",
                self.name,
                String::from_utf8_lossy(&output.stderr)
            ))
        }
    }

    fn install_binary(&self, url: &str, _extract_path: &str) -> Result<(), String> {
        // Download binary
        tracing::info!("Downloading {} from {}...", self.name, url);

        let response = ureq::get(url).call().map_err(|e| format!("Failed to download: {}", e))?;

        let mut bytes = Vec::new();
        response
            .into_reader()
            .read_to_end(&mut bytes)
            .map_err(|e| format!("Failed to read response: {}", e))?;

        // Determine install location
        let install_dir = get_tool_install_dir()?;
        std::fs::create_dir_all(&install_dir)
            .map_err(|e| format!("Failed to create install dir: {}", e))?;

        let binary_path = install_dir.join(self.check_command);

        // Write binary
        std::fs::write(&binary_path, bytes)
            .map_err(|e| format!("Failed to write binary: {}", e))?;

        // Make executable on Unix
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&binary_path)
                .map_err(|e| format!("Failed to get metadata: {}", e))?
                .permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(&binary_path, perms)
                .map_err(|e| format!("Failed to set permissions: {}", e))?;
        }

        tracing::info!("Successfully installed {} to {:?}", self.name, binary_path);
        Ok(())
    }
}

/// Get the directory where tools should be installed
fn get_tool_install_dir() -> Result<PathBuf, String> {
    if let Ok(home) = std::env::var("HOME") {
        Ok(PathBuf::from(home).join(".dx").join("bin"))
    } else if let Ok(userprofile) = std::env::var("USERPROFILE") {
        Ok(PathBuf::from(userprofile).join(".dx").join("bin"))
    } else {
        Err("Could not determine home directory".to_string())
    }
}

/// Registry of all supported tools
pub struct ToolRegistry {
    tools: Vec<Tool>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: vec![
                Tool {
                    name: "ruff",
                    check_command: "ruff",
                    install_method: InstallMethod::Pip { package: "ruff" },
                },
                Tool {
                    name: "black",
                    check_command: "black",
                    install_method: InstallMethod::Pip { package: "black" },
                },
                Tool {
                    name: "rustfmt",
                    check_command: "rustfmt",
                    install_method: InstallMethod::System {
                        instructions: "Install Rust toolchain from https://rustup.rs/\nThen run: rustup component add rustfmt",
                    },
                },
                Tool {
                    name: "clippy",
                    check_command: "cargo",
                    install_method: InstallMethod::System {
                        instructions: "Install Rust toolchain from https://rustup.rs/\nThen run: rustup component add clippy",
                    },
                },
                Tool {
                    name: "gofmt",
                    check_command: "gofmt",
                    install_method: InstallMethod::System {
                        instructions: "Install Go from https://go.dev/dl/",
                    },
                },
            ],
        }
    }

    /// Get tool by name
    pub fn get(&self, name: &str) -> Option<&Tool> {
        self.tools.iter().find(|t| t.name == name)
    }

    /// Ensure tool is installed, installing if necessary
    pub fn ensure_installed(&self, name: &str) -> Result<(), String> {
        let tool = self.get(name).ok_or_else(|| format!("Unknown tool: {}", name))?;

        if tool.is_installed() {
            tracing::debug!("{} is already installed", name);
            Ok(())
        } else {
            tracing::info!("{} not found, attempting automatic installation...", name);
            tool.install()
        }
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_registry() {
        let registry = ToolRegistry::new();
        assert!(registry.get("ruff").is_some());
        assert!(registry.get("black").is_some());
        assert!(registry.get("rustfmt").is_some());
        assert!(registry.get("unknown").is_none());
    }

    #[test]
    fn test_get_install_dir() {
        let dir = get_tool_install_dir();
        assert!(dir.is_ok());
    }
}
