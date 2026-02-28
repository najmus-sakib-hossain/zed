//! Global Tool Manager
//!
//! Manages globally installed Python tools in isolated virtual environments.
//! Similar to pipx functionality.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::{Error, Result};

/// Tool Manager for global Python tools
///
/// Each tool is installed in its own isolated virtual environment
/// with wrapper scripts in a shared bin directory.
pub struct ToolManager {
    /// Base directory for tool installations
    tools_dir: PathBuf,
    /// Bin directory for wrapper scripts
    bin_dir: PathBuf,
    /// Python interpreter to use
    python: PathBuf,
}

impl ToolManager {
    /// Create a new tool manager with default directories
    pub fn new() -> Result<Self> {
        let base_dir =
            dirs::data_local_dir().unwrap_or_else(|| PathBuf::from(".local")).join("dx-py");

        Self::with_dirs(base_dir.join("tools"), base_dir.join("bin"))
    }

    /// Create a new tool manager with custom directories
    pub fn with_dirs(tools_dir: PathBuf, bin_dir: PathBuf) -> Result<Self> {
        // Find Python interpreter
        let python = Self::find_python()?;

        Ok(Self {
            tools_dir,
            bin_dir,
            python,
        })
    }

    /// Find a Python interpreter
    fn find_python() -> Result<PathBuf> {
        let candidates = if cfg!(windows) {
            vec!["python.exe", "python3.exe", "py.exe"]
        } else {
            vec!["python3", "python"]
        };

        for candidate in candidates {
            if let Ok(output) = Command::new(candidate)
                .arg("--version")
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
            {
                if output.success() {
                    return Ok(PathBuf::from(candidate));
                }
            }
        }

        Err(Error::Cache(
            "No Python interpreter found. Please install Python 3.8+".to_string(),
        ))
    }

    /// Get the tools directory
    pub fn tools_dir(&self) -> &Path {
        &self.tools_dir
    }

    /// Get the bin directory
    pub fn bin_dir(&self) -> &Path {
        &self.bin_dir
    }

    /// Check if a tool is installed
    pub fn is_installed(&self, name: &str) -> bool {
        self.tool_dir(name).exists()
    }

    /// Get the directory for a specific tool
    pub fn tool_dir(&self, name: &str) -> PathBuf {
        self.tools_dir.join(name)
    }

    /// Install a tool globally
    pub fn install(&self, name: &str) -> Result<InstalledTool> {
        let tool_dir = self.tool_dir(name);

        if tool_dir.exists() {
            return Err(Error::Cache(format!(
                "Tool '{}' is already installed. Uninstall first.",
                name
            )));
        }

        // Create directories
        std::fs::create_dir_all(&self.tools_dir)?;
        std::fs::create_dir_all(&self.bin_dir)?;

        // Create isolated virtual environment
        let venv_path = tool_dir.join("venv");
        let status = Command::new(&self.python)
            .args(["-m", "venv", &venv_path.to_string_lossy()])
            .status()
            .map_err(|e| Error::Cache(format!("Failed to create venv: {}", e)))?;

        if !status.success() {
            return Err(Error::Cache(format!(
                "Failed to create virtual environment for tool '{}'",
                name
            )));
        }

        // Get pip path in venv
        let pip = if cfg!(windows) {
            venv_path.join("Scripts").join("pip.exe")
        } else {
            venv_path.join("bin").join("pip")
        };

        // Install the tool package
        let status = Command::new(&pip)
            .args(["install", "--quiet", name])
            .status()
            .map_err(|e| Error::Cache(format!("Failed to install {}: {}", name, e)))?;

        if !status.success() {
            // Clean up on failure
            let _ = std::fs::remove_dir_all(&tool_dir);
            return Err(Error::Cache(format!("Failed to install package '{}'", name)));
        }

        // Create wrapper scripts
        let scripts = self.create_wrapper_scripts(name, &venv_path)?;

        Ok(InstalledTool {
            name: name.to_string(),
            tool_dir,
            venv_path,
            scripts,
        })
    }

    /// Create wrapper scripts for a tool
    fn create_wrapper_scripts(&self, name: &str, venv_path: &Path) -> Result<Vec<PathBuf>> {
        let mut scripts = Vec::new();

        // Find executables in the venv's bin/Scripts directory
        let venv_bin = if cfg!(windows) {
            venv_path.join("Scripts")
        } else {
            venv_path.join("bin")
        };

        // The main tool should have an executable with the same name
        let tool_exe = if cfg!(windows) {
            venv_bin.join(format!("{}.exe", name))
        } else {
            venv_bin.join(name)
        };

        if tool_exe.exists() {
            let wrapper = self.create_wrapper(name, &tool_exe)?;
            scripts.push(wrapper);
        }

        Ok(scripts)
    }

