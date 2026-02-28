//! Python runtime detection
//!
//! Detects Python installations across the system.

use std::path::{Path, PathBuf};
use std::process::Command;

use super::{InstallationSource, PythonRuntime, PythonVersion, RuntimeCapabilities};
use crate::platform::Architecture;

/// Runtime detection errors
#[derive(Debug, thiserror::Error)]
pub enum DetectionError {
    /// No Python installation found
    #[error("No Python installation found")]
    NoPythonFound,
    /// Python version is not supported (requires 3.8-3.13)
    #[error("Python version {0} is not supported (requires 3.8-3.13)")]
    UnsupportedVersion(PythonVersion),
    /// Failed to execute Python
    #[error("Failed to execute Python: {0}")]
    ExecutionError(#[from] std::io::Error),
    /// Failed to parse Python output
    #[error("Failed to parse Python output: {0}")]
    ParseError(String),
}

/// Detection cache for performance
#[derive(Debug, Default)]
pub struct DetectionCache {
    #[allow(dead_code)]
    runtimes: Vec<PythonRuntime>,
}

/// Detects Python installations across the system
pub struct RuntimeDetector {
    search_paths: Vec<PathBuf>,
    cache: Option<DetectionCache>,
}

impl Default for RuntimeDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl RuntimeDetector {
    /// Create a new detector with default search paths
    pub fn new() -> Self {
        Self {
            search_paths: Self::default_search_paths(),
            cache: None,
        }
    }

    /// Get default search paths based on platform
    fn default_search_paths() -> Vec<PathBuf> {
        let mut paths = Vec::new();

        #[cfg(target_os = "windows")]
        {
            // Windows Store Python
            if let Some(local_app_data) = dirs::data_local_dir() {
                paths.push(local_app_data.join("Programs").join("Python"));
            }
            // Standard Windows paths
            paths.push(PathBuf::from("C:\\Python"));
            paths.push(PathBuf::from("C:\\Program Files\\Python"));
            paths.push(PathBuf::from("C:\\Program Files (x86)\\Python"));
            // pyenv-win
            if let Some(home) = dirs::home_dir() {
                paths.push(home.join(".pyenv").join("pyenv-win").join("versions"));
            }
        }

        #[cfg(target_os = "macos")]
        {
            // Homebrew paths
            paths.push(PathBuf::from("/opt/homebrew/bin"));
            paths.push(PathBuf::from("/usr/local/bin"));
            // System Python
            paths.push(PathBuf::from("/usr/bin"));
            // pyenv
            if let Some(home) = dirs::home_dir() {
                paths.push(home.join(".pyenv").join("versions"));
            }
        }

        #[cfg(target_os = "linux")]
        {
            // Standard Linux paths
            paths.push(PathBuf::from("/usr/bin"));
            paths.push(PathBuf::from("/usr/local/bin"));
            // pyenv
            if let Some(home) = dirs::home_dir() {
                paths.push(home.join(".pyenv").join("versions"));
            }
            // Conda
            if let Some(home) = dirs::home_dir() {
                paths.push(home.join("miniconda3").join("bin"));
                paths.push(home.join("anaconda3").join("bin"));
            }
        }

        // Common conda paths
        if let Some(home) = dirs::home_dir() {
            paths.push(home.join(".conda").join("envs"));
        }

        paths
    }

    /// Detect all Python installations
    pub fn detect_all(&self) -> Result<Vec<PythonRuntime>, DetectionError> {
        let mut runtimes = Vec::new();

        for path in &self.search_paths {
            if let Ok(found) = self.detect_in_path(path) {
                runtimes.extend(found);
            }
        }

        // Also check PATH environment variable
        if let Ok(path_runtimes) = self.detect_from_path_env() {
            for runtime in path_runtimes {
                if !runtimes.iter().any(|r| r.executable == runtime.executable) {
                    runtimes.push(runtime);
                }
            }
        }

        if runtimes.is_empty() {
            return Err(DetectionError::NoPythonFound);
        }

        // Sort by version (newest first)
        runtimes.sort_by(|a, b| b.version.cmp(&a.version));

        Ok(runtimes)
    }

