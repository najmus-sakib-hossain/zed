//! PHP Language Handler
//!
//! This module provides formatting and linting support for PHP files
//! using external PHP tools (php-cs-fixer, phpcs) or the mago toolchain.

use std::fs;
use std::path::Path;

use crate::languages::diagnostic::Diagnostic;
use crate::languages::external_tools::ExternalToolManager;
use crate::languages::{FileStatus, LanguageHandler};

/// PHP file extension
const PHP_EXTENSION: &str = "php";

/// Config file names for PHP-CS-Fixer
const PHP_CS_FIXER_CONFIG_FILES: &[&str] = &[
    ".php-cs-fixer.php",
    ".php-cs-fixer.dist.php",
    ".php_cs",
    ".php_cs.dist",
];

/// PHP language handler
///
/// Supports `.php` file extension.
/// Uses php-cs-fixer for formatting and phpcs/phpstan for linting.
pub struct PhpHandler {
    /// Path to the php-cs-fixer executable (if found)
    php_cs_fixer_path: Option<std::path::PathBuf>,
    /// Path to the php executable (if found)
    php_path: Option<std::path::PathBuf>,
}

impl PhpHandler {
    /// Create a new PHP handler
    #[must_use]
    pub fn new() -> Self {
        Self {
            php_cs_fixer_path: ExternalToolManager::find_tool("php-cs-fixer"),
            php_path: ExternalToolManager::find_tool("php"),
        }
    }

    /// Ensure php-cs-fixer is available
    fn ensure_php_cs_fixer(&self) -> Result<std::path::PathBuf, Diagnostic> {
        if let Some(ref path) = self.php_cs_fixer_path {
            return Ok(path.clone());
        }

        Err(Diagnostic::error(
            "php-cs-fixer-not-found",
            format!(
                "php-cs-fixer is required for PHP formatting but was not found.\n\n{}",
                Self::get_install_instructions()
            ),
            "tool/php",
        ))
    }

    /// Ensure PHP is available for syntax checking
    fn ensure_php(&self) -> Result<std::path::PathBuf, Diagnostic> {
        if let Some(ref path) = self.php_path {
            return Ok(path.clone());
        }

        Err(Diagnostic::error(
            "php-not-found",
            "PHP is required for syntax checking but was not found.\n\nPlease install PHP from https://www.php.net/downloads",
            "tool/php",
        ))
    }

    /// Get installation instructions for php-cs-fixer
    fn get_install_instructions() -> String {
        concat!(
            "To install php-cs-fixer:\n\n",
            "Option 1 - Composer (recommended):\n",
            "  composer global require friendsofphp/php-cs-fixer\n\n",
            "Option 2 - Download PHAR:\n",
            "  curl -L https://cs.symfony.com/download/php-cs-fixer-v3.phar -o php-cs-fixer\n",
            "  chmod +x php-cs-fixer\n",
            "  sudo mv php-cs-fixer /usr/local/bin/\n\n",
            "Option 3 - Homebrew (macOS):\n",
            "  brew install php-cs-fixer"
        )
        .to_string()
    }

    /// Detect the config file to use for php-cs-fixer
    fn detect_config(&self, path: &Path) -> Option<std::path::PathBuf> {
        ExternalToolManager::find_config_file(path, PHP_CS_FIXER_CONFIG_FILES)
    }

    /// Check if a config file exists for the given path
    #[must_use]
    pub fn has_config_file(path: &Path) -> bool {
        ExternalToolManager::find_config_file(path, PHP_CS_FIXER_CONFIG_FILES).is_some()
    }

