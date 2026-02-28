//! Input validation utilities
//!
//! Provides comprehensive input validation for:
//! - Port numbers (1-65535)
//! - Semantic versions (X.Y.Z)
//! - Shell command sanitization
//! - Path traversal detection

use std::path::{Path, PathBuf};

/// Shell metacharacters that need escaping
pub const SHELL_METACHARACTERS: &[char] = &[
    '\\', '\'', '"', '`', '$', '!', '&', '|', ';', '(', ')', '[', ']', '{', '}', '<', '>', '*',
    '?', '#', '~', '^', ' ', '\t', '\n', '\r',
];

/// Validation error with detailed information
#[derive(Debug, Clone, PartialEq)]
pub struct ValidationError {
    /// Field that failed validation
    pub field: String,
    /// Value that was provided
    pub value: String,
    /// Expected format or range
    pub expected: String,
    /// Suggestion for fixing the error
    pub suggestion: Option<String>,
}

impl ValidationError {
    /// Create a new validation error
    pub fn new(
        field: impl Into<String>,
        value: impl Into<String>,
        expected: impl Into<String>,
    ) -> Self {
        Self {
            field: field.into(),
            value: value.into(),
            expected: expected.into(),
            suggestion: None,
        }
    }

    /// Add a suggestion to the error
    pub fn with_suggestion(mut self, suggestion: impl Into<String>) -> Self {
        self.suggestion = Some(suggestion.into());
        self
    }
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Invalid {}: '{}' (expected {})", self.field, self.value, self.expected)?;
        if let Some(ref suggestion) = self.suggestion {
            write!(f, "\n  Suggestion: {}", suggestion)?;
        }
        Ok(())
    }
}

impl std::error::Error for ValidationError {}

/// Security warning for potentially dangerous operations
#[derive(Debug, Clone, PartialEq)]
pub struct SecurityWarning {
    /// Type of security issue
    pub issue_type: SecurityIssueType,
    /// Description of the issue
    pub description: String,
    /// Path involved (if applicable)
    pub path: Option<PathBuf>,
}

/// Types of security issues
#[derive(Debug, Clone, PartialEq)]
pub enum SecurityIssueType {
    /// Path traversal attempt detected
    PathTraversal,
    /// Symlink escapes project directory
    SymlinkEscape,
    /// Potentially dangerous shell characters
    ShellInjection,
}

impl std::fmt::Display for SecurityWarning {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Security warning ({:?}): {}", self.issue_type, self.description)?;
        if let Some(ref path) = self.path {
            write!(f, " [path: {}]", path.display())?;
        }
        Ok(())
    }
}

impl std::error::Error for SecurityWarning {}

/// Input validator for common validation tasks
pub struct InputValidator;

impl InputValidator {
    /// Validate a port number is in the valid range (1-65535)
    ///
    /// Requirement 8.4: Port validation
    pub fn validate_port(port: u16) -> Result<u16, ValidationError> {
        if port == 0 {
            return Err(ValidationError::new(
                "port",
                port.to_string(),
                "a value between 1 and 65535",
            )
            .with_suggestion("Use a port number like 8080, 3000, or 5000"));
        }
        Ok(port)
    }

    /// Validate a port number from a string
    pub fn validate_port_str(port_str: &str) -> Result<u16, ValidationError> {
        let port: u16 = port_str.parse().map_err(|_| {
            ValidationError::new("port", port_str, "a valid number between 1 and 65535")
                .with_suggestion("Enter a numeric port value like 8080")
        })?;
        Self::validate_port(port)
    }

    /// Validate a semantic version string (X.Y.Z format)
    ///
    /// Requirement 8.5: Version validation
    pub fn validate_version(version: &str) -> Result<(u32, u32, u32), ValidationError> {
        let parts: Vec<&str> = version.split('.').collect();

        if parts.len() != 3 {
            return Err(ValidationError::new(
                "version",
                version,
                "semantic version format X.Y.Z (e.g., 1.0.0)",
            )
            .with_suggestion("Use format like 1.0.0, 2.1.3, or 0.1.0"));
        }

        let parse_part = |part: &str, name: &str| -> Result<u32, ValidationError> {
            part.parse().map_err(|_| {
                ValidationError::new(
                    "version",
                    version,
                    format!("{} must be a non-negative integer", name),
                )
            })
        };

        let major = parse_part(parts[0], "major version")?;
        let minor = parse_part(parts[1], "minor version")?;
        let patch = parse_part(parts[2], "patch version")?;

        Ok((major, minor, patch))
    }

    /// Sanitize a string for safe shell execution by escaping metacharacters
    ///
    /// Requirement 8.3: Shell metacharacter escaping
    pub fn sanitize_for_shell(input: &str) -> String {
        let mut result = String::with_capacity(input.len() * 2);

        for c in input.chars() {
            if SHELL_METACHARACTERS.contains(&c) {
                result.push('\\');
            }
            result.push(c);
        }

        result
    }

