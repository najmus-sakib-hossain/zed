//! Lifecycle script execution for npm packages
//!
//! This module provides functionality to execute npm lifecycle scripts
//! (preinstall, install, postinstall, prepare, etc.) during package installation.

use dx_pkg_core::Result;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant};

/// Lifecycle script types in execution order
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LifecycleScript {
    /// Runs before package is installed
    Preinstall,
    /// Runs after package is installed (native modules)
    Install,
    /// Runs after package is installed
    Postinstall,
    /// Runs after package is prepared (git deps)
    Prepare,
    /// Runs before package is published
    Prepublish,
    /// Runs before package is packed
    Prepack,
    /// Runs after package is packed
    Postpack,
}

impl LifecycleScript {
    /// Get the script name as it appears in package.json
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Preinstall => "preinstall",
            Self::Install => "install",
            Self::Postinstall => "postinstall",
            Self::Prepare => "prepare",
            Self::Prepublish => "prepublish",
            Self::Prepack => "prepack",
            Self::Postpack => "postpack",
        }
    }

    /// Get the execution order for install lifecycle
    pub fn install_order() -> &'static [LifecycleScript] {
        &[Self::Preinstall, Self::Install, Self::Postinstall]
    }

    /// Get the execution order for git dependencies
    pub fn git_dep_order() -> &'static [LifecycleScript] {
        &[
            Self::Preinstall,
            Self::Install,
            Self::Postinstall,
            Self::Prepare,
        ]
    }
}

/// Result of executing a script
#[derive(Debug, Clone)]
pub struct ScriptResult {
    /// Script that was executed
    pub script: LifecycleScript,
    /// Package name
    pub package: String,
    /// Exit code (0 = success)
    pub exit_code: i32,
    /// Standard output
    pub stdout: String,
    /// Standard error
    pub stderr: String,
    /// Execution duration
    pub duration: Duration,
    /// Whether the script was skipped (not defined)
    pub skipped: bool,
}

impl ScriptResult {
    /// Check if the script succeeded
    pub fn success(&self) -> bool {
        self.skipped || self.exit_code == 0
    }
}

/// Script execution configuration
#[derive(Debug, Clone)]
pub struct ScriptConfig {
    /// Whether to ignore script failures
    pub ignore_scripts: bool,
    /// Timeout for script execution
    pub timeout: Option<Duration>,
    /// Additional environment variables
    pub env: HashMap<String, String>,
    /// Whether to capture output (vs streaming)
    pub capture_output: bool,
    /// Shell to use for execution
    pub shell: Option<String>,
}

impl Default for ScriptConfig {
    fn default() -> Self {
        Self {
            ignore_scripts: false,
            timeout: Some(Duration::from_secs(300)), // 5 minute default
            env: HashMap::new(),
            capture_output: true,
            shell: None,
        }
    }
}

/// Executor for npm lifecycle scripts
#[derive(Clone)]
pub struct ScriptExecutor {
    /// Configuration
    config: ScriptConfig,
    /// Node modules bin directory
    node_modules_bin: Option<PathBuf>,
}

impl ScriptExecutor {
    /// Create a new script executor
    pub fn new() -> Self {
        Self {
            config: ScriptConfig::default(),
            node_modules_bin: None,
        }
    }

    /// Create with custom configuration
    pub fn with_config(config: ScriptConfig) -> Self {
        Self {
            config,
            node_modules_bin: None,
        }
    }

    /// Set the node_modules/.bin directory for PATH
    pub fn set_node_modules_bin(&mut self, path: impl AsRef<Path>) {
        self.node_modules_bin = Some(path.as_ref().to_path_buf());
    }

    /// Execute a single lifecycle script
    pub fn execute_script(
        &self,
        script: LifecycleScript,
        package_name: &str,
        package_dir: &Path,
        scripts: &HashMap<String, String>,
    ) -> Result<ScriptResult> {
        let script_name = script.as_str();

        // Check if script is defined
        let script_cmd = match scripts.get(script_name) {
            Some(cmd) => cmd,
            None => {
                return Ok(ScriptResult {
                    script,
                    package: package_name.to_string(),
                    exit_code: 0,
                    stdout: String::new(),
                    stderr: String::new(),
                    duration: Duration::ZERO,
                    skipped: true,
                });
            }
        };

        // Skip if ignore_scripts is set
        if self.config.ignore_scripts {
            return Ok(ScriptResult {
                script,
                package: package_name.to_string(),
                exit_code: 0,
                stdout: String::new(),
                stderr: String::new(),
                duration: Duration::ZERO,
                skipped: true,
            });
        }

        let start = Instant::now();
        let output = self.run_command(script_cmd, package_name, package_dir)?;
        let duration = start.elapsed();

        Ok(ScriptResult {
            script,
            package: package_name.to_string(),
            exit_code: output.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            duration,
            skipped: false,
        })
    }

