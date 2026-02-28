//! Git repository processing for the DX Markdown Context Compiler.
//!
//! This module provides functionality to process all Markdown files in a
//! Git repository, optionally filtering by changed files since a reference.

use crate::compiler::DxMarkdown;
use crate::error::CompileError;
use crate::types::{CompilerConfig, SavingsBreakdown};
use std::fs;
use std::path::{Path, PathBuf};

/// Process all Markdown files in a directory.
///
/// # Arguments
/// * `path` - Path to the directory
/// * `config` - Compiler configuration
///
/// # Returns
/// Combined compilation result for all files
pub fn process_directory(
    path: &Path,
    config: &CompilerConfig,
) -> Result<RepoCompileResult, CompileError> {
    let compiler = DxMarkdown::new(config.clone())?;
    let mut results = Vec::new();
    let mut total_tokens_before = 0;
    let mut total_tokens_after = 0;

    // Find all markdown files
    let md_files = find_markdown_files(path)?;

    for file_path in md_files {
        let content = fs::read_to_string(&file_path)?;
        let result = compiler.compile(&content)?;

        total_tokens_before += result.tokens_before;
        total_tokens_after += result.tokens_after;

        results.push(FileResult {
            path: file_path,
            tokens_before: result.tokens_before,
            tokens_after: result.tokens_after,
            output: result.output,
        });
    }

    Ok(RepoCompileResult {
        files: results,
        total_tokens_before,
        total_tokens_after,
        breakdown: SavingsBreakdown::default(),
    })
}

/// Bundle all Markdown files into a single output.
///
/// # Arguments
/// * `path` - Path to the directory
/// * `config` - Compiler configuration
///
/// # Returns
/// Single bundled output with file headers
pub fn bundle_directory(
    path: &Path,
    config: &CompilerConfig,
) -> Result<BundleResult, CompileError> {
    let compiler = DxMarkdown::new(config.clone())?;
    let mut bundled_output = String::new();
    let mut total_tokens_before = 0;
    let mut file_count = 0;

    // Find all markdown files
    let md_files = find_markdown_files(path)?;

    for file_path in md_files {
        let content = fs::read_to_string(&file_path)?;
        let result = compiler.compile(&content)?;

        // Add file header
        let relative_path = file_path.strip_prefix(path).unwrap_or(&file_path).display();
        bundled_output.push_str(&format!("\n# FILE: {}\n\n", relative_path));
        bundled_output.push_str(&result.output);
        bundled_output.push_str("\n\n");

        total_tokens_before += result.tokens_before;
        file_count += 1;
    }

    // Count tokens for the bundled output
    let bundled_tokens = compiler.compile(&bundled_output)?.tokens_after;

    Ok(BundleResult {
        output: bundled_output.trim().to_string(),
        file_count,
        total_tokens_before,
        total_tokens_after: bundled_tokens,
    })
}

/// Find all Markdown files in a directory recursively.
fn find_markdown_files(path: &Path) -> Result<Vec<PathBuf>, CompileError> {
    let mut files = Vec::new();
    find_markdown_files_recursive(path, &mut files)?;
    files.sort();
    Ok(files)
}

/// Recursive helper for finding Markdown files.
fn find_markdown_files_recursive(
    path: &Path,
    files: &mut Vec<PathBuf>,
) -> Result<(), CompileError> {
    if path.is_file() {
        if is_markdown_file(path) {
            files.push(path.to_path_buf());
        }
        return Ok(());
    }

    if path.is_dir() {
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            let entry_path = entry.path();

            // Skip hidden directories and common non-content directories
            if entry_path.is_dir() {
                let dir_name = entry_path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if dir_name.starts_with('.') || dir_name == "node_modules" || dir_name == "target" {
                    continue;
                }
            }

            find_markdown_files_recursive(&entry_path, files)?;
        }
    }

    Ok(())
}

/// Check if a file is a Markdown file.
fn is_markdown_file(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.eq_ignore_ascii_case("md") || ext.eq_ignore_ascii_case("markdown"))
        .unwrap_or(false)
}

/// Result of compiling a repository.
#[derive(Debug, Clone)]
pub struct RepoCompileResult {
    /// Results for each file
    pub files: Vec<FileResult>,
    /// Total tokens before optimization
    pub total_tokens_before: usize,
    /// Total tokens after optimization
    pub total_tokens_after: usize,
    /// Savings breakdown
    pub breakdown: SavingsBreakdown,
}

impl RepoCompileResult {
    /// Calculate total savings percentage.
    pub fn savings_percent(&self) -> f64 {
        if self.total_tokens_before == 0 {
            return 0.0;
        }
        let saved = self.total_tokens_before.saturating_sub(self.total_tokens_after);
        (saved as f64 / self.total_tokens_before as f64) * 100.0
    }

    /// Get total tokens saved.
    pub fn tokens_saved(&self) -> usize {
        self.total_tokens_before.saturating_sub(self.total_tokens_after)
    }
}