    /// Create a wrapper script for an executable
    fn create_wrapper(&self, name: &str, exe_path: &Path) -> Result<PathBuf> {
        let wrapper_path = if cfg!(windows) {
            self.bin_dir.join(format!("{}.cmd", name))
        } else {
            self.bin_dir.join(name)
        };

        let content = if cfg!(windows) {
            format!("@echo off\r\n\"{}\" %*\r\n", exe_path.display())
        } else {
            format!("#!/bin/sh\nexec \"{}\" \"$@\"\n", exe_path.display())
        };

        std::fs::write(&wrapper_path, &content)?;

        // Make executable on Unix
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&wrapper_path)?.permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(&wrapper_path, perms)?;
        }

        Ok(wrapper_path)
    }

    /// Uninstall a tool
    pub fn uninstall(&self, name: &str) -> Result<()> {
        let tool_dir = self.tool_dir(name);

        if !tool_dir.exists() {
            return Err(Error::Cache(format!("Tool '{}' is not installed", name)));
        }

        // Remove wrapper scripts
        let wrapper_path = if cfg!(windows) {
            self.bin_dir.join(format!("{}.cmd", name))
        } else {
            self.bin_dir.join(name)
        };

        if wrapper_path.exists() {
            std::fs::remove_file(&wrapper_path)?;
        }

        // Remove tool directory
        std::fs::remove_dir_all(&tool_dir)?;

        Ok(())
    }

    /// List installed tools
    pub fn list(&self) -> Result<Vec<String>> {
        if !self.tools_dir.exists() {
            return Ok(Vec::new());
        }

        let tools: Vec<String> = std::fs::read_dir(&self.tools_dir)?
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_dir())
            .filter_map(|e| e.file_name().to_str().map(|s| s.to_string()))
            .collect();

        Ok(tools)
    }

    /// Upgrade a tool to the latest version
    pub fn upgrade(&self, name: &str) -> Result<()> {
        let tool_dir = self.tool_dir(name);

        if !tool_dir.exists() {
            return Err(Error::Cache(format!("Tool '{}' is not installed", name)));
        }

        let venv_path = tool_dir.join("venv");
        let pip = if cfg!(windows) {
            venv_path.join("Scripts").join("pip.exe")
        } else {
            venv_path.join("bin").join("pip")
        };

        let status = Command::new(&pip)
            .args(["install", "--upgrade", "--quiet", name])
            .status()
            .map_err(|e| Error::Cache(format!("Failed to upgrade {}: {}", name, e)))?;

        if !status.success() {
            return Err(Error::Cache(format!("Failed to upgrade '{}'", name)));
        }

        Ok(())
    }

    /// Run a tool ephemerally (install, run, cleanup)
    pub fn run_ephemeral(&self, name: &str, args: &[String]) -> Result<i32> {
        // Create temporary directory
        let temp_dir = tempfile::tempdir()
            .map_err(|e| Error::Cache(format!("Failed to create temp dir: {}", e)))?;

        let venv_path = temp_dir.path().join("venv");

        // Create virtual environment
        let status = Command::new(&self.python)
            .args(["-m", "venv", &venv_path.to_string_lossy()])
            .status()
            .map_err(|e| Error::Cache(format!("Failed to create venv: {}", e)))?;

        if !status.success() {
            return Err(Error::Cache("Failed to create temporary venv".to_string()));
        }

        // Install the tool
        let pip = if cfg!(windows) {
            venv_path.join("Scripts").join("pip.exe")
        } else {
            venv_path.join("bin").join("pip")
        };

        let status = Command::new(&pip)
            .args(["install", "--quiet", name])
            .status()
            .map_err(|e| Error::Cache(format!("Failed to install {}: {}", name, e)))?;

        if !status.success() {
            return Err(Error::Cache(format!("Failed to install '{}'", name)));
        }

        // Run the tool
        let tool_exe = if cfg!(windows) {
            venv_path.join("Scripts").join(format!("{}.exe", name))
        } else {
            venv_path.join("bin").join(name)
        };

        let status = Command::new(&tool_exe)
            .args(args)
            .status()
            .map_err(|e| Error::Cache(format!("Failed to run {}: {}", name, e)))?;

        // Temp directory is automatically cleaned up when dropped
        Ok(status.code().unwrap_or(1))
    }
}

impl Default for ToolManager {
    fn default() -> Self {
        Self::new().expect("Failed to create ToolManager")
    }
}

/// Information about an installed tool
#[derive(Debug, Clone)]
pub struct InstalledTool {
    /// Tool name
    pub name: String,
    /// Tool installation directory
    pub tool_dir: PathBuf,
    /// Virtual environment path
    pub venv_path: PathBuf,
    /// Wrapper script paths
    pub scripts: Vec<PathBuf>,
}
