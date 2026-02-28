//! Validate command - validate rules

use crate::{Result, RuleSet, validation};
use std::path::Path;

/// Validate command handler
#[derive(Debug)]
pub struct ValidateCommand;

impl ValidateCommand {
    /// Run validation
    pub fn run(rules_path: &Path) -> Result<()> {
        let spinner = super::create_spinner("Loading and validating rules...");

        // Load rules
        let rule_set = RuleSet::load(rules_path)?;
        let rules = rule_set.as_unified();

        // Validate
        let result = validation::validate(&rules)?;

        spinner.finish_and_clear();

        // Display results
        if result.is_valid() {
            super::print_success("All rules are valid!");
        } else {
            super::print_error(&format!(
                "Validation failed: {} errors, {} warnings",
                result.error_count(),
                result.warning_count()
            ));
        }

        // Show lint issues
        if !result.lint_issues.is_empty() {
            println!();
            println!("📋 Lint Issues:");
            for issue in &result.lint_issues {
                let prefix = match issue.severity {
                    validation::LintSeverity::Error => "  ❌",
                    validation::LintSeverity::Warning => "  ⚠️",
                    validation::LintSeverity::Info => "  ℹ️",
                };
                println!("{} {}", prefix, issue.message);
                if let Some(suggestion) = &issue.suggestion {
                    println!("     💡 {}", suggestion);
                }
            }
        }

        // Show conflicts
        if !result.conflicts.is_empty() {
            println!();
            println!("⚔️ Conflicts:");
            for conflict in &result.conflicts {
                println!("  ❌ {}", conflict.description);
                for rule in &conflict.rules {
                    println!("     - {}", rule);
                }
                if let Some(suggestion) = &conflict.suggestion {
                    println!("     💡 {}", suggestion);
                }
            }
        }

        // Show coverage gaps
        if !result.coverage_gaps.is_empty() {
            println!();
            println!("📭 Coverage Gaps:");
            for gap in &result.coverage_gaps {
                println!("  ⚠️ {}", gap);
            }
        }

        if result.is_valid() {
            Ok(())
        } else {
            Err(crate::DrivenError::Validation(format!("{} errors found", result.error_count())))
        }
    }

    /// Run validation in strict mode
    pub fn run_strict(rules_path: &Path) -> Result<()> {
        let result = Self::run(rules_path);

        // In strict mode, warnings also fail
        if result.is_ok() {
            let rule_set = RuleSet::load(rules_path)?;
            let rules = rule_set.as_unified();
            let validation_result = validation::validate(&rules)?;

            if validation_result.warning_count() > 0 {
                return Err(crate::DrivenError::Validation(format!(
                    "{} warnings in strict mode",
                    validation_result.warning_count()
                )));
            }
        }

        result
    }
}
