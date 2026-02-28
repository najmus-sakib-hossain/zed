//! Validate workspace configuration.

use crate::{Result, WorkspaceConfig};
use console::{Emoji, style};
use std::path::PathBuf;

static CHECK: Emoji<'_, '_> = Emoji("✓ ", "");
static WARN: Emoji<'_, '_> = Emoji("⚠️ ", "!");
static ERROR: Emoji<'_, '_> = Emoji("✗ ", "X");

/// Options for validation.
#[derive(Debug, Default)]
pub struct ValidateOptions {
    /// Project directory.
    pub path: Option<PathBuf>,
    /// Show detailed validation output.
    pub verbose: bool,
}

/// Validation result.
#[derive(Debug)]
pub struct ValidationResult {
    /// Whether validation passed.
    pub valid: bool,
    /// Error messages.
    pub errors: Vec<String>,
    /// Warning messages.
    pub warnings: Vec<String>,
    /// Informational messages.
    pub info: Vec<String>,
}

/// Command to validate workspace configuration.
pub struct ValidateCommand;

impl ValidateCommand {
    /// Execute the validate command.
    pub fn execute(options: ValidateOptions) -> Result<ValidationResult> {
        let project_dir = options.path.unwrap_or_else(|| PathBuf::from("."));
        let project_dir = std::fs::canonicalize(&project_dir).unwrap_or(project_dir);

        println!(
            "{} Validating workspace configuration...",
            style("[dx-workspace]").bold().cyan()
        );

        let config_path = project_dir.join("dx-workspace.json");
        let mut result = ValidationResult {
            valid: true,
            errors: Vec::new(),
            warnings: Vec::new(),
            info: Vec::new(),
        };

        // Check if config exists
        if !config_path.exists() {
            result.warnings.push(
                "No dx-workspace.json found. Run 'dx workspace init' to create one.".to_string(),
            );
        } else {
            // Load and validate config
            match WorkspaceConfig::load(&config_path) {
                Ok(config) => {
                    // Validate the config
                    if let Err(e) = config.validate() {
                        result.valid = false;
                        result.errors.push(e.to_string());
                    }

                    // Check for potential issues
                    Self::check_config(&config, &mut result);

                    result.info.push(format!("Project: {}", config.name));
                    result.info.push(format!("Schema version: {}", config.schema_version));
                }
                Err(e) => {
                    result.valid = false;
                    result.errors.push(format!("Failed to load config: {}", e));
                }
            }
        }

        // Print results
        Self::print_results(&result, options.verbose);

        Ok(result)
    }

    fn check_config(config: &WorkspaceConfig, result: &mut ValidationResult) {
        let features = &config.detected_features;

        // Check for potential issues
        if features.is_cargo_project && config.tasks.tasks.is_empty() {
            result.warnings.push(
                "Cargo project detected but no tasks defined. Consider running 'dx workspace generate'.".to_string()
            );
        }

        if features.has_dx_client && !features.has_dx_www {
            result.info.push(
                "dx-client detected without dx-www. This is valid for WASM-only projects."
                    .to_string(),
            );
        }

        if config.extensions.core.is_empty() && config.extensions.recommended.is_empty() {
            result.warnings.push("No extension recommendations configured.".to_string());
        }

        // Check for existing but possibly outdated configs
        if features.has_vscode_config {
            result.info.push(
                "Existing .vscode configuration detected. Use 'dx workspace sync' to synchronize."
                    .to_string(),
            );
        }
    }

    fn print_results(result: &ValidationResult, verbose: bool) {
        println!();

        // Print errors
        for error in &result.errors {
            println!("  {} {}", style(ERROR).red(), error);
        }

        // Print warnings
        for warning in &result.warnings {
            println!("  {} {}", style(WARN).yellow(), warning);
        }

        // Print info (only in verbose mode)
        if verbose {
            for info in &result.info {
                println!("  {} {}", style("ℹ").blue(), info);
            }
        }

        println!();

        if result.valid {
            println!(
                "{} {}Workspace configuration is valid!",
                style("[dx-workspace]").bold().cyan(),
                CHECK
            );
        } else {
            println!(
                "{} Workspace configuration has {} error(s)",
                style("[dx-workspace]").bold().red(),
                result.errors.len()
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validation_result_default() {
        let result = ValidationResult {
            valid: true,
            errors: vec![],
            warnings: vec![],
            info: vec![],
        };
        assert!(result.valid);
    }
}