    /// Detect Python installations in a specific path
    fn detect_in_path(&self, path: &Path) -> Result<Vec<PythonRuntime>, DetectionError> {
        let mut runtimes = Vec::new();

        if !path.exists() {
            return Ok(runtimes);
        }

        // Look for python executables
        let python_names = if cfg!(windows) {
            vec!["python.exe", "python3.exe"]
        } else {
            vec![
                "python",
                "python3",
                "python3.8",
                "python3.9",
                "python3.10",
                "python3.11",
                "python3.12",
                "python3.13",
            ]
        };

        for name in python_names {
            let exe_path = path.join(name);
            if exe_path.exists() {
                if let Ok(runtime) = self.detect_runtime(&exe_path) {
                    if runtime.version.is_supported() {
                        runtimes.push(runtime);
                    }
                }
            }
        }

        Ok(runtimes)
    }

    /// Detect Python from PATH environment variable
    fn detect_from_path_env(&self) -> Result<Vec<PythonRuntime>, DetectionError> {
        let mut runtimes = Vec::new();

        // Try common python commands
        let commands = if cfg!(windows) {
            vec!["python", "python3", "py"]
        } else {
            vec!["python3", "python"]
        };

        for cmd in commands {
            if let Ok(output) = Command::new(cmd).arg("--version").output() {
                if output.status.success() {
                    if let Ok(runtime) = self.detect_runtime_from_command(cmd) {
                        if runtime.version.is_supported()
                            && !runtimes
                                .iter()
                                .any(|r: &PythonRuntime| r.executable == runtime.executable)
                        {
                            runtimes.push(runtime);
                        }
                    }
                }
            }
        }

        Ok(runtimes)
    }

    /// Detect runtime from a Python executable path
    fn detect_runtime(&self, exe_path: &PathBuf) -> Result<PythonRuntime, DetectionError> {
        let version = self.get_version(exe_path)?;
        let source = self.determine_source(exe_path);
        let architecture = self.get_architecture(exe_path)?;
        let capabilities = self.get_capabilities(exe_path)?;

        Ok(PythonRuntime {
            executable: exe_path.clone(),
            version,
            architecture,
            source,
            capabilities,
        })
    }

    /// Detect runtime from a command name
    fn detect_runtime_from_command(&self, cmd: &str) -> Result<PythonRuntime, DetectionError> {
        // Get the actual path
        let exe_path = self.get_executable_path(cmd)?;
        self.detect_runtime(&exe_path)
    }

    /// Get the executable path for a command
    fn get_executable_path(&self, cmd: &str) -> Result<PathBuf, DetectionError> {
        #[cfg(windows)]
        let which_cmd = "where";
        #[cfg(not(windows))]
        let which_cmd = "which";

        let output = Command::new(which_cmd).arg(cmd).output()?;

        if output.status.success() {
            let path_str = String::from_utf8_lossy(&output.stdout);
            let path = path_str
                .lines()
                .next()
                .ok_or_else(|| DetectionError::ParseError("Empty which output".to_string()))?;
            Ok(PathBuf::from(path.trim()))
        } else {
            Err(DetectionError::ParseError(format!("Could not find {}", cmd)))
        }
    }

    /// Get Python version from executable
    fn get_version(&self, exe_path: &PathBuf) -> Result<PythonVersion, DetectionError> {
        let output = Command::new(exe_path).arg("--version").output()?;

        if !output.status.success() {
            return Err(DetectionError::ParseError("Python --version failed".to_string()));
        }

        let version_str = String::from_utf8_lossy(&output.stdout);
        let version_str = version_str.trim();

        // Handle both "Python 3.12.0" and just "3.12.0"
        let version_part = version_str.strip_prefix("Python ").unwrap_or(version_str);

        PythonVersion::parse(version_part).map_err(|e| DetectionError::ParseError(e.to_string()))
    }

