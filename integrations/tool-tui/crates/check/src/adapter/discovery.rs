//! Tool Discovery
//!
//! Discovers tools in PATH, common locations, and configured paths.

use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Result of tool discovery
#[derive(Debug, Clone)]
pub struct DiscoveryResult {
    /// Tool name
    pub name: String,
    /// Path to executable
    pub path: PathBuf,
    /// Version string (if available)
    pub version: Option<String>,
}

/// Tool discovery service
pub struct ToolDiscovery {
    /// Additional search paths
    search_paths: Vec<PathBuf>,
    /// Cache of discovered tools
    cache: parking_lot::RwLock<std::collections::HashMap<String, Option<DiscoveryResult>>>,
}

impl ToolDiscovery {
    /// Create a new discovery service
    #[must_use]
    pub fn new() -> Self {
        Self {
            search_paths: Self::default_search_paths(),
            cache: parking_lot::RwLock::new(std::collections::HashMap::new()),
        }
    }

    /// Create with additional search paths
    pub fn with_paths(mut self, paths: Vec<PathBuf>) -> Self {
        self.search_paths.extend(paths);
        self
    }

    /// Get default search paths based on OS
    fn default_search_paths() -> Vec<PathBuf> {
        let mut paths = Vec::new();

        // Cargo bin directory
        if let Some(home) = dirs::home_dir() {
            paths.push(home.join(".cargo").join("bin"));
        }

        // Node modules bin
        paths.push(PathBuf::from("node_modules").join(".bin"));

        // Common Unix paths
        #[cfg(unix)]
        {
            paths.push(PathBuf::from("/usr/local/bin"));
            paths.push(PathBuf::from("/usr/bin"));
            paths.push(PathBuf::from("/opt/homebrew/bin"));
        }

        // Windows paths
        #[cfg(windows)]
        {
            if let Some(local_app_data) = dirs::data_local_dir() {
                paths.push(local_app_data.join("Programs").join("Python").join("Python311"));
                paths.push(local_app_data.join("Programs").join("Python").join("Python312"));
            }
            if let Some(program_files) = env::var_os("ProgramFiles") {
                let pf = PathBuf::from(program_files);
                paths.push(pf.join("Go").join("bin"));
                paths.push(pf.join("nodejs"));
            }
        }

        paths
    }

    /// Discover a tool by name
    pub fn discover(&self, name: &str) -> Option<DiscoveryResult> {
        // Check cache
        if let Some(result) = self.cache.read().get(name) {
            return result.clone();
        }

        // Try to find the tool
        let result = self.find_tool(name);

        // Cache the result
        self.cache.write().insert(name.to_string(), result.clone());

        result
    }

    /// Find a tool in PATH and search paths
    fn find_tool(&self, name: &str) -> Option<DiscoveryResult> {
        // Try `which` command first (most reliable)
        if let Some(path) = self.which(name) {
            let version = self.get_version(name, &path);
            return Some(DiscoveryResult {
                name: name.to_string(),
                path,
                version,
            });
        }

        // Search in configured paths
        for search_path in &self.search_paths {
            let candidate = self.executable_name(search_path, name);
            if candidate.exists() && candidate.is_file() {
                let version = self.get_version(name, &candidate);
                return Some(DiscoveryResult {
                    name: name.to_string(),
                    path: candidate,
                    version,
                });
            }
        }

        None
    }

    /// Use `which` (Unix) or `where` (Windows) to find executable
    fn which(&self, name: &str) -> Option<PathBuf> {
        #[cfg(windows)]
        let cmd = "where";
        #[cfg(not(windows))]
        let cmd = "which";

        Command::new(cmd)
            .arg(name)
            .output()
            .ok()
            .filter(|o| o.status.success())
            .and_then(|o| {
                String::from_utf8(o.stdout)
                    .ok()
                    .map(|s| PathBuf::from(s.lines().next().unwrap_or("").trim()))
            })
            .filter(|p| !p.as_os_str().is_empty())
    }

    /// Get executable name with platform extension
    fn executable_name(&self, dir: &Path, name: &str) -> PathBuf {
        #[cfg(windows)]
        {
            let exe = dir.join(format!("{name}.exe"));
            if exe.exists() {
                return exe;
            }
            let cmd = dir.join(format!("{name}.cmd"));
            if cmd.exists() {
                return cmd;
            }
            let bat = dir.join(format!("{name}.bat"));
            if bat.exists() {
                return bat;
            }
        }
        dir.join(name)
    }

    /// Get tool version by running `tool --version`
    fn get_version(&self, name: &str, path: &Path) -> Option<String> {
        let version_args = match name {
            "rustfmt" | "cargo-clippy" => vec!["--version"],
            "ruff" => vec!["--version"],
            "gofmt" => vec![], // gofmt doesn't have --version
            "golangci-lint" => vec!["--version"],
            "prettier" => vec!["--version"],
            "eslint" => vec!["--version"],
            "clang-format" => vec!["--version"],
            "clang-tidy" => vec!["--version"],
            "ktlint" => vec!["--version"],
            "phpcs" => vec!["--version"],
            "rubocop" => vec!["--version"],
            _ => vec!["--version"],
        };

        if version_args.is_empty() {
            return None;
        }

        Command::new(path)
            .args(&version_args)
            .output()
            .ok()
            .filter(|o| o.status.success())
            .and_then(|o| String::from_utf8(o.stdout).or_else(|_| String::from_utf8(o.stderr)).ok())
            .map(|s| {
                // Extract version number from output
                self.extract_version(&s)
            })
    }

    /// Extract version number from version output
    fn extract_version(&self, output: &str) -> String {
        // Common patterns: "tool 1.2.3", "tool version 1.2.3", "v1.2.3"
        let output = output.trim();

        // Try to find semver pattern
        let re = regex::Regex::new(r"(\d+\.\d+\.\d+(?:-[a-zA-Z0-9]+)?)").unwrap();

        if let Some(captures) = re.captures(output)
            && let Some(version) = captures.get(1)
        {
            return version.as_str().to_string();
        }

        // Fall back to first line
        output.lines().next().unwrap_or(output).to_string()
    }

    /// Clear the discovery cache
    pub fn clear_cache(&self) {
        self.cache.write().clear();
    }

    /// Check if a specific path is a valid executable
    #[must_use]
    pub fn is_valid_executable(path: &Path) -> bool {
        if !path.exists() || !path.is_file() {
            return false;
        }

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Ok(metadata) = path.metadata() {
                let permissions = metadata.permissions();
                return permissions.mode() & 0o111 != 0;
            }
            false
        }

        #[cfg(windows)]
        {
            // On Windows, check extension
            path.extension().is_some_and(|ext| {
                let ext = ext.to_string_lossy().to_lowercase();
                ext == "exe" || ext == "cmd" || ext == "bat" || ext == "com"
            })
        }
    }
}

impl Default for ToolDiscovery {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_discovery_creation() {
        let discovery = ToolDiscovery::new();
        assert!(!discovery.search_paths.is_empty());
    }

    #[test]
    fn test_extract_version() {
        let discovery = ToolDiscovery::new();

        assert_eq!(discovery.extract_version("rustfmt 1.5.2-stable"), "1.5.2");
        assert_eq!(discovery.extract_version("ruff 0.1.5"), "0.1.5");
        assert_eq!(discovery.extract_version("v2.0.0"), "2.0.0");
    }
}
