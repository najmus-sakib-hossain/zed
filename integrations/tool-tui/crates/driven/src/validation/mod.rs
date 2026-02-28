//! Rule Validation
//!
//! Linting, conflict detection, and coverage analysis for AI rules.

mod completeness;
mod conflicts;
mod linter;

pub use completeness::CoverageAnalyzer;
pub use conflicts::ConflictDetector;
pub use linter::{LintResult, LintSeverity, Linter};

use crate::{Result, parser::UnifiedRule};

/// Result of validating rules
#[derive(Debug, Default)]
pub struct ValidationResult {
    /// Lint issues found
    pub lint_issues: Vec<LintResult>,
    /// Conflicts detected
    pub conflicts: Vec<Conflict>,
    /// Coverage gaps
    pub coverage_gaps: Vec<String>,
}

impl ValidationResult {
    /// Check if validation passed (no errors)
    pub fn is_valid(&self) -> bool {
        !self.lint_issues.iter().any(|i| matches!(i.severity, LintSeverity::Error))
            && self.conflicts.is_empty()
    }

    /// Get error count
    pub fn error_count(&self) -> usize {
        self.lint_issues
            .iter()
            .filter(|i| matches!(i.severity, LintSeverity::Error))
            .count()
            + self.conflicts.len()
    }

    /// Get warning count
    pub fn warning_count(&self) -> usize {
        self.lint_issues
            .iter()
            .filter(|i| matches!(i.severity, LintSeverity::Warning))
            .count()
    }
}

/// A conflict between rules
#[derive(Debug, Clone)]
pub struct Conflict {
    /// Description of the conflict
    pub description: String,
    /// Rules involved
    pub rules: Vec<String>,
    /// Suggested resolution
    pub suggestion: Option<String>,
}

/// Validate a set of rules
pub fn validate(rules: &[UnifiedRule]) -> Result<ValidationResult> {
    let mut result = ValidationResult::default();

    // Run linter
    let linter = Linter::new();
    result.lint_issues = linter.lint(rules)?;

    // Detect conflicts
    let detector = ConflictDetector::new();
    result.conflicts = detector.detect(rules)?;

    // Analyze coverage
    let analyzer = CoverageAnalyzer::new();
    result.coverage_gaps = analyzer.analyze(rules)?;

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validation_result_default() {
        let result = ValidationResult::default();
        assert!(result.is_valid());
        assert_eq!(result.error_count(), 0);
    }

    #[test]
    fn test_validate_empty() {
        let result = validate(&[]).unwrap();
        assert!(result.is_valid());
    }
}
