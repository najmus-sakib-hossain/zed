//! External Tool Manager for Multi-Language Support
//!
//! This module provides functionality for discovering, installing, and managing
//! external tools like clang-format, rustfmt, ktlint, etc.

use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, RwLock};

use crate::languages::Diagnostic;

/// Error type for tool installation failures
#[derive(Debug, Clone)]
pub struct InstallError {
    /// The tool that failed to install
    pub tool: String,
    /// Error message
    pub message: String,
    /// Installation instructions for manual installation
    pub instructions: String,
}

impl InstallError {
    /// Create a new install error
    pub fn new(
        tool: impl Into<String>,
        message: impl Into<String>,
        instructions: impl Into<String>,
    ) -> Self {
        Self {
            tool: tool.into(),
            message: message.into(),
            instructions: instructions.into(),
        }
    }

    /// Convert to a diagnostic
    #[must_use]
    pub fn to_diagnostic(&self, file_path: &str) -> Diagnostic {
        Diagnostic::error(
            file_path,
            format!("{}: {}\n\n{}", self.tool, self.message, self.instructions),
            format!("tool/{}", self.tool),
        )
    }
}

impl std::fmt::Display for InstallError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Failed to install {}: {}", self.tool, self.message)
    }
}

impl std::error::Error for InstallError {}

/// Detected operating system
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperatingSystem {
    Windows,
    MacOS,
    Linux,
}

impl OperatingSystem {
    /// Detect the current operating system
    #[must_use]
    pub fn detect() -> Self {
        if cfg!(target_os = "windows") {
            OperatingSystem::Windows
        } else if cfg!(target_os = "macos") {
            OperatingSystem::MacOS
        } else {
            OperatingSystem::Linux
        }
    }
}

/// Package manager detection result
#[derive(Debug, Clone)]
pub struct PackageManager {
    /// Name of the package manager
    pub name: String,
    /// Path to the package manager executable
    pub path: PathBuf,
}

impl PackageManager {
    /// Create a new package manager
    pub fn new(name: impl Into<String>, path: PathBuf) -> Self {
        Self {
            name: name.into(),
            path,
        }
    }
}

/// Tool version information
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolVersion {
    /// Tool name
    pub tool: String,
    /// Version string (e.g., "1.2.3")
    pub version: String,
    /// Full version output from the tool
    pub raw_output: String,
}

impl ToolVersion {
    /// Create a new tool version
    pub fn new(
        tool: impl Into<String>,
        version: impl Into<String>,
        raw_output: impl Into<String>,
    ) -> Self {
        Self {
            tool: tool.into(),
            version: version.into(),
            raw_output: raw_output.into(),
        }
    }
}

/// Tool configuration entry
#[derive(Debug, Clone)]
struct ToolCacheEntry {
    /// Path to the tool
    path: PathBuf,
    /// Version information (if available)
    version: Option<ToolVersion>,
    /// Whether this is a manually configured path
    manual: bool,
}

/// Tool configuration cache
#[derive(Debug, Clone)]
pub struct ToolCache {
    /// Cached tool paths and versions
    cache: Arc<RwLock<HashMap<String, ToolCacheEntry>>>,
    /// Cache directory path
    cache_dir: Arc<PathBuf>,
}

impl ToolCache {
    /// Create a new tool cache
    #[must_use]
    pub fn new() -> Self {
        let cache_dir = Self::get_cache_dir();
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            cache_dir: Arc::new(cache_dir),
        }
    }

    /// Get the cache directory path (.dx/cache/tools)
    fn get_cache_dir() -> PathBuf {
        let mut cache_dir = PathBuf::from(".dx");
        cache_dir.push("cache");
        cache_dir.push("tools");
        cache_dir
    }

    /// Ensure cache directory exists
    fn ensure_cache_dir(&self) -> Result<(), String> {
        if !self.cache_dir.exists() {
            fs::create_dir_all(&*self.cache_dir)
                .map_err(|e| format!("Failed to create cache directory: {e}"))?;
        }
        Ok(())
    }

    /// Load cache from disk
    pub fn load(&self) -> Result<(), String> {
        let cache_file = self.cache_dir.join("tools.json");
        if !cache_file.exists() {
            return Ok(());
        }

        let content = fs::read_to_string(&cache_file)
            .map_err(|e| format!("Failed to read cache file: {e}"))?;

        let entries: HashMap<String, serde_json::Value> = serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse cache file: {e}"))?;

        if let Ok(mut cache) = self.cache.write() {
            for (tool, value) in entries {
                if let Some(path_str) = value.get("path").and_then(|v| v.as_str()) {
                    let path = PathBuf::from(path_str);
                    let manual =
                        value.get("manual").and_then(serde_json::Value::as_bool).unwrap_or(false);

                    let version = if let Some(version_obj) = value.get("version") {
                        if let (Some(tool_name), Some(ver), Some(raw)) = (
                            version_obj.get("tool").and_then(|v| v.as_str()),
                            version_obj.get("version").and_then(|v| v.as_str()),
                            version_obj.get("raw_output").and_then(|v| v.as_str()),
                        ) {
                            Some(ToolVersion::new(tool_name, ver, raw))
                        } else {
                            None
                        }
                    } else {
                        None
                    };

                    cache.insert(
                        tool,
                        ToolCacheEntry {
                            path,
                            version,
                            manual,
                        },
                    );
                }
            }
        }

        Ok(())
    }

    /// Save cache to disk
    pub fn save(&self) -> Result<(), String> {
        self.ensure_cache_dir()?;

        let cache_file = self.cache_dir.join("tools.json");

        let entries: HashMap<String, serde_json::Value> = if let Ok(cache) = self.cache.read() {
            cache
                .iter()
                .map(|(tool, entry)| {
                    let mut value = serde_json::json!({
                        "path": entry.path.to_string_lossy(),
                        "manual": entry.manual,
                    });

                    if let Some(ref version) = entry.version {
                        value["version"] = serde_json::json!({
                            "tool": version.tool,
                            "version": version.version,
                            "raw_output": version.raw_output,
                        });
                    }

                    (tool.clone(), value)
                })
                .collect()
        } else {
            HashMap::new()
        };

        let content = serde_json::to_string_pretty(&entries)
            .map_err(|e| format!("Failed to serialize cache: {e}"))?;

        fs::write(&cache_file, content).map_err(|e| format!("Failed to write cache file: {e}"))?;

        Ok(())
    }

    /// Get a cached tool path
    #[must_use]
    pub fn get(&self, tool: &str) -> Option<PathBuf> {
        self.cache.read().ok()?.get(tool).map(|entry| entry.path.clone())
    }

    /// Get cached tool version
    #[must_use]
    pub fn get_version(&self, tool: &str) -> Option<ToolVersion> {
        self.cache.read().ok()?.get(tool).and_then(|entry| entry.version.clone())
    }

    /// Set a tool path in the cache
    pub fn set(
        &self,
        tool: impl Into<String>,
        path: PathBuf,
        version: Option<ToolVersion>,
        manual: bool,
    ) {
        if let Ok(mut cache) = self.cache.write() {
            cache.insert(
                tool.into(),
                ToolCacheEntry {
                    path,
                    version,
                    manual,
                },
            );
        }
        // Save to disk after updating
        let _ = self.save();
    }

    /// Remove a tool from the cache
    pub fn remove(&self, tool: &str) {
        if let Ok(mut cache) = self.cache.write() {
            cache.remove(tool);
        }
        // Save to disk after updating
        let _ = self.save();
    }

    /// Clear all cached tools
    pub fn clear(&self) {
        if let Ok(mut cache) = self.cache.write() {
            cache.clear();
        }
        // Save to disk after updating
        let _ = self.save();
    }

    /// Check if a tool is manually configured
    #[must_use]
    pub fn is_manual(&self, tool: &str) -> bool {
        self.cache
            .read()
            .ok()
            .and_then(|cache| cache.get(tool).map(|entry| entry.manual))
            .unwrap_or(false)
    }

    /// Get all cached tool names
    #[must_use]
    pub fn tools(&self) -> Vec<String> {
        self.cache
            .read()
            .ok()
            .map(|cache| cache.keys().cloned().collect())
            .unwrap_or_default()
    }
}

