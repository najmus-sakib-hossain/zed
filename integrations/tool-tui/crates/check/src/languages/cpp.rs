//! C/C++ Language Handler
//!
//! This module provides formatting and linting support for C and C++ files
//! using clang-format for formatting and clang-tidy for linting.

use std::fs;
use std::path::Path;

use crate::languages::diagnostic::Diagnostic;
use crate::languages::external_tools::ExternalToolManager;
use crate::languages::{FileStatus, LanguageHandler};

/// C/C++ file extensions
const C_EXTENSIONS: &[&str] = &["c", "h"];
const CPP_EXTENSIONS: &[&str] = &["cpp", "cc", "cxx", "hpp", "hxx"];
const ALL_EXTENSIONS: &[&str] = &["c", "cpp", "cc", "cxx", "h", "hpp", "hxx"];

/// Config file names for clang-format
const CLANG_FORMAT_CONFIG_FILES: &[&str] = &[".clang-format", "_clang-format"];

/// C/C++ language handler
///
/// Supports `.c`, `.cpp`, `.cc`, `.cxx`, `.h`, `.hpp`, `.hxx` file extensions.
/// Uses clang-format for formatting and clang-tidy for linting.
pub struct CppHandler {
    /// Path to the clang-format executable (if found)
    clang_format_path: Option<std::path::PathBuf>,
    /// Path to the clang-tidy executable (if found)
    clang_tidy_path: Option<std::path::PathBuf>,
}

impl CppHandler {
    /// Create a new C/C++ handler
    #[must_use]
    pub fn new() -> Self {
        Self {
            clang_format_path: ExternalToolManager::find_tool("clang-format"),
            clang_tidy_path: ExternalToolManager::find_tool("clang-tidy"),
        }
    }

    /// Ensure clang-format is available, attempting installation if needed
    fn ensure_clang_format(&self) -> Result<std::path::PathBuf, Diagnostic> {
        if let Some(ref path) = self.clang_format_path {
            return Ok(path.clone());
        }

        // Try to install clang-format
        match ExternalToolManager::install_tool("clang-format") {
            Ok(path) => Ok(path),
            Err(e) => Err(Diagnostic::error(
                "",
                format!(
                    "clang-format is required for C/C++ formatting but was not found.\n\n{}",
                    e.instructions
                ),
                "tool/cpp",
            )),
        }
    }

    /// Ensure clang-tidy is available, attempting installation if needed
    fn ensure_clang_tidy(&self) -> Result<std::path::PathBuf, Diagnostic> {
        if let Some(ref path) = self.clang_tidy_path {
            return Ok(path.clone());
        }

        // Try to install clang-tidy
        match ExternalToolManager::install_tool("clang-tidy") {
            Ok(path) => Ok(path),
            Err(e) => Err(Diagnostic::error(
                "",
                format!(
                    "clang-tidy is required for C/C++ linting but was not found.\n\n{}",
                    e.instructions
                ),
                "tool/cpp",
            )),
        }
    }