    /// Execute all install lifecycle scripts for a package
    pub fn execute_install_scripts(
        &self,
        package_name: &str,
        package_dir: &Path,
        scripts: &HashMap<String, String>,
    ) -> Result<Vec<ScriptResult>> {
        let mut results = Vec::new();

        for &script in LifecycleScript::install_order() {
            let result = self.execute_script(script, package_name, package_dir, scripts)?;
            let success = result.success();
            results.push(result);

            // Stop on failure unless ignore_scripts is set
            if !success && !self.config.ignore_scripts {
                break;
            }
        }

        Ok(results)
    }

    /// Execute scripts for a git dependency (includes prepare)
    pub fn execute_git_dep_scripts(
        &self,
        package_name: &str,
        package_dir: &Path,
        scripts: &HashMap<String, String>,
    ) -> Result<Vec<ScriptResult>> {
        let mut results = Vec::new();

        for &script in LifecycleScript::git_dep_order() {
            let result = self.execute_script(script, package_name, package_dir, scripts)?;
            let success = result.success();
            results.push(result);

            if !success && !self.config.ignore_scripts {
                break;
            }
        }

        Ok(results)
    }

    /// Run a shell command in the package directory
    fn run_command(&self, cmd: &str, package_name: &str, package_dir: &Path) -> Result<Output> {
        let shell = self.get_shell();
        let shell_args = self.get_shell_args();

        let mut command = Command::new(&shell);
        command.args(&shell_args).arg(cmd).current_dir(package_dir);

        // Set up environment
        self.setup_environment(&mut command, package_name, package_dir);

        // Configure output capture
        if self.config.capture_output {
            command.stdout(Stdio::piped());
            command.stderr(Stdio::piped());
        }

        let output = command.output().map_err(|e| dx_pkg_core::Error::Io {
            message: format!("Failed to execute script: {}", e),
            path: Some(package_dir.to_path_buf()),
        })?;

        Ok(output)
    }

    /// Get the shell to use for script execution
    fn get_shell(&self) -> String {
        if let Some(ref shell) = self.config.shell {
            return shell.clone();
        }

        #[cfg(windows)]
        {
            std::env::var("COMSPEC").unwrap_or_else(|_| "cmd.exe".to_string())
        }

        #[cfg(not(windows))]
        {
            std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string())
        }
    }

    /// Get shell arguments for command execution
    fn get_shell_args(&self) -> Vec<&'static str> {
        #[cfg(windows)]
        {
            vec!["/d", "/s", "/c"]
        }

        #[cfg(not(windows))]
        {
            vec!["-c"]
        }
    }

    /// Set up environment variables for script execution
    fn setup_environment(&self, command: &mut Command, package_name: &str, package_dir: &Path) {
        // Inherit current environment
        command.envs(std::env::vars());

        // Add npm_* environment variables
        command.env("npm_package_name", package_name);
        command.env("npm_lifecycle_event", "install");
        command.env("npm_node_execpath", self.get_node_path());

        // Set npm_config_* variables
        command.env("npm_config_user_agent", "dx-pkg/0.1.0");

        // Add package directory to npm_package_json
        let package_json = package_dir.join("package.json");
        if package_json.exists() {
            command.env("npm_package_json", package_json);
        }

        // Add node_modules/.bin to PATH
        if let Some(ref bin_dir) = self.node_modules_bin {
            let path = std::env::var("PATH").unwrap_or_default();
            let new_path = format!("{}{}{}", bin_dir.display(), path_separator(), path);
            command.env("PATH", new_path);
        }

        // Add custom environment variables
        for (key, value) in &self.config.env {
            command.env(key, value);
        }
    }

    /// Get the path to the Node.js executable
    fn get_node_path(&self) -> String {
        std::env::var("NODE").unwrap_or_else(|_| {
            #[cfg(windows)]
            {
                "node.exe".to_string()
            }
            #[cfg(not(windows))]
            {
                "node".to_string()
            }
        })
    }
}

