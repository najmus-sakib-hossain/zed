//! Rule linter

use crate::{Result, parser::UnifiedRule};

/// Lints rules for common issues
#[derive(Debug, Default)]
pub struct Linter {
    /// Whether to check for duplicates
    check_duplicates: bool,
    /// Whether to check for vague descriptions
    check_vague: bool,
}

impl Linter {
    /// Create a new linter with default settings
    pub fn new() -> Self {
        Self {
            check_duplicates: true,
            check_vague: true,
        }
    }

    /// Enable/disable duplicate checking
    pub fn with_duplicate_check(mut self, enabled: bool) -> Self {
        self.check_duplicates = enabled;
        self
    }

    /// Lint a set of rules
    pub fn lint(&self, rules: &[UnifiedRule]) -> Result<Vec<LintResult>> {
        let mut results = Vec::new();

        // Check for duplicates
        if self.check_duplicates {
            results.extend(self.check_for_duplicates(rules));
        }

        // Check for vague descriptions
        if self.check_vague {
            results.extend(self.check_for_vague(rules));
        }

        // Check for empty content
        results.extend(self.check_for_empty(rules));

        Ok(results)
    }

    fn check_for_duplicates(&self, rules: &[UnifiedRule]) -> Vec<LintResult> {
        let mut results = Vec::new();
        let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();

        for rule in rules {
            if let UnifiedRule::Standard { description, .. } = rule {
                if !seen.insert(description.clone()) {
                    results.push(LintResult {
                        severity: LintSeverity::Warning,
                        message: format!("Duplicate rule: {}", description),
                        suggestion: Some("Remove one of the duplicate rules".to_string()),
                    });
                }
            }
        }

        results
    }

    fn check_for_vague(&self, rules: &[UnifiedRule]) -> Vec<LintResult> {
        let mut results = Vec::new();
        let vague_words = ["good", "proper", "appropriate", "nice", "clean", "better"];

        for rule in rules {
            if let UnifiedRule::Standard { description, .. } = rule {
                let lower = description.to_lowercase();
                for word in &vague_words {
                    if lower.contains(word) && description.len() < 30 {
                        results.push(LintResult {
                            severity: LintSeverity::Info,
                            message: format!(
                                "Rule may be too vague: '{}' contains '{}'",
                                description, word
                            ),
                            suggestion: Some("Consider being more specific".to_string()),
                        });
                        break;
                    }
                }
            }
        }

        results
    }

    fn check_for_empty(&self, rules: &[UnifiedRule]) -> Vec<LintResult> {
        let mut results = Vec::new();

        for rule in rules {
            match rule {
                UnifiedRule::Standard { description, .. } if description.trim().is_empty() => {
                    results.push(LintResult {
                        severity: LintSeverity::Error,
                        message: "Empty rule description".to_string(),
                        suggestion: Some("Add a meaningful description".to_string()),
                    });
                }
                UnifiedRule::Persona { name, role, .. }
                    if name.trim().is_empty() || role.trim().is_empty() =>
                {
                    results.push(LintResult {
                        severity: LintSeverity::Error,
                        message: "Persona missing name or role".to_string(),
                        suggestion: Some("Add name and role to persona".to_string()),
                    });
                }
                _ => {}
            }
        }

        results
    }
}

/// Lint result
#[derive(Debug, Clone)]
pub struct LintResult {
    /// Severity of the issue
    pub severity: LintSeverity,
    /// Description of the issue
    pub message: String,
    /// Suggested fix
    pub suggestion: Option<String>,
}

/// Lint severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LintSeverity {
    /// Error - must be fixed
    Error,
    /// Warning - should be fixed
    Warning,
    /// Info - optional improvement
    Info,
}

impl std::fmt::Display for LintSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LintSeverity::Error => write!(f, "error"),
            LintSeverity::Warning => write!(f, "warning"),
            LintSeverity::Info => write!(f, "info"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::format::RuleCategory;

    #[test]
    fn test_linter_empty() {
        let linter = Linter::new();
        let results = linter.lint(&[]).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn test_linter_detects_duplicates() {
        let linter = Linter::new();
        let rules = vec![
            UnifiedRule::Standard {
                category: RuleCategory::Style,
                priority: 0,
                description: "Same rule".to_string(),
                pattern: None,
            },
            UnifiedRule::Standard {
                category: RuleCategory::Style,
                priority: 1,
                description: "Same rule".to_string(),
                pattern: None,
            },
        ];

        let results = linter.lint(&rules).unwrap();
        assert!(!results.is_empty());
        assert!(results.iter().any(|r| r.message.contains("Duplicate")));
    }

    #[test]
    fn test_linter_detects_empty() {
        let linter = Linter::new();
        let rules = vec![UnifiedRule::Standard {
            category: RuleCategory::Style,
            priority: 0,
            description: "".to_string(),
            pattern: None,
        }];

        let results = linter.lint(&rules).unwrap();
        assert!(!results.is_empty());
        assert!(results.iter().any(|r| matches!(r.severity, LintSeverity::Error)));
    }
}
