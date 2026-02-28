//! System information collection

use serde::{Deserialize, Serialize};
use std::process::Command;
use sysinfo::System;

/// System information recorded with benchmark results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemInfo {
    pub os: String,
    pub os_version: String,
    pub cpu_model: String,
    pub cpu_cores: usize,
    pub memory_gb: f64,
    pub python_version: String,
    pub dxpy_version: String,
    pub uv_version: Option<String>,
    pub pytest_version: Option<String>,
}

impl Default for SystemInfo {
    fn default() -> Self {
        Self {
            os: String::new(),
            os_version: String::new(),
            cpu_model: String::new(),
            cpu_cores: 0,
            memory_gb: 0.0,
            python_version: String::new(),
            dxpy_version: String::new(),
            uv_version: None,
            pytest_version: None,
        }
    }
}

impl SystemInfo {
    /// Collect system information from the current environment
    pub fn collect() -> Self {
        let mut sys = System::new_all();
        sys.refresh_all();

        let os = System::name().unwrap_or_else(|| "Unknown".to_string());
        let os_version = System::os_version().unwrap_or_else(|| "Unknown".to_string());
        let cpu_model = sys
            .cpus()
            .first()
            .map(|cpu| cpu.brand().to_string())
            .unwrap_or_else(|| "Unknown".to_string());
        let cpu_cores = sys.cpus().len();
        let memory_gb = sys.total_memory() as f64 / (1024.0 * 1024.0 * 1024.0);

        let python_version = Self::get_version("python", &["--version"]);
        let dxpy_version = Self::get_version("dx-py", &["--version"]);
        let uv_version = Self::get_optional_version("uv", &["--version"]);
        let pytest_version = Self::get_optional_version("pytest", &["--version"]);

        Self {
            os,
            os_version,
            cpu_model,
            cpu_cores,
            memory_gb,
            python_version,
            dxpy_version,
            uv_version,
            pytest_version,
        }
    }

    /// Get version string from a command, returning "Unknown" if it fails
    fn get_version(cmd: &str, args: &[&str]) -> String {
        Command::new(cmd)
            .args(args)
            .output()
            .ok()
            .and_then(|output| {
                if output.status.success() {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    // Some tools output to stderr (like Python)
                    let version_str = if stdout.trim().is_empty() {
                        stderr.trim().to_string()
                    } else {
                        stdout.trim().to_string()
                    };
                    Self::parse_version(&version_str)
                } else {
                    None
                }
            })
            .unwrap_or_else(|| "Unknown".to_string())
    }

    /// Get optional version string from a command
    fn get_optional_version(cmd: &str, args: &[&str]) -> Option<String> {
        Command::new(cmd).args(args).output().ok().and_then(|output| {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);
                let version_str = if stdout.trim().is_empty() {
                    stderr.trim().to_string()
                } else {
                    stdout.trim().to_string()
                };
                Self::parse_version(&version_str)
            } else {
                None
            }
        })
    }

    /// Parse version string from command output
    fn parse_version(output: &str) -> Option<String> {
        // Handle various version output formats:
        // "Python 3.11.0" -> "3.11.0"
        // "uv 0.1.0" -> "0.1.0"
        // "pytest 7.4.0" -> "7.4.0"
        let trimmed = output.trim();
        if trimmed.is_empty() {
            return None;
        }

        // Try to extract version number
        let parts: Vec<&str> = trimmed.split_whitespace().collect();
        if parts.len() >= 2 {
            // Return the version part (usually second word)
            Some(parts[1].to_string())
        } else {
            // Return the whole string if it's just a version
            Some(trimmed.to_string())
        }
    }

    /// Check if all required fields are populated (non-empty)
    pub fn is_complete(&self) -> bool {
        !self.os.is_empty()
            && !self.os_version.is_empty()
            && !self.cpu_model.is_empty()
            && self.cpu_cores > 0
            && self.memory_gb > 0.0
            && !self.python_version.is_empty()
    }

    /// Get a list of missing required fields
    pub fn missing_fields(&self) -> Vec<&'static str> {
        let mut missing = Vec::new();
        if self.os.is_empty() {
            missing.push("os");
        }
        if self.os_version.is_empty() {
            missing.push("os_version");
        }
        if self.cpu_model.is_empty() {
            missing.push("cpu_model");
        }
        if self.cpu_cores == 0 {
            missing.push("cpu_cores");
        }
        if self.memory_gb <= 0.0 {
            missing.push("memory_gb");
        }
        if self.python_version.is_empty() {
            missing.push("python_version");
        }
        missing
    }
}