impl Default for ScriptExecutor {
    fn default() -> Self {
        Self::new()
    }
}

/// Get the PATH separator for the current platform
fn path_separator() -> &'static str {
    #[cfg(windows)]
    {
        ";"
    }
    #[cfg(not(windows))]
    {
        ":"
    }
}

/// Parse scripts from package.json content
pub fn parse_scripts(package_json: &str) -> Result<HashMap<String, String>> {
    let json: serde_json::Value =
        serde_json::from_str(package_json).map_err(|e| dx_pkg_core::Error::Parse {
            message: format!("Failed to parse package.json: {}", e),
            file: None,
            line: None,
            column: None,
        })?;

    let mut scripts = HashMap::new();

    if let Some(scripts_obj) = json.get("scripts").and_then(|s| s.as_object()) {
        for (name, value) in scripts_obj {
            if let Some(cmd) = value.as_str() {
                scripts.insert(name.clone(), cmd.to_string());
            }
        }
    }

    Ok(scripts)
}

/// Statistics for script execution
#[derive(Debug, Clone, Default)]
pub struct ScriptStats {
    /// Total scripts executed
    pub total: usize,
    /// Scripts that succeeded
    pub succeeded: usize,
    /// Scripts that failed
    pub failed: usize,
    /// Scripts that were skipped
    pub skipped: usize,
    /// Total execution time
    pub total_duration: Duration,
}

impl ScriptStats {
    /// Create stats from a list of results
    pub fn from_results(results: &[ScriptResult]) -> Self {
        let mut stats = Self::default();

        for result in results {
            stats.total += 1;
            stats.total_duration += result.duration;

            if result.skipped {
                stats.skipped += 1;
            } else if result.success() {
                stats.succeeded += 1;
            } else {
                stats.failed += 1;
            }
        }

        stats
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_lifecycle_script_names() {
        assert_eq!(LifecycleScript::Preinstall.as_str(), "preinstall");
        assert_eq!(LifecycleScript::Install.as_str(), "install");
        assert_eq!(LifecycleScript::Postinstall.as_str(), "postinstall");
        assert_eq!(LifecycleScript::Prepare.as_str(), "prepare");
    }

    #[test]
    fn test_install_order() {
        let order = LifecycleScript::install_order();
        assert_eq!(order.len(), 3);
        assert_eq!(order[0], LifecycleScript::Preinstall);
        assert_eq!(order[1], LifecycleScript::Install);
        assert_eq!(order[2], LifecycleScript::Postinstall);
    }

    #[test]
    fn test_git_dep_order() {
        let order = LifecycleScript::git_dep_order();
        assert_eq!(order.len(), 4);
        assert!(order.contains(&LifecycleScript::Prepare));
    }

    #[test]
    fn test_parse_scripts() {
        let package_json = r#"{
            "name": "test-package",
            "scripts": {
                "preinstall": "echo preinstall",
                "install": "echo install",
                "postinstall": "echo postinstall",
                "test": "jest"
            }
        }"#;

        let scripts = parse_scripts(package_json).unwrap();
        assert_eq!(scripts.get("preinstall"), Some(&"echo preinstall".to_string()));
        assert_eq!(scripts.get("install"), Some(&"echo install".to_string()));
        assert_eq!(scripts.get("postinstall"), Some(&"echo postinstall".to_string()));
        assert_eq!(scripts.get("test"), Some(&"jest".to_string()));
    }

    #[test]
    fn test_parse_scripts_empty() {
        let package_json = r#"{"name": "test-package"}"#;
        let scripts = parse_scripts(package_json).unwrap();
        assert!(scripts.is_empty());
    }

    #[test]
    fn test_script_executor_skips_undefined() {
        let executor = ScriptExecutor::new();
        let scripts = HashMap::new();
        let temp_dir = TempDir::new().unwrap();

        let result = executor
            .execute_script(LifecycleScript::Preinstall, "test-pkg", temp_dir.path(), &scripts)
            .unwrap();

        assert!(result.skipped);
        assert!(result.success());
    }