    /// Get the config file names that php-cs-fixer looks for
    #[must_use]
    pub fn config_file_names() -> &'static [&'static str] {
        PHP_CS_FIXER_CONFIG_FILES
    }

    /// Format PHP code using php-cs-fixer
    fn format_with_php_cs_fixer(&self, path: &Path, content: &str) -> Result<String, Diagnostic> {
        let php_cs_fixer_path = self.ensure_php_cs_fixer()?;
        let file_path_str = path.to_string_lossy().to_string();

        // Create a temporary file for php-cs-fixer
        let temp_dir = std::env::temp_dir();
        let temp_file_name = format!("dx_check_php_temp_{}.php", std::process::id());
        let temp_path = temp_dir.join(&temp_file_name);

        // Write content to temp file
        fs::write(&temp_path, content).map_err(|e| {
            Diagnostic::error(
                &file_path_str,
                format!("Failed to create temporary file for formatting: {e}"),
                "io/php",
            )
        })?;

        // Build arguments for php-cs-fixer
        let temp_path_str = temp_path.to_string_lossy().to_string();
        let mut args = vec!["fix", &temp_path_str, "--using-cache=no", "--quiet"];

        // Add config file if found
        let config_arg;
        if let Some(config_path) = self.detect_config(path) {
            config_arg = format!("--config={}", config_path.display());
            args.push(&config_arg);
        }

        let result = ExternalToolManager::run_tool(&php_cs_fixer_path, &args, None);

        // Read the formatted content
        let formatted = fs::read_to_string(&temp_path).map_err(|e| {
            let _ = fs::remove_file(&temp_path);
            Diagnostic::error(
                &file_path_str,
                format!("Failed to read formatted content: {e}"),
                "io/php",
            )
        })?;

        // Clean up temp file
        let _ = fs::remove_file(&temp_path);

        match result {
            Ok(_) => Ok(formatted),
            Err(e) => {
                // php-cs-fixer might return non-zero even on success (when changes were made)
                // Check if the file was actually formatted
                if formatted == content {
                    Err(Diagnostic::error(
                        file_path_str,
                        format!("php-cs-fixer failed: {e}"),
                        "format/php",
                    ))
                } else {
                    Ok(formatted)
                }
            }
        }
    }

    /// Validate PHP syntax using php -l
    fn validate_syntax(&self, path: &Path, content: &str) -> Vec<Diagnostic> {
        let file_path_str = path.to_string_lossy().to_string();
        let mut diagnostics = Vec::new();

        // Try to get php path
        let php_path = match self.ensure_php() {
            Ok(path) => path,
            Err(diag) => {
                diagnostics.push(diag);
                return diagnostics;
            }
        };

        // Create a temporary file for php -l
        let temp_dir = std::env::temp_dir();
        let temp_file_name = format!("dx_check_php_lint_{}.php", std::process::id());
        let temp_path = temp_dir.join(&temp_file_name);

        // Write content to temp file
        if let Err(e) = fs::write(&temp_path, content) {
            diagnostics.push(Diagnostic::error(
                &file_path_str,
                format!("Failed to create temporary file for linting: {e}"),
                "io/php",
            ));
            return diagnostics;
        }

        // Run php -l to check syntax
        let temp_path_str = temp_path.to_string_lossy().to_string();
        let args = vec!["-l", &temp_path_str];

        match ExternalToolManager::run_tool(&php_path, &args, None) {
            Ok((stdout, stderr)) => {
                // Clean up temp file
                let _ = fs::remove_file(&temp_path);

                // Parse any errors from output
                let output = if stderr.is_empty() { &stdout } else { &stderr };

                for line in output.lines() {
                    if let Some(diag) = self.parse_php_error(line, &file_path_str) {
                        diagnostics.push(diag);
                    }
                }
            }
            Err(e) => {
                let _ = fs::remove_file(&temp_path);
                diagnostics.push(Diagnostic::error(
                    &file_path_str,
                    format!("Failed to run php -l: {e}"),
                    "lint/php",
                ));
            }
        }

        diagnostics
    }

    /// Detect framework from project structure
    #[must_use]
    pub fn detect_framework(&self, project_path: &Path) -> Option<String> {
        // Check for composer.json
        let composer_json = project_path.join("composer.json");
        if !composer_json.exists() {
            return None;
        }

        // Read composer.json
        if let Ok(content) = fs::read_to_string(&composer_json) {
            // Check for Laravel
            if content.contains("laravel/framework") || content.contains("illuminate") {
                return Some("laravel".to_string());
            }

            // Check for Symfony
            if content.contains("symfony/framework-bundle") || content.contains("symfony/console") {
                return Some("symfony".to_string());
            }

            // Check for WordPress
            if content.contains("wp-cli/wp-cli") || content.contains("johnpbloch/wordpress") {
                return Some("wordpress".to_string());
            }

            // Check for CakePHP
            if content.contains("cakephp/cakephp") {
                return Some("cakephp".to_string());
            }

            // Check for CodeIgniter
            if content.contains("codeigniter/framework") {
                return Some("codeigniter".to_string());
            }
        }

        None
    }

    /// Parse a PHP error line into a Diagnostic
    ///
    /// PHP error format: Parse error: syntax error, ... in /path/to/file.php on line N
    fn parse_php_error(&self, line: &str, original_file: &str) -> Option<Diagnostic> {
        // Skip empty lines and "No syntax errors" messages
        if line.trim().is_empty() || line.contains("No syntax errors") {
            return None;
        }

        // Format: Parse error: message in /path on line N
        // or: PHP Parse error: message in /path on line N
        if line.contains("Parse error") || line.contains("syntax error") {
            // Try to extract line number
            if let Some(line_pos) = line.rfind(" on line ") {
                let line_num_str = &line[line_pos + 9..];
                if let Ok(line_num) = line_num_str.trim().parse::<usize>() {
                    // Extract the error message
                    let message = if let Some(colon_pos) = line.find(": ") {
                        line[colon_pos + 2..line_pos].trim().to_string()
                    } else {
                        line.trim().to_string()
                    };

                    return Some(
                        Diagnostic::error(original_file, message, "lint/php").with_line(line_num),
                    );
                }
            }

            // Couldn't parse line number, return the whole message
            return Some(Diagnostic::error(original_file, line.trim(), "lint/php"));
        }

        // Check for other error types
        if line.contains("Fatal error") || line.contains("Warning") {
            return Some(Diagnostic::warning(original_file, line.trim(), "lint/php"));
        }

        None
    }
}