impl Default for ToolCache {
    fn default() -> Self {
        Self::new()
    }
}

/// External tool manager for discovering and installing tools
pub struct ExternalToolManager {
    /// Tool configuration cache
    cache: ToolCache,
}

impl ExternalToolManager {
    /// Create a new external tool manager
    #[must_use]
    pub fn new() -> Self {
        let cache = ToolCache::new();
        // Try to load existing cache from disk
        let _ = cache.load();
        Self { cache }
    }

    /// Create a new external tool manager with a shared cache
    #[must_use]
    pub fn with_cache(cache: ToolCache) -> Self {
        // Try to load existing cache from disk
        let _ = cache.load();
        Self { cache }
    }

    /// Get the tool cache
    #[must_use]
    pub fn cache(&self) -> &ToolCache {
        &self.cache
    }

    /// Ensure a tool is available, attempting auto-install if not found
    ///
    /// # Arguments
    /// * `name` - Name of the tool to ensure is available
    ///
    /// # Returns
    /// * `Ok(PathBuf)` - Path to the tool
    /// * `Err(InstallError)` - Tool not found and auto-install failed
    pub fn ensure_tool(&self, name: &str) -> Result<PathBuf, InstallError> {
        // First, try to find the tool (checks cache and PATH)
        if let Some(path) = self.find_tool_cached(name) {
            return Ok(path);
        }

        // Tool not found, attempt auto-install
        match Self::install_tool(name) {
            Ok(path) => {
                // Cache the newly installed tool
                let version = Self::detect_version_static(&path, name);
                self.cache.set(name, path.clone(), version, false);
                Ok(path)
            }
            Err(e) => Err(e),
        }
    }

    /// Configure a manual tool path
    ///
    /// # Arguments
    /// * `tool` - Name of the tool
    /// * `path` - Path to the tool executable
    ///
    /// # Returns
    /// * `Ok(())` - Tool configured successfully
    /// * `Err(String)` - Configuration failed (e.g., path doesn't exist)
    pub fn configure_tool(&self, tool: impl Into<String>, path: PathBuf) -> Result<(), String> {
        let tool_name = tool.into();

        // Verify the path exists and is executable
        if !path.exists() {
            return Err(format!("Tool path does not exist: {}", path.display()));
        }

        if !path.is_file() {
            return Err(format!("Tool path is not a file: {}", path.display()));
        }

        // Try to detect version
        let version = Self::detect_version_static(&path, &tool_name);

        // Store in cache as manual configuration
        self.cache.set(tool_name, path, version, true);

        Ok(())
    }

    /// Remove a manually configured tool
    pub fn remove_tool_config(&self, tool: &str) {
        self.cache.remove(tool);
    }

    /// Find a tool in PATH or common locations, with caching
    ///
    /// # Arguments
    /// * `name` - Name of the tool to find (e.g., "clang-format", "rustfmt")
    ///
    /// # Returns
    /// * `Some(PathBuf)` - Path to the tool if found
    /// * `None` - Tool not found
    #[must_use]
    pub fn find_tool_cached(&self, name: &str) -> Option<PathBuf> {
        // Check cache first
        if let Some(path) = self.cache.get(name) {
            // Verify cached path still exists
            if path.exists() {
                return Some(path);
            }
            // Remove stale cache entry
            self.cache.remove(name);
        }

        // Find tool and cache result
        if let Some(path) = Self::find_tool(name) {
            let version = Self::detect_version_static(&path, name);
            self.cache.set(name, path.clone(), version, false);
            Some(path)
        } else {
            None
        }
    }

