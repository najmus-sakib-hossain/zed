//! Command Logic Module
//!
//! Unit-testable command logic separated from CLI parsing.
//! **Validates: Requirements 1.7, 2.7 - Unit tests for format/lint command logic**

use crate::config::CheckerConfig;
use crate::diagnostics::Diagnostic;
use crate::languages::{FileProcessor, FileStatus};
use std::path::{Path, PathBuf};

/// Result of a format operation
#[derive(Debug)]
pub struct FormatResult {
    /// Number of files processed
    pub files_processed: usize,
    /// Number of files that were changed
    pub files_changed: usize,
    /// Number of files that had errors
    pub files_errored: usize,
    /// Duration of the operation
    pub duration_ms: u64,
    /// Whether any files need formatting (in check mode)
    pub needs_formatting: bool,
}

impl FormatResult {
    #[must_use]
    pub fn new() -> Self {
        Self {
            files_processed: 0,
            files_changed: 0,
            files_errored: 0,
            duration_ms: 0,
            needs_formatting: false,
        }
    }

    /// Returns true if the operation was successful (no errors)
    #[must_use]
    pub fn is_success(&self) -> bool {
        self.files_errored == 0
    }
}

impl Default for FormatResult {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of a lint operation
#[derive(Debug)]
pub struct LintResult {
    /// Number of files processed
    pub files_processed: usize,
    /// Total number of errors found
    pub error_count: usize,
    /// Total number of warnings found
    pub warning_count: usize,
    /// All diagnostics
    pub diagnostics: Vec<Diagnostic>,
    /// Duration of the operation
    pub duration_ms: u64,
}

impl LintResult {
    #[must_use]
    pub fn new() -> Self {
        Self {
            files_processed: 0,
            error_count: 0,
            warning_count: 0,
            diagnostics: Vec::new(),
            duration_ms: 0,
        }
    }

    /// Returns true if there are any errors
    #[must_use]
    pub fn has_errors(&self) -> bool {
        self.error_count > 0
    }