impl Default for PhpHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl LanguageHandler for PhpHandler {
    fn extensions(&self) -> &[&str] {
        &[PHP_EXTENSION]
    }

    fn format(&self, path: &Path, content: &str, write: bool) -> Result<FileStatus, Diagnostic> {
        let file_path_str = path.to_string_lossy().to_string();

        // Format the content
        let formatted = self.format_with_php_cs_fixer(path, content)?;

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
                    "io/php",
                )
            })?;
        }

        Ok(FileStatus::Changed)
    }

    fn lint(&self, path: &Path, content: &str) -> Result<Vec<Diagnostic>, Diagnostic> {
        // Validate syntax using php -l
        let diagnostics = self.validate_syntax(path, content);
        Ok(diagnostics)
    }

    fn check(&self, path: &Path, content: &str, write: bool) -> Result<FileStatus, Diagnostic> {
        // First, lint the file
        let lint_diagnostics = self.lint(path, content)?;

        // If there are syntax errors, report them
        if !lint_diagnostics.is_empty() {
            // Return the first error as the main error
            return Err(lint_diagnostics.into_iter().next().unwrap());
        }

        // Then format
        self.format(path, content, write)
    }

    fn name(&self) -> &'static str {
        "php"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_php_handler_extensions() {
        let handler = PhpHandler::new();
        let extensions = handler.extensions();
        assert!(extensions.contains(&"php"));
        assert_eq!(extensions.len(), 1);
    }

    #[test]
    fn test_php_handler_name() {
        let handler = PhpHandler::new();
        assert_eq!(handler.name(), "php");
    }

    #[test]
    fn test_config_file_names() {
        let names = PhpHandler::config_file_names();
        assert!(names.contains(&".php-cs-fixer.php"));
        assert!(names.contains(&".php-cs-fixer.dist.php"));
        assert!(names.contains(&".php_cs"));
        assert!(names.contains(&".php_cs.dist"));
    }

    #[test]
    fn test_has_config_file_false() {
        assert!(!PhpHandler::has_config_file(Path::new("/nonexistent/path/test.php")));
    }

    #[test]
    fn test_parse_php_error_with_line() {
        let handler = PhpHandler::new();
        let line = "Parse error: syntax error, unexpected '}' in /tmp/test.php on line 10";
        let diag = handler.parse_php_error(line, "test.php");

        assert!(diag.is_some());
        let diag = diag.unwrap();
        assert_eq!(diag.file_path, "test.php");
        assert_eq!(diag.line, Some(10));
        assert!(diag.message.contains("syntax error"));
        assert_eq!(diag.category, "lint/php");
    }

    #[test]
    fn test_parse_php_error_no_syntax_errors() {
        let handler = PhpHandler::new();
        let line = "No syntax errors detected in /tmp/test.php";
        let diag = handler.parse_php_error(line, "test.php");
        assert!(diag.is_none());
    }

    #[test]
    fn test_parse_php_error_empty_line() {
        let handler = PhpHandler::new();
        let diag = handler.parse_php_error("", "test.php");
        assert!(diag.is_none());
    }

    #[test]
    fn test_parse_php_error_whitespace_line() {
        let handler = PhpHandler::new();
        let diag = handler.parse_php_error("   ", "test.php");
        assert!(diag.is_none());
    }

    #[test]
    fn test_parse_php_error_fatal_error() {
        let handler = PhpHandler::new();
        let line = "Fatal error: Cannot redeclare function in test.php";
        let diag = handler.parse_php_error(line, "test.php");

        assert!(diag.is_some());
        let diag = diag.unwrap();
        assert_eq!(diag.severity, crate::languages::Severity::Warning);
    }

    #[test]
    fn test_get_install_instructions() {
        let instructions = PhpHandler::get_install_instructions();
        assert!(instructions.contains("composer"));
        assert!(instructions.contains("php-cs-fixer"));
    }
}

