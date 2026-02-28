//! Markdown workflow: .md (human format) → .llm (LLM-optimized) + .machine (binary)
//!
//! This module implements the complete workflow for processing markdown files:
//! 1. Read .md files from disk (human format with ASCII tables)
//! 2. Generate .dx/markdown/*.llm (LLM-optimized format)
//! 3. Generate .dx/markdown/*.machine (binary format)
//! 4. Keep .md files in human format on disk

use crate::beautifier::{MarkdownBeautifier, autofix_markdown, lint_markdown};
use crate::compiler::DxMarkdown;
use crate::error::CompileError;
use crate::types::CompilerConfig;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Workflow configuration
#[derive(Debug, Clone)]
pub struct WorkflowConfig {
    /// Root directory to scan for .md files
    pub root_dir: PathBuf,
    /// Output directory for .llm and .machine files (default: .dx/markdown)
    pub output_dir: PathBuf,
    /// Whether to auto-fix lint issues
    pub autofix: bool,
    /// Whether to show lint warnings
    pub show_lint: bool,
    /// Compiler configuration for LLM optimization
    pub compiler_config: CompilerConfig,
}

impl WorkflowConfig {
    /// Create a new workflow configuration
    pub fn new(root_dir: impl Into<PathBuf>) -> Self {
        let root = root_dir.into();
        let output_dir = root.join(".dx").join("markdown");

        Self {
            root_dir: root,
            output_dir,
            autofix: true,
            show_lint: true,
            compiler_config: CompilerConfig::default(),
        }
    }
}

/// Workflow result
#[derive(Debug)]
pub struct WorkflowResult {
    /// Number of files processed
    pub files_processed: usize,
    /// Number of files with lint issues
    pub files_with_issues: usize,
    /// Total tokens saved
    pub total_tokens_saved: usize,
    /// Total tokens before optimization
    pub total_tokens_before: usize,
    /// Total tokens after optimization
    pub total_tokens_after: usize,
}

impl WorkflowResult {
    /// Calculate savings percentage
    pub fn savings_percent(&self) -> f64 {
        if self.total_tokens_before == 0 {
            return 0.0;
        }
        ((self.total_tokens_before - self.total_tokens_after) as f64
            / self.total_tokens_before as f64)
            * 100.0
    }
}

/// Markdown workflow processor
pub struct MarkdownWorkflow {
    config: WorkflowConfig,
    beautifier: MarkdownBeautifier,
    compiler: DxMarkdown,
}

impl MarkdownWorkflow {
    /// Create a new workflow processor
    pub fn new(config: WorkflowConfig) -> Result<Self, CompileError> {
        let beautifier = MarkdownBeautifier::new();
        let compiler = DxMarkdown::new(config.compiler_config.clone())?;

        Ok(Self {
            config,
            beautifier,
            compiler,
        })
    }

    /// Run the complete workflow
    pub fn run(&self) -> Result<WorkflowResult, CompileError> {
        let mut result = WorkflowResult {
            files_processed: 0,
            files_with_issues: 0,
            total_tokens_saved: 0,
            total_tokens_before: 0,
            total_tokens_after: 0,
        };

        // Create output directory
        std::fs::create_dir_all(&self.config.output_dir)
            .map_err(|e| CompileError::io(e.to_string()))?;

        // Find all .md files
        let md_files = self.find_markdown_files()?;

        for md_file in md_files {
            match self.process_file(&md_file) {
                Ok(file_result) => {
                    result.files_processed += 1;
                    if file_result.had_lint_issues {
                        result.files_with_issues += 1;
                    }
                    result.total_tokens_before += file_result.tokens_before;
                    result.total_tokens_after += file_result.tokens_after;
                    result.total_tokens_saved +=
                        file_result.tokens_before.saturating_sub(file_result.tokens_after);
                }
                Err(e) => {
                    eprintln!("Error processing {}: {}", md_file.display(), e);
                }
            }
        }

        Ok(result)
    }

    /// Find all markdown files in the root directory
    fn find_markdown_files(&self) -> Result<Vec<PathBuf>, CompileError> {
        let mut files = Vec::new();

        for entry in WalkDir::new(&self.config.root_dir)
            .follow_links(true)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();

            // Skip .dx directory
            if path.starts_with(&self.config.output_dir) {
                continue;
            }

            // Only process .md files
            if path.extension().and_then(|s| s.to_str()) == Some("md") {
                files.push(path.to_path_buf());
            }
        }

        Ok(files)
    }

    /// Process a single markdown file
    fn process_file(&self, md_file: &Path) -> Result<FileResult, CompileError> {
        // Read original content
        let content =
            std::fs::read_to_string(md_file).map_err(|e| CompileError::io(e.to_string()))?;

        // Lint and optionally autofix
        let mut processed_content = content.clone();
        let mut had_lint_issues = false;

        if self.config.show_lint {
            let issues = lint_markdown(&processed_content);
            if !issues.is_empty() {
                had_lint_issues = true;
                if self.config.show_lint {
                    println!("Lint issues in {}:", md_file.display());
                    for issue in &issues {
                        println!("  - {}", issue);
                    }
                }
            }
        }

        if self.config.autofix {
            processed_content = autofix_markdown(&processed_content);
        }

        // Convert to LLM-optimized format
        let llm_result = self.compiler.compile(&processed_content)?;

        // Create relative path structure in .dx/markdown
        let relative_path = md_file.strip_prefix(&self.config.root_dir).unwrap_or(md_file);

        let llm_path = self.config.output_dir.join(relative_path).with_extension("llm");
        let machine_path = self.config.output_dir.join(relative_path).with_extension("machine");

        // Create parent directories
        if let Some(parent) = llm_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| CompileError::io(e.to_string()))?;
        }

        // Write .llm file (LLM-optimized format)
        std::fs::write(&llm_path, &llm_result.output)
            .map_err(|e| CompileError::io(e.to_string()))?;

        // Write .machine file (binary format - placeholder for now)
        // TODO: Implement actual binary serialization
        std::fs::write(&machine_path, &llm_result.output.as_bytes())
            .map_err(|e| CompileError::io(e.to_string()))?;

        // Keep .md file in human format (don't overwrite)

        Ok(FileResult {
            had_lint_issues,
            tokens_before: llm_result.tokens_before,
            tokens_after: llm_result.tokens_after,
        })
    }
}

/// Result for a single file
struct FileResult {
    had_lint_issues: bool,
    tokens_before: usize,
    tokens_after: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_workflow_config_creation() {
        let temp_dir = TempDir::new().unwrap();
        let config = WorkflowConfig::new(temp_dir.path());

        assert_eq!(config.root_dir, temp_dir.path());
        assert_eq!(config.output_dir, temp_dir.path().join(".dx").join("markdown"));
    }

    #[test]
    fn test_find_markdown_files() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        // Create test files
        std::fs::write(root.join("test1.md"), "# Test 1").unwrap();
        std::fs::write(root.join("test2.md"), "# Test 2").unwrap();
        std::fs::write(root.join("test.txt"), "Not markdown").unwrap();

        let config = WorkflowConfig::new(root);
        let workflow = MarkdownWorkflow::new(config).unwrap();
        let files = workflow.find_markdown_files().unwrap();

        assert_eq!(files.len(), 2);
    }
}
