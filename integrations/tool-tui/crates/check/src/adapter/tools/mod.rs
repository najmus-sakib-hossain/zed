//! Built-in Tool Adapters
//!
//! Implementations of `ToolAdapter` for common formatters and linters.

mod cpp;
mod go;
mod javascript;
mod kotlin;
mod php;
mod python;
mod ruby;
mod rust;

pub use cpp::{ClangFormatAdapter, ClangTidyAdapter};
pub use go::{GofmtAdapter, GolangciLintAdapter};
pub use javascript::{ESLintAdapter, PrettierAdapter};
pub use kotlin::KtlintAdapter;
pub use php::PhpCsAdapter;
pub use python::RuffAdapter;
pub use ruby::RubocopAdapter;
pub use rust::{ClippyAdapter, RustfmtAdapter};

use super::discovery::ToolDiscovery;
use super::traits::ToolError;
use crate::diagnostics::{Diagnostic, DiagnosticSeverity, Span};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Instant;

/// Common base for tool adapters
pub struct BaseAdapter {
    /// Tool discovery service
    discovery: ToolDiscovery,
    /// Cached executable path
    cached_path: parking_lot::RwLock<Option<PathBuf>>,
}

impl BaseAdapter {
    /// Create a new base adapter
    #[must_use]
    pub fn new() -> Self {
        Self {
            discovery: ToolDiscovery::new(),
            cached_path: parking_lot::RwLock::new(None),
        }
    }

    /// Get or discover the tool path
    pub fn get_path(&self, tool_name: &str) -> Option<PathBuf> {
        // Check cache first
        if let Some(path) = self.cached_path.read().clone() {
            return Some(path);
        }

        // Discover and cache
        if let Some(result) = self.discovery.discover(tool_name) {
            *self.cached_path.write() = Some(result.path.clone());
            Some(result.path)
        } else {
            None
        }
    }

    /// Check if tool is available
    pub fn is_available(&self, tool_name: &str) -> bool {
        self.get_path(tool_name).is_some()
    }

    /// Get tool version
    pub fn get_version(&self, tool_name: &str) -> Option<String> {
        self.discovery.discover(tool_name).and_then(|r| r.version)
    }

    /// Run a tool and capture output
    pub fn run_tool(
        &self,
        tool_name: &str,
        args: &[&str],
        stdin: Option<&[u8]>,
    ) -> Result<ToolOutput, ToolError> {
        let path = self.get_path(tool_name).ok_or_else(|| ToolError::not_found(tool_name))?;

        let start = Instant::now();

        let mut cmd = Command::new(&path);
        cmd.args(args)
            .stdin(if stdin.is_some() {
                Stdio::piped()
            } else {
                Stdio::null()
            })
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let mut child =
            cmd.spawn().map_err(|e| ToolError::execution_failed(tool_name, e.to_string()))?;

        // Write stdin if provided
        if let Some(input) = stdin {
            use std::io::Write;
            if let Some(mut stdin_pipe) = child.stdin.take() {
                stdin_pipe
                    .write_all(input)
                    .map_err(|e| ToolError::execution_failed(tool_name, e.to_string()))?;
            }
        }

        let output = child
            .wait_with_output()
            .map_err(|e| ToolError::execution_failed(tool_name, e.to_string()))?;

        let duration_ms = start.elapsed().as_millis() as u64;

        Ok(ToolOutput {
            stdout: output.stdout,
            stderr: output.stderr,
            exit_code: output.status.code().unwrap_or(-1),
            duration_ms,
        })
    }

    /// Run tool with file path argument
    pub fn run_on_file(
        &self,
        tool_name: &str,
        args: &[&str],
        file: &Path,
    ) -> Result<ToolOutput, ToolError> {
        let path = self.get_path(tool_name).ok_or_else(|| ToolError::not_found(tool_name))?;

        let start = Instant::now();

        let mut cmd = Command::new(&path);
        cmd.args(args).arg(file).stdout(Stdio::piped()).stderr(Stdio::piped());

        let output = cmd
            .output()
            .map_err(|e| ToolError::execution_failed(tool_name, e.to_string()))?;

        let duration_ms = start.elapsed().as_millis() as u64;

        Ok(ToolOutput {
            stdout: output.stdout,
            stderr: output.stderr,
            exit_code: output.status.code().unwrap_or(-1),
            duration_ms,
        })
    }
}

impl Default for BaseAdapter {
    fn default() -> Self {
        Self::new()
    }
}

