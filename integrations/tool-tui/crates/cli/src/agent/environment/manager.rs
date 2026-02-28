//! Environment Manager
//!
//! Handles runtime detection, installation, and health verification.

use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;
use tokio::sync::RwLock;

use super::{
    EnvironmentConfig, EnvironmentError, EnvironmentResult, EnvironmentsSchema, Runtime,
    RuntimeEntry,
};

/// Status of a runtime installation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeStatus {
    /// Runtime is installed and healthy
    Available,
    /// Runtime is installed but needs update
    NeedsUpdate,
    /// Runtime is not installed
    NotInstalled,
    /// Runtime is installed but unhealthy
    Unhealthy,
    /// Currently installing
    Installing,
}

/// Information about an installed runtime
#[derive(Debug, Clone)]
pub struct RuntimeInfo {
    /// The runtime type
    pub runtime: Runtime,
    /// Current status
    pub status: RuntimeStatus,
    /// Installed version (if available)
    pub version: Option<String>,
    /// Installation path
    pub path: Option<PathBuf>,
    /// Compiler version (if available)
    pub compiler_version: Option<String>,
    /// Last health check timestamp
    pub last_check: u64,
}

/// Progress callback for installation/verification
pub type ProgressCallback = Box<dyn Fn(f32, &str) + Send + Sync>;

/// Manages runtime environments
pub struct EnvironmentManager {
    config: EnvironmentConfig,
    runtimes: RwLock<HashMap<Runtime, RuntimeInfo>>,
    schema: RwLock<Option<EnvironmentsSchema>>,
}

impl EnvironmentManager {
    /// Create a new environment manager
    pub fn new(config: EnvironmentConfig) -> Self {
        Self {
            config,
            runtimes: RwLock::new(HashMap::new()),
            schema: RwLock::new(None),
        }
    }

    /// Create with default configuration
    pub fn with_defaults() -> Self {
        Self::new(EnvironmentConfig::default())
    }

    /// Initialize the manager and scan for runtimes
    pub async fn initialize(&self) -> EnvironmentResult<()> {
        // Ensure directories exist
        tokio::fs::create_dir_all(&self.config.cache_dir).await?;
        tokio::fs::create_dir_all(&self.config.runtimes_dir).await?;

        // Load existing schema if present
        self.load_schema().await?;

        // Scan for available runtimes
        self.scan_runtimes().await?;

        Ok(())
    }

    /// Scan system for installed runtimes
    pub async fn scan_runtimes(&self) -> EnvironmentResult<()> {
        let mut runtimes = self.runtimes.write().await;

        for runtime in Runtime::all() {
            let info = self.detect_runtime(*runtime).await;
            runtimes.insert(*runtime, info);
        }

        Ok(())
    }

    /// Detect a specific runtime
    async fn detect_runtime(&self, runtime: Runtime) -> RuntimeInfo {
        let (cmd, version_arg) = match runtime {
            Runtime::NodeJs => ("node", "--version"),
            Runtime::Python => ("python3", "--version"),
            Runtime::Go => ("go", "version"),
            Runtime::Rust => ("rustc", "--version"),
            Runtime::Deno => ("deno", "--version"),
            Runtime::Bun => ("bun", "--version"),
        };

        // Try to run the version command
        let version_result = Command::new(cmd).arg(version_arg).output();

        let (status, version, path) = match version_result {
            Ok(output) if output.status.success() => {
                let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
                let path = Self::find_executable(cmd);
                (RuntimeStatus::Available, Some(version), path)
            }
            _ => (RuntimeStatus::NotInstalled, None, None),
        };

        // Check compiler availability
        let compiler_version = self.check_compiler(runtime).await;

        RuntimeInfo {
            runtime,
            status,
            version,
            path,
            compiler_version,
            last_check: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        }
    }

    /// Check if the compiler for a runtime is available
    async fn check_compiler(&self, runtime: Runtime) -> Option<String> {
        let (cmd, args): (&str, &[&str]) = match runtime {
            Runtime::NodeJs | Runtime::Bun => ("javy", &["--version"]),
            Runtime::Python => ("componentize-py", &["--version"]),
            Runtime::Go => ("tinygo", &["version"]),
            Runtime::Rust => ("cargo", &["component", "--version"]),
            Runtime::Deno => ("deno", &["--version"]),
        };

        let result = Command::new(cmd).args(args).output();

        match result {
            Ok(output) if output.status.success() => {
                Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
            }
            _ => None,
        }
    }