    /// Determine if a file is a C file (vs C++)
    #[must_use]
    pub fn is_c_file(path: &Path) -> bool {
        path.extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| C_EXTENSIONS.contains(&ext))
    }

    /// Determine if a file is a C++ file
    #[must_use]
    pub fn is_cpp_file(path: &Path) -> bool {
        path.extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| CPP_EXTENSIONS.contains(&ext))
    }

    /// Get the appropriate language standard for a file
    ///
    /// Returns "c11" for C files and "c++17" for C++ files
    #[must_use]
    pub fn get_language_standard(path: &Path) -> &'static str {
        if Self::is_c_file(path) {
            "c11"
        } else {
            "c++17"
        }
    }

    /// Get the clang language flag for a file
    ///
    /// Returns "-std=c11" for C files and "-std=c++17" for C++ files
    #[must_use]
    pub fn get_std_flag(path: &Path) -> &'static str {
        if Self::is_c_file(path) {
            "-std=c11"
        } else {
            "-std=c++17"
        }
    }

    /// Detect the style to use for clang-format
    ///
    /// If a .clang-format config file exists in the project, returns "file".
    /// Otherwise, returns "Google" as the default style.
    fn detect_style(&self, path: &Path) -> String {
        if let Some(config_path) =
            ExternalToolManager::find_config_file(path, CLANG_FORMAT_CONFIG_FILES)
        {
            // Config file found, use "file" style which tells clang-format to use it
            format!("file:{}", config_path.display())
        } else {
            // No config file, use Google style as default
            "Google".to_string()
        }
    }

    /// Check if a config file exists for the given path
    #[must_use]
    pub fn has_config_file(path: &Path) -> bool {
        ExternalToolManager::find_config_file(path, CLANG_FORMAT_CONFIG_FILES).is_some()
    }

    /// Format C/C++ code using clang-format
    fn format_with_clang_format(&self, path: &Path, content: &str) -> Result<String, Diagnostic> {
        let clang_format_path = self.ensure_clang_format()?;
        let file_path_str = path.to_string_lossy().to_string();
        let style = self.detect_style(path);

        // Build arguments for clang-format
        let style_arg = format!("--style={style}");
        let assume_filename_arg = format!("--assume-filename={file_path_str}");

        let args = vec![style_arg.as_str(), assume_filename_arg.as_str()];

        match ExternalToolManager::run_tool_checked(&clang_format_path, &args, Some(content)) {
            Ok(formatted) => Ok(formatted),
            Err(stderr) => Err(Diagnostic::error(
                file_path_str,
                format!("clang-format failed: {stderr}"),
                "format/cpp",
            )),
        }
    }

    /// Lint C/C++ code using clang-tidy
    fn lint_with_clang_tidy(
        &self,
        path: &Path,
        content: &str,
    ) -> Result<Vec<Diagnostic>, Diagnostic> {
        let clang_tidy_path = self.ensure_clang_tidy()?;
        let file_path_str = path.to_string_lossy().to_string();
        let std_flag = Self::get_std_flag(path);

        // Create a temporary file for clang-tidy since it doesn't support stdin well
        let temp_dir = std::env::temp_dir();
        let temp_file_name = format!("dx_check_temp_{}", std::process::id());
        let temp_ext = path.extension().and_then(|e| e.to_str()).unwrap_or("cpp");
        let temp_path = temp_dir.join(format!("{temp_file_name}.{temp_ext}"));

        // Write content to temp file
        fs::write(&temp_path, content).map_err(|e| {
            Diagnostic::error(
                &file_path_str,
                format!("Failed to create temporary file for linting: {e}"),
                "io/cpp",
            )
        })?;

        // Build arguments for clang-tidy
        let temp_path_str = temp_path.to_string_lossy().to_string();
        let extra_arg = format!("--extra-arg={std_flag}");

        let args = vec![temp_path_str.as_str(), "--", extra_arg.as_str()];

        let result = ExternalToolManager::run_tool(&clang_tidy_path, &args, None);

        // Clean up temp file
        let _ = fs::remove_file(&temp_path);

        match result {
            Ok((stdout, stderr)) => {
                let mut diagnostics = Vec::new();

                // Parse clang-tidy output
                // Format: file:line:column: severity: message [check-name]
                let output = if stdout.is_empty() { &stderr } else { &stdout };

                for line in output.lines() {
                    if let Some(diag) = self.parse_clang_tidy_line(line, &file_path_str) {
                        diagnostics.push(diag);
                    }
                }

                Ok(diagnostics)
            }
            Err(e) => {
                Err(Diagnostic::error(file_path_str, format!("clang-tidy failed: {e}"), "lint/cpp"))
            }
        }
    }

    /// Parse a single line of clang-tidy output into a Diagnostic
    fn parse_clang_tidy_line(&self, line: &str, original_file: &str) -> Option<Diagnostic> {
        // Skip empty lines and non-diagnostic lines
        if line.is_empty() || !line.contains(": ") {
            return None;
        }

        // Format: file:line:column: severity: message [check-name]
        // Example: test.cpp:10:5: warning: unused variable 'x' [clang-diagnostic-unused-variable]

        let parts: Vec<&str> = line.splitn(4, ':').collect();
        if parts.len() < 4 {
            return None;
        }

        let line_num: usize = parts[1].trim().parse().ok()?;
        let column: usize = parts[2].trim().parse().ok()?;

        let rest = parts[3].trim();

        // Parse severity and message
        let (severity, message, rule) = if rest.starts_with("error:") {
            let msg = rest.strip_prefix("error:")?.trim();
            let (msg, rule) = Self::extract_rule(msg);
            (crate::languages::Severity::Error, msg, rule)
        } else if rest.starts_with("warning:") {
            let msg = rest.strip_prefix("warning:")?.trim();
            let (msg, rule) = Self::extract_rule(msg);
            (crate::languages::Severity::Warning, msg, rule)
        } else if rest.starts_with("note:") {
            let msg = rest.strip_prefix("note:")?.trim();
            let (msg, rule) = Self::extract_rule(msg);
            (crate::languages::Severity::Info, msg, rule)
        } else {
            return None;
        };

        let mut diag = Diagnostic::new(original_file, message, severity, "lint/cpp")
            .with_location(line_num, column);

        if let Some(r) = rule {
            diag = diag.with_rule(r);
        }

        Some(diag)
    }

    /// Extract the rule name from a clang-tidy message
    /// Messages often end with [rule-name]
    fn extract_rule(message: &str) -> (String, Option<String>) {
        if let Some(bracket_start) = message.rfind('[')
            && let Some(bracket_end) = message.rfind(']')
            && bracket_end > bracket_start
        {
            let rule = message[bracket_start + 1..bracket_end].to_string();
            let msg = message[..bracket_start].trim().to_string();
            return (msg, Some(rule));
        }
        (message.to_string(), None)
    }
}