    /// Determine installation source from path
    fn determine_source(&self, exe_path: &Path) -> InstallationSource {
        let path_str = exe_path.to_string_lossy().to_lowercase();

        if path_str.contains("pyenv") {
            InstallationSource::Pyenv
        } else if path_str.contains("conda")
            || path_str.contains("miniconda")
            || path_str.contains("anaconda")
        {
            InstallationSource::Conda
        } else if path_str.contains("homebrew") || path_str.contains("/opt/homebrew") {
            InstallationSource::Homebrew
        } else if path_str.contains("windowsapps") || path_str.contains("microsoft") {
            InstallationSource::WindowsStore
        } else if path_str.starts_with("/usr") || path_str.starts_with("c:\\windows") {
            InstallationSource::System
        } else {
            InstallationSource::Custom(exe_path.parent().unwrap_or(exe_path).to_path_buf())
        }
    }

    /// Get architecture from Python
    fn get_architecture(&self, exe_path: &PathBuf) -> Result<Architecture, DetectionError> {
        let output = Command::new(exe_path)
            .args(["-c", "import platform; print(platform.machine())"])
            .output()?;

        if !output.status.success() {
            return Ok(Architecture::default());
        }

        let arch_str = String::from_utf8_lossy(&output.stdout);
        Ok(Architecture::parse(arch_str.trim()))
    }

    /// Get capabilities from Python
    fn get_capabilities(&self, exe_path: &PathBuf) -> Result<RuntimeCapabilities, DetectionError> {
        let script = r#"
import sys
import json

caps = {
    "has_pip": False,
    "has_venv": False,
    "has_ssl": False,
    "has_sqlite": False,
    "abi_tag": ""
}

try:
    import pip
    caps["has_pip"] = True
except ImportError:
    pass

try:
    import venv
    caps["has_venv"] = True
except ImportError:
    pass

try:
    import ssl
    caps["has_ssl"] = True
except ImportError:
    pass

try:
    import sqlite3
    caps["has_sqlite"] = True
except ImportError:
    pass

# Get ABI tag
try:
    import sys
    caps["abi_tag"] = f"cp{sys.version_info.major}{sys.version_info.minor}"
except:
    pass

print(json.dumps(caps))
"#;

        let output = Command::new(exe_path).args(["-c", script]).output()?;

        if !output.status.success() {
            return Ok(RuntimeCapabilities::default());
        }

        let json_str = String::from_utf8_lossy(&output.stdout);

        #[derive(serde::Deserialize)]
        struct Caps {
            has_pip: bool,
            has_venv: bool,
            has_ssl: bool,
            has_sqlite: bool,
            abi_tag: String,
        }

        if let Ok(caps) = serde_json::from_str::<Caps>(&json_str) {
            Ok(RuntimeCapabilities {
                has_pip: caps.has_pip,
                has_venv: caps.has_venv,
                has_ssl: caps.has_ssl,
                has_sqlite: caps.has_sqlite,
                abi_tag: caps.abi_tag,
            })
        } else {
            Ok(RuntimeCapabilities::default())
        }
    }

    /// Find a specific Python version
    pub fn find_version(
        &self,
        major: u8,
        minor: u8,
    ) -> Result<Option<PythonRuntime>, DetectionError> {
        let runtimes = self.detect_all()?;
        Ok(runtimes
            .into_iter()
            .find(|r| r.version.major == major && r.version.minor == minor))
    }

    /// Add custom search path
    pub fn add_search_path(&mut self, path: PathBuf) {
        self.search_paths.push(path);
    }

    /// Enable caching
    pub fn with_cache(mut self) -> Self {
        self.cache = Some(DetectionCache::default());
        self
    }
}