    #[test]
    fn test_script_executor_with_ignore_scripts() {
        let config = ScriptConfig {
            ignore_scripts: true,
            ..Default::default()
        };
        let executor = ScriptExecutor::with_config(config);

        let mut scripts = HashMap::new();
        scripts.insert("preinstall".to_string(), "echo hello".to_string());

        let temp_dir = TempDir::new().unwrap();
        let result = executor
            .execute_script(LifecycleScript::Preinstall, "test-pkg", temp_dir.path(), &scripts)
            .unwrap();

        assert!(result.skipped);
    }

    #[test]
    fn test_script_config_default() {
        let config = ScriptConfig::default();
        assert!(!config.ignore_scripts);
        assert!(config.timeout.is_some());
        assert!(config.capture_output);
    }

    #[test]
    fn test_script_stats_from_results() {
        let results = vec![
            ScriptResult {
                script: LifecycleScript::Preinstall,
                package: "test".to_string(),
                exit_code: 0,
                stdout: String::new(),
                stderr: String::new(),
                duration: Duration::from_millis(100),
                skipped: false,
            },
            ScriptResult {
                script: LifecycleScript::Install,
                package: "test".to_string(),
                exit_code: 0,
                stdout: String::new(),
                stderr: String::new(),
                duration: Duration::from_millis(50),
                skipped: true,
            },
            ScriptResult {
                script: LifecycleScript::Postinstall,
                package: "test".to_string(),
                exit_code: 1,
                stdout: String::new(),
                stderr: "error".to_string(),
                duration: Duration::from_millis(200),
                skipped: false,
            },
        ];

        let stats = ScriptStats::from_results(&results);
        assert_eq!(stats.total, 3);
        assert_eq!(stats.succeeded, 1);
        assert_eq!(stats.failed, 1);
        assert_eq!(stats.skipped, 1);
        assert_eq!(stats.total_duration, Duration::from_millis(350));
    }

    #[test]
    fn test_execute_simple_script() {
        let executor = ScriptExecutor::new();
        let temp_dir = TempDir::new().unwrap();

        // Create a simple script
        let mut scripts = HashMap::new();
        #[cfg(windows)]
        scripts.insert("preinstall".to_string(), "echo hello".to_string());
        #[cfg(not(windows))]
        scripts.insert("preinstall".to_string(), "echo hello".to_string());

        let result = executor
            .execute_script(LifecycleScript::Preinstall, "test-pkg", temp_dir.path(), &scripts)
            .unwrap();

        assert!(!result.skipped);
        assert!(result.success());
        assert!(result.stdout.contains("hello"));
    }

    #[test]
    fn test_execute_failing_script() {
        let executor = ScriptExecutor::new();
        let temp_dir = TempDir::new().unwrap();

        let mut scripts = HashMap::new();
        #[cfg(windows)]
        scripts.insert("preinstall".to_string(), "exit 1".to_string());
        #[cfg(not(windows))]
        scripts.insert("preinstall".to_string(), "exit 1".to_string());

        let result = executor
            .execute_script(LifecycleScript::Preinstall, "test-pkg", temp_dir.path(), &scripts)
            .unwrap();

        assert!(!result.skipped);
        assert!(!result.success());
        assert_eq!(result.exit_code, 1);
    }
}

