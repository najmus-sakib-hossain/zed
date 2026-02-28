//! Analyze command - analyze codebase for context

use crate::{Result, context::ProjectScanner};
use std::path::Path;

/// Analyze command handler
#[derive(Debug)]
pub struct AnalyzeCommand;

impl AnalyzeCommand {
    /// Run analysis
    pub fn run(project_root: &Path) -> Result<()> {
        let spinner = super::create_spinner("Analyzing project structure...");

        let scanner = ProjectScanner::new();
        let result = scanner.scan(project_root)?;

        spinner.finish_and_clear();

        // Display results
        println!("📊 Project Analysis:");
        println!();

        println!("  Languages:");
        for lang in &result.languages {
            println!("    - {}", lang);
        }

        println!();
        println!("  Frameworks:");
        for framework in &result.frameworks {
            println!("    - {}", framework);
        }

        println!();
        println!("  Statistics:");
        let total_files: usize = result.file_counts.values().sum();
        println!("    - Files: {}", total_files);
        println!("    - Directories: {}", result.key_directories.len());

        if !result.config_files.is_empty() {
            println!();
            println!("  Config files found:");
            for config in result.config_files.iter().take(10) {
                println!("    - {}", config);
            }
            if result.config_files.len() > 10 {
                println!("    ... and {} more", result.config_files.len() - 10);
            }
        }

        Ok(())
    }

    /// Generate context rules from analysis
    pub fn generate_context(project_root: &Path, output: &Path) -> Result<()> {
        use crate::context::ContextProvider;

        let spinner = super::create_spinner("Analyzing and generating context...");

        let provider = ContextProvider::new();
        let context = provider.generate(project_root)?;

        spinner.finish_and_clear();

        // Write context to output
        std::fs::write(output, context)
            .map_err(|e| crate::DrivenError::Context(format!("Failed to write context: {}", e)))?;

        super::print_success(&format!("Generated context rules: {}", output.display()));

        Ok(())
    }

    /// Index codebase for fast lookups
    pub fn index(project_root: &Path) -> Result<()> {
        use crate::context::CodebaseIndexer;

        let spinner = super::create_spinner("Indexing codebase...");

        let indexer = CodebaseIndexer::new();
        let index = indexer.index(project_root)?;

        // Save index
        let index_path = project_root.join(".driven/index.bin");
        if let Some(parent) = index_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                crate::DrivenError::Context(format!("Failed to create directory: {}", e))
            })?;
        }

        index.save(&index_path)?;

        spinner.finish_and_clear();

        super::print_success(&format!(
            "Indexed {} files ({} bytes)",
            index.file_count(),
            index.size_bytes()
        ));

        Ok(())
    }
}
