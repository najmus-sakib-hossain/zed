//! Example: Beautify markdown files
//!
//! This example demonstrates the markdown beautification workflow:
//! 1. Read .md files
//! 2. Auto-format and lint
//! 3. Save beautified version to .dx/markdown/*.human
//! 4. Convert to LLM-optimized format
//! 5. Write back to .md files

use markdown::workflow::{MarkdownWorkflow, WorkflowConfig};
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Get directory from args or use current directory
    let dir = std::env::args().nth(1).map(PathBuf::from).unwrap_or_else(|| PathBuf::from("."));

    println!("🎨 DX Markdown Beautifier");
    println!("==========================\n");
    println!("Processing directory: {}\n", dir.display());

    // Create workflow configuration
    let config = WorkflowConfig::new(&dir);

    // Create and run workflow
    let workflow = MarkdownWorkflow::new(config)?;
    let result = workflow.run()?;

    // Print results
    println!("\n✅ Workflow Complete!");
    println!("====================");
    println!("Files processed: {}", result.files_processed);
    println!("Files with lint issues: {}", result.files_with_issues);
    println!("\nToken Optimization:");
    println!("  Before: {} tokens", result.total_tokens_before);
    println!("  After:  {} tokens", result.total_tokens_after);
    println!(
        "  Saved:  {} tokens ({:.1}%)",
        result.total_tokens_saved,
        result.savings_percent()
    );

    println!("\n📁 Human-readable files saved to: .dx/markdown/");
    println!("📄 LLM-optimized files written back to: *.md");

    Ok(())
}