    /// Find executable path
    fn find_executable(name: &str) -> Option<PathBuf> {
        #[cfg(target_os = "windows")]
        let which_cmd = "where";
        #[cfg(not(target_os = "windows"))]
        let which_cmd = "which";

        Command::new(which_cmd)
            .arg(name)
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| {
                PathBuf::from(
                    String::from_utf8_lossy(&o.stdout).lines().next().unwrap_or("").trim(),
                )
            })
    }

    /// Get status of a specific runtime
    pub async fn get_status(&self, runtime: Runtime) -> RuntimeInfo {
        let runtimes = self.runtimes.read().await;
        runtimes.get(&runtime).cloned().unwrap_or(RuntimeInfo {
            runtime,
            status: RuntimeStatus::NotInstalled,
            version: None,
            path: None,
            compiler_version: None,
            last_check: 0,
        })
    }

    /// Get all runtime statuses
    pub async fn get_all_status(&self) -> Vec<RuntimeInfo> {
        let runtimes = self.runtimes.read().await;
        Runtime::all()
            .iter()
            .map(|r| {
                runtimes.get(r).cloned().unwrap_or(RuntimeInfo {
                    runtime: *r,
                    status: RuntimeStatus::NotInstalled,
                    version: None,
                    path: None,
                    compiler_version: None,
                    last_check: 0,
                })
            })
            .collect()
    }

    /// Install a runtime and its compiler
    pub async fn install_runtime(
        &self,
        runtime: Runtime,
        progress: Option<ProgressCallback>,
    ) -> EnvironmentResult<()> {
        let report = |pct: f32, msg: &str| {
            if let Some(ref cb) = progress {
                cb(pct, msg);
            }
        };

        report(0.0, &format!("Installing {} runtime...", runtime));

        // Update status to installing
        {
            let mut runtimes = self.runtimes.write().await;
            if let Some(info) = runtimes.get_mut(&runtime) {
                info.status = RuntimeStatus::Installing;
            }
        }

        // Platform-specific installation
        let result = match runtime {
            Runtime::NodeJs => self.install_nodejs(progress.as_ref()).await,
            Runtime::Python => self.install_python(progress.as_ref()).await,
            Runtime::Go => self.install_go(progress.as_ref()).await,
            Runtime::Rust => self.install_rust(progress.as_ref()).await,
            Runtime::Deno => self.install_deno(progress.as_ref()).await,
            Runtime::Bun => self.install_bun(progress.as_ref()).await,
        };

        // Re-scan after installation
        let info = self.detect_runtime(runtime).await;
        {
            let mut runtimes = self.runtimes.write().await;
            runtimes.insert(runtime, info);
        }

        report(1.0, &format!("{} installation complete", runtime));

        result
    }

    async fn install_nodejs(&self, progress: Option<&ProgressCallback>) -> EnvironmentResult<()> {
        let report = |pct: f32, msg: &str| {
            if let Some(cb) = progress {
                cb(pct, msg);
            }
        };

        report(0.1, "Checking for Node.js...");

        // On Windows, suggest using winget or direct download
        #[cfg(target_os = "windows")]
        {
            report(0.2, "Installing via winget...");
            let output = Command::new("winget")
                .args(["install", "-e", "--id", "OpenJS.NodeJS.LTS"])
                .output()?;

            if !output.status.success() {
                return Err(EnvironmentError::InstallationFailed {
                    reason: "winget install failed. Please install Node.js manually.".into(),
                });
            }
        }

        #[cfg(target_os = "macos")]
        {
            report(0.2, "Installing via Homebrew...");
            let output = Command::new("brew").args(["install", "node"]).output()?;

            if !output.status.success() {
                return Err(EnvironmentError::InstallationFailed {
                    reason: "brew install failed".into(),
                });
            }
        }

        #[cfg(target_os = "linux")]
        {
            report(0.2, "Installing via package manager...");
            // Try apt first
            let output =
                Command::new("sudo").args(["apt", "install", "-y", "nodejs", "npm"]).output()?;

            if !output.status.success() {
                return Err(EnvironmentError::InstallationFailed {
                    reason: "apt install failed".into(),
                });
            }
        }

        // Install javy compiler
        report(0.6, "Installing javy compiler...");
        self.install_javy().await?;

        Ok(())
    }

    async fn install_javy(&self) -> EnvironmentResult<()> {
        // Install javy via cargo
        let output = Command::new("cargo").args(["install", "javy-cli"]).output()?;

        if !output.status.success() {
            return Err(EnvironmentError::InstallationFailed {
                reason: "Failed to install javy-cli".into(),
            });
        }

        Ok(())
    }

    async fn install_python(&self, _progress: Option<&ProgressCallback>) -> EnvironmentResult<()> {
        #[cfg(target_os = "windows")]
        {
            let output = Command::new("winget")
                .args(["install", "-e", "--id", "Python.Python.3.12"])
                .output()?;

            if !output.status.success() {
                return Err(EnvironmentError::InstallationFailed {
                    reason: "winget install failed".into(),
                });
            }
        }

        #[cfg(target_os = "macos")]
        {
            let output = Command::new("brew").args(["install", "python@3.12"]).output()?;

            if !output.status.success() {
                return Err(EnvironmentError::InstallationFailed {
                    reason: "brew install failed".into(),
                });
            }
        }

        // Install componentize-py
        let output = Command::new("pip3").args(["install", "componentize-py"]).output()?;

        if !output.status.success() {
            return Err(EnvironmentError::InstallationFailed {
                reason: "pip install componentize-py failed".into(),
            });
        }

        Ok(())
    }

    async fn install_go(&self, _progress: Option<&ProgressCallback>) -> EnvironmentResult<()> {
        #[cfg(target_os = "windows")]
        {
            let output =
                Command::new("winget").args(["install", "-e", "--id", "GoLang.Go"]).output()?;

            if !output.status.success() {
                return Err(EnvironmentError::InstallationFailed {
                    reason: "winget install failed".into(),
                });
            }
        }

        #[cfg(target_os = "macos")]
        {
            let output = Command::new("brew").args(["install", "go", "tinygo"]).output()?;

            if !output.status.success() {
                return Err(EnvironmentError::InstallationFailed {
                    reason: "brew install failed".into(),
                });
            }
        }

        Ok(())
    }

    async fn install_rust(&self, _progress: Option<&ProgressCallback>) -> EnvironmentResult<()> {
        // Check if rustup exists
        let rustup_check = Command::new("rustup").arg("--version").output();

        if rustup_check.is_err() || !rustup_check.unwrap().status.success() {
            return Err(EnvironmentError::InstallationFailed {
                reason: "Please install Rust via rustup: https://rustup.rs".into(),
            });
        }

        // Install cargo-component
        let output = Command::new("cargo").args(["install", "cargo-component"]).output()?;

        if !output.status.success() {
            return Err(EnvironmentError::InstallationFailed {
                reason: "Failed to install cargo-component".into(),
            });
        }

        Ok(())
    }

    async fn install_deno(&self, _progress: Option<&ProgressCallback>) -> EnvironmentResult<()> {
        #[cfg(target_os = "windows")]
        {
            let output = Command::new("powershell")
                .args(["-Command", "irm https://deno.land/install.ps1 | iex"])
                .output()?;

            if !output.status.success() {
                return Err(EnvironmentError::InstallationFailed {
                    reason: "Deno installation failed".into(),
                });
            }
        }

        #[cfg(not(target_os = "windows"))]
        {
            let output = Command::new("sh")
                .args(["-c", "curl -fsSL https://deno.land/install.sh | sh"])
                .output()?;

            if !output.status.success() {
                return Err(EnvironmentError::InstallationFailed {
                    reason: "Deno installation failed".into(),
                });
            }
        }

        Ok(())
    }

    async fn install_bun(&self, _progress: Option<&ProgressCallback>) -> EnvironmentResult<()> {
        #[cfg(target_os = "windows")]
        {
            let output = Command::new("powershell")
                .args(["-Command", "irm bun.sh/install.ps1 | iex"])
                .output()?;

            if !output.status.success() {
                return Err(EnvironmentError::InstallationFailed {
                    reason: "Bun installation failed".into(),
                });
            }
        }

        #[cfg(not(target_os = "windows"))]
        {
            let output = Command::new("sh")
                .args(["-c", "curl -fsSL https://bun.sh/install | bash"])
                .output()?;

            if !output.status.success() {
                return Err(EnvironmentError::InstallationFailed {
                    reason: "Bun installation failed".into(),
                });
            }
        }

        Ok(())
    }

    /// Verify runtime health
    pub async fn verify_runtime(&self, runtime: Runtime) -> EnvironmentResult<bool> {
        let info = self.get_status(runtime).await;

        if info.status == RuntimeStatus::NotInstalled {
            return Ok(false);
        }

        // Run a simple test
        let healthy = match runtime {
            Runtime::NodeJs => Command::new("node")
                .args(["-e", "console.log('ok')"])
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false),
            Runtime::Python => Command::new("python3")
                .args(["-c", "print('ok')"])
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false),
            Runtime::Go => Command::new("go")
                .arg("version")
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false),
            Runtime::Rust => Command::new("rustc")
                .arg("--version")
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false),
            Runtime::Deno => Command::new("deno")
                .args(["eval", "console.log('ok')"])
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false),
            Runtime::Bun => Command::new("bun")
                .args(["--version"])
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false),
        };

        // Update status
        {
            let mut runtimes = self.runtimes.write().await;
            if let Some(info) = runtimes.get_mut(&runtime) {
                info.status = if healthy {
                    RuntimeStatus::Available
                } else {
                    RuntimeStatus::Unhealthy
                };
            }
        }

        Ok(healthy)
    }

    /// Load environments.sr schema
    async fn load_schema(&self) -> EnvironmentResult<()> {
        let schema_path = self.config.dx_root.join("environments.sr");

        if !schema_path.exists() {
            return Ok(());
        }

        // TODO: Implement .sr parsing when dx-serializer is integrated
        // For now, we'll just scan fresh each time

        Ok(())
    }

    /// Save environments.sr schema
    pub async fn save_schema(&self) -> EnvironmentResult<()> {
        let runtimes = self.runtimes.read().await;
        let schema_path = self.config.dx_root.join("environments.sr");

        let entries: Vec<RuntimeEntry> = runtimes
            .values()
            .filter(|info| info.status == RuntimeStatus::Available)
            .map(|info| RuntimeEntry {
                runtime: info.runtime,
                version: info.version.clone().unwrap_or_default(),
                path: info.path.clone().unwrap_or_default(),
                last_verified: info.last_check,
                healthy: info.status == RuntimeStatus::Available,
            })
            .collect();

        // TODO: Serialize to .sr format
        // For now, write as JSON placeholder
        let json = serde_json::to_string_pretty(
            &entries
                .iter()
                .map(|e| {
                    serde_json::json!({
                        "runtime": format!("{}", e.runtime),
                        "version": e.version,
                        "path": e.path,
                        "healthy": e.healthy,
                    })
                })
                .collect::<Vec<_>>(),
        )
        .unwrap_or_default();

        tokio::fs::write(&schema_path, json).await?;

        Ok(())
    }

    /// Get configuration
    pub fn config(&self) -> &EnvironmentConfig {
        &self.config
    }

    /// Get runtime executable path
    pub fn get_runtime_path(&self, runtime: Runtime) -> PathBuf {
        #[cfg(target_os = "windows")]
        let ext = ".exe";
        #[cfg(not(target_os = "windows"))]
        let ext = "";

        match runtime {
            Runtime::NodeJs => PathBuf::from(format!("node{}", ext)),
            Runtime::Python => PathBuf::from(format!("python3{}", ext)),
            Runtime::Go => PathBuf::from(format!("go{}", ext)),
            Runtime::Rust => PathBuf::from(format!("rustc{}", ext)),
            Runtime::Deno => PathBuf::from(format!("deno{}", ext)),
            Runtime::Bun => PathBuf::from(format!("bun{}", ext)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_manager_creation() {
        let manager = EnvironmentManager::with_defaults();
        let all_status = manager.get_all_status().await;
        assert_eq!(all_status.len(), 6);
    }

    #[tokio::test]
    async fn test_runtime_detection() {
        let manager = EnvironmentManager::with_defaults();
        // This will actually detect if Node is installed on the test machine
        let node_status = manager.get_status(Runtime::NodeJs).await;
        // Just verify we get a result, don't assume it's installed
        assert!(matches!(
            node_status.status,
            RuntimeStatus::Available | RuntimeStatus::NotInstalled
        ));
    }
}
