//! Security hardening for DX Markdown Context Compiler.
//!
//! This module provides input validation, size limits, and other security
//! measures to protect against malicious or malformed input.
//!
//! # Security Requirements (from Requirement 15)
//!
//! - Reject inputs larger than 100MB
//! - Limit recursion depth to 1000 levels
//! - Validate UTF-8 encoding
//! - Do not execute any code from input
//! - Handle malformed Markdown gracefully (no panics)

use crate::error::CompileError;

/// Maximum input size in bytes (100MB).
pub const MAX_INPUT_SIZE: usize = 100 * 1024 * 1024;

/// Maximum recursion depth for nested structures.
pub const MAX_RECURSION_DEPTH: usize = 1000;

/// Maximum line length in bytes (1MB).
pub const MAX_LINE_LENGTH: usize = 1024 * 1024;

/// Maximum number of table columns.
pub const MAX_TABLE_COLUMNS: usize = 1000;

/// Maximum number of table rows.
pub const MAX_TABLE_ROWS: usize = 100_000;

/// Maximum number of list items.
pub const MAX_LIST_ITEMS: usize = 100_000;

/// Maximum number of code blocks.
pub const MAX_CODE_BLOCKS: usize = 10_000;

/// Maximum code block size in bytes (10MB).
pub const MAX_CODE_BLOCK_SIZE: usize = 10 * 1024 * 1024;

/// Maximum dictionary size (number of entries).
pub const MAX_DICTIONARY_SIZE: usize = 1000;

/// Security validation result.
pub type SecurityResult<T> = Result<T, CompileError>;

/// Validate input size.
///
/// Returns an error if the input exceeds the maximum allowed size.
#[inline]
pub fn validate_input_size(input: &[u8]) -> SecurityResult<()> {
    if input.len() > MAX_INPUT_SIZE {
        return Err(CompileError::input_too_large(input.len(), MAX_INPUT_SIZE));
    }
    Ok(())
}

/// Validate input size for strings.
#[inline]
pub fn validate_input_size_str(input: &str) -> SecurityResult<()> {
    validate_input_size(input.as_bytes())
}

/// Validate UTF-8 encoding.
///
/// Returns an error if the input contains invalid UTF-8 sequences.
#[inline]
pub fn validate_utf8(input: &[u8]) -> SecurityResult<&str> {
    std::str::from_utf8(input).map_err(|e| CompileError::invalid_utf8(e.valid_up_to()))
}

/// Validate recursion depth.
///
/// Returns an error if the depth exceeds the maximum allowed.
#[inline]
pub fn validate_recursion_depth(depth: usize) -> SecurityResult<()> {
    if depth > MAX_RECURSION_DEPTH {
        return Err(CompileError::recursion_limit(depth, MAX_RECURSION_DEPTH));
    }
    Ok(())
}

/// Validate line length.
///
/// Returns an error if any line exceeds the maximum allowed length.
pub fn validate_line_lengths(input: &str) -> SecurityResult<()> {
    for (line_num, line) in input.lines().enumerate() {
        if line.len() > MAX_LINE_LENGTH {
            return Err(CompileError::Io {
                message: format!(
                    "line {} exceeds maximum length of {} bytes (has {} bytes)",
                    line_num + 1,
                    MAX_LINE_LENGTH,
                    line.len()
                ),
            });
        }
    }
    Ok(())
}

/// Validate table dimensions.
///
/// Returns an error if the table exceeds maximum dimensions.
pub fn validate_table_dimensions(columns: usize, rows: usize) -> SecurityResult<()> {
    if columns > MAX_TABLE_COLUMNS {
        return Err(CompileError::Io {
            message: format!("table has {} columns, maximum is {}", columns, MAX_TABLE_COLUMNS),
        });
    }
    if rows > MAX_TABLE_ROWS {
        return Err(CompileError::Io {
            message: format!("table has {} rows, maximum is {}", rows, MAX_TABLE_ROWS),
        });
    }
    Ok(())
}

/// Validate code block size.
///
/// Returns an error if the code block exceeds maximum size.
pub fn validate_code_block_size(size: usize) -> SecurityResult<()> {
    if size > MAX_CODE_BLOCK_SIZE {
        return Err(CompileError::Io {
            message: format!(
                "code block size {} bytes exceeds maximum {} bytes",
                size, MAX_CODE_BLOCK_SIZE
            ),
        });
    }
    Ok(())
}

/// Input validator that performs all security checks.
pub struct InputValidator {
    /// Maximum input size
    pub max_input_size: usize,
    /// Maximum recursion depth
    pub max_recursion_depth: usize,
    /// Maximum line length
    pub max_line_length: usize,
    /// Whether to validate UTF-8
    pub validate_utf8: bool,
    /// Whether to validate line lengths
    pub validate_lines: bool,
}

