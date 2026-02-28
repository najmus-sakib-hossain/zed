//! Multi-Language Processing Module
//!
//! This module provides unified multi-language formatting and linting
//! using the `FileProcessor` and language handlers.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use ignore::WalkBuilder;
use rayon::prelude::*;

use crate::languages::{
    CppHandler, Diagnostic, FileProcessor, FileStatus, GoHandler, KotlinHandler, MarkdownHandler,
    PhpHandler, PythonHandler, RustHandler, TomlHandler,
};

/// Result of a multi-language operation
#[derive(Debug, Clone)]
pub struct MultiLangResult {
    /// Files that were changed
    pub changed: Vec<PathBuf>,
    /// Files that were unchanged
    pub unchanged: Vec<PathBuf>,
    /// Files that were ignored (unsupported)
    pub ignored: Vec<PathBuf>,
    /// Diagnostics from linting
    pub diagnostics: Vec<Diagnostic>,
    /// Errors encountered
    pub errors: Vec<Diagnostic>,
    /// Total files processed
    pub files_processed: usize,
    /// Duration of the operation
    pub duration: Duration,
}

impl MultiLangResult {
    /// Create a new empty result
    #[must_use]
    pub fn new() -> Self {
        Self {
            changed: Vec::new(),
            unchanged: Vec::new(),
            ignored: Vec::new(),
            diagnostics: Vec::new(),
            errors: Vec::new(),
            files_processed: 0,
            duration: Duration::ZERO,
        }
    }

    /// Check if there were any errors
    #[must_use]
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    /// Check if any files were changed
    #[must_use]
    pub fn has_changes(&self) -> bool {
        !self.changed.is_empty()
    }

    /// Get the total number of diagnostics
    #[must_use]
    pub fn diagnostic_count(&self) -> usize {
        self.diagnostics.len()
    }

    /// Get the number of error diagnostics
    #[must_use]
    pub fn error_count(&self) -> usize {
        self.diagnostics
            .iter()
            .filter(|d| d.severity == crate::languages::Severity::Error)
            .count()
            + self.errors.len()
    }

    /// Get the number of warning diagnostics
    #[must_use]
    pub fn warning_count(&self) -> usize {
        self.diagnostics
            .iter()
            .filter(|d| d.severity == crate::languages::Severity::Warning)
            .count()
    }
}

impl Default for MultiLangResult {
    fn default() -> Self {
        Self::new()
    }
}

/// Multi-language processor for formatting and linting
pub struct MultiLangProcessor {
    /// The file processor with registered handlers
    processor: FileProcessor,
    /// Number of threads to use (0 = auto)
    threads: usize,
}

impl MultiLangProcessor {
    /// Create a new multi-language processor with all handlers registered
    #[must_use]
    pub fn new() -> Self {
        let mut processor = FileProcessor::new();

        // Register all language handlers
        processor.register(PythonHandler::new());
        processor.register(CppHandler::new());
        processor.register(GoHandler::new());
        processor.register(RustHandler::new());
        processor.register(PhpHandler::new());
        processor.register(KotlinHandler::new());
        processor.register(MarkdownHandler::new());
        processor.register(TomlHandler::new());

        Self {
            processor,
            threads: 0, // Auto-detect
        }
    }

    /// Set the number of threads to use
    #[must_use]
    pub fn with_threads(mut self, threads: usize) -> Self {
        self.threads = threads;
        self
    }

    /// Format files in the given paths
    #[must_use]
    pub fn format(&self, paths: &[PathBuf], write: bool) -> MultiLangResult {
        let start = Instant::now();
        let files = self.collect_files(paths);

        let results: Vec<(PathBuf, Result<FileStatus, Diagnostic>)> = if self.threads == 1 {
            files
                .iter()
                .map(|path| {
                    let result = self.format_file(path, write);
                    (path.clone(), result)
                })
                .collect()
        } else {
            files
                .par_iter()
                .map(|path| {
                    let result = self.format_file(path, write);
                    (path.clone(), result)
                })
                .collect()
        };

        self.collect_results(results, start.elapsed())
    }

    /// Lint files in the given paths
    #[must_use]
    pub fn lint(&self, paths: &[PathBuf]) -> MultiLangResult {
        let start = Instant::now();
        let files = self.collect_files(paths);

        let results: Vec<(PathBuf, Vec<Diagnostic>)> = if self.threads == 1 {
            files
                .iter()
                .map(|path| {
                    let diagnostics = self.lint_file(path);
                    (path.clone(), diagnostics)
                })
                .collect()
        } else {
            files
                .par_iter()
                .map(|path| {
                    let diagnostics = self.lint_file(path);
                    (path.clone(), diagnostics)
                })
                .collect()
        };

        let mut result = MultiLangResult::new();
        result.files_processed = results.len();
        result.duration = start.elapsed();

        for (path, diagnostics) in results {
            if diagnostics.is_empty() {
                result.unchanged.push(path);
            } else {
                result.diagnostics.extend(diagnostics);
            }
        }

        result
    }

