//! Convert command - convert rules between formats

use crate::{Editor, Result, RuleSet};
use std::path::Path;

/// Convert command handler
#[derive(Debug)]
pub struct ConvertCommand;

impl ConvertCommand {
    /// Convert rules from one format to another
    pub fn run(input: &Path, output: &Path, target: Option<Editor>) -> Result<()> {
        // Load input
        let spinner = super::create_spinner("Loading rules...");
        let rules = RuleSet::load(input)?;
        spinner.finish_and_clear();

        super::print_info(&format!("Loaded {} rules from {}", rules.len(), input.display()));

        // Convert based on output format
        if let Some(editor) = target {
            // Emit to editor format
            let spinner = super::create_spinner(&format!("Converting to {} format...", editor));
            rules.emit(editor, output)?;
            spinner.finish_and_clear();

            super::print_success(&format!("Converted to {} format: {}", editor, output.display()));
        } else if output.extension().is_some_and(|ext| ext == "drv") {
            // Convert to binary
            let spinner = super::create_spinner("Converting to binary format...");
            rules.save_binary(output)?;
            spinner.finish_and_clear();

            super::print_success(&format!("Converted to binary: {}", output.display()));
        } else {
            // Convert to markdown (default)
            let spinner = super::create_spinner("Converting to markdown...");
            rules.emit(Editor::Copilot, output)?;
            spinner.finish_and_clear();

            super::print_success(&format!("Converted to markdown: {}", output.display()));
        }

        Ok(())
    }

    /// Batch convert to all editor formats
    pub fn batch_convert(input: &Path, project_root: &Path, editors: &[Editor]) -> Result<()> {
        let rules = RuleSet::load(input)?;

        super::print_info(&format!("Converting to {} editor formats...", editors.len()));

        let pb = super::create_progress_bar(editors.len() as u64, "Converting...");

        for editor in editors {
            let output = project_root.join(editor.rule_path());
            rules.emit(*editor, &output)?;
            pb.inc(1);
        }

        pb.finish_with_message("Done!");
        super::print_success("All conversions complete");

        Ok(())
    }
}