impl Default for CppHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl LanguageHandler for CppHandler {
    fn extensions(&self) -> &[&str] {
        ALL_EXTENSIONS
    }

    fn format(&self, path: &Path, content: &str, write: bool) -> Result<FileStatus, Diagnostic> {
        let file_path_str = path.to_string_lossy().to_string();

        // Format the content
        let formatted = self.format_with_clang_format(path, content)?;

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
                    "io/cpp",
                )
            })?;
        }

        Ok(FileStatus::Changed)
    }

    fn lint(&self, path: &Path, content: &str) -> Result<Vec<Diagnostic>, Diagnostic> {
        self.lint_with_clang_tidy(path, content)
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
        "cpp"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cpp_handler_extensions() {
        let handler = CppHandler::new();
        let extensions = handler.extensions();
        assert!(extensions.contains(&"c"));
        assert!(extensions.contains(&"cpp"));
        assert!(extensions.contains(&"cc"));
        assert!(extensions.contains(&"cxx"));
        assert!(extensions.contains(&"h"));
        assert!(extensions.contains(&"hpp"));
        assert!(extensions.contains(&"hxx"));
    }

    #[test]
    fn test_cpp_handler_name() {
        let handler = CppHandler::new();
        assert_eq!(handler.name(), "cpp");
    }

    #[test]
    fn test_is_c_file() {
        assert!(CppHandler::is_c_file(Path::new("test.c")));
        assert!(CppHandler::is_c_file(Path::new("test.h")));
        assert!(!CppHandler::is_c_file(Path::new("test.cpp")));
        assert!(!CppHandler::is_c_file(Path::new("test.hpp")));
        assert!(!CppHandler::is_c_file(Path::new("test.txt")));
    }

    #[test]
    fn test_is_cpp_file() {
        assert!(CppHandler::is_cpp_file(Path::new("test.cpp")));
        assert!(CppHandler::is_cpp_file(Path::new("test.cc")));
        assert!(CppHandler::is_cpp_file(Path::new("test.cxx")));
        assert!(CppHandler::is_cpp_file(Path::new("test.hpp")));
        assert!(CppHandler::is_cpp_file(Path::new("test.hxx")));
        assert!(!CppHandler::is_cpp_file(Path::new("test.c")));
        assert!(!CppHandler::is_cpp_file(Path::new("test.h")));
    }

    #[test]
    fn test_get_language_standard_c() {
        assert_eq!(CppHandler::get_language_standard(Path::new("test.c")), "c11");
        assert_eq!(CppHandler::get_language_standard(Path::new("test.h")), "c11");
    }

    #[test]
    fn test_get_language_standard_cpp() {
        assert_eq!(CppHandler::get_language_standard(Path::new("test.cpp")), "c++17");
        assert_eq!(CppHandler::get_language_standard(Path::new("test.cc")), "c++17");
        assert_eq!(CppHandler::get_language_standard(Path::new("test.cxx")), "c++17");
        assert_eq!(CppHandler::get_language_standard(Path::new("test.hpp")), "c++17");
        assert_eq!(CppHandler::get_language_standard(Path::new("test.hxx")), "c++17");
    }

    #[test]
    fn test_get_std_flag_c() {
        assert_eq!(CppHandler::get_std_flag(Path::new("test.c")), "-std=c11");
        assert_eq!(CppHandler::get_std_flag(Path::new("test.h")), "-std=c11");
    }

    #[test]
    fn test_get_std_flag_cpp() {
        assert_eq!(CppHandler::get_std_flag(Path::new("test.cpp")), "-std=c++17");
        assert_eq!(CppHandler::get_std_flag(Path::new("test.cc")), "-std=c++17");
        assert_eq!(CppHandler::get_std_flag(Path::new("test.hpp")), "-std=c++17");
    }

    #[test]
    fn test_extract_rule_with_rule() {
        let (msg, rule) =
            CppHandler::extract_rule("unused variable 'x' [clang-diagnostic-unused-variable]");
        assert_eq!(msg, "unused variable 'x'");
        assert_eq!(rule, Some("clang-diagnostic-unused-variable".to_string()));
    }

    #[test]
    fn test_extract_rule_without_rule() {
        let (msg, rule) = CppHandler::extract_rule("simple error message");
        assert_eq!(msg, "simple error message");
        assert_eq!(rule, None);
    }

    #[test]
    fn test_extract_rule_malformed_brackets() {
        let (msg, rule) = CppHandler::extract_rule("message with [unclosed bracket");
        assert_eq!(msg, "message with [unclosed bracket");
        assert_eq!(rule, None);
    }

    #[test]
    fn test_parse_clang_tidy_line_warning() {
        let handler = CppHandler::new();
        let line = "test.cpp:10:5: warning: unused variable 'x' [clang-diagnostic-unused-variable]";
        let diag = handler.parse_clang_tidy_line(line, "test.cpp");

        assert!(diag.is_some());
        let diag = diag.unwrap();
        assert_eq!(diag.file_path, "test.cpp");
        assert_eq!(diag.line, Some(10));
        assert_eq!(diag.column, Some(5));
        assert_eq!(diag.severity, crate::languages::Severity::Warning);
        assert!(diag.message.contains("unused variable"));
        assert_eq!(diag.rule, Some("clang-diagnostic-unused-variable".to_string()));
    }

    #[test]
    fn test_parse_clang_tidy_line_error() {
        let handler = CppHandler::new();
        let line = "test.cpp:5:1: error: expected ';' after expression";
        let diag = handler.parse_clang_tidy_line(line, "test.cpp");

        assert!(diag.is_some());
        let diag = diag.unwrap();
        assert_eq!(diag.severity, crate::languages::Severity::Error);
    }

    #[test]
    fn test_parse_clang_tidy_line_note() {
        let handler = CppHandler::new();
        let line = "test.cpp:5:1: note: in expansion of macro";
        let diag = handler.parse_clang_tidy_line(line, "test.cpp");

        assert!(diag.is_some());
        let diag = diag.unwrap();
        assert_eq!(diag.severity, crate::languages::Severity::Info);
    }

    #[test]
    fn test_parse_clang_tidy_line_invalid() {
        let handler = CppHandler::new();

        // Empty line
        assert!(handler.parse_clang_tidy_line("", "test.cpp").is_none());

        // Non-diagnostic line
        assert!(handler.parse_clang_tidy_line("Some random text", "test.cpp").is_none());

        // Incomplete line
        assert!(handler.parse_clang_tidy_line("test.cpp:10", "test.cpp").is_none());
    }

    #[test]
    fn test_detect_style_no_config() {
        let handler = CppHandler::new();
        // For a path without a config file, should return "Google"
        let style = handler.detect_style(Path::new("/nonexistent/path/test.cpp"));
        assert_eq!(style, "Google");
    }

    #[test]
    fn test_has_config_file_false() {
        assert!(!CppHandler::has_config_file(Path::new("/nonexistent/path/test.cpp")));
    }
}