impl Default for InputValidator {
    fn default() -> Self {
        Self {
            max_input_size: MAX_INPUT_SIZE,
            max_recursion_depth: MAX_RECURSION_DEPTH,
            max_line_length: MAX_LINE_LENGTH,
            validate_utf8: true,
            validate_lines: true,
        }
    }
}

impl InputValidator {
    /// Create a new validator with default settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set maximum input size.
    pub fn with_max_input_size(mut self, size: usize) -> Self {
        self.max_input_size = size;
        self
    }

    /// Set maximum recursion depth.
    pub fn with_max_recursion_depth(mut self, depth: usize) -> Self {
        self.max_recursion_depth = depth;
        self
    }

    /// Set maximum line length.
    pub fn with_max_line_length(mut self, length: usize) -> Self {
        self.max_line_length = length;
        self
    }

    /// Disable UTF-8 validation.
    pub fn skip_utf8_validation(mut self) -> Self {
        self.validate_utf8 = false;
        self
    }

    /// Disable line length validation.
    pub fn skip_line_validation(mut self) -> Self {
        self.validate_lines = false;
        self
    }

    /// Validate input bytes.
    pub fn validate_bytes<'a>(&self, input: &'a [u8]) -> SecurityResult<&'a str> {
        // Check size
        if input.len() > self.max_input_size {
            return Err(CompileError::input_too_large(input.len(), self.max_input_size));
        }

        // Validate UTF-8
        let text = if self.validate_utf8 {
            std::str::from_utf8(input).map_err(|e| CompileError::invalid_utf8(e.valid_up_to()))?
        } else {
            // SAFETY: The caller has explicitly disabled UTF-8 validation via `validate_utf8 = false`.
            // This is only safe if the caller guarantees the input is valid UTF-8.
            // Invalid UTF-8 will cause undefined behavior in downstream string operations.
            unsafe { std::str::from_utf8_unchecked(input) }
        };

        // Validate line lengths
        if self.validate_lines {
            for (line_num, line) in text.lines().enumerate() {
                if line.len() > self.max_line_length {
                    return Err(CompileError::Io {
                        message: format!(
                            "line {} exceeds maximum length of {} bytes",
                            line_num + 1,
                            self.max_line_length
                        ),
                    });
                }
            }
        }

        Ok(text)
    }

    /// Validate input string.
    pub fn validate_str(&self, input: &str) -> SecurityResult<()> {
        // Check size
        if input.len() > self.max_input_size {
            return Err(CompileError::input_too_large(input.len(), self.max_input_size));
        }

        // Validate line lengths
        if self.validate_lines {
            for (line_num, line) in input.lines().enumerate() {
                if line.len() > self.max_line_length {
                    return Err(CompileError::Io {
                        message: format!(
                            "line {} exceeds maximum length of {} bytes",
                            line_num + 1,
                            self.max_line_length
                        ),
                    });
                }
            }
        }

        Ok(())
    }

    /// Check recursion depth.
    pub fn check_recursion(&self, depth: usize) -> SecurityResult<()> {
        if depth > self.max_recursion_depth {
            return Err(CompileError::recursion_limit(depth, self.max_recursion_depth));
        }
        Ok(())
    }
}

/// Sanitize input by removing potentially dangerous content.
///
/// This function removes:
/// - Null bytes
/// - Control characters (except newline, tab, carriage return)
/// - Invalid UTF-8 sequences (replaced with replacement character)
pub fn sanitize_input(input: &str) -> String {
    input
        .chars()
        .filter(|c| {
            // Allow printable characters, newlines, tabs, and carriage returns
            !c.is_control() || *c == '\n' || *c == '\t' || *c == '\r'
        })
        .collect()
}

