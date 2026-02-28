//! Rust Language Handler
//!
//! This module provides formatting and linting support for Rust files
//! using rustfmt for formatting and cargo clippy for linting.

use std::fs;
use std::path::Path;

use crate::languages::diagnostic::Diagnostic;
use crate::languages::external_tools::ExternalToolManager;
use crate::languages::{FileStatus, LanguageHandler};

/// Rust file extensions
const RUST_EXTENSIONS: &[&str] = &["rs"];

/// Config file names for rustfmt
const RUSTFMT_CONFIG_FILES: &[&str] = &["rustfmt.toml", ".rustfmt.toml"];

/// Default Rust edition when no config is found
const DEFAULT_RUST_EDITION: &str = "2021";

/// Rust language handler
///
/// Supports `.rs` file extension.
/// Uses rustfmt for formatting and cargo clippy for linting.
pub struct RustHandler {
    /// Path to the rustfmt executable (if found)
    pub(crate) rustfmt_path: Option<std::path::PathBuf>,
    /// Path to the cargo executable (if found)
    cargo_path: Option<std::path::PathBuf>,
}

impl RustHandler {
    /// Create a new Rust handler
    #[must_use]
    pub fn new() -> Self {
        Self {
            rustfmt_path: ExternalToolManager::find_tool("rustfmt"),
            cargo_path: ExternalToolManager::find_tool("cargo"),
        }
    }

    /// Ensure rustfmt is available, attempting installation if needed
    fn ensure_rustfmt(&self) -> Result<std::path::PathBuf, Diagnostic> {
        if let Some(ref path) = self.rustfmt_path {
            return Ok(path.clone());
        }

        // Use tool_installer for automatic installation
        use crate::tool_installer::ToolRegistry;
        let registry = ToolRegistry::new();

        match registry.ensure_installed("rustfmt") {
            Ok(()) => {
                // Tool installed, find it now
                if let Some(path) = ExternalToolManager::find_tool("rustfmt") {
                    Ok(path)
                } else {
                    Err(Diagnostic::error(
                        "",
                        "rustfmt was installed but could not be found in PATH",
                        "tool/rust",
                    ))
                }
            }
            Err(e) => Err(Diagnostic::error(
                "",
                format!("rustfmt is required for Rust formatting but was not found.\n\n{}", e),
                "tool/rust",
            )),
        }
    }

    /// Ensure cargo is available for clippy
    fn ensure_cargo(&self) -> Result<std::path::PathBuf, Diagnostic> {
        if let Some(ref path) = self.cargo_path {
            return Ok(path.clone());
        }

        // Cargo should be available if Rust is installed
        Err(Diagnostic::error(
            "",
            "cargo is required for Rust linting but was not found.\n\n\
             Please install Rust from https://rustup.rs/",
            "tool/rust",
        ))
    }

    /// Detect the style/config to use for rustfmt
    ///
    /// If a rustfmt.toml or .rustfmt.toml config file exists in the project, returns the path.
    /// Otherwise, returns None (use default settings with edition 2021).
    fn detect_config(&self, path: &Path) -> Option<std::path::PathBuf> {
        ExternalToolManager::find_config_file(path, RUSTFMT_CONFIG_FILES)
    }

    /// Check if a config file exists for the given path
    #[must_use]
    pub fn has_config_file(path: &Path) -> bool {
        ExternalToolManager::find_config_file(path, RUSTFMT_CONFIG_FILES).is_some()
    }

