//! C/C++ Tool Adapters (clang-format, clang-tidy)

use super::BaseAdapter;
use crate::adapter::traits::{ToolAdapter, ToolCapabilities, ToolError, ToolErrorKind, ToolResult};
use crate::diagnostics::{Diagnostic, DiagnosticSeverity, Span};
use std::path::{Path, PathBuf};

/// Clang-format adapter for C/C++ formatting
pub struct ClangFormatAdapter {
    base: BaseAdapter,
}

impl ClangFormatAdapter {
    #[must_use]
    pub fn new() -> Self {
        Self {
            base: BaseAdapter::new(),
        }
    }
}

impl Default for ClangFormatAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl ToolAdapter for ClangFormatAdapter {
    fn name(&self) -> &'static str {
        "clang-format"
    }

    fn extensions(&self) -> &[&'static str] {
        &["c", "cc", "cpp", "cxx", "h", "hh", "hpp", "hxx", "m", "mm"]
    }

    fn capabilities(&self) -> ToolCapabilities {
        ToolCapabilities {
            can_format: true,
            can_lint: false,
            can_fix: false,
            supports_stdin: true,
            supports_config: true,
            supports_json_output: false,
            supports_caching: false,
        }
    }

    fn format(&self, path: &Path, content: &[u8]) -> Result<ToolResult, ToolError> {
        let output = self.base.run_tool(
            "clang-format",
            &["--assume-filename", &path.to_string_lossy()],
            Some(content),
        )?;

        let mut result = if output.success() {
            let formatted = output.stdout.clone();
            let changed = formatted != content;
            ToolResult::with_formatted(self.name(), formatted, changed)
        } else {
            let mut result = ToolResult::success(self.name());
            let stderr = output.stderr_str();
            if !stderr.is_empty() {
                result.add_diagnostic(Diagnostic {
                    file: path.to_path_buf(),
                    span: Span::new(1, 1),
                    severity: DiagnosticSeverity::Error,
                    rule_id: "clang-format/format-error".to_string(),
                    message: stderr,
                    suggestion: None,
                    related: Vec::new(),
                    fix: None,
                });
            }
            result.exit_code = output.exit_code;
            result
        };

        result.set_duration(output.duration_ms);
        Ok(result)
    }

    fn lint(&self, _path: &Path, _content: &[u8]) -> Result<ToolResult, ToolError> {
        Err(ToolError::new(
            ToolErrorKind::UnsupportedLanguage,
            "clang-format does not support linting, use clang-tidy instead",
        ))
    }

    fn is_available(&self) -> bool {
        self.base.is_available("clang-format")
    }

    fn version(&self) -> Option<String> {
        self.base.get_version("clang-format")
    }

    fn executable_path(&self) -> Option<PathBuf> {
        self.base.get_path("clang-format")
    }

    fn install_instructions(&self) -> &'static str {
        "Install LLVM/Clang from https://releases.llvm.org/\nOr: brew install clang-format (macOS)\nOr: apt install clang-format (Ubuntu)"
    }
}

/// Clang-tidy adapter for C/C++ linting
pub struct ClangTidyAdapter {
    base: BaseAdapter,
}

impl ClangTidyAdapter {
    #[must_use]
    pub fn new() -> Self {
        Self {
            base: BaseAdapter::new(),
        }
    }
}

impl Default for ClangTidyAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl ToolAdapter for ClangTidyAdapter {
    fn name(&self) -> &'static str {
        "clang-tidy"
    }

    fn extensions(&self) -> &[&'static str] {
        &["c", "cc", "cpp", "cxx", "h", "hh", "hpp", "hxx", "m", "mm"]
    }

    fn capabilities(&self) -> ToolCapabilities {
        ToolCapabilities {
            can_format: false,
            can_lint: true,
            can_fix: true,
            supports_stdin: false,
            supports_config: true,
            supports_json_output: false,
            supports_caching: false,
        }
    }

    fn format(&self, _path: &Path, _content: &[u8]) -> Result<ToolResult, ToolError> {
        Err(ToolError::new(
            ToolErrorKind::UnsupportedLanguage,
            "clang-tidy does not support formatting, use clang-format instead",
        ))
    }

    fn lint(&self, path: &Path, _content: &[u8]) -> Result<ToolResult, ToolError> {
        let output = self.base.run_on_file("clang-tidy", &["--quiet"], path)?;

        let mut result = ToolResult::success(self.name());
        result.set_duration(output.duration_ms);
        result.exit_code = output.exit_code;

        // Parse clang-tidy output (format: file:line:col: severity: message [check-name])
        let combined = format!("{}{}", output.stdout_str(), output.stderr_str());
        for line in combined.lines() {
            if let Some(diag) = parse_clang_tidy_line(line) {
                result.add_diagnostic(diag);
            }
        }

        Ok(result)
    }

    fn fix(&self, path: &Path, _content: &[u8]) -> Result<ToolResult, ToolError> {
        let output = self.base.run_on_file("clang-tidy", &["--fix", "--quiet"], path)?;

        let mut result = ToolResult::success(self.name());
        result.set_duration(output.duration_ms);
        result.exit_code = output.exit_code;
        result.changed = output.success();

        Ok(result)
    }

    fn is_available(&self) -> bool {
        self.base.is_available("clang-tidy")
    }

    fn version(&self) -> Option<String> {
        self.base.get_version("clang-tidy")
    }

    fn executable_path(&self) -> Option<PathBuf> {
        self.base.get_path("clang-tidy")
    }

    fn install_instructions(&self) -> &'static str {
        "Install LLVM/Clang from https://releases.llvm.org/\nOr: brew install llvm (macOS)\nOr: apt install clang-tidy (Ubuntu)"
    }
}

/// Parse clang-tidy output line
fn parse_clang_tidy_line(line: &str) -> Option<Diagnostic> {
    // Format: /path/to/file.cpp:42:13: warning: message [check-name]
    let re = regex::Regex::new(
        r"^(.+):(\d+):(\d+):\s*(error|warning|note):\s*(.+?)(?:\s*\[([^\]]+)\])?$",
    )
    .ok()?;

    let captures = re.captures(line)?;

    let file = PathBuf::from(&captures[1]);
    let line_num = captures[2].parse::<u32>().ok()?;
    let _col = captures[3].parse::<u32>().ok()?;
    let severity_str = &captures[4];
    let message = captures[5].trim();
    let check_name = captures.get(6).map_or("unknown", |m| m.as_str());

    let severity = match severity_str {
        "error" => DiagnosticSeverity::Error,
        "warning" => DiagnosticSeverity::Warning,
        "note" => DiagnosticSeverity::Info,
        _ => DiagnosticSeverity::Hint,
    };

    Some(Diagnostic {
        file,
        span: Span::new(line_num, line_num),
        severity,
        rule_id: format!("clang-tidy/{check_name}"),
        message: message.to_string(),
        suggestion: None,
        related: Vec::new(),
        fix: None,
    })
}