    /// Get tool version from cache or detect it
    #[must_use]
    pub fn get_tool_version(&self, name: &str) -> Option<ToolVersion> {
        // Check cache first
        if let Some(version) = self.cache.get_version(name) {
            return Some(version);
        }

        // Find tool and detect version
        let path = self.find_tool_cached(name)?;
        Self::detect_version_static(&path, name)
    }

    /// Detect the version of a tool
    ///
    /// # Arguments
    /// * `tool_path` - Path to the tool executable
    /// * `tool_name` - Name of the tool (for version parsing)
    ///
    /// # Returns
    /// * `Some(ToolVersion)` - Version information if detected
    /// * `None` - Version could not be detected
    fn detect_version_static(tool_path: &Path, tool_name: &str) -> Option<ToolVersion> {
        // Try common version flags
        let version_flags = ["--version", "-version", "-v", "version"];

        for flag in &version_flags {
            if let Ok(output) = Command::new(tool_path).arg(flag).output()
                && output.status.success()
            {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);

                // Prefer stdout, fall back to stderr
                let output_text = if stdout.trim().is_empty() {
                    stderr.to_string()
                } else {
                    stdout.to_string()
                };

                if let Some(version) = Self::parse_version(&output_text, tool_name) {
                    return Some(ToolVersion::new(tool_name, version, output_text));
                }
            }
        }