    /// Get the config file names that rustfmt looks for
    #[must_use]
    pub fn config_file_names() -> &'static [&'static str] {
        RUSTFMT_CONFIG_FILES
    }

    /// Get the default Rust edition
    #[must_use]
    pub fn default_edition() -> &'static str {
        DEFAULT_RUST_EDITION
    }

    /// Format Rust code using rustfmt
    fn format_with_rustfmt(&self, path: &Path, content: &str) -> Result<String, Diagnostic> {
        let rustfmt_path = self.ensure_rustfmt()?;
        let file_path_str = path.to_string_lossy().to_string();

        // Build arguments for rustfmt
        let mut args = vec!["--emit", "stdout"];

        // If no config file exists, use default edition
        let edition_arg;
        if self.detect_config(path).is_none() {
            edition_arg = format!("--edition={DEFAULT_RUST_EDITION}");
            args.push(&edition_arg);
        }

        match ExternalToolManager::run_tool_checked(&rustfmt_path, &args, Some(content)) {
            Ok(formatted) => Ok(formatted),
            Err(stderr) => Err(Diagnostic::error(
                file_path_str,
                format!("rustfmt failed: {stderr}"),
                "format/rust",
            )),
        }
    }

    /// Lint Rust code using cargo clippy
    ///
    /// Note: clippy requires a Cargo project context, so we need to find the
    /// Cargo.toml and run clippy from there. For standalone files, we fall back
    /// to basic syntax checking via rustfmt --check.
    fn lint_with_clippy(&self, path: &Path, content: &str) -> Result<Vec<Diagnostic>, Diagnostic> {
        let file_path_str = path.to_string_lossy().to_string();

        // First, try to find a Cargo.toml to determine if we're in a Cargo project
        let cargo_toml = Self::find_cargo_toml(path);

        if let Some(cargo_toml_path) = cargo_toml {
            // We're in a Cargo project, use cargo clippy
            let cargo_path = match self.ensure_cargo() {
                Ok(p) => p,
                Err(_) => return self.lint_syntax_only(path, content),
            };

            let project_dir = match cargo_toml_path.parent() {
                Some(dir) => dir,
                None => return self.lint_syntax_only(path, content),
            };

            // Run cargo clippy from the project directory
            let result = Self::run_cargo_clippy(&cargo_path, project_dir);

            match result {
                Ok(output) => {
                    let diagnostics = self.parse_clippy_output(&output, &file_path_str);
                    Ok(diagnostics)
                }
                Err(_) => {
                    // Clippy failed, fall back to syntax checking
                    self.lint_syntax_only(path, content)
                }
            }
        } else {
            // Not in a Cargo project, fall back to syntax checking via rustfmt
            self.lint_syntax_only(path, content)
        }
    }

    /// Find Cargo.toml by walking up the directory tree
    fn find_cargo_toml(path: &Path) -> Option<std::path::PathBuf> {
        let mut current = if path.is_file() { path.parent()? } else { path };

        loop {
            let cargo_toml = current.join("Cargo.toml");
            if cargo_toml.exists() && cargo_toml.is_file() {
                return Some(cargo_toml);
            }

            match current.parent() {
                Some(parent) => current = parent,
                None => break,
            }
        }

        None
    }

    /// Run cargo clippy in a specific directory with JSON output
    fn run_cargo_clippy(cargo_path: &Path, project_dir: &Path) -> Result<String, String> {
        use std::process::Command;

        // Convert to absolute path to avoid "os error 123" on Windows
        let abs_project_dir = project_dir
            .canonicalize()
            .map_err(|e| format!("Failed to canonicalize project dir: {e}"))?;

        let output = Command::new(cargo_path)
            .args(["clippy", "--message-format=json", "--quiet"])
            .current_dir(&abs_project_dir)
            .output()
            .map_err(|e| format!("Failed to run cargo clippy: {e}"))?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        // Clippy outputs to stdout with JSON format
        if !stdout.is_empty() {
            Ok(stdout)
        } else if !stderr.is_empty() {
            Ok(stderr)
        } else {
            Ok(String::new())
        }
    }

    /// Parse clippy output into diagnostics
    fn parse_clippy_output(&self, output: &str, original_file: &str) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        // Try to parse as JSON first (new format)
        if let Ok(json_diagnostics) = self.parse_clippy_json(output, original_file) {
            return json_diagnostics;
        }

        // Fall back to line-by-line parsing for short format
        for line in output.lines() {
            if let Some(diag) = self.parse_clippy_line(line, original_file) {
                diagnostics.push(diag);
            }
        }

        diagnostics
    }

    /// Parse clippy JSON output into diagnostics
    pub(crate) fn parse_clippy_json(
        &self,
        output: &str,
        original_file: &str,
    ) -> Result<Vec<Diagnostic>, ()> {
        use serde_json::Value;

        let mut diagnostics = Vec::new();

        // Parse each line as a separate JSON object
        for line in output.lines() {
            if line.trim().is_empty() {
                continue;
            }

            let parsed: Value = serde_json::from_str(line).map_err(|_| ())?;

            // Only process compiler messages
            if parsed["reason"].as_str() != Some("compiler-message") {
                continue;
            }

            let message = &parsed["message"];
            if message.is_null() {
                continue;
            }

            // Extract message text
            let msg_text = message["message"].as_str().unwrap_or("Unknown error").to_string();

            // Extract severity level
            let level = message["level"].as_str().unwrap_or("warning");
            let severity = match level {
                "error" => crate::languages::Severity::Error,
                "warning" => crate::languages::Severity::Warning,
                "note" | "help" => crate::languages::Severity::Info,
                _ => crate::languages::Severity::Warning,
            };

            // Extract location information
            let spans = message["spans"].as_array();
            if let Some(spans) = spans
                && let Some(span) = spans.first()
            {
                let file_name = span["file_name"].as_str().unwrap_or("");
                let line_start = span["line_start"].as_u64().unwrap_or(1) as usize;
                let column_start = span["column_start"].as_u64().unwrap_or(1) as usize;

                // Use the original file path if the diagnostic is for that file
                let diag_file =
                    if file_name.ends_with(original_file) || original_file.ends_with(file_name) {
                        original_file.to_string()
                    } else {
                        file_name.to_string()
                    };

                let diag = Diagnostic::new(diag_file, msg_text, severity, "lint/rust")
                    .with_location(line_start, column_start);

                diagnostics.push(diag);
                continue;
            }

            // If no span information, create diagnostic without location
            diagnostics.push(Diagnostic::new(original_file, msg_text, severity, "lint/rust"));
        }

        Ok(diagnostics)
    }

    /// Parse a single line of clippy output into a Diagnostic (short format fallback)
    fn parse_clippy_line(&self, line: &str, original_file: &str) -> Option<Diagnostic> {
        // Skip empty lines and non-diagnostic lines
        if line.is_empty() {
            return None;
        }

        // Clippy short format: file:line:column: severity: message
        // Example: src/main.rs:10:5: warning: unused variable `x`

        // Check if line contains a file path pattern
        if !line.contains(": ") {
            return None;
        }

        let parts: Vec<&str> = line.splitn(4, ':').collect();
        if parts.len() < 4 {
            return None;
        }

        // Try to parse line and column numbers
        let line_num: usize = parts[1].trim().parse().ok()?;
        let column: usize = parts[2].trim().parse().ok()?;

        let rest = parts[3].trim();

        // Parse severity and message
        let (severity, message) = if rest.starts_with("error") {
            let msg = rest.strip_prefix("error")?.trim_start_matches(':').trim();
            (crate::languages::Severity::Error, msg.to_string())
        } else if rest.starts_with("warning") {
            let msg = rest.strip_prefix("warning")?.trim_start_matches(':').trim();
            (crate::languages::Severity::Warning, msg.to_string())
        } else if rest.starts_with("note") {
            let msg = rest.strip_prefix("note")?.trim_start_matches(':').trim();
            (crate::languages::Severity::Info, msg.to_string())
        } else {
            return None;
        };

        // Use the original file path if the diagnostic is for that file
        let file_path = parts[0].trim();
        let diag_file = if file_path.ends_with(original_file) || original_file.ends_with(file_path)
        {
            original_file.to_string()
        } else {
            file_path.to_string()
        };

        Some(
            Diagnostic::new(diag_file, message, severity, "lint/rust")
                .with_location(line_num, column),
        )
    }

    /// Lint syntax only using rustfmt --check
    fn lint_syntax_only(&self, path: &Path, content: &str) -> Result<Vec<Diagnostic>, Diagnostic> {
        let rustfmt_path = self.ensure_rustfmt()?;
        let file_path_str = path.to_string_lossy().to_string();

        // Build arguments for rustfmt --check
        let mut args = vec!["--check"];

        // If no config file exists, use default edition
        let edition_arg;
        if self.detect_config(path).is_none() {
            edition_arg = format!("--edition={DEFAULT_RUST_EDITION}");
            args.push(&edition_arg);
        }

        match ExternalToolManager::run_tool(&rustfmt_path, &args, Some(content)) {
            Ok((_, stderr)) => {
                // Parse any errors from stderr
                let diagnostics = self.parse_rustfmt_errors(&stderr, &file_path_str);
                Ok(diagnostics)
            }
            Err(e) => {
                // rustfmt failed, likely a syntax error
                Ok(vec![Diagnostic::error(
                    &file_path_str,
                    format!("Syntax error: {e}"),
                    "lint/rust",
                )])
            }
        }
    }

    /// Parse rustfmt error output into diagnostics
    fn parse_rustfmt_errors(&self, stderr: &str, file_path: &str) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        for line in stderr.lines() {
            if line.contains("error") || line.contains("Error") {
                diagnostics.push(Diagnostic::error(file_path, line.trim(), "lint/rust"));
            } else if line.contains("warning") || line.contains("Warning") {
                diagnostics.push(Diagnostic::warning(file_path, line.trim(), "lint/rust"));
            }
        }

        diagnostics
    }
}