// TODO: Fix proptest string literal issues
/*
#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;

    /// Generator for valid PHP identifiers
    fn arb_php_identifier() -> impl Strategy<Value = String> {
        "[a-z][a-zA-Z0-9_]{0,10}".prop_map(String::from)
    }

    /// Generator for simple PHP integer literals
    fn arb_php_int() -> impl Strategy<Value = String> {
        (1i64..1000).prop_map(|n| n.to_string())
    }

    /// Generator for simple PHP string literals
    fn arb_php_string() -> impl Strategy<Value = String> {
        "[a-zA-Z0-9 ]{0,20}".prop_map(|s| format!("\"{}\"", s))
    }

    /// Generator for simple PHP expressions
    fn arb_php_expr() -> impl Strategy<Value = String> {
        prop_oneof![
            arb_php_int(),
            arb_php_string(),
            (arb_php_int(), arb_php_int()).prop_map(|(a, b)| format!("{} + {}", a, b)),
            (arb_php_int(), arb_php_int()).prop_map(|(a, b)| format!("{} * {}", a, b)),
        ]
    }

    /// Generator for simple PHP variable assignments
    fn arb_php_assignment() -> impl Strategy<Value = String> {
        (arb_php_identifier(), arb_php_expr())
            .prop_map(|(name, expr)| format!("${} = {};", name, expr))
    }

    /// Generator for simple PHP function definitions
    fn arb_php_function() -> impl Strategy<Value = String> {
        (arb_php_identifier(), prop::collection::vec(arb_php_assignment(), 0..3)).prop_map(
            |(name, body)| {
                let body_str = if body.is_empty() {
                    String::new()
                } else {
                    body.join("\\n    ") + "\\n"
                };
                format!("function {}() {{\\n    {}}}\\n", name, body_str)
            },
        )
    }

    /// Generator for valid PHP code snippets
    fn arb_valid_php_code() -> impl Strategy<Value = String> {
        prop_oneof![
            // Simple PHP with echo
            arb_php_string().prop_map(|s| format!("<?php\\necho {};\\n", s)),
            // PHP with variable assignment
            arb_php_assignment().prop_map(|stmt| format!("<?php\\n{}\\n", stmt)),
            // PHP with function
            arb_php_function().prop_map(|func| format!("<?php\\n{}\\n", func)),
            // PHP with class
            (arb_php_identifier(), arb_php_identifier()).prop_map(|(class_name, prop_name)| {
                format!("<?php\\nclass {} {{\\n    public ${};\\n}}\\n", class_name, prop_name)
            }),
            // Empty PHP file
            Just("<?php\\n".to_string()),
        ]
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_php_formatting_idempotence(code in arb_valid_php_code()) {
            let handler = PhpHandler::new();

            if handler.php_cs_fixer_path.is_none() {
                return Ok(());
            }

            let path = Path::new("test.php");

            if handler.php_path.is_some() {
                let diagnostics = handler.validate_syntax(path, &code);
                if !diagnostics.is_empty() {
                    return Ok(());
                }
            }

            let first_format = match handler.format_with_php_cs_fixer(path, &code) {
                Ok(formatted) => formatted,
                Err(_) => {
                    return Ok(());
                }
            };

            let second_format = match handler.format_with_php_cs_fixer(path, &first_format) {
                Ok(formatted) => formatted,
                Err(_) => {
                    return Ok(());
                }
            };

            prop_assert_eq!(
                first_format,
                second_format,
                "Formatting should be idempotent"
            );
        }
    }
}
*/