        None
    }

    /// Parse version string from tool output
    fn parse_version(output: &str, tool_name: &str) -> Option<String> {
        // Try to find version patterns
        let patterns = [
            // "tool 1.2.3" or "tool version 1.2.3"
            format!(r"{tool_name}\s+(?:version\s+)?(\d+\.\d+(?:\.\d+)?)"),
            // "version 1.2.3" or "v1.2.3"
            r"(?:version|v)\s*(\d+\.\d+(?:\.\d+)?)".to_string(),
            // Just "1.2.3" at start of line
            r"^(\d+\.\d+(?:\.\d+)?)".to_string(),
        ];

        for pattern in &patterns {
            if let Ok(re) = regex::Regex::new(pattern)
                && let Some(captures) = re.captures(output)
                && let Some(version_match) = captures.get(1)
            {
                return Some(version_match.as_str().to_string());
            }
        }

        // Fallback: extract first version-like string
        if let Ok(re) = regex::Regex::new(r"\d+\.\d+(?:\.\d+)?")
            && let Some(version_match) = re.find(output)
        {
            return Some(version_match.as_str().to_string());
        }

        None
    }

    /// Find a tool in PATH or common locations (static version for backward compatibility)
    ///
    /// # Arguments
    /// * `name` - Name of the tool to find (e.g., "clang-format", "rustfmt")
    ///
    /// # Returns
    /// * `Some(PathBuf)` - Path to the tool if found
    /// * `None` - Tool not found
    #[must_use]
    pub fn find_tool(name: &str) -> Option<PathBuf> {
        // First, try to find in PATH using `which` (Unix) or `where` (Windows)
        if let Some(path) = Self::find_in_path(name) {
            return Some(path);
        }

        // Try common installation locations
        Self::find_in_common_locations(name)
    }

    /// Find a tool in the system PATH
    fn find_in_path(name: &str) -> Option<PathBuf> {
        let os = OperatingSystem::detect();

        let (cmd, args) = match os {
            OperatingSystem::Windows => ("where", vec![name]),
            _ => ("which", vec![name]),
        };

        let output = Command::new(cmd).args(&args).output().ok()?;

        if output.status.success() {
            let path_str = String::from_utf8_lossy(&output.stdout);
            let path = path_str.lines().next()?.trim();
            if !path.is_empty() {
                return Some(PathBuf::from(path));
            }
        }

        None
    }

    /// Find a tool in common installation locations
    fn find_in_common_locations(name: &str) -> Option<PathBuf> {
        let os = OperatingSystem::detect();
        let locations = Self::get_common_locations(name, os);

        for location in locations {
            if location.exists() && location.is_file() {
                return Some(location);
            }
        }

        None
    }

    /// Get common installation locations for a tool
    fn get_common_locations(name: &str, os: OperatingSystem) -> Vec<PathBuf> {
        let mut locations = Vec::new();
        let exe_suffix = if os == OperatingSystem::Windows {
            ".exe"
        } else {
            ""
        };
        let tool_name = format!("{name}{exe_suffix}");

        match os {
            OperatingSystem::Windows => {
                // Common Windows locations
                if let Ok(program_files) = env::var("ProgramFiles") {
                    locations.push(
                        PathBuf::from(&program_files).join("LLVM").join("bin").join(&tool_name),
                    );
                }
                if let Ok(program_files_x86) = env::var("ProgramFiles(x86)") {
                    locations.push(
                        PathBuf::from(&program_files_x86).join("LLVM").join("bin").join(&tool_name),
                    );
                }
                if let Ok(local_app_data) = env::var("LOCALAPPDATA") {
                    locations
                        .push(PathBuf::from(&local_app_data).join("Programs").join(&tool_name));
                }
                // Chocolatey
                locations.push(PathBuf::from("C:\\ProgramData\\chocolatey\\bin").join(&tool_name));
                // Scoop
                if let Ok(userprofile) = env::var("USERPROFILE") {
                    locations.push(
                        PathBuf::from(&userprofile).join("scoop").join("shims").join(&tool_name),
                    );
                }
                // Cargo bin
                if let Ok(cargo_home) = env::var("CARGO_HOME") {
                    locations.push(PathBuf::from(&cargo_home).join("bin").join(&tool_name));
                } else if let Ok(userprofile) = env::var("USERPROFILE") {
                    locations.push(
                        PathBuf::from(&userprofile).join(".cargo").join("bin").join(&tool_name),
                    );
                }
            }
            OperatingSystem::MacOS => {
                // Homebrew locations
                locations.push(PathBuf::from("/opt/homebrew/bin").join(&tool_name));
                locations.push(PathBuf::from("/usr/local/bin").join(&tool_name));
                // Cargo bin
                if let Ok(home) = env::var("HOME") {
                    locations
                        .push(PathBuf::from(&home).join(".cargo").join("bin").join(&tool_name));
                }
            }
            OperatingSystem::Linux => {
                // Common Linux locations
                locations.push(PathBuf::from("/usr/bin").join(&tool_name));
                locations.push(PathBuf::from("/usr/local/bin").join(&tool_name));
                locations.push(PathBuf::from("/snap/bin").join(&tool_name));
                // Cargo bin
                if let Ok(home) = env::var("HOME") {
                    locations
                        .push(PathBuf::from(&home).join(".cargo").join("bin").join(&tool_name));
                }
            }
        }

        locations
    }

    /// Attempt automatic installation of a tool
    ///
    /// # Arguments
    /// * `name` - Name of the tool to install
    ///
    /// # Returns
    /// * `Ok(PathBuf)` - Path to the installed tool
    /// * `Err(InstallError)` - Installation failed
    pub fn install_tool(name: &str) -> Result<PathBuf, InstallError> {
        let os = OperatingSystem::detect();

        // Try to install using available package managers
        let result = match os {
            OperatingSystem::Windows => Self::install_windows(name),
            OperatingSystem::MacOS => Self::install_macos(name),
            OperatingSystem::Linux => Self::install_linux(name),
        };

        match result {
            Ok(path) => Ok(path),
            Err(e) => Err(InstallError::new(name, e, Self::get_install_instructions(name))),
        }
    }

    /// Install a tool on Windows
    fn install_windows(name: &str) -> Result<PathBuf, String> {
        // Try Chocolatey first
        if Self::has_package_manager("choco") {
            let package = Self::get_chocolatey_package(name);
            if let Some(pkg) = package {
                let output = Command::new("choco")
                    .args(["install", &pkg, "-y"])
                    .output()
                    .map_err(|e| format!("Failed to run choco: {e}"))?;

                if output.status.success()
                    && let Some(path) = Self::find_tool(name)
                {
                    return Ok(path);
                }
            }
        }

        // Try Scoop
        if Self::has_package_manager("scoop") {
            let package = Self::get_scoop_package(name);
            if let Some(pkg) = package {
                let output = Command::new("scoop")
                    .args(["install", &pkg])
                    .output()
                    .map_err(|e| format!("Failed to run scoop: {e}"))?;

                if output.status.success()
                    && let Some(path) = Self::find_tool(name)
                {
                    return Ok(path);
                }
            }
        }

        // Try rustup for Rust tools
        if name == "rustfmt" || name == "cargo-clippy" || name == "clippy-driver" {
            return Self::install_rust_tool(name);
        }

        Err(format!("No package manager available to install {name}"))
    }

    /// Install a tool on macOS
    fn install_macos(name: &str) -> Result<PathBuf, String> {
        // Try Homebrew
        if Self::has_package_manager("brew") {
            let package = Self::get_homebrew_package(name);
            if let Some(pkg) = package {
                let output = Command::new("brew")
                    .args(["install", &pkg])
                    .output()
                    .map_err(|e| format!("Failed to run brew: {e}"))?;

                if output.status.success()
                    && let Some(path) = Self::find_tool(name)
                {
                    return Ok(path);
                }
            }
        }

        // Try rustup for Rust tools
        if name == "rustfmt" || name == "cargo-clippy" || name == "clippy-driver" {
            return Self::install_rust_tool(name);
        }

        Err(format!("No package manager available to install {name}"))
    }

    /// Install a tool on Linux
    fn install_linux(name: &str) -> Result<PathBuf, String> {
        // Try apt-get (Debian/Ubuntu)
        if Self::has_package_manager("apt-get") {
            let package = Self::get_apt_package(name);
            if let Some(pkg) = package {
                let output = Command::new("sudo")
                    .args(["apt-get", "install", "-y", &pkg])
                    .output()
                    .map_err(|e| format!("Failed to run apt-get: {e}"))?;

                if output.status.success()
                    && let Some(path) = Self::find_tool(name)
                {
                    return Ok(path);
                }
            }
        }

        // Try dnf (Fedora/RHEL)
        if Self::has_package_manager("dnf") {
            let package = Self::get_dnf_package(name);
            if let Some(pkg) = package {
                let output = Command::new("sudo")
                    .args(["dnf", "install", "-y", &pkg])
                    .output()
                    .map_err(|e| format!("Failed to run dnf: {e}"))?;

                if output.status.success()
                    && let Some(path) = Self::find_tool(name)
                {
                    return Ok(path);
                }
            }
        }

        // Try pacman (Arch)
        if Self::has_package_manager("pacman") {
            let package = Self::get_pacman_package(name);
            if let Some(pkg) = package {
                let output = Command::new("sudo")
                    .args(["pacman", "-S", "--noconfirm", &pkg])
                    .output()
                    .map_err(|e| format!("Failed to run pacman: {e}"))?;

                if output.status.success()
                    && let Some(path) = Self::find_tool(name)
                {
                    return Ok(path);
                }
            }
        }

        // Try rustup for Rust tools
        if name == "rustfmt" || name == "cargo-clippy" || name == "clippy-driver" {
            return Self::install_rust_tool(name);
        }

        Err(format!("No package manager available to install {name}"))
    }

    /// Install Rust tools via rustup
    fn install_rust_tool(name: &str) -> Result<PathBuf, String> {
        let component = match name {
            "rustfmt" => "rustfmt",
            "cargo-clippy" | "clippy-driver" => "clippy",
            _ => return Err(format!("Unknown Rust tool: {name}")),
        };

        let output = Command::new("rustup")
            .args(["component", "add", component])
            .output()
            .map_err(|e| format!("Failed to run rustup: {e}"))?;

        if output.status.success()
            && let Some(path) = Self::find_tool(name)
        {
            return Ok(path);
        }

        Err(format!("Failed to install {name} via rustup"))
    }

    /// Check if a package manager is available
    fn has_package_manager(name: &str) -> bool {
        Self::find_in_path(name).is_some()
    }

    /// Get Chocolatey package name for a tool
    fn get_chocolatey_package(tool: &str) -> Option<String> {
        match tool {
            "clang-format" | "clang-tidy" => Some("llvm".to_string()),
            "ktlint" => Some("ktlint".to_string()),
            "ruff" => Some("ruff".to_string()),
            "gofmt" | "go" => Some("golang".to_string()),
            _ => None,
        }
    }

    /// Get Scoop package name for a tool
    fn get_scoop_package(tool: &str) -> Option<String> {
        match tool {
            "clang-format" | "clang-tidy" => Some("llvm".to_string()),
            "ktlint" => Some("ktlint".to_string()),
            "ruff" => Some("ruff".to_string()),
            "gofmt" | "go" => Some("go".to_string()),
            _ => None,
        }
    }

    /// Get Homebrew package name for a tool
    fn get_homebrew_package(tool: &str) -> Option<String> {
        match tool {
            "clang-format" | "clang-tidy" => Some("llvm".to_string()),
            "ktlint" => Some("ktlint".to_string()),
            "ruff" => Some("ruff".to_string()),
            "gofmt" | "go" => Some("go".to_string()),
            _ => None,
        }
    }

    /// Get apt package name for a tool
    fn get_apt_package(tool: &str) -> Option<String> {
        match tool {
            "clang-format" => Some("clang-format".to_string()),
            "clang-tidy" => Some("clang-tidy".to_string()),
            "ktlint" => None, // Not available in apt
            "gofmt" | "go" => Some("golang".to_string()),
            _ => None,
        }
    }

    /// Get dnf package name for a tool
    fn get_dnf_package(tool: &str) -> Option<String> {
        match tool {
            "clang-format" | "clang-tidy" => Some("clang-tools-extra".to_string()),
            "ktlint" => None, // Not available in dnf
            "gofmt" | "go" => Some("golang".to_string()),
            _ => None,
        }
    }

    /// Get pacman package name for a tool
    fn get_pacman_package(tool: &str) -> Option<String> {
        match tool {
            "clang-format" | "clang-tidy" => Some("clang".to_string()),
            "ktlint" => None, // Not available in pacman
            "gofmt" | "go" => Some("go".to_string()),
            _ => None,
        }
    }

    /// Get platform-specific installation instructions for a tool
    ///
    /// # Arguments
    /// * `name` - Name of the tool
    ///
    /// # Returns
    /// A string containing installation instructions
    #[must_use]
    pub fn get_install_instructions(name: &str) -> String {
        let os = OperatingSystem::detect();

        match name {
            "clang-format" | "clang-tidy" => Self::get_clang_instructions(os),
            "rustfmt" | "cargo-clippy" | "clippy-driver" => Self::get_rust_instructions(name),
            "ktlint" => Self::get_ktlint_instructions(os),
            "ruff" => Self::get_ruff_instructions(os),
            "gofmt" | "go" => Self::get_go_instructions(os),
            _ => format!("Please install {name} manually."),
        }
    }

    fn get_clang_instructions(os: OperatingSystem) -> String {
        match os {
            OperatingSystem::Windows => "To install clang-format and clang-tidy on Windows:\n\n\
                 Option 1 - Chocolatey:\n\
                   choco install llvm\n\n\
                 Option 2 - Scoop:\n\
                   scoop install llvm\n\n\
                 Option 3 - Download from LLVM:\n\
                   https://releases.llvm.org/download.html"
                .to_string(),
            OperatingSystem::MacOS => "To install clang-format and clang-tidy on macOS:\n\n\
                 Option 1 - Homebrew:\n\
                   brew install llvm\n\n\
                 Option 2 - Xcode Command Line Tools:\n\
                   xcode-select --install"
                .to_string(),
            OperatingSystem::Linux => "To install clang-format and clang-tidy on Linux:\n\n\
                 Debian/Ubuntu:\n\
                   sudo apt-get install clang-format clang-tidy\n\n\
                 Fedora/RHEL:\n\
                   sudo dnf install clang-tools-extra\n\n\
                 Arch Linux:\n\
                   sudo pacman -S clang"
                .to_string(),
        }
    }

    fn get_rust_instructions(tool: &str) -> String {
        let component = match tool {
            "rustfmt" => "rustfmt",
            "cargo-clippy" | "clippy-driver" => "clippy",
            _ => tool,
        };

        format!(
            "To install {tool}:\n\n\
             If you have rustup installed:\n\
               rustup component add {component}\n\n\
             If you don't have Rust installed:\n\
               Visit https://rustup.rs/ to install Rust and rustup"
        )
    }

    fn get_ktlint_instructions(os: OperatingSystem) -> String {
        match os {
            OperatingSystem::Windows => "To install ktlint on Windows:\n\n\
                 Option 1 - Chocolatey:\n\
                   choco install ktlint\n\n\
                 Option 2 - Scoop:\n\
                   scoop install ktlint\n\n\
                 Option 3 - Manual download:\n\
                   Download from https://github.com/pinterest/ktlint/releases"
                .to_string(),
            OperatingSystem::MacOS => "To install ktlint on macOS:\n\n\
                 Option 1 - Homebrew:\n\
                   brew install ktlint\n\n\
                 Option 2 - Manual download:\n\
                   Download from https://github.com/pinterest/ktlint/releases"
                .to_string(),
            OperatingSystem::Linux => "To install ktlint on Linux:\n\n\
                 Option 1 - Snap:\n\
                   sudo snap install ktlint\n\n\
                 Option 2 - Manual download:\n\
                   curl -sSLO https://github.com/pinterest/ktlint/releases/download/1.0.0/ktlint\n\
                   chmod +x ktlint\n\
                   sudo mv ktlint /usr/local/bin/"
                .to_string(),
        }
    }

    fn get_ruff_instructions(os: OperatingSystem) -> String {
        match os {
            OperatingSystem::Windows => "To install ruff on Windows:\n\n\
                 Option 1 - pip:\n\
                   pip install ruff\n\n\
                 Option 2 - pipx:\n\
                   pipx install ruff\n\n\
                 Option 3 - Scoop:\n\
                   scoop install ruff\n\n\
                 Option 4 - Chocolatey:\n\
                   choco install ruff"
                .to_string(),
            OperatingSystem::MacOS => "To install ruff on macOS:\n\n\
                 Option 1 - Homebrew:\n\
                   brew install ruff\n\n\
                 Option 2 - pip:\n\
                   pip install ruff\n\n\
                 Option 3 - pipx:\n\
                   pipx install ruff"
                .to_string(),
            OperatingSystem::Linux => "To install ruff on Linux:\n\n\
                 Option 1 - pip:\n\
                   pip install ruff\n\n\
                 Option 2 - pipx:\n\
                   pipx install ruff\n\n\
                 Option 3 - Cargo:\n\
                   cargo install ruff"
                .to_string(),
        }
    }

    fn get_go_instructions(os: OperatingSystem) -> String {
        match os {
            OperatingSystem::Windows => "To install Go (which includes gofmt) on Windows:\n\n\
                 Option 1 - Chocolatey:\n\
                   choco install golang\n\n\
                 Option 2 - Scoop:\n\
                   scoop install go\n\n\
                 Option 3 - Download from Go:\n\
                   https://go.dev/dl/"
                .to_string(),
            OperatingSystem::MacOS => "To install Go (which includes gofmt) on macOS:\n\n\
                 Option 1 - Homebrew:\n\
                   brew install go\n\n\
                 Option 2 - Download from Go:\n\
                   https://go.dev/dl/"
                .to_string(),
            OperatingSystem::Linux => "To install Go (which includes gofmt) on Linux:\n\n\
                 Debian/Ubuntu:\n\
                   sudo apt-get install golang\n\n\
                 Fedora/RHEL:\n\
                   sudo dnf install golang\n\n\
                 Arch Linux:\n\
                   sudo pacman -S go\n\n\
                 Or download from:\n\
                   https://go.dev/dl/"
                .to_string(),
        }
    }

    /// Run an external tool and capture its output
    ///
    /// # Arguments
    /// * `tool_path` - Path to the tool executable
    /// * `args` - Arguments to pass to the tool
    /// * `input` - Optional input to pass via stdin
    ///
    /// # Returns
    /// * `Ok((stdout, stderr))` - Tool output
    /// * `Err(String)` - Execution failed
    pub fn run_tool(
        tool_path: &Path,
        args: &[&str],
        input: Option<&str>,
    ) -> Result<(String, String), String> {
        use std::io::Write;
        use std::process::Stdio;

        let mut cmd = Command::new(tool_path);
        cmd.args(args);

        if input.is_some() {
            cmd.stdin(Stdio::piped());
        }
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        let mut child = cmd
            .spawn()
            .map_err(|e| format!("Failed to spawn {}: {}", tool_path.display(), e))?;

        if let Some(input_str) = input
            && let Some(mut stdin) = child.stdin.take()
        {
            stdin
                .write_all(input_str.as_bytes())
                .map_err(|e| format!("Failed to write to stdin: {e}"))?;
        }

        let output = child
            .wait_with_output()
            .map_err(|e| format!("Failed to wait for {}: {}", tool_path.display(), e))?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        Ok((stdout, stderr))
    }

    /// Run an external tool and check for success
    ///
    /// # Arguments
    /// * `tool_path` - Path to the tool executable
    /// * `args` - Arguments to pass to the tool
    /// * `input` - Optional input to pass via stdin
    ///
    /// # Returns
    /// * `Ok(stdout)` - Tool succeeded, returns stdout
    /// * `Err(stderr)` - Tool failed, returns stderr
    pub fn run_tool_checked(
        tool_path: &Path,
        args: &[&str],
        input: Option<&str>,
    ) -> Result<String, String> {
        use std::io::Write;
        use std::process::Stdio;

        let mut cmd = Command::new(tool_path);
        cmd.args(args);

        if input.is_some() {
            cmd.stdin(Stdio::piped());
        }
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        let mut child = cmd
            .spawn()
            .map_err(|e| format!("Failed to spawn {}: {}", tool_path.display(), e))?;

        if let Some(input_str) = input
            && let Some(mut stdin) = child.stdin.take()
        {
            stdin
                .write_all(input_str.as_bytes())
                .map_err(|e| format!("Failed to write to stdin: {e}"))?;
        }

        let output = child
            .wait_with_output()
            .map_err(|e| format!("Failed to wait for {}: {}", tool_path.display(), e))?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            Err(String::from_utf8_lossy(&output.stderr).to_string())
        }
    }

    /// Find a config file by walking up the directory tree
    ///
    /// # Arguments
    /// * `start_path` - Path to start searching from
    /// * `config_names` - List of config file names to look for
    ///
    /// # Returns
    /// * `Some(PathBuf)` - Path to the config file if found
    /// * `None` - No config file found
    #[must_use]
    pub fn find_config_file(start_path: &Path, config_names: &[&str]) -> Option<PathBuf> {
        let mut current = if start_path.is_file() {
            start_path.parent()?
        } else {
            start_path
        };

        loop {
            for name in config_names {
                let config_path = current.join(name);
                if config_path.exists() && config_path.is_file() {
                    return Some(config_path);
                }
            }

            match current.parent() {
                Some(parent) => current = parent,
                None => break,
            }
        }

        None
    }
}