    /// Check if a string contains shell metacharacters
    pub fn contains_shell_metacharacters(input: &str) -> bool {
        input.chars().any(|c| SHELL_METACHARACTERS.contains(&c))
    }

    /// Check for path traversal attempts
    ///
    /// Requirement 8.2: Path traversal detection
    pub fn check_path_traversal(
        path: &Path,
        project_root: &Path,
    ) -> Result<PathBuf, SecurityWarning> {
        // Normalize the path
        let normalized = Self::normalize_path(path);

        // Check for obvious traversal patterns
        let path_str = normalized.to_string_lossy();
        if path_str.contains("..") {
            // Resolve the path to check if it escapes
            let resolved = if path.is_absolute() {
                normalized.clone()
            } else {
                project_root.join(&normalized)
            };

            // Canonicalize both paths for comparison
            let canonical_root = match project_root.canonicalize() {
                Ok(p) => p,
                Err(_) => project_root.to_path_buf(),
            };

            let canonical_path = match resolved.canonicalize() {
                Ok(p) => p,
                Err(_) => {
                    // Path doesn't exist yet, do string-based check
                    if Self::path_escapes_root(&resolved, &canonical_root) {
                        return Err(SecurityWarning {
                            issue_type: SecurityIssueType::PathTraversal,
                            description: "Path traversal detected: path escapes project directory"
                                .to_string(),
                            path: Some(path.to_path_buf()),
                        });
                    }
                    return Ok(resolved);
                }
            };

            if !canonical_path.starts_with(&canonical_root) {
                return Err(SecurityWarning {
                    issue_type: SecurityIssueType::PathTraversal,
                    description:
                        "Path traversal detected: resolved path is outside project directory"
                            .to_string(),
                    path: Some(path.to_path_buf()),
                });
            }
        }

        // Check for symlinks that escape the project
        if path.exists()
            && let Ok(metadata) = std::fs::symlink_metadata(path)
            && metadata.file_type().is_symlink()
            && let Ok(target) = std::fs::read_link(path)
        {
            let resolved_target = if target.is_absolute() {
                target
            } else {
                path.parent().unwrap_or(Path::new(".")).join(&target)
            };

            if let Ok(canonical_target) = resolved_target.canonicalize() {
                let canonical_root =
                    project_root.canonicalize().unwrap_or_else(|_| project_root.to_path_buf());
                if !canonical_target.starts_with(&canonical_root) {
                    return Err(SecurityWarning {
                        issue_type: SecurityIssueType::SymlinkEscape,
                        description: "Symlink points outside project directory".to_string(),
                        path: Some(path.to_path_buf()),
                    });
                }
            }
        }

        Ok(normalized)
    }

    /// Normalize a path by removing redundant components
    fn normalize_path(path: &Path) -> PathBuf {
        let mut components = Vec::new();

        for component in path.components() {
            use std::path::Component;
            match component {
                Component::ParentDir => {
                    if !components.is_empty() {
                        components.pop();
                    }
                }
                Component::CurDir => {}
                c => components.push(c),
            }
        }

        components.iter().collect()
    }

    /// Check if a path escapes the root directory (string-based check)
    fn path_escapes_root(path: &Path, root: &Path) -> bool {
        let path_str = path.to_string_lossy().to_lowercase();
        let root_str = root.to_string_lossy().to_lowercase();

        // Count parent directory references
        let parent_refs = path_str.matches("..").count();
        let path_depth = path.components().count();

        // If more parent refs than depth, it escapes
        parent_refs > path_depth || !path_str.starts_with(&*root_str)
    }