/// Check if input contains potentially dangerous patterns.
///
/// Returns a list of warnings about potentially dangerous content.
pub fn check_dangerous_patterns(input: &str) -> Vec<String> {
    let mut warnings = Vec::new();

    // Check for extremely long lines (potential DoS)
    for (i, line) in input.lines().enumerate() {
        if line.len() > 100_000 {
            warnings.push(format!(
                "Line {} is very long ({} bytes), may cause performance issues",
                i + 1,
                line.len()
            ));
        }
    }

    // Check for deeply nested structures
    let max_consecutive_markers = input
        .lines()
        .map(|line| {
            let trimmed = line.trim_start();
            if trimmed.starts_with('#') {
                trimmed.chars().take_while(|c| *c == '#').count()
            } else if trimmed.starts_with('-') || trimmed.starts_with('*') {
                // Count indentation for lists
                line.len() - trimmed.len()
            } else {
                0
            }
        })
        .max()
        .unwrap_or(0);

    if max_consecutive_markers > 20 {
        warnings.push(format!(
            "Document contains deeply nested structures (depth {})",
            max_consecutive_markers
        ));
    }

    // Check for excessive repetition (potential DoS)
    let unique_lines: std::collections::HashSet<_> = input.lines().collect();
    let total_lines = input.lines().count();
    if total_lines > 100 && unique_lines.len() < total_lines / 10 {
        warnings.push(format!(
            "Document has high repetition ({} unique lines out of {})",
            unique_lines.len(),
            total_lines
        ));
    }

    warnings
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_input_size() {
        let small = vec![0u8; 1000];
        assert!(validate_input_size(&small).is_ok());

        let large = vec![0u8; MAX_INPUT_SIZE + 1];
        assert!(validate_input_size(&large).is_err());
    }

    #[test]
    fn test_validate_utf8() {
        assert!(validate_utf8(b"hello world").is_ok());
        assert!(validate_utf8("héllo wörld".as_bytes()).is_ok());
        assert!(validate_utf8(&[0xff, 0xfe]).is_err());
    }

    #[test]
    fn test_validate_recursion_depth() {
        assert!(validate_recursion_depth(100).is_ok());
        assert!(validate_recursion_depth(MAX_RECURSION_DEPTH).is_ok());
        assert!(validate_recursion_depth(MAX_RECURSION_DEPTH + 1).is_err());
    }

    #[test]
    fn test_input_validator() {
        let validator = InputValidator::new();

        assert!(validator.validate_str("hello world").is_ok());
        assert!(validator.check_recursion(100).is_ok());
        assert!(validator.check_recursion(MAX_RECURSION_DEPTH + 1).is_err());
    }

    #[test]
    fn test_input_validator_custom_limits() {
        let validator = InputValidator::new().with_max_input_size(100).with_max_recursion_depth(10);

        assert!(validator.validate_str("short").is_ok());
        assert!(validator.validate_str(&"x".repeat(101)).is_err());
        assert!(validator.check_recursion(5).is_ok());
        assert!(validator.check_recursion(11).is_err());
    }

    #[test]
    fn test_sanitize_input() {
        // Null bytes and control characters should be removed
        let input = "hello\x00world\x01test";
        let sanitized = sanitize_input(input);
        // Both \x00 (null) and \x01 (control) should be removed
        assert_eq!(sanitized, "helloworldtest");
        assert!(!sanitized.contains('\x00'));
        assert!(!sanitized.contains('\x01'));
    }

    #[test]
    fn test_sanitize_preserves_whitespace() {
        let input = "hello\nworld\ttab\rcarriage";
        let sanitized = sanitize_input(input);
        assert!(sanitized.contains('\n'));
        assert!(sanitized.contains('\t'));
        assert!(sanitized.contains('\r'));
    }

    #[test]
    fn test_check_dangerous_patterns_long_lines() {
        let long_line = "x".repeat(200_000);
        let warnings = check_dangerous_patterns(&long_line);
        assert!(!warnings.is_empty());
        assert!(warnings[0].contains("very long"));
    }

    #[test]
    fn test_check_dangerous_patterns_repetition() {
        let repeated = "same line\n".repeat(1000);
        let warnings = check_dangerous_patterns(&repeated);
        assert!(!warnings.is_empty());
        assert!(warnings.iter().any(|w| w.contains("repetition")));
    }

    #[test]
    fn test_validate_table_dimensions() {
        assert!(validate_table_dimensions(10, 100).is_ok());
        assert!(validate_table_dimensions(MAX_TABLE_COLUMNS + 1, 10).is_err());
        assert!(validate_table_dimensions(10, MAX_TABLE_ROWS + 1).is_err());
    }

    #[test]
    fn test_validate_code_block_size() {
        assert!(validate_code_block_size(1000).is_ok());
        assert!(validate_code_block_size(MAX_CODE_BLOCK_SIZE + 1).is_err());
    }
}

// =============================================================================
// Fuzz Testing with Proptest
// =============================================================================