#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;
    use std::collections::HashMap;
    use tempfile::TempDir;

    /// Generate valid package names
    fn arb_package_name() -> impl Strategy<Value = String> {
        "[a-z][a-z0-9-]{0,20}".prop_map(|s| s.to_string())
    }

    /// Generate valid script commands (simple echo commands)
    fn arb_script_command() -> impl Strategy<Value = String> {
        "[a-zA-Z0-9 ]{1,20}".prop_map(|s| format!("echo {}", s))
    }

    /// Generate a scripts map
    fn arb_scripts_map() -> impl Strategy<Value = HashMap<String, String>> {
        prop::collection::hash_map(
            prop::sample::select(vec![
                "preinstall".to_string(),
                "install".to_string(),
                "postinstall".to_string(),
                "prepare".to_string(),
            ]),
            arb_script_command(),
            0..4,
        )
    }

    proptest! {
        /// Property: Lifecycle scripts run in correct order
        /// For any package, preinstall runs before install, install before postinstall.
        ///
        /// **Validates: Requirements 8.1, 8.2, 8.5**
        #[test]
        fn prop_lifecycle_order_preserved(
            _package_name in arb_package_name()
        ) {
            let order = LifecycleScript::install_order();

            // Verify order is preinstall -> install -> postinstall
            prop_assert_eq!(order[0], LifecycleScript::Preinstall);
            prop_assert_eq!(order[1], LifecycleScript::Install);
            prop_assert_eq!(order[2], LifecycleScript::Postinstall);
        }

        /// Property: Skipped scripts always succeed
        /// For any undefined script, execution should return success with skipped=true.
        ///
        /// **Validates: Requirements 8.1**
        #[test]
        fn prop_undefined_scripts_skipped(
            package_name in arb_package_name()
        ) {
            let executor = ScriptExecutor::new();
            let scripts = HashMap::new(); // No scripts defined
            let temp_dir = TempDir::new().unwrap();

            for &script in LifecycleScript::install_order() {
                let result = executor
                    .execute_script(script, &package_name, temp_dir.path(), &scripts)
                    .unwrap();

                prop_assert!(result.skipped, "Undefined script should be skipped");
                prop_assert!(result.success(), "Skipped script should succeed");
            }
        }

        /// Property: ignore_scripts config skips all scripts
        /// When ignore_scripts is true, all scripts should be skipped.
        ///
        /// **Validates: Requirements 8.1**
        #[test]
        fn prop_ignore_scripts_skips_all(
            package_name in arb_package_name(),
            scripts in arb_scripts_map()
        ) {
            let config = ScriptConfig {
                ignore_scripts: true,
                ..Default::default()
            };
            let executor = ScriptExecutor::with_config(config);
            let temp_dir = TempDir::new().unwrap();

            for &script in LifecycleScript::install_order() {
                let result = executor
                    .execute_script(script, &package_name, temp_dir.path(), &scripts)
                    .unwrap();

                prop_assert!(result.skipped, "Script should be skipped when ignore_scripts=true");
            }
        }

        /// Property: Script stats are consistent
        /// Total = succeeded + failed + skipped
        ///
        /// **Validates: Requirements 8.5**
        #[test]
        fn prop_stats_consistency(
            num_succeeded in 0usize..10,
            num_failed in 0usize..10,
            num_skipped in 0usize..10
        ) {
            let mut results = Vec::new();

            // Add succeeded results
            for _ in 0..num_succeeded {
                results.push(ScriptResult {
                    script: LifecycleScript::Install,
                    package: "test".to_string(),
                    exit_code: 0,
                    stdout: String::new(),
                    stderr: String::new(),
                    duration: Duration::from_millis(10),
                    skipped: false,
                });
            }

            // Add failed results
            for _ in 0..num_failed {
                results.push(ScriptResult {
                    script: LifecycleScript::Install,
                    package: "test".to_string(),
                    exit_code: 1,
                    stdout: String::new(),
                    stderr: String::new(),
                    duration: Duration::from_millis(10),
                    skipped: false,
                });
            }

            // Add skipped results
            for _ in 0..num_skipped {
                results.push(ScriptResult {
                    script: LifecycleScript::Install,
                    package: "test".to_string(),
                    exit_code: 0,
                    stdout: String::new(),
                    stderr: String::new(),
                    duration: Duration::ZERO,
                    skipped: true,
                });
            }

            let stats = ScriptStats::from_results(&results);

            prop_assert_eq!(
                stats.total,
                stats.succeeded + stats.failed + stats.skipped,
                "Total should equal sum of succeeded, failed, and skipped"
            );
            prop_assert_eq!(stats.succeeded, num_succeeded);
            prop_assert_eq!(stats.failed, num_failed);
            prop_assert_eq!(stats.skipped, num_skipped);
        }

        /// Property: Parse scripts extracts all defined scripts
        /// For any valid package.json with scripts, all scripts should be extracted.
        ///
        /// **Validates: Requirements 8.1**
        #[test]
        fn prop_parse_scripts_extracts_all(
            scripts in arb_scripts_map()
        ) {
            // Build package.json
            let scripts_json: serde_json::Value = scripts.iter()
                .map(|(k, v)| (k.clone(), serde_json::Value::String(v.clone())))
                .collect();

            let package_json = serde_json::json!({
                "name": "test-package",
                "scripts": scripts_json
            });

            let parsed = parse_scripts(&package_json.to_string()).unwrap();

            // All scripts should be present
            for (name, cmd) in &scripts {
                prop_assert_eq!(
                    parsed.get(name),
                    Some(cmd),
                    "Script '{}' should be parsed correctly",
                    name
                );
            }
        }
    }
}