#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;

    /// Generator for C file extensions
    fn arb_c_extension() -> impl Strategy<Value = &'static str> {
        prop_oneof![Just("c"), Just("h"),]
    }

    /// Generator for C++ file extensions
    fn arb_cpp_extension() -> impl Strategy<Value = &'static str> {
        prop_oneof![
            Just("cpp"),
            Just("cc"),
            Just("cxx"),
            Just("hpp"),
            Just("hxx"),
        ]
    }

    /// Generator for all C/C++ file extensions
    fn arb_all_extension() -> impl Strategy<Value = &'static str> {
        prop_oneof![arb_c_extension(), arb_cpp_extension(),]
    }

    /// Generator for file names with C extensions
    fn arb_c_file_path() -> impl Strategy<Value = String> {
        ("[a-z][a-z0-9_]{0,10}", arb_c_extension())
            .prop_map(|(name, ext)| format!("{}.{}", name, ext))
    }

    /// Generator for file names with C++ extensions
    fn arb_cpp_file_path() -> impl Strategy<Value = String> {
        ("[a-z][a-z0-9_]{0,10}", arb_cpp_extension())
            .prop_map(|(name, ext)| format!("{}.{}", name, ext))
    }

    /// Generator for file names with any C/C++ extension
    fn arb_any_cpp_file_path() -> impl Strategy<Value = String> {
        ("[a-z][a-z0-9_]{0,10}", arb_all_extension())
            .prop_map(|(name, ext)| format!("{}.{}", name, ext))
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// **Feature: multi-language-formatter-linter, Property 10: Language Standard Selection**
        /// *For any* C file (.c, .h), the C11 standard SHALL be used.
        /// *For any* C++ file (.cpp, .cc, .cxx, .hpp, .hxx), the C++17 standard SHALL be used.
        /// **Validates: Requirements 3.7**
        #[test]
        fn prop_c_files_use_c11_standard(file_path in arb_c_file_path()) {
            let path = Path::new(&file_path);

            // C files should use C11 standard
            prop_assert_eq!(
                CppHandler::get_language_standard(path),
                "c11",
                "C files should use C11 standard, got different for: {}",
                file_path
            );

            // C files should use -std=c11 flag
            prop_assert_eq!(
                CppHandler::get_std_flag(path),
                "-std=c11",
                "C files should use -std=c11 flag, got different for: {}",
                file_path
            );

            // Verify it's detected as a C file
            prop_assert!(
                CppHandler::is_c_file(path),
                "File {} should be detected as a C file",
                file_path
            );

            // Verify it's NOT detected as a C++ file
            prop_assert!(
                !CppHandler::is_cpp_file(path),
                "File {} should NOT be detected as a C++ file",
                file_path
            );
        }

        /// **Feature: multi-language-formatter-linter, Property 10: Language Standard Selection (C++)**
        /// *For any* C++ file (.cpp, .cc, .cxx, .hpp, .hxx), the C++17 standard SHALL be used.
        /// **Validates: Requirements 3.7**
        #[test]
        fn prop_cpp_files_use_cpp17_standard(file_path in arb_cpp_file_path()) {
            let path = Path::new(&file_path);

            // C++ files should use C++17 standard
            prop_assert_eq!(
                CppHandler::get_language_standard(path),
                "c++17",
                "C++ files should use C++17 standard, got different for: {}",
                file_path
            );

            // C++ files should use -std=c++17 flag
            prop_assert_eq!(
                CppHandler::get_std_flag(path),
                "-std=c++17",
                "C++ files should use -std=c++17 flag, got different for: {}",
                file_path
            );

            // Verify it's detected as a C++ file
            prop_assert!(
                CppHandler::is_cpp_file(path),
                "File {} should be detected as a C++ file",
                file_path
            );

            // Verify it's NOT detected as a C file
            prop_assert!(
                !CppHandler::is_c_file(path),
                "File {} should NOT be detected as a C file",
                file_path
            );
        }

        /// **Feature: multi-language-formatter-linter, Property 10: Language Standard Selection (All)**
        /// *For any* C/C++ file, the handler SHALL correctly identify the language and standard.
        /// **Validates: Requirements 3.7**
        #[test]
        fn prop_all_cpp_files_have_correct_standard(file_path in arb_any_cpp_file_path()) {
            let path = Path::new(&file_path);
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");

            // Every file should be either C or C++, not both
            let is_c = CppHandler::is_c_file(path);
            let is_cpp = CppHandler::is_cpp_file(path);

            prop_assert!(
                is_c ^ is_cpp,  // XOR: exactly one should be true
                "File {} should be either C or C++, not both or neither (is_c={}, is_cpp={})",
                file_path, is_c, is_cpp
            );

            // Standard should match the file type
            let standard = CppHandler::get_language_standard(path);
            let std_flag = CppHandler::get_std_flag(path);

            if is_c {
                prop_assert_eq!(standard, "c11");
                prop_assert_eq!(std_flag, "-std=c11");
            } else {
                prop_assert_eq!(standard, "c++17");
                prop_assert_eq!(std_flag, "-std=c++17");
            }

            // Verify the extension is in the expected list
            prop_assert!(
                ALL_EXTENSIONS.contains(&ext),
                "Extension {} should be in ALL_EXTENSIONS",
                ext
            );
        }
    }
}