impl Default for RustHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl LanguageHandler for RustHandler {
    fn extensions(&self) -> &[&str] {
        RUST_EXTENSIONS
    }

    fn format(&self, path: &Path, content: &str, write: bool) -> Result<FileStatus, Diagnostic> {
        let file_path_str = path.to_string_lossy().to_string();

        // Format the content
        let formatted = self.format_with_rustfmt(path, content)?;

        // Check if content changed
        if formatted == content {
            return Ok(FileStatus::Unchanged);
        }

        // Write if requested
        if write {
            fs::write(path, &formatted).map_err(|e| {
                Diagnostic::error(
                    &file_path_str,
                    format!("Failed to write formatted content: {e}"),
                    "io/rust",
                )
            })?;
        }

        Ok(FileStatus::Changed)
    }

    fn lint(&self, path: &Path, content: &str) -> Result<Vec<Diagnostic>, Diagnostic> {
        self.lint_with_clippy(path, content)
    }

    fn check(&self, path: &Path, content: &str, write: bool) -> Result<FileStatus, Diagnostic> {
        // First, lint the file
        let lint_diagnostics = self.lint(path, content)?;

        // If there are errors (not just warnings), report the first one
        let errors: Vec<_> = lint_diagnostics
            .iter()
            .filter(|d| d.severity == crate::languages::Severity::Error)
            .collect();

        if !errors.is_empty() {
            return Err(errors[0].clone());
        }

        // Then format
        self.format(path, content, write)
    }