/// Result for a single file.
#[derive(Debug, Clone)]
pub struct FileResult {
    /// File path
    pub path: PathBuf,
    /// Tokens before optimization
    pub tokens_before: usize,
    /// Tokens after optimization
    pub tokens_after: usize,
    /// Optimized output
    pub output: String,
}

impl FileResult {
    /// Calculate savings percentage for this file.
    pub fn savings_percent(&self) -> f64 {
        if self.tokens_before == 0 {
            return 0.0;
        }
        let saved = self.tokens_before.saturating_sub(self.tokens_after);
        (saved as f64 / self.tokens_before as f64) * 100.0
    }
}

/// Result of bundling a repository.
#[derive(Debug, Clone)]
pub struct BundleResult {
    /// Bundled output
    pub output: String,
    /// Number of files bundled
    pub file_count: usize,
    /// Total tokens before optimization
    pub total_tokens_before: usize,
    /// Total tokens after optimization
    pub total_tokens_after: usize,
}

impl BundleResult {
    /// Calculate savings percentage.
    pub fn savings_percent(&self) -> f64 {
        if self.total_tokens_before == 0 {
            return 0.0;
        }
        let saved = self.total_tokens_before.saturating_sub(self.total_tokens_after);
        (saved as f64 / self.total_tokens_before as f64) * 100.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::TempDir;

    fn create_test_dir() -> TempDir {
        TempDir::new().unwrap()
    }

    fn create_md_file(dir: &Path, name: &str, content: &str) {
        let path = dir.join(name);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        let mut file = File::create(path).unwrap();
        file.write_all(content.as_bytes()).unwrap();
    }

    #[test]
    fn test_is_markdown_file() {
        assert!(is_markdown_file(Path::new("README.md")));
        assert!(is_markdown_file(Path::new("docs/guide.MD")));
        assert!(is_markdown_file(Path::new("file.markdown")));
        assert!(!is_markdown_file(Path::new("file.txt")));
        assert!(!is_markdown_file(Path::new("file.rs")));
    }

    #[test]
    fn test_find_markdown_files() {
        let dir = create_test_dir();

        create_md_file(dir.path(), "README.md", "# Hello");
        create_md_file(dir.path(), "docs/guide.md", "# Guide");
        create_md_file(dir.path(), "src/main.rs", "fn main() {}");

        let files = find_markdown_files(dir.path()).unwrap();

        assert_eq!(files.len(), 2);
        assert!(files.iter().any(|p| p.ends_with("README.md")));
        assert!(files.iter().any(|p| p.ends_with("guide.md")));
    }

    #[test]
    fn test_find_markdown_files_skips_hidden() {
        let dir = create_test_dir();

        create_md_file(dir.path(), "README.md", "# Hello");
        create_md_file(dir.path(), ".hidden/secret.md", "# Secret");

        let files = find_markdown_files(dir.path()).unwrap();

        assert_eq!(files.len(), 1);
        assert!(files[0].ends_with("README.md"));
    }

    #[test]
    fn test_process_directory() {
        let dir = create_test_dir();

        create_md_file(dir.path(), "README.md", "# Hello\n\nWorld");
        create_md_file(dir.path(), "docs/guide.md", "# Guide\n\nContent");

        let config = CompilerConfig::default();
        let result = process_directory(dir.path(), &config).unwrap();

        assert_eq!(result.files.len(), 2);
        assert!(result.total_tokens_before > 0);
    }

    #[test]
    fn test_bundle_directory() {
        let dir = create_test_dir();

        create_md_file(dir.path(), "README.md", "# Hello");
        create_md_file(dir.path(), "docs/guide.md", "# Guide");

        let config = CompilerConfig::default();
        let result = bundle_directory(dir.path(), &config).unwrap();

        assert_eq!(result.file_count, 2);
        assert!(result.output.contains("FILE:"));
        assert!(result.output.contains("Hello"));
        assert!(result.output.contains("Guide"));
    }

    #[test]
    fn test_repo_compile_result_savings() {
        let result = RepoCompileResult {
            files: vec![],
            total_tokens_before: 100,
            total_tokens_after: 60,
            breakdown: SavingsBreakdown::default(),
        };

        assert_eq!(result.tokens_saved(), 40);
        assert!((result.savings_percent() - 40.0).abs() < 0.01);
    }

    #[test]
    fn test_file_result_savings() {
        let result = FileResult {
            path: PathBuf::from("test.md"),
            tokens_before: 100,
            tokens_after: 75,
            output: String::new(),
        };

        assert!((result.savings_percent() - 25.0).abs() < 0.01);
    }

    #[test]
    fn test_bundle_result_savings() {
        let result = BundleResult {
            output: String::new(),
            file_count: 2,
            total_tokens_before: 200,
            total_tokens_after: 150,
        };

        assert!((result.savings_percent() - 25.0).abs() < 0.01);
    }
}