#[cfg(test)]
mod config_file_property_tests {
    use super::*;
    use proptest::prelude::*;
    use std::fs;
    use tempfile::TempDir;

    /// Generator for clang-format config file names
    fn arb_config_file_name() -> impl Strategy<Value = &'static str> {
        prop_oneof![Just(".clang-format"), Just("_clang-format"),]
    }

    /// Generator for C/C++ file extensions
    fn arb_cpp_extension() -> impl Strategy<Value = &'static str> {
        prop_oneof![
            Just("c"),
            Just("cpp"),
            Just("cc"),
            Just("cxx"),
            Just("h"),
            Just("hpp"),
            Just("hxx"),
        ]
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

        /// **Feature: multi-language-formatter-linter, Property 9: Config File Detection (C/C++)**
        /// *For any* project directory containing a formatter config file (.clang-format or _clang-format),
        /// the CppHandler SHALL detect and use that config for formatting style.
        /// **Validates: Requirements 3.6**
        #[test]
        fn prop_config_file_detection_when_present(
            config_name in arb_config_file_name(),
            file_name in arb_file_name(),
            ext in arb_cpp_extension(),
        ) {
            // Create a temporary directory structure
            let temp_dir = TempDir::new().expect("Failed to create temp dir");
            let temp_path = temp_dir.path();

            // Create the config file in the temp directory
            let config_path = temp_path.join(config_name);
            fs::write(&config_path, "# clang-format config\nBasedOnStyle: LLVM\n")
                .expect("Failed to write config file");

            // Create a source file path in the same directory
            let source_file = temp_path.join(format!("{}.{}", file_name, ext));

            // The handler should detect the config file
            let handler = CppHandler::new();
            let style = handler.detect_style(&source_file);

            // Style should reference the config file (starts with "file:")
            prop_assert!(
                style.starts_with("file:"),
                "When config file exists, style should start with 'file:', got: {}",
                style
            );

            // The style should contain the path to the config file
            prop_assert!(
                style.contains(config_name),
                "Style should reference the config file name '{}', got: {}",
                config_name, style
            );

            // has_config_file should return true
            prop_assert!(
                CppHandler::has_config_file(&source_file),
                "has_config_file should return true when config exists"
            );
        }

        /// **Feature: multi-language-formatter-linter, Property 9: Config File Detection (C/C++) - No Config**
        /// *For any* project directory WITHOUT a formatter config file,
        /// the CppHandler SHALL use the default "Google" style.
        /// **Validates: Requirements 3.6**
        #[test]
        fn prop_config_file_detection_when_absent(
            file_name in arb_file_name(),
            ext in arb_cpp_extension(),
        ) {
            // Create a temporary directory structure WITHOUT a config file
            let temp_dir = TempDir::new().expect("Failed to create temp dir");
            let temp_path = temp_dir.path();

            // Create a source file path (but no config file)
            let source_file = temp_path.join(format!("{}.{}", file_name, ext));

            // The handler should NOT detect a config file
            let handler = CppHandler::new();
            let style = handler.detect_style(&source_file);

            // Style should be "Google" (the default)
            prop_assert_eq!(
                style,
                "Google",
                "When no config file exists, style should be 'Google'"
            );

            // has_config_file should return false
            prop_assert!(
                !CppHandler::has_config_file(&source_file),
                "has_config_file should return false when no config exists"
            );
        }

        /// **Feature: multi-language-formatter-linter, Property 9: Config File Detection (C/C++) - Parent Directory**
        /// *For any* source file in a subdirectory, the CppHandler SHALL find config files
        /// in parent directories.
        /// **Validates: Requirements 3.6**
        #[test]
        fn prop_config_file_detection_in_parent_dir(
            config_name in arb_config_file_name(),
            file_name in arb_file_name(),
            ext in arb_cpp_extension(),
            subdir in arb_subdir_name(),
        ) {
            // Create a temporary directory structure
            let temp_dir = TempDir::new().expect("Failed to create temp dir");
            let temp_path = temp_dir.path();

            // Create the config file in the ROOT temp directory
            let config_path = temp_path.join(config_name);
            fs::write(&config_path, "# clang-format config\nBasedOnStyle: Google\n")
                .expect("Failed to write config file");

            // Create a subdirectory
            let sub_path = temp_path.join(&subdir);
            fs::create_dir_all(&sub_path).expect("Failed to create subdirectory");

            // Create a source file path in the SUBDIRECTORY
            let source_file = sub_path.join(format!("{}.{}", file_name, ext));

            // The handler should detect the config file in the parent directory
            let handler = CppHandler::new();
            let style = handler.detect_style(&source_file);

            // Style should reference the config file (starts with "file:")
            prop_assert!(
                style.starts_with("file:"),
                "When config file exists in parent, style should start with 'file:', got: {}",
                style
            );

            // has_config_file should return true
            prop_assert!(
                CppHandler::has_config_file(&source_file),
                "has_config_file should return true when config exists in parent directory"
            );
        }
    }
}