    fn name(&self) -> &'static str {
        "rust"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rust_handler_extensions() {
        let handler = RustHandler::new();
        let extensions = handler.extensions();
        assert!(extensions.contains(&"rs"));
        assert_eq!(extensions.len(), 1);
    }

    #[test]
    fn test_rust_handler_name() {
        let handler = RustHandler::new();
        assert_eq!(handler.name(), "rust");
    }

    #[test]
    fn test_default_edition() {
        assert_eq!(RustHandler::default_edition(), "2021");
    }

    #[test]
    fn test_config_file_names() {
        let names = RustHandler::config_file_names();
        assert!(names.contains(&"rustfmt.toml"));
        assert!(names.contains(&".rustfmt.toml"));
    }

    #[test]
    fn test_has_config_file_false() {
        assert!(!RustHandler::has_config_file(Path::new("/nonexistent/path/test.rs")));
    }

    #[test]
    fn test_find_cargo_toml_not_found() {
        let result = RustHandler::find_cargo_toml(Path::new("/nonexistent/path/test.rs"));
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_clippy_line_warning() {
        let handler = RustHandler::new();
        let line = "src/main.rs:10:5: warning: unused variable `x`";
        let diag = handler.parse_clippy_line(line, "src/main.rs");

        assert!(diag.is_some());
        let diag = diag.unwrap();
        assert_eq!(diag.line, Some(10));
        assert_eq!(diag.column, Some(5));
        assert_eq!(diag.severity, crate::languages::Severity::Warning);
        assert!(diag.message.contains("unused variable"));
    }

    #[test]
    fn test_parse_clippy_line_error() {
        let handler = RustHandler::new();
        let line = "src/main.rs:5:1: error: expected `;`";
        let diag = handler.parse_clippy_line(line, "src/main.rs");

        assert!(diag.is_some());
        let diag = diag.unwrap();
        assert_eq!(diag.severity, crate::languages::Severity::Error);
    }

    #[test]
    fn test_parse_clippy_line_invalid() {
        let handler = RustHandler::new();

        // Empty line
        assert!(handler.parse_clippy_line("", "test.rs").is_none());

        // Non-diagnostic line
        assert!(handler.parse_clippy_line("Some random text", "test.rs").is_none());

        // Incomplete line
        assert!(handler.parse_clippy_line("test.rs:10", "test.rs").is_none());
    }

    #[test]
    fn test_parse_clippy_json_single_warning() {
        let handler = RustHandler::new();
        let json_output = r#"{"reason":"compiler-message","package_id":"test 0.1.0","manifest_path":"/path/Cargo.toml","target":{"kind":["bin"],"crate_types":["bin"],"name":"test","src_path":"/path/src/main.rs","edition":"2021","doc":true,"doctest":false,"test":false},"message":{"rendered":"warning: unused variable: `x`\n --> src/main.rs:2:9\n  |\n2 |     let x = 5;\n  |         ^ help: if this is intentional, prefix it with an underscore: `_x`\n  |\n  = note: `#[warn(unused_variables)]` on by default\n\n","children":[{"children":[],"code":null,"level":"note","message":"`#[warn(unused_variables)]` on by default","rendered":null,"spans":[]},{"children":[],"code":null,"level":"help","message":"if this is intentional, prefix it with an underscore","rendered":null,"spans":[{"byte_end":18,"byte_start":17,"column_end":10,"column_start":9,"expansion":null,"file_name":"src/main.rs","is_primary":true,"label":null,"line_end":2,"line_start":2,"suggested_replacement":"_x","suggestion_applicability":"MachineApplicable","text":[{"highlight_end":10,"highlight_start":9,"text":"    let x = 5;"}]}]}],"code":{"code":"unused_variables","explanation":null},"level":"warning","message":"unused variable: `x`","spans":[{"byte_end":18,"byte_start":17,"column_end":10,"column_start":9,"expansion":null,"file_name":"src/main.rs","is_primary":true,"label":"help: if this is intentional, prefix it with an underscore: `_x`","line_end":2,"line_start":2,"suggested_replacement":null,"suggestion_applicability":null,"text":[{"highlight_end":10,"highlight_start":9,"text":"    let x = 5;"}]}]}}"#;

        let diagnostics = handler.parse_clippy_json(json_output, "src/main.rs").unwrap();

        assert_eq!(diagnostics.len(), 1);
        let diag = &diagnostics[0];
        assert_eq!(diag.severity, crate::languages::Severity::Warning);
        assert!(diag.message.contains("unused variable"));
        assert_eq!(diag.line, Some(2));
        assert_eq!(diag.column, Some(9));
    }

    #[test]
    fn test_parse_clippy_json_error() {
        let handler = RustHandler::new();
        let json_output = r#"{"reason":"compiler-message","package_id":"test 0.1.0","manifest_path":"/path/Cargo.toml","target":{"kind":["bin"],"crate_types":["bin"],"name":"test","src_path":"/path/src/main.rs","edition":"2021","doc":true,"doctest":false,"test":false},"message":{"rendered":"error: expected `;`\n --> src/main.rs:2:14\n  |\n2 |     let x = 5\n  |              ^ help: add `;` here\n\n","children":[{"children":[],"code":null,"level":"help","message":"add `;` here","rendered":null,"spans":[{"byte_end":17,"byte_start":17,"column_end":14,"column_start":14,"expansion":null,"file_name":"src/main.rs","is_primary":true,"label":null,"line_end":2,"line_start":2,"suggested_replacement":";","suggestion_applicability":"MachineApplicable","text":[{"highlight_end":14,"highlight_start":14,"text":"    let x = 5"}]}]}],"code":null,"level":"error","message":"expected `;`","spans":[{"byte_end":17,"byte_start":17,"column_end":14,"column_start":14,"expansion":null,"file_name":"src/main.rs","is_primary":true,"label":"help: add `;` here","line_end":2,"line_start":2,"suggested_replacement":null,"suggestion_applicability":null,"text":[{"highlight_end":14,"highlight_start":14,"text":"    let x = 5"}]}]}}"#;

        let diagnostics = handler.parse_clippy_json(json_output, "src/main.rs").unwrap();

        assert_eq!(diagnostics.len(), 1);
        let diag = &diagnostics[0];
        assert_eq!(diag.severity, crate::languages::Severity::Error);
        assert!(diag.message.contains("expected"));
        assert_eq!(diag.line, Some(2));
        assert_eq!(diag.column, Some(14));
    }

    #[test]
    fn test_parse_clippy_json_multiple_messages() {
        let handler = RustHandler::new();
        let json_output = r#"{"reason":"compiler-message","message":{"level":"warning","message":"unused variable: `x`","spans":[{"file_name":"src/main.rs","line_start":2,"column_start":9,"line_end":2,"column_end":10}]}}
{"reason":"compiler-message","message":{"level":"warning","message":"unused variable: `y`","spans":[{"file_name":"src/main.rs","line_start":3,"column_start":9,"line_end":3,"column_end":10}]}}"#;

        let diagnostics = handler.parse_clippy_json(json_output, "src/main.rs").unwrap();

        assert_eq!(diagnostics.len(), 2);
        assert!(diagnostics[0].message.contains("unused variable"));
        assert!(diagnostics[1].message.contains("unused variable"));
    }

    #[test]
    fn test_parse_clippy_json_invalid() {
        let handler = RustHandler::new();

        // Invalid JSON
        let result = handler.parse_clippy_json("not json", "test.rs");
        assert!(result.is_err());

        // Empty string
        let result = handler.parse_clippy_json("", "test.rs");
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 0);
    }