    /// Returns true if there are any issues (errors or warnings)
    #[must_use]
    pub fn has_issues(&self) -> bool {
        self.error_count > 0 || self.warning_count > 0
    }
}

impl Default for LintResult {
    fn default() -> Self {
        Self::new()
    }
}

/// Format a single file
pub fn format_file(
    processor: &FileProcessor,
    path: &Path,
    check: bool,
    write: bool,
) -> Result<FileStatus, String> {
    let content = std::fs::read_to_string(path).map_err(|e| e.to_string())?;

    // In check mode, we don't write; otherwise respect the write flag
    let should_write = !check && write;

    processor.format(path, &content, should_write).map_err(|d| d.message)
}

/// Collect files to process based on paths and config
pub fn collect_files(
    paths: &[PathBuf],
    config: &CheckerConfig,
    supported_extensions: &[&str],
) -> Result<Vec<PathBuf>, String> {
    use ignore::WalkBuilder;

    let mut files = Vec::new();

    for path in paths {
        if path.is_file() {
            files.push(path.clone());
            continue;
        }

        let walker = WalkBuilder::new(path).standard_filters(true).hidden(true).build();

        for entry in walker.flatten() {
            let entry_path = entry.path();

            if !entry_path.is_file() {
                continue;
            }

            // Check extension
            let ext = entry_path.extension().and_then(|e| e.to_str()).unwrap_or("");
            if !supported_extensions.contains(&ext) {
                continue;
            }

            // Check exclude patterns
            let matches_exclude = config.exclude.iter().any(|pattern| {
                glob::Pattern::new(pattern).map(|p| p.matches_path(entry_path)).unwrap_or(false)
            });

            if matches_exclude {
                continue;
            }

            files.push(entry_path.to_path_buf());
        }
    }

    Ok(files)
}

/// Check if a file extension is supported for JavaScript/TypeScript
#[must_use]
pub fn is_js_ts_file(path: &Path) -> bool {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    matches!(ext, "js" | "jsx" | "ts" | "tsx" | "mjs" | "cjs")
}

/// Check if a file extension is supported for formatting
#[must_use]
pub fn is_formattable_file(path: &Path) -> bool {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    matches!(
        ext,
        "js" | "jsx" | "ts" | "tsx" | "mjs" | "cjs" | "py" | "pyi" | "go" | "rs" | "toml" | "md"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    // ========================================================================
    // FormatResult Tests
    // ========================================================================

    #[test]
    fn test_format_result_new() {
        let result = FormatResult::new();
        assert_eq!(result.files_processed, 0);
        assert_eq!(result.files_changed, 0);
        assert_eq!(result.files_errored, 0);
        assert!(!result.needs_formatting);
    }

    #[test]
    fn test_format_result_is_success() {
        let mut result = FormatResult::new();
        assert!(result.is_success());

        result.files_errored = 1;
        assert!(!result.is_success());
    }

    // ========================================================================
    // LintResult Tests
    // ========================================================================

    #[test]
    fn test_lint_result_new() {
        let result = LintResult::new();
        assert_eq!(result.files_processed, 0);
        assert_eq!(result.error_count, 0);
        assert_eq!(result.warning_count, 0);
        assert!(result.diagnostics.is_empty());
    }

    #[test]
    fn test_lint_result_has_errors() {
        let mut result = LintResult::new();
        assert!(!result.has_errors());

        result.error_count = 1;
        assert!(result.has_errors());
    }

    #[test]
    fn test_lint_result_has_issues() {
        let mut result = LintResult::new();
        assert!(!result.has_issues());

        result.warning_count = 1;
        assert!(result.has_issues());

        result.warning_count = 0;
        result.error_count = 1;
        assert!(result.has_issues());
    }

    // ========================================================================
    // File Extension Tests
    // ========================================================================

    #[test]
    fn test_is_js_ts_file() {
        assert!(is_js_ts_file(Path::new("test.js")));
        assert!(is_js_ts_file(Path::new("test.jsx")));
        assert!(is_js_ts_file(Path::new("test.ts")));
        assert!(is_js_ts_file(Path::new("test.tsx")));
        assert!(is_js_ts_file(Path::new("test.mjs")));
        assert!(is_js_ts_file(Path::new("test.cjs")));

        assert!(!is_js_ts_file(Path::new("test.py")));
        assert!(!is_js_ts_file(Path::new("test.rs")));
        assert!(!is_js_ts_file(Path::new("test.go")));
    }

    #[test]
    fn test_is_formattable_file() {
        // JS/TS files
        assert!(is_formattable_file(Path::new("test.js")));
        assert!(is_formattable_file(Path::new("test.ts")));

        // Other languages
        assert!(is_formattable_file(Path::new("test.py")));
        assert!(is_formattable_file(Path::new("test.go")));
        assert!(is_formattable_file(Path::new("test.rs")));
        assert!(is_formattable_file(Path::new("test.toml")));
        assert!(is_formattable_file(Path::new("test.md")));

        // Unsupported
        assert!(!is_formattable_file(Path::new("test.xyz")));
        assert!(!is_formattable_file(Path::new("test.java")));
    }

    // ========================================================================
    // File Collection Tests
    // ========================================================================

    #[test]
    fn test_collect_files_single_file() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.js");
        std::fs::write(&file_path, "const x = 1;").unwrap();

        let config = CheckerConfig::default();
        let files = collect_files(&[file_path.clone()], &config, &["js"]).unwrap();

        assert_eq!(files.len(), 1);
        assert_eq!(files[0], file_path);
    }

    #[test]
    fn test_collect_files_directory() {
        let dir = tempdir().unwrap();
        std::fs::write(dir.path().join("a.js"), "const a = 1;").unwrap();
        std::fs::write(dir.path().join("b.js"), "const b = 2;").unwrap();
        std::fs::write(dir.path().join("c.txt"), "not js").unwrap();

        let config = CheckerConfig::default();
        let files = collect_files(&[dir.path().to_path_buf()], &config, &["js"]).unwrap();

        assert_eq!(files.len(), 2);
    }

    #[test]
    fn test_collect_files_respects_exclude() {
        let dir = tempdir().unwrap();
        std::fs::write(dir.path().join("a.js"), "const a = 1;").unwrap();
        std::fs::write(dir.path().join("b.test.js"), "const b = 2;").unwrap();

        let mut config = CheckerConfig::default();
        config.exclude = vec!["*.test.js".to_string()];

        let files = collect_files(&[dir.path().to_path_buf()], &config, &["js"]).unwrap();

        assert_eq!(files.len(), 1);
        assert!(files[0].to_string_lossy().contains("a.js"));
    }

    #[test]
    fn test_collect_files_multiple_extensions() {
        let dir = tempdir().unwrap();
        std::fs::write(dir.path().join("a.js"), "const a = 1;").unwrap();
        std::fs::write(dir.path().join("b.ts"), "const b: number = 2;").unwrap();
        std::fs::write(dir.path().join("c.py"), "x = 1").unwrap();

        let config = CheckerConfig::default();
        let files = collect_files(&[dir.path().to_path_buf()], &config, &["js", "ts"]).unwrap();

        assert_eq!(files.len(), 2);
    }

    // ========================================================================
    // Format File Tests
    // ========================================================================

    #[test]
    fn test_format_file_nonexistent() {
        let processor = FileProcessor::new();
        let result = format_file(&processor, Path::new("/nonexistent/file.js"), false, false);

        assert!(result.is_err());
    }

    #[test]
    fn test_format_file_unsupported_extension() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.xyz");
        std::fs::write(&file_path, "content").unwrap();

        let processor = FileProcessor::new();
        let result = format_file(&processor, &file_path, false, false);

        // Should return Ignored for unsupported files
        assert!(matches!(result, Ok(FileStatus::Ignored)));
    }
}