/// Output from running a tool
#[derive(Debug, Clone)]
pub struct ToolOutput {
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub exit_code: i32,
    pub duration_ms: u64,
}

impl ToolOutput {
    /// Get stdout as string
    #[must_use]
    pub fn stdout_str(&self) -> String {
        String::from_utf8_lossy(&self.stdout).to_string()
    }

    /// Get stderr as string
    #[must_use]
    pub fn stderr_str(&self) -> String {
        String::from_utf8_lossy(&self.stderr).to_string()
    }

    /// Check if tool succeeded
    #[must_use]
    pub fn success(&self) -> bool {
        self.exit_code == 0
    }
}

/// Parse JSON diagnostics from tool output
pub fn parse_json_diagnostics(
    json: &str,
    file: &Path,
    tool_name: &str,
) -> Result<Vec<Diagnostic>, ToolError> {
    let value: serde_json::Value =
        serde_json::from_str(json).map_err(|e| ToolError::parse_error(tool_name, e.to_string()))?;

    let mut diagnostics = Vec::new();

    // Handle array of diagnostics
    if let Some(array) = value.as_array() {
        for item in array {
            if let Some(diag) = parse_single_diagnostic(item, file, tool_name) {
                diagnostics.push(diag);
            }
        }
    }
    // Handle object with diagnostics array
    else if let Some(obj) = value.as_object() {
        for key in ["diagnostics", "errors", "warnings", "issues", "messages"] {
            if let Some(array) = obj.get(key).and_then(|v| v.as_array()) {
                for item in array {
                    if let Some(diag) = parse_single_diagnostic(item, file, tool_name) {
                        diagnostics.push(diag);
                    }
                }
            }
        }
    }

    Ok(diagnostics)
}

/// Parse a single diagnostic from JSON
fn parse_single_diagnostic(
    value: &serde_json::Value,
    file: &Path,
    tool_name: &str,
) -> Option<Diagnostic> {
    let obj = value.as_object()?;

    // Try various field names for message
    let message = obj
        .get("message")
        .or_else(|| obj.get("msg"))
        .or_else(|| obj.get("text"))
        .and_then(|v| v.as_str())
        .unwrap_or("Unknown issue")
        .to_string();

    // Try various field names for severity
    let severity = obj
        .get("severity")
        .or_else(|| obj.get("level"))
        .or_else(|| obj.get("type"))
        .and_then(|v| v.as_str())
        .map_or(DiagnosticSeverity::Warning, |s| match s.to_lowercase().as_str() {
            "error" | "err" | "e" | "fatal" => DiagnosticSeverity::Error,
            "warning" | "warn" | "w" => DiagnosticSeverity::Warning,
            "info" | "i" | "note" => DiagnosticSeverity::Info,
            _ => DiagnosticSeverity::Hint,
        });

    // Try various field names for line
    let line = obj
        .get("line")
        .or_else(|| obj.get("start_line"))
        .or_else(|| obj.get("location").and_then(|l| l.get("line")))
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(1) as u32;

    let column = obj
        .get("column")
        .or_else(|| obj.get("start_column"))
        .or_else(|| obj.get("col"))
        .or_else(|| obj.get("location").and_then(|l| l.get("column")))
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(1) as u32;

    let end_line = obj
        .get("end_line")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(u64::from(line)) as u32;

    let _end_column = obj
        .get("end_column")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(u64::from(column)) as u32;

    // Rule ID
    let rule_id = obj
        .get("code")
        .or_else(|| obj.get("rule"))
        .or_else(|| obj.get("rule_id"))
        .and_then(|v| v.as_str())
        .map_or_else(|| format!("{tool_name}/unknown"), |s| format!("{tool_name}/{s}"));

    // Suggestion/fix
    let suggestion = obj.get("fix").or_else(|| obj.get("suggestion")).and_then(|v| {
        if let Some(s) = v.as_str() {
            Some(s.to_string())
        } else if let Some(obj) = v.as_object() {
            obj.get("text")
                .or_else(|| obj.get("replacement"))
                .and_then(|t| t.as_str())
                .map(std::string::ToString::to_string)
        } else {
            None
        }
    });

    Some(Diagnostic {
        file: file.to_path_buf(),
        span: Span::new(line, end_line), // Using line numbers as span for now
        severity,
        rule_id,
        message,
        suggestion,
        related: Vec::new(),
        fix: None,
    })
}
