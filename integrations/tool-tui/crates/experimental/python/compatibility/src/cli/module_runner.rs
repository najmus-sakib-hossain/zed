//! Python module runner
//!
//! Implements `dx-py -m module` execution.

use std::collections::HashMap;
use std::path::PathBuf;
use std::process::{Command, ExitStatus, Stdio};
use std::time::{Duration, Instant};

use thiserror::Error;

/// Result of running a module
#[derive(Debug)]
pub struct ModuleResult {
    /// Exit code
    pub exit_code: i32,
    /// Standard output
    pub stdout: String,
    /// Standard error
    pub stderr: String,
    /// Execution duration
    pub duration: Duration,
    /// Module name
    pub module: String,
}

impl ModuleResult {
    /// Check if module execution succeeded
    pub fn success(&self) -> bool {
        self.exit_code == 0
    }
}

/// Module runner error
#[derive(Error, Debug)]
pub enum ModuleError {
    #[error("Module not found: {0}")]
    ModuleNotFound(String),
    #[error("Execution failed: {0}")]
    ExecutionFailed(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Python module runner
pub struct ModuleRunner {
    python_path: PathBuf,
    working_dir: Option<PathBuf>,
    env_vars: HashMap<String, String>,
    python_path_additions: Vec<PathBuf>,
}

impl ModuleRunner {
    /// Create a new module runner
    pub fn new(python_path: PathBuf) -> Self {
        Self {
            python_path,
            working_dir: None,
            env_vars: HashMap::new(),
            python_path_additions: Vec::new(),
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

    /// Add path to PYTHONPATH
    pub fn with_python_path(mut self, path: PathBuf) -> Self {
        self.python_path_additions.push(path);
        self
    }

    /// Run a Python module
    pub fn run(&self, module: &str, args: &[String]) -> Result<ModuleResult, ModuleError> {
        let start = Instant::now();
        
        let mut cmd = Command::new(&self.python_path);
        cmd.args(["-m", module]);
        cmd.args(args);
        
        if let Some(ref dir) = self.working_dir {
            cmd.current_dir(dir);
        }

        // Set PYTHONPATH if we have additions
        if !self.python_path_additions.is_empty() {
            let sep = if cfg!(windows) { ";" } else { ":" };
            let paths: Vec<String> = self.python_path_additions
                .iter()
                .map(|p| p.to_string_lossy().to_string())
                .collect();
            let python_path = paths.join(sep);
            
            // Append to existing PYTHONPATH if present
            if let Ok(existing) = std::env::var("PYTHONPATH") {
                cmd.env("PYTHONPATH", format!("{}{}{}", python_path, sep, existing));
            } else {
                cmd.env("PYTHONPATH", python_path);
            }
        }

        for (key, value) in &self.env_vars {
            cmd.env(key, value);
        }

        let output = cmd.output()?;
        let duration = start.elapsed();

        Ok(ModuleResult {
            exit_code: output.status.code().unwrap_or(1),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            duration,
            module: module.to_string(),
        })
    }

    /// Run module interactively
    pub fn run_interactive(&self, module: &str, args: &[String]) -> Result<ExitStatus, ModuleError> {
        let mut cmd = Command::new(&self.python_path);
        cmd.args(["-m", module]);
        cmd.args(args);
        cmd.stdin(Stdio::inherit());
        cmd.stdout(Stdio::inherit());
        cmd.stderr(Stdio::inherit());
        
        if let Some(ref dir) = self.working_dir {
            cmd.current_dir(dir);
        }

        if !self.python_path_additions.is_empty() {
            let sep = if cfg!(windows) { ";" } else { ":" };
            let paths: Vec<String> = self.python_path_additions
                .iter()
                .map(|p| p.to_string_lossy().to_string())
                .collect();
            let python_path = paths.join(sep);
            cmd.env("PYTHONPATH", python_path);
        }

        for (key, value) in &self.env_vars {
            cmd.env(key, value);
        }

        let status = cmd.status()?;
        Ok(status)
    }

    /// Check if a module exists
    pub fn module_exists(&self, module: &str) -> bool {
        let code = format!("import importlib.util; print(importlib.util.find_spec('{}') is not None)", module);
        
        let output = Command::new(&self.python_path)
            .args(["-c", &code])
            .output();

        matches!(output, Ok(o) if o.status.success() && String::from_utf8_lossy(&o.stdout).trim() == "True")
    }

    /// Get module location
    pub fn module_location(&self, module: &str) -> Option<PathBuf> {
        let code = format!(
            "import importlib.util; spec = importlib.util.find_spec('{}'); print(spec.origin if spec and spec.origin else '')",
            module
        );
        
        let output = Command::new(&self.python_path)
            .args(["-c", &code])
            .output()
            .ok()?;

        if output.status.success() {
            let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !path.is_empty() {
                return Some(PathBuf::from(path));
            }
        }
        None
    }

    /// Run pytest
    pub fn run_pytest(&self, args: &[String]) -> Result<ModuleResult, ModuleError> {
        self.run("pytest", args)
    }

    /// Run venv
    pub fn run_venv(&self, args: &[String]) -> Result<ModuleResult, ModuleError> {
        self.run("venv", args)
    }

    /// Run pip
    pub fn run_pip(&self, args: &[String]) -> Result<ModuleResult, ModuleError> {
        self.run("pip", args)
    }

    /// Run http.server
    pub fn run_http_server(&self, args: &[String]) -> Result<ExitStatus, ModuleError> {
        self.run_interactive("http.server", args)
    }

    /// Run json.tool
    pub fn run_json_tool(&self, args: &[String]) -> Result<ModuleResult, ModuleError> {
        self.run("json.tool", args)
    }

    /// List available modules
    pub fn list_modules(&self) -> Result<Vec<String>, ModuleError> {
        let code = r#"
import pkgutil
import sys
modules = []
for importer, modname, ispkg in pkgutil.iter_modules():
    modules.append(modname)
for name in sys.builtin_module_names:
    modules.append(name)
print('\n'.join(sorted(set(modules))))
"#;

        let output = Command::new(&self.python_path)
            .args(["-c", code])
            .output()?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout)
                .lines()
                .map(|s| s.to_string())
                .collect())
        } else {
            Err(ModuleError::ExecutionFailed(
                String::from_utf8_lossy(&output.stderr).to_string()
            ))
        }
    }
}

/// Common module shortcuts
pub struct CommonModules;

impl CommonModules {
    /// Get list of common runnable modules
    pub fn list() -> Vec<(&'static str, &'static str)> {
        vec![
            ("pytest", "Run pytest test framework"),
            ("pip", "Package installer"),
            ("venv", "Create virtual environments"),
            ("http.server", "Simple HTTP server"),
            ("json.tool", "JSON formatter"),
            ("timeit", "Time execution of code"),
            ("cProfile", "Profile Python code"),
            ("pdb", "Python debugger"),
            ("doctest", "Run doctests"),
            ("unittest", "Run unit tests"),
            ("zipfile", "Work with ZIP archives"),
            ("tarfile", "Work with TAR archives"),
            ("base64", "Base64 encoding/decoding"),
            ("calendar", "Display calendar"),
            ("compileall", "Compile Python files"),
            ("dis", "Disassemble bytecode"),
            ("ensurepip", "Bootstrap pip"),
            ("idlelib", "IDLE IDE"),
            ("pydoc", "Documentation generator"),
            ("site", "Site configuration"),
            ("sysconfig", "System configuration"),
            ("trace", "Trace execution"),
            ("webbrowser", "Open web browser"),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_result_success() {
        let result = ModuleResult {
            exit_code: 0,
            stdout: "OK".to_string(),
            stderr: String::new(),
            duration: Duration::from_millis(100),
            module: "pytest".to_string(),
        };
        
        assert!(result.success());
    }

    #[test]
    fn test_common_modules_list() {
        let modules = CommonModules::list();
        assert!(!modules.is_empty());
        assert!(modules.iter().any(|(name, _)| *name == "pytest"));
        assert!(modules.iter().any(|(name, _)| *name == "pip"));
    }

    #[test]
    fn test_module_error_display() {
        let err = ModuleError::ModuleNotFound("nonexistent".to_string());
        assert!(err.to_string().contains("nonexistent"));
    }
}