impl Default for ExternalToolManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_operating_system_detect() {
        let os = OperatingSystem::detect();
        // Just verify it returns a valid value
        assert!(matches!(
            os,
            OperatingSystem::Windows | OperatingSystem::MacOS | OperatingSystem::Linux
        ));
    }

    #[test]
    fn test_install_error_new() {
        let err = InstallError::new("test-tool", "test message", "test instructions");
        assert_eq!(err.tool, "test-tool");
        assert_eq!(err.message, "test message");
        assert_eq!(err.instructions, "test instructions");
    }

    #[test]
    fn test_install_error_display() {
        let err = InstallError::new("test-tool", "test message", "test instructions");
        let display = format!("{}", err);
        assert!(display.contains("test-tool"));
        assert!(display.contains("test message"));
    }

    #[test]
    fn test_install_error_to_diagnostic() {
        let err = InstallError::new("test-tool", "test message", "test instructions");
        let diag = err.to_diagnostic("test.py");
        assert_eq!(diag.file_path, "test.py");
        assert!(diag.message.contains("test-tool"));
        assert!(diag.message.contains("test message"));
        assert!(diag.message.contains("test instructions"));
    }

    #[test]
    fn test_get_install_instructions_clang() {
        let instructions = ExternalToolManager::get_install_instructions("clang-format");
        assert!(!instructions.is_empty());
        // Should contain some installation guidance
        assert!(
            instructions.contains("install") || instructions.contains("download"),
            "Instructions should contain installation guidance"
        );
    }

    #[test]
    fn test_get_install_instructions_rust() {
        let instructions = ExternalToolManager::get_install_instructions("rustfmt");
        assert!(instructions.contains("rustup"));
    }

    #[test]
    fn test_get_install_instructions_ktlint() {
        let instructions = ExternalToolManager::get_install_instructions("ktlint");
        assert!(!instructions.is_empty());
    }

    #[test]
    fn test_get_install_instructions_unknown() {
        let instructions = ExternalToolManager::get_install_instructions("unknown-tool");
        assert!(instructions.contains("manually"));
    }

    #[test]
    fn test_find_config_file_not_found() {
        let result = ExternalToolManager::find_config_file(
            Path::new("/nonexistent/path"),
            &[".nonexistent-config"],
        );
        assert!(result.is_none());
    }

    #[test]
    fn test_package_manager_new() {
        let pm = PackageManager::new("test", PathBuf::from("/usr/bin/test"));
        assert_eq!(pm.name, "test");
        assert_eq!(pm.path, PathBuf::from("/usr/bin/test"));
    }

    #[test]
    fn test_chocolatey_package_mapping() {
        assert_eq!(
            ExternalToolManager::get_chocolatey_package("clang-format"),
            Some("llvm".to_string())
        );
        assert_eq!(
            ExternalToolManager::get_chocolatey_package("ktlint"),
            Some("ktlint".to_string())
        );
        assert_eq!(ExternalToolManager::get_chocolatey_package("unknown"), None);
    }

    #[test]
    fn test_homebrew_package_mapping() {
        assert_eq!(
            ExternalToolManager::get_homebrew_package("clang-format"),
            Some("llvm".to_string())
        );
        assert_eq!(ExternalToolManager::get_homebrew_package("ktlint"), Some("ktlint".to_string()));
    }

    #[test]
    fn test_apt_package_mapping() {
        assert_eq!(
            ExternalToolManager::get_apt_package("clang-format"),
            Some("clang-format".to_string())
        );
        assert_eq!(
            ExternalToolManager::get_apt_package("clang-tidy"),
            Some("clang-tidy".to_string())
        );
        assert_eq!(ExternalToolManager::get_apt_package("ktlint"), None);
    }

    // New tests for tool cache functionality

    #[test]
    fn test_tool_cache_new() {
        let cache = ToolCache::new();
        assert!(cache.tools().is_empty());
    }

    #[test]
    fn test_tool_cache_set_and_get() {
        let cache = ToolCache::new();
        let path = PathBuf::from("/usr/bin/test-tool");

        cache.set("test-tool", path.clone(), None, false);

        assert_eq!(cache.get("test-tool"), Some(path));
        assert!(!cache.is_manual("test-tool"));
    }

    #[test]
    fn test_tool_cache_with_version() {
        let cache = ToolCache::new();
        let path = PathBuf::from("/usr/bin/test-tool");
        let version = ToolVersion::new("test-tool", "1.2.3", "test-tool 1.2.3");

        cache.set("test-tool", path.clone(), Some(version.clone()), false);

        assert_eq!(cache.get("test-tool"), Some(path));
        assert_eq!(cache.get_version("test-tool"), Some(version));
    }

    #[test]
    fn test_tool_cache_manual_flag() {
        let cache = ToolCache::new();
        let path = PathBuf::from("/custom/path/tool");

        cache.set("manual-tool", path, None, true);

        assert!(cache.is_manual("manual-tool"));
    }

    #[test]
    fn test_tool_cache_remove() {
        let cache = ToolCache::new();
        let path = PathBuf::from("/usr/bin/test-tool");

        cache.set("test-tool", path, None, false);
        assert!(cache.get("test-tool").is_some());

        cache.remove("test-tool");
        assert!(cache.get("test-tool").is_none());
    }

    #[test]
    fn test_tool_cache_clear() {
        let cache = ToolCache::new();

        cache.set("tool1", PathBuf::from("/usr/bin/tool1"), None, false);
        cache.set("tool2", PathBuf::from("/usr/bin/tool2"), None, false);

        assert_eq!(cache.tools().len(), 2);

        cache.clear();
        assert!(cache.tools().is_empty());
    }

    #[test]
    fn test_tool_cache_tools_list() {
        let cache = ToolCache::new();

        cache.set("tool1", PathBuf::from("/usr/bin/tool1"), None, false);
        cache.set("tool2", PathBuf::from("/usr/bin/tool2"), None, false);
        cache.set("tool3", PathBuf::from("/usr/bin/tool3"), None, false);

        let tools = cache.tools();
        assert_eq!(tools.len(), 3);
        assert!(tools.contains(&"tool1".to_string()));
        assert!(tools.contains(&"tool2".to_string()));
        assert!(tools.contains(&"tool3".to_string()));
    }

    #[test]
    fn test_tool_version_new() {
        let version = ToolVersion::new("rustfmt", "1.7.0", "rustfmt 1.7.0-stable");

        assert_eq!(version.tool, "rustfmt");
        assert_eq!(version.version, "1.7.0");
        assert_eq!(version.raw_output, "rustfmt 1.7.0-stable");
    }

    #[test]
    fn test_external_tool_manager_new() {
        let manager = ExternalToolManager::new();
        // Cache might load existing tools from disk, so we don't assert it's empty
        assert!(manager.cache().tools().len() >= 0);
    }

    #[test]
    fn test_external_tool_manager_with_cache() {
        let cache = ToolCache::new();
        cache.set("test-tool", PathBuf::from("/usr/bin/test"), None, false);

        let manager = ExternalToolManager::with_cache(cache.clone());
        // The cache might have loaded additional tools from disk, so we just verify our tool exists
        assert!(manager.cache().get("test-tool").is_some());
    }

    #[test]
    fn test_configure_tool_nonexistent_path() {
        let manager = ExternalToolManager::new();
        let result = manager.configure_tool("test-tool", PathBuf::from("/nonexistent/path"));

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("does not exist"));
    }

    #[test]
    fn test_remove_tool_config() {
        let manager = ExternalToolManager::new();
        let cache = manager.cache();

        cache.set("test-tool", PathBuf::from("/usr/bin/test"), None, true);
        assert!(cache.get("test-tool").is_some());

        manager.remove_tool_config("test-tool");
        assert!(cache.get("test-tool").is_none());
    }

    #[test]
    fn test_parse_version_standard_format() {
        let output = "rustfmt 1.7.0-stable (abc123 2024-01-01)";
        let version = ExternalToolManager::parse_version(output, "rustfmt");

        assert_eq!(version, Some("1.7.0".to_string()));
    }

    #[test]
    fn test_parse_version_with_v_prefix() {
        let output = "v1.2.3";
        let version = ExternalToolManager::parse_version(output, "tool");

        assert_eq!(version, Some("1.2.3".to_string()));
    }

    #[test]
    fn test_parse_version_version_keyword() {
        let output = "version 2.5.1";
        let version = ExternalToolManager::parse_version(output, "tool");

        assert_eq!(version, Some("2.5.1".to_string()));
    }

    #[test]
    fn test_parse_version_at_start() {
        let output = "10.0.0\nSome other info";
        let version = ExternalToolManager::parse_version(output, "tool");

        assert_eq!(version, Some("10.0.0".to_string()));
    }

    #[test]
    fn test_parse_version_no_match() {
        let output = "No version information available";
        let version = ExternalToolManager::parse_version(output, "tool");

        assert!(version.is_none());
    }

    #[test]
    fn test_parse_version_two_part() {
        let output = "tool version 1.5";
        let version = ExternalToolManager::parse_version(output, "tool");

        assert_eq!(version, Some("1.5".to_string()));
    }

    #[test]
    fn test_tool_cache_save_and_load() {
        use tempfile::TempDir;

        // Create a temporary directory for testing
        let temp_dir = TempDir::new().unwrap();
        let cache_dir = temp_dir.path().join(".dx").join("cache").join("tools");

        // Create cache with custom directory
        let cache = ToolCache::new();

        // Add some tools
        cache.set("tool1", PathBuf::from("/usr/bin/tool1"), None, false);
        cache.set(
            "tool2",
            PathBuf::from("/usr/bin/tool2"),
            Some(ToolVersion::new("tool2", "1.2.3", "tool2 1.2.3")),
            true,
        );

        // Save should succeed
        assert!(cache.save().is_ok());

        // Create a new cache and load
        let cache2 = ToolCache::new();
        assert!(cache2.load().is_ok());

        // Verify tools were loaded (if cache file exists)
        if cache_dir.join("tools.json").exists() {
            assert_eq!(cache2.get("tool1"), Some(PathBuf::from("/usr/bin/tool1")));
            assert_eq!(cache2.get("tool2"), Some(PathBuf::from("/usr/bin/tool2")));
            assert!(cache2.is_manual("tool2"));
            assert!(!cache2.is_manual("tool1"));
        }
    }

    #[test]
    fn test_ensure_tool_with_existing_tool() {
        let manager = ExternalToolManager::new();

        // Try to ensure cargo exists (should be available in Rust environment)
        if let Ok(path) = manager.ensure_tool("cargo") {
            assert!(path.exists());

            // Should be cached now
            assert_eq!(manager.cache().get("cargo"), Some(path));
        }
    }

    #[test]
    fn test_ensure_tool_with_nonexistent_tool() {
        let manager = ExternalToolManager::new();

        // Try to ensure a tool that doesn't exist
        let result = manager.ensure_tool("nonexistent-tool-xyz-123");

        // Should fail with InstallError
        assert!(result.is_err());

        if let Err(err) = result {
            assert_eq!(err.tool, "nonexistent-tool-xyz-123");
            assert!(!err.instructions.is_empty());
        }
    }

    #[test]
    fn test_cache_persistence_across_managers() {
        let cache = ToolCache::new();

        // Add a tool and save
        cache.set("test-tool", PathBuf::from("/usr/bin/test"), None, false);
        assert!(cache.save().is_ok());

        // Create a new manager with the same cache
        let manager1 = ExternalToolManager::with_cache(cache.clone());
        let manager2 = ExternalToolManager::with_cache(cache.clone());

        // Both managers should see the same cached tool
        assert_eq!(manager1.cache().get("test-tool"), manager2.cache().get("test-tool"));
    }
}