#[cfg(test)]
mod fuzz_tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Fuzz test: Compiler should never panic on arbitrary input.
        /// **Validates: Requirement 15.5 - Handle malformed Markdown gracefully**
        #[test]
        fn fuzz_compiler_no_panic(input in ".*") {
            use crate::compiler::DxMarkdown;
            use crate::types::CompilerConfig;

            // Should not panic on any input
            let config = CompilerConfig::default();
            if let Ok(compiler) = DxMarkdown::new(config) {
                // Result can be Ok or Err, but should never panic
                let _ = compiler.compile(&input);
            }
        }

        /// Fuzz test: Input validation should never panic.
        #[test]
        fn fuzz_input_validation_no_panic(input in prop::collection::vec(any::<u8>(), 0..10000)) {
            // Should not panic on any byte sequence
            let _ = validate_input_size(&input);
            let _ = validate_utf8(&input);
        }

        /// Fuzz test: Sanitize should never panic and always produce valid UTF-8.
        #[test]
        fn fuzz_sanitize_produces_valid_utf8(input in ".*") {
            let sanitized = sanitize_input(&input);
            // Result should always be valid UTF-8
            assert!(sanitized.is_ascii() || sanitized.chars().all(|c| c.len_utf8() > 0));
        }

        /// Fuzz test: Check dangerous patterns should never panic.
        #[test]
        fn fuzz_check_dangerous_patterns_no_panic(input in ".*") {
            // Should not panic on any input
            let _ = check_dangerous_patterns(&input);
        }

        /// Fuzz test: InputValidator should handle any input gracefully.
        #[test]
        fn fuzz_input_validator_no_panic(input in prop::collection::vec(any::<u8>(), 0..10000)) {
            let validator = InputValidator::new();
            // Should not panic, may return error
            let _ = validator.validate_bytes(&input);
        }

        /// Fuzz test: Table dimension validation should never panic.
        #[test]
        fn fuzz_table_dimensions_no_panic(cols in 0usize..1_000_000, rows in 0usize..1_000_000) {
            // Should not panic on any dimensions
            let _ = validate_table_dimensions(cols, rows);
        }

        /// Fuzz test: Recursion depth validation should never panic.
        #[test]
        fn fuzz_recursion_depth_no_panic(depth in 0usize..1_000_000) {
            // Should not panic on any depth
            let _ = validate_recursion_depth(depth);
        }

        /// Fuzz test: Code block size validation should never panic.
        #[test]
        fn fuzz_code_block_size_no_panic(size in 0usize..1_000_000_000) {
            // Should not panic on any size
            let _ = validate_code_block_size(size);
        }

        /// Fuzz test: SIMD operations should never panic on arbitrary input.
        #[test]
        fn fuzz_simd_operations_no_panic(input in prop::collection::vec(any::<u8>(), 0..10000)) {
            use crate::simd;

            // All SIMD operations should handle any input gracefully
            let _ = simd::find_byte(&input, b'\n');
            let _ = simd::find_pipe(&input);
            let _ = simd::find_newline(&input);
            let _ = simd::count_byte(&input, b'\n');
            let _ = simd::find_any_byte(&input, b"\n|#");
            let _ = simd::find_code_fence(&input);
            let _ = simd::find_header_marker(&input);
        }

        /// Fuzz test: Minification should never panic on arbitrary code.
        #[test]
        fn fuzz_minify_no_panic(code in ".*", lang in "(javascript|python|rust|json|)") {
            use crate::minify::minify_code;

            // Should not panic on any input
            let _ = minify_code(&code, &lang);
        }

        /// Fuzz test: Table conversion should never panic.
        #[test]
        fn fuzz_table_conversion_no_panic(
            headers in prop::collection::vec("[a-zA-Z0-9 ]{1,20}", 1..10),
            rows in prop::collection::vec(
                prop::collection::vec("[a-zA-Z0-9 ]{0,20}", 1..10),
                0..20
            )
        ) {
            use crate::types::TableInfo;
            use crate::table::{table_to_tsv, table_to_csv};

            let table = TableInfo {
                headers: headers.clone(),
                rows: rows.iter().map(|r| {
                    let mut row = r.clone();
                    row.resize(headers.len(), String::new());
                    row
                }).collect(),
                start_line: 0,
                end_line: 0,
                original: String::new(),
            };

            // Should not panic
            let _ = table_to_tsv(&table);
            let _ = table_to_csv(&table);
        }

        /// Fuzz test: Dictionary operations should never panic.
        #[test]
        fn fuzz_dictionary_no_panic(
            phrases in prop::collection::vec("[a-zA-Z]{5,20}", 0..50)
        ) {
            use crate::dictionary::Dictionary;

            let mut dict = Dictionary::new();
            for phrase in &phrases {
                dict.add(phrase.clone());
            }

            // Should not panic
            let _ = dict.header();
            for phrase in &phrases {
                let _ = dict.apply(phrase);
            }
        }

        /// Fuzz test: Filler detection should never panic.
        #[test]
        fn fuzz_filler_detection_no_panic(input in ".*") {
            use crate::filler::{strip_filler, starts_with_filler, contains_filler};

            // Should not panic on any input
            let _ = strip_filler(&input);
            let _ = starts_with_filler(&input);
            let _ = contains_filler(&input);
        }
    }
}