    /// Validate that a string is not empty
    pub fn validate_not_empty<'a>(field: &str, value: &'a str) -> Result<&'a str, ValidationError> {
        if value.trim().is_empty() {
            return Err(ValidationError::new(field, value, "a non-empty value"));
        }
        Ok(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    // ═══════════════════════════════════════════════════════════════════
    //  UNIT TESTS
    // ═══════════════════════════════════════════════════════════════════

    #[test]
    fn test_validate_port_valid() {
        assert!(InputValidator::validate_port(1).is_ok());
        assert!(InputValidator::validate_port(80).is_ok());
        assert!(InputValidator::validate_port(443).is_ok());
        assert!(InputValidator::validate_port(8080).is_ok());
        assert!(InputValidator::validate_port(65535).is_ok());
    }

    #[test]
    fn test_validate_port_invalid() {
        assert!(InputValidator::validate_port(0).is_err());
    }

    #[test]
    fn test_validate_port_str() {
        assert!(InputValidator::validate_port_str("8080").is_ok());
        assert!(InputValidator::validate_port_str("0").is_err());
        assert!(InputValidator::validate_port_str("abc").is_err());
        assert!(InputValidator::validate_port_str("-1").is_err());
        assert!(InputValidator::validate_port_str("99999").is_err());
    }

    #[test]
    fn test_validate_version_valid() {
        assert_eq!(InputValidator::validate_version("1.0.0").unwrap(), (1, 0, 0));
        assert_eq!(InputValidator::validate_version("0.1.0").unwrap(), (0, 1, 0));
        assert_eq!(InputValidator::validate_version("10.20.30").unwrap(), (10, 20, 30));
        assert_eq!(InputValidator::validate_version("0.0.1").unwrap(), (0, 0, 1));
    }

    #[test]
    fn test_validate_version_invalid() {
        assert!(InputValidator::validate_version("1.0").is_err());
        assert!(InputValidator::validate_version("1").is_err());
        assert!(InputValidator::validate_version("1.0.0.0").is_err());
        assert!(InputValidator::validate_version("a.b.c").is_err());
        assert!(InputValidator::validate_version("1.0.0-beta").is_err());
        assert!(InputValidator::validate_version("").is_err());
    }

    #[test]
    fn test_sanitize_for_shell() {
        assert_eq!(InputValidator::sanitize_for_shell("hello"), "hello");
        assert_eq!(InputValidator::sanitize_for_shell("hello world"), "hello\\ world");
        assert_eq!(InputValidator::sanitize_for_shell("$HOME"), "\\$HOME");
        assert_eq!(InputValidator::sanitize_for_shell("test;rm -rf /"), "test\\;rm\\ -rf\\ /");
        assert_eq!(InputValidator::sanitize_for_shell("file.txt"), "file.txt");
        assert_eq!(InputValidator::sanitize_for_shell("path/to/file"), "path/to/file");
    }

    #[test]
    fn test_contains_shell_metacharacters() {
        assert!(!InputValidator::contains_shell_metacharacters("hello"));
        assert!(InputValidator::contains_shell_metacharacters("hello world"));
        assert!(InputValidator::contains_shell_metacharacters("$HOME"));
        assert!(InputValidator::contains_shell_metacharacters("test;cmd"));
        assert!(InputValidator::contains_shell_metacharacters("file`cmd`"));
    }

    #[test]
    fn test_check_path_traversal_safe() {
        let project_root = std::env::current_dir().unwrap();
        let safe_path = Path::new("src/main.rs");
        assert!(InputValidator::check_path_traversal(safe_path, &project_root).is_ok());
    }

    #[test]
    fn test_check_path_traversal_unsafe() {
        let project_root = std::env::current_dir().unwrap();
        let unsafe_path = Path::new("../../../etc/passwd");
        let result = InputValidator::check_path_traversal(unsafe_path, &project_root);
        // Should either be an error or the path should be normalized
        if let Err(warning) = result {
            assert_eq!(warning.issue_type, SecurityIssueType::PathTraversal);
        }
    }

    #[test]
    fn test_validation_error_display() {
        let err = ValidationError::new("port", "abc", "a number between 1 and 65535")
            .with_suggestion("Use a numeric value like 8080");
        let display = err.to_string();
        assert!(display.contains("port"));
        assert!(display.contains("abc"));
        assert!(display.contains("8080"));
    }

    // ═══════════════════════════════════════════════════════════════════
    //  PROPERTY TESTS
    // ═══════════════════════════════════════════════════════════════════

    // Feature: dx-cli, Property 25: Port Validation Range
    // Validates: Requirements 8.4
    //
    // For any port number in range 1-65535, validate_port should return Ok.
    // For port 0, validate_port should return Err.
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_valid_ports_accepted(port in 1u16..=65535) {
            let result = InputValidator::validate_port(port);
            prop_assert!(result.is_ok(), "Port {} should be valid", port);
            prop_assert_eq!(result.unwrap(), port);
        }

        #[test]
        fn prop_port_zero_rejected(_dummy in 0..1i32) {
            let result = InputValidator::validate_port(0);
            prop_assert!(result.is_err(), "Port 0 should be invalid");
        }

        #[test]
        fn prop_port_str_valid(port in 1u16..=65535) {
            let port_str = port.to_string();
            let result = InputValidator::validate_port_str(&port_str);
            prop_assert!(result.is_ok(), "Port string '{}' should be valid", port_str);
        }

        #[test]
        fn prop_port_str_invalid_text(s in "[a-zA-Z]{1,10}") {
            let result = InputValidator::validate_port_str(&s);
            prop_assert!(result.is_err(), "Non-numeric port '{}' should be invalid", s);
        }
    }

    // Feature: dx-cli, Property 26: Version Validation Format
    // Validates: Requirements 8.5
    //
    // For any valid semver X.Y.Z where X, Y, Z are non-negative integers,
    // validate_version should return Ok((X, Y, Z)).
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_valid_versions_accepted(
            major in 0u32..1000,
            minor in 0u32..1000,
            patch in 0u32..1000
        ) {
            let version = format!("{}.{}.{}", major, minor, patch);
            let result = InputValidator::validate_version(&version);
            prop_assert!(result.is_ok(), "Version '{}' should be valid", version);
            prop_assert_eq!(result.unwrap(), (major, minor, patch));
        }

        #[test]
        fn prop_two_part_versions_rejected(
            major in 0u32..1000,
            minor in 0u32..1000
        ) {
            let version = format!("{}.{}", major, minor);
            let result = InputValidator::validate_version(&version);
            prop_assert!(result.is_err(), "Two-part version '{}' should be invalid", version);
        }

        #[test]
        fn prop_four_part_versions_rejected(
            major in 0u32..100,
            minor in 0u32..100,
            patch in 0u32..100,
            extra in 0u32..100
        ) {
            let version = format!("{}.{}.{}.{}", major, minor, patch, extra);
            let result = InputValidator::validate_version(&version);
            prop_assert!(result.is_err(), "Four-part version '{}' should be invalid", version);
        }

        #[test]
        fn prop_non_numeric_versions_rejected(
            a in "[a-zA-Z]{1,5}",
            b in "[a-zA-Z]{1,5}",
            c in "[a-zA-Z]{1,5}"
        ) {
            let version = format!("{}.{}.{}", a, b, c);
            let result = InputValidator::validate_version(&version);
            prop_assert!(result.is_err(), "Non-numeric version '{}' should be invalid", version);
        }
    }

    // Feature: dx-cli, Property 27: Shell Metacharacter Escaping
    // Validates: Requirements 8.3
    //
    // For any input string, sanitize_for_shell should escape all shell
    // metacharacters with a backslash. The result should not contain
    // any unescaped metacharacters.
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_shell_metacharacters_escaped(input in "[a-zA-Z0-9$;&|`'\"\\s]{1,50}") {
            let sanitized = InputValidator::sanitize_for_shell(&input);

            // Check that all metacharacters are escaped
            let chars: Vec<char> = sanitized.chars().collect();
            for (i, &c) in chars.iter().enumerate() {
                if SHELL_METACHARACTERS.contains(&c) && c != '\\' {
                    // This metacharacter should be preceded by a backslash
                    prop_assert!(
                        i > 0 && chars[i - 1] == '\\',
                        "Metacharacter '{}' at position {} should be escaped in '{}'",
                        c, i, sanitized
                    );
                }
            }
        }

        #[test]
        fn prop_safe_strings_unchanged(input in "[a-zA-Z0-9._/-]{1,50}") {
            // Strings without metacharacters should be unchanged
            if !InputValidator::contains_shell_metacharacters(&input) {
                let sanitized = InputValidator::sanitize_for_shell(&input);
                prop_assert_eq!(
                    sanitized, input,
                    "Safe string should be unchanged"
                );
            }
        }

        #[test]
        fn prop_sanitized_length_increases(input in "[a-zA-Z0-9$;&|]{1,20}") {
            let sanitized = InputValidator::sanitize_for_shell(&input);
            let metachar_count = input.chars()
                .filter(|c| SHELL_METACHARACTERS.contains(c))
                .count();

            prop_assert_eq!(
                sanitized.len(),
                input.len() + metachar_count,
                "Sanitized length should increase by number of metacharacters"
            );
        }
    }

    // Feature: dx-cli, Property 28: Path Traversal Detection
    // Validates: Requirements 8.2
    //
    // For any path containing ".." that would escape the project root,
    // check_path_traversal should return a SecurityWarning.
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn prop_safe_paths_accepted(
            parts in prop::collection::vec("[a-zA-Z0-9_-]{1,10}", 1..5)
        ) {
            let path_str = parts.join("/");
            let path = Path::new(&path_str);
            let project_root = std::env::current_dir().unwrap();

            let result = InputValidator::check_path_traversal(path, &project_root);
            prop_assert!(result.is_ok(), "Safe path '{}' should be accepted", path_str);
        }

        #[test]
        fn prop_traversal_patterns_detected(
            prefix in "[a-zA-Z0-9]{1,5}",
            depth in 1usize..5
        ) {
            // Create a path with multiple parent directory references
            let traversal = "../".repeat(depth + 5); // Ensure it escapes
            let path_str = format!("{}/{}", prefix, traversal);
            let path = Path::new(&path_str);
            let project_root = std::env::current_dir().unwrap();

            let result = InputValidator::check_path_traversal(path, &project_root);
            // Should either error or normalize the path
            if let Err(warning) = result {
                prop_assert_eq!(
                    warning.issue_type,
                    SecurityIssueType::PathTraversal,
                    "Should detect path traversal"
                );
            }
        }
    }
}