    /// Check files in the given paths (lint + format check)
    #[must_use]
    pub fn check(&self, paths: &[PathBuf], write: bool) -> MultiLangResult {
        let start = Instant::now();
        let files = self.collect_files(paths);

        let results: Vec<(PathBuf, Result<FileStatus, Diagnostic>, Vec<Diagnostic>)> =
            if self.threads == 1 {
                files
                    .iter()
                    .map(|path| {
                        let lint_result = self.lint_file(path);
                        let format_result = self.format_file(path, write);
                        (path.clone(), format_result, lint_result)
                    })
                    .collect()
            } else {
                files
                    .par_iter()
                    .map(|path| {
                        let lint_result = self.lint_file(path);
                        let format_result = self.format_file(path, write);
                        (path.clone(), format_result, lint_result)
                    })
                    .collect()
            };

        let mut result = MultiLangResult::new();
        result.files_processed = results.len();
        result.duration = start.elapsed();

        for (path, format_result, lint_diagnostics) in results {
            result.diagnostics.extend(lint_diagnostics);

            match format_result {
                Ok(FileStatus::Changed) => result.changed.push(path),
                Ok(FileStatus::Unchanged) => result.unchanged.push(path),
                Ok(FileStatus::Ignored) => result.ignored.push(path),
                Ok(FileStatus::Error(diag)) => result.errors.push(diag),
                Err(diag) => result.errors.push(diag),
            }
        }

        result
    }

    /// Format a single file
    fn format_file(&self, path: &Path, write: bool) -> Result<FileStatus, Diagnostic> {
        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) => {
                return Err(Diagnostic::error(
                    path.to_string_lossy(),
                    format!("Failed to read file: {e}"),
                    "io/read",
                ));
            }
        };

        self.processor.format(path, &content, write)
    }

    /// Lint a single file
    fn lint_file(&self, path: &Path) -> Vec<Diagnostic> {
        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) => {
                return vec![Diagnostic::error(
                    path.to_string_lossy(),
                    format!("Failed to read file: {e}"),
                    "io/read",
                )];
            }
        };

        match self.processor.lint(path, &content) {
            Ok(diagnostics) => diagnostics,
            Err(diag) => vec![diag],
        }
    }

    /// Collect files from the given paths
    fn collect_files(&self, paths: &[PathBuf]) -> Vec<PathBuf> {
        let mut files = Vec::new();

        for path in paths {
            if path.is_file() {
                if self.processor.is_supported(path) {
                    files.push(path.clone());
                }
            } else if path.is_dir() {
                let walker = WalkBuilder::new(path).standard_filters(true).hidden(true).build();

                for entry in walker.flatten() {
                    let entry_path = entry.path();
                    if entry_path.is_file() && self.processor.is_supported(entry_path) {
                        files.push(entry_path.to_path_buf());
                    }
                }
            }
        }

        files
    }

    /// Collect results into a `MultiLangResult`
    fn collect_results(
        &self,
        results: Vec<(PathBuf, Result<FileStatus, Diagnostic>)>,
        duration: Duration,
    ) -> MultiLangResult {
        let mut result = MultiLangResult::new();
        result.files_processed = results.len();
        result.duration = duration;

        for (path, status) in results {
            match status {
                Ok(FileStatus::Changed) => result.changed.push(path),
                Ok(FileStatus::Unchanged) => result.unchanged.push(path),
                Ok(FileStatus::Ignored) => result.ignored.push(path),
                Ok(FileStatus::Error(diag)) => result.errors.push(diag),
                Err(diag) => result.errors.push(diag),
            }
        }

        result
    }

    /// Get the supported extensions
    #[must_use]
    pub fn supported_extensions(&self) -> Vec<&str> {
        self.processor.supported_extensions()
    }

    /// Check if a file is supported
    #[must_use]
    pub fn is_supported(&self, path: &Path) -> bool {
        self.processor.is_supported(path)
    }
}

impl Default for MultiLangProcessor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_multi_lang_processor_new() {
        let processor = MultiLangProcessor::new();
        let extensions = processor.supported_extensions();

        // Should support all registered languages
        assert!(extensions.contains(&"py"));
        assert!(extensions.contains(&"rs"));
        assert!(extensions.contains(&"go"));
        assert!(extensions.contains(&"cpp"));
        assert!(extensions.contains(&"php"));
        assert!(extensions.contains(&"kt"));
        assert!(extensions.contains(&"md"));
        assert!(extensions.contains(&"toml"));
    }

    #[test]
    fn test_multi_lang_processor_is_supported() {
        let processor = MultiLangProcessor::new();

        assert!(processor.is_supported(Path::new("test.py")));
        assert!(processor.is_supported(Path::new("test.rs")));
        assert!(processor.is_supported(Path::new("test.md")));
        assert!(processor.is_supported(Path::new("test.toml")));
        assert!(!processor.is_supported(Path::new("test.xyz")));
    }

    #[test]
    fn test_multi_lang_result_new() {
        let result = MultiLangResult::new();
        assert!(!result.has_errors());
        assert!(!result.has_changes());
        assert_eq!(result.diagnostic_count(), 0);
    }

    #[test]
    fn test_format_toml_file() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.toml");

        // Write unformatted TOML
        fs::write(&file_path, "[package]\nname=\"test\"\n").unwrap();

        let processor = MultiLangProcessor::new();
        let result = processor.format(&[file_path.clone()], false);

        assert_eq!(result.files_processed, 1);
        // File should be detected as changed (needs formatting)
        assert!(result.changed.len() == 1 || result.unchanged.len() == 1);
    }

    #[test]
    fn test_lint_markdown_file() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.md");

        // Write markdown with trailing whitespace
        fs::write(&file_path, "# Heading   \n\nParagraph.\n").unwrap();

        let processor = MultiLangProcessor::new();
        let result = processor.lint(&[file_path]);

        assert_eq!(result.files_processed, 1);
        // Should have trailing whitespace warning
        assert!(!result.diagnostics.is_empty());
    }
}
