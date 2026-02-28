//! Python script runner
//!
//! Implements `dx-py script.py` execution.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};
use std::time::{Duration, Instant};

use thiserror::Error;

/// Result of running a script
#[derive(Debug)]
pub struct ScriptResult {
    /// Exit code
    pub exit_code: i32,
    /// Standard output
    pub stdout: String,
    /// Standard error
    pub stderr: String,
    /// Execution duration
    pub duration: Duration,
    /// Script path
    pub script: PathBuf,
}

impl ScriptResult {
    /// Check if script succeeded
    pub fn success(&self) -> bool {
        self.exit_code == 0
    }
}

/// Script runner error
#[derive(Error, Debug)]
pub enum RunnerError {
    #[error("Script not found: {0}")]
    ScriptNotFound(PathBuf),
    #[error("Invalid script: {0}")]
    InvalidScript(String),
    #[error("Execution failed: {0}")]
    ExecutionFailed(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Timeout after {0:?}")]
    Timeout(Duration),
}

/// Python script runner
pub struct ScriptRunner {
    python_path: PathBuf,
    working_dir: Option<PathBuf>,
    env_vars: HashMap<String, String>,
    timeout: Option<Duration>,
    inherit_env: bool,
}

impl ScriptRunner {
    /// Create a new script runner
    pub fn new(python_path: PathBuf) -> Self {
        Self {
            python_path,
            working_dir: None,
            env_vars: HashMap::new(),
            timeout: None,
            inherit_env: true,
        }
    }

    /// Set working directory
    pub fn with_working_dir(mut self, dir: PathBuf) -> Self {
        self.working_dir = Some(dir);
        self
    }

    /// Set environment variable
    pub fn with_env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env_vars.insert(key.into(), value.into());
        self
    }

    /// Set multiple environment variables
    pub fn with_envs(mut self, vars: HashMap<String, String>) -> Self {
        self.env_vars.extend(vars);
        self
    }

    /// Set execution timeout
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Set whether to inherit parent environment
    pub fn inherit_env(mut self, inherit: bool) -> Self {
        self.inherit_env = inherit;
        self
    }

    /// Run a Python script
    pub fn run(&self, script: &Path, args: &[String]) -> Result<ScriptResult, RunnerError> {
        if !script.exists() {
            return Err(RunnerError::ScriptNotFound(script.to_path_buf()));
        }

        let start = Instant::now();
        
        let mut cmd = Command::new(&self.python_path);
        cmd.arg(script);
        cmd.args(args);
        
        if let Some(ref dir) = self.working_dir {
            cmd.current_dir(dir);
        }

        if !self.inherit_env {
            cmd.env_clear();
        }

        for (key, value) in &self.env_vars {
            cmd.env(key, value);
        }

        let output = cmd.output()?;
        let duration = start.elapsed();

        Ok(ScriptResult {
            exit_code: output.status.code().unwrap_or(1),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            duration,
            script: script.to_path_buf(),
        })
    }

    /// Run Python code directly
    pub fn run_code(&self, code: &str, args: &[String]) -> Result<ScriptResult, RunnerError> {
        let start = Instant::now();
        
        let mut cmd = Command::new(&self.python_path);
        cmd.args(["-c", code]);
        cmd.args(args);
        
        if let Some(ref dir) = self.working_dir {
            cmd.current_dir(dir);
        }

        if !self.inherit_env {
            cmd.env_clear();
        }

        for (key, value) in &self.env_vars {
            cmd.env(key, value);
        }

        let output = cmd.output()?;
        let duration = start.elapsed();

        Ok(ScriptResult {
            exit_code: output.status.code().unwrap_or(1),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            duration,
            script: PathBuf::from("<code>"),
        })
    }

    /// Run script interactively (with stdin/stdout connected)
    pub fn run_interactive(&self, script: &Path, args: &[String]) -> Result<ExitStatus, RunnerError> {
        if !script.exists() {
            return Err(RunnerError::ScriptNotFound(script.to_path_buf()));
        }

        let mut cmd = Command::new(&self.python_path);
        cmd.arg(script);
        cmd.args(args);
        cmd.stdin(Stdio::inherit());
        cmd.stdout(Stdio::inherit());
        cmd.stderr(Stdio::inherit());
        
        if let Some(ref dir) = self.working_dir {
            cmd.current_dir(dir);
        }

        if !self.inherit_env {
            cmd.env_clear();
        }

        for (key, value) in &self.env_vars {
            cmd.env(key, value);
        }

        let status = cmd.status()?;
        Ok(status)
    }

    /// Check if a script is valid Python
    pub fn validate_script(&self, script: &Path) -> Result<bool, RunnerError> {
        if !script.exists() {
            return Err(RunnerError::ScriptNotFound(script.to_path_buf()));
        }

        let script_str = script.to_string_lossy();
        let code = format!(
            "import ast; ast.parse(open('{}').read())",
            script_str.replace('\\', "\\\\").replace('\'', "\\'")
        );

        let output = Command::new(&self.python_path)
            .args(["-c", &code])
            .output()?;

        Ok(output.status.success())
    }

    /// Get Python version
    pub fn python_version(&self) -> Result<String, RunnerError> {
        let output = Command::new(&self.python_path)
            .args(["--version"])
            .output()?;

        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_script_result_success() {
        let result = ScriptResult {
            exit_code: 0,
            stdout: "Hello".to_string(),
            stderr: String::new(),
            duration: Duration::from_millis(100),
            script: PathBuf::from("test.py"),
        };
        
        assert!(result.success());
    }

    #[test]
    fn test_script_result_failure() {
        let result = ScriptResult {
            exit_code: 1,
            stdout: String::new(),
            stderr: "Error".to_string(),
            duration: Duration::from_millis(100),
            script: PathBuf::from("test.py"),
        };
        
        assert!(!result.success());
    }

    #[test]
    fn test_runner_error_display() {
        let err = RunnerError::ScriptNotFound(PathBuf::from("missing.py"));
        assert!(err.to_string().contains("missing.py"));
    }
}