    #[test]
    fn test_parse_clippy_json_non_compiler_message() {
        let handler = RustHandler::new();
        let json_output = r#"{"reason":"build-finished","success":true}"#;

        let diagnostics = handler.parse_clippy_json(json_output, "src/main.rs").unwrap();

        // Should skip non-compiler-message entries
        assert_eq!(diagnostics.len(), 0);
    }

    #[test]
    fn test_detect_config_no_config() {
        let handler = RustHandler::new();
        // For a path without a config file, should return None
        let config = handler.detect_config(Path::new("/nonexistent/path/test.rs"));
        assert!(config.is_none());
    }
}

#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;
    use std::fs;
    use tempfile::TempDir;

    /// Generator for rustfmt config file names
    fn arb_config_file_name() -> impl Strategy<Value = &'static str> {
        prop_oneof![Just("rustfmt.toml"), Just(".rustfmt.toml"),]
    }

    /// Generator for file names
    fn arb_file_name() -> impl Strategy<Value = String> {
        "[a-z][a-z0-9_]{0,10}".prop_map(String::from)
    }

    /// Generator for subdirectory names
    fn arb_subdir_name() -> impl Strategy<Value = String> {
        "[a-z][a-z0-9_]{0,5}".prop_map(String::from)
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// **Feature: multi-language-formatter-linter, Property 9: Config File Detection (Rust)**
        /// *For any* project directory containing a formatter config file (rustfmt.toml or .rustfmt.toml),
        /// the RustHandler SHALL detect and use that config for formatting style.
        /// **Validates: Requirements 5.6**
        #[test]
        fn prop_config_file_detection_when_present(
            config_name in arb_config_file_name(),
            file_name in arb_file_name(),
        ) {
            // Create a temporary directory structure
            let temp_dir = TempDir::new().expect("Failed to create temp dir");
            let temp_path = temp_dir.path();

            // Create the config file in the temp directory
            let config_path = temp_path.join(config_name);
            fs::write(&config_path, "# rustfmt config\nedition = \"2021\"\n")
                .expect("Failed to write config file");

            // Create a source file path in the same directory
            let source_file = temp_path.join(format!("{}.rs", file_name));

            // The handler should detect the config file
            let handler = RustHandler::new();
            let config = handler.detect_config(&source_file);

            // Config should be found
            prop_assert!(
                config.is_some(),
                "When config file exists, detect_config should return Some, got None for config: {}",
                config_name
            );

            // The config path should point to the correct file
            let config_path_found = config.unwrap();
            prop_assert!(
                config_path_found.ends_with(config_name),
                "Config path should end with '{}', got: {}",
                config_name,
                config_path_found.display()
            );

            // has_config_file should return true
            prop_assert!(
                RustHandler::has_config_file(&source_file),
                "has_config_file should return true when config exists"
            );
        }

        /// **Feature: multi-language-formatter-linter, Property 9: Config File Detection (Rust) - No Config**
        /// *For any* project directory WITHOUT a formatter config file,
        /// the RustHandler SHALL use the default edition 2021.
        /// **Validates: Requirements 5.6, 5.7**
        #[test]
        fn prop_config_file_detection_when_absent(
            file_name in arb_file_name(),
        ) {
            // Create a temporary directory structure WITHOUT a config file
            let temp_dir = TempDir::new().expect("Failed to create temp dir");
            let temp_path = temp_dir.path();

            // Create a source file path (but no config file)
            let source_file = temp_path.join(format!("{}.rs", file_name));

            // The handler should NOT detect a config file
            let handler = RustHandler::new();
            let config = handler.detect_config(&source_file);

            // Config should be None
            prop_assert!(
                config.is_none(),
                "When no config file exists, detect_config should return None"
            );

            // has_config_file should return false
            prop_assert!(
                !RustHandler::has_config_file(&source_file),
                "has_config_file should return false when no config exists"
            );

            // Default edition should be 2021
            prop_assert_eq!(
                RustHandler::default_edition(),
                "2021",
                "Default edition should be 2021"
            );
        }

        /// **Feature: multi-language-formatter-linter, Property 9: Config File Detection (Rust) - Parent Directory**
        /// *For any* source file in a subdirectory, the RustHandler SHALL find config files
        /// in parent directories.
        /// **Validates: Requirements 5.6**
        #[test]
        fn prop_config_file_detection_in_parent_dir(
            config_name in arb_config_file_name(),
            file_name in arb_file_name(),
            subdir in arb_subdir_name(),
        ) {
            // Create a temporary directory structure
            let temp_dir = TempDir::new().expect("Failed to create temp dir");
            let temp_path = temp_dir.path();

            // Create the config file in the ROOT temp directory
            let config_path = temp_path.join(config_name);
            fs::write(&config_path, "# rustfmt config\nedition = \"2021\"\nmax_width = 100\n")
                .expect("Failed to write config file");

            // Create a subdirectory
            let sub_path = temp_path.join(&subdir);
            fs::create_dir_all(&sub_path).expect("Failed to create subdirectory");

            // Create a source file path in the SUBDIRECTORY
            let source_file = sub_path.join(format!("{}.rs", file_name));

            // The handler should detect the config file in the parent directory
            let handler = RustHandler::new();
            let config = handler.detect_config(&source_file);

            // Config should be found
            prop_assert!(
                config.is_some(),
                "When config file exists in parent, detect_config should return Some"
            );

            // has_config_file should return true
            prop_assert!(
                RustHandler::has_config_file(&source_file),
                "has_config_file should return true when config exists in parent directory"
            );
        }

        /// **Feature: multi-language-formatter-linter, Property 9: Config File Detection (Rust) - Nested Subdirectories**
        /// *For any* source file in deeply nested subdirectories, the RustHandler SHALL find config files
        /// by walking up the directory tree.
        /// **Validates: Requirements 5.6**
        #[test]
        fn prop_config_file_detection_in_deeply_nested_dir(
            config_name in arb_config_file_name(),
            file_name in arb_file_name(),
            subdir1 in arb_subdir_name(),
            subdir2 in arb_subdir_name(),
        ) {
            // Create a temporary directory structure
            let temp_dir = TempDir::new().expect("Failed to create temp dir");
            let temp_path = temp_dir.path();

            // Create the config file in the ROOT temp directory
            let config_path = temp_path.join(config_name);
            fs::write(&config_path, "# rustfmt config\nedition = \"2021\"\n")
                .expect("Failed to write config file");

            // Create nested subdirectories
            let nested_path = temp_path.join(&subdir1).join(&subdir2);
            fs::create_dir_all(&nested_path).expect("Failed to create nested subdirectories");

            // Create a source file path in the NESTED SUBDIRECTORY
            let source_file = nested_path.join(format!("{}.rs", file_name));

            // The handler should detect the config file by walking up
            let handler = RustHandler::new();
            let config = handler.detect_config(&source_file);

            // Config should be found
            prop_assert!(
                config.is_some(),
                "When config file exists in ancestor directory, detect_config should return Some"
            );

            // has_config_file should return true
            prop_assert!(
                RustHandler::has_config_file(&source_file),
                "has_config_file should return true when config exists in ancestor directory"
            );
        }
    }
}
