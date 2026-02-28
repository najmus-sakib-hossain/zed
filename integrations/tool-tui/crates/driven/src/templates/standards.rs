//! Coding standards templates

use super::{Template, TemplateCategory};
use crate::{Result, format::RuleCategory, parser::UnifiedRule};

/// Standards template definition
#[derive(Debug, Clone)]
pub struct StandardsTemplate {
    name: String,
    description: String,
    rules: Vec<(RuleCategory, u8, String)>,
    tags: Vec<String>,
}

impl StandardsTemplate {
    /// Rust idiomatic patterns
    pub fn rust_idioms() -> Self {
        Self {
            name: "rust-idioms".to_string(),
            description: "Idiomatic Rust coding patterns".to_string(),
            rules: vec![
                (
                    RuleCategory::Naming,
                    0,
                    "Use snake_case for functions and variables".to_string(),
                ),
                (RuleCategory::Naming, 1, "Use PascalCase for types and traits".to_string()),
                (RuleCategory::Naming, 2, "Use SCREAMING_SNAKE_CASE for constants".to_string()),
                (
                    RuleCategory::Style,
                    0,
                    "Prefer &str over String for function parameters".to_string(),
                ),
                (
                    RuleCategory::Style,
                    1,
                    "Use iterators instead of manual loops when possible".to_string(),
                ),
                (
                    RuleCategory::Style,
                    2,
                    "Prefer Option and Result over nulls and exceptions".to_string(),
                ),
                (
                    RuleCategory::ErrorHandling,
                    0,
                    "Use ? operator for error propagation".to_string(),
                ),
                (
                    RuleCategory::ErrorHandling,
                    1,
                    "Create custom error types for libraries".to_string(),
                ),
                (
                    RuleCategory::Performance,
                    0,
                    "Prefer references over cloning when possible".to_string(),
                ),
                (
                    RuleCategory::Performance,
                    1,
                    "Use Cow<str> for potentially owned strings".to_string(),
                ),
            ],
            tags: vec![
                "rust".to_string(),
                "idioms".to_string(),
                "patterns".to_string(),
            ],
        }
    }

    /// Error handling standards
    pub fn error_handling() -> Self {
        Self {
            name: "error-handling".to_string(),
            description: "Error handling best practices".to_string(),
            rules: vec![
                (RuleCategory::ErrorHandling, 0, "Never silently swallow errors".to_string()),
                (
                    RuleCategory::ErrorHandling,
                    1,
                    "Provide context with error messages".to_string(),
                ),
                (
                    RuleCategory::ErrorHandling,
                    2,
                    "Use structured error types, not strings".to_string(),
                ),
                (
                    RuleCategory::ErrorHandling,
                    3,
                    "Handle errors at the appropriate level".to_string(),
                ),
                (
                    RuleCategory::ErrorHandling,
                    4,
                    "Log errors with sufficient context for debugging".to_string(),
                ),
                (
                    RuleCategory::ErrorHandling,
                    5,
                    "Consider user experience when displaying errors".to_string(),
                ),
            ],
            tags: vec![
                "errors".to_string(),
                "handling".to_string(),
                "exceptions".to_string(),
            ],
        }
    }

    /// Testing standards
    pub fn testing() -> Self {
        Self {
            name: "testing".to_string(),
            description: "Testing best practices".to_string(),
            rules: vec![
                (RuleCategory::Testing, 0, "Write tests for all public functions".to_string()),
                (
                    RuleCategory::Testing,
                    1,
                    "Use descriptive test names that explain the scenario".to_string(),
                ),
                (RuleCategory::Testing, 2, "Follow Arrange-Act-Assert pattern".to_string()),
                (RuleCategory::Testing, 3, "Test edge cases and error conditions".to_string()),
                (RuleCategory::Testing, 4, "Keep tests independent and isolated".to_string()),
                (
                    RuleCategory::Testing,
                    5,
                    "Prefer integration tests for complex workflows".to_string(),
                ),
                (RuleCategory::Testing, 6, "Mock external dependencies".to_string()),
            ],
            tags: vec![
                "testing".to_string(),
                "tests".to_string(),
                "quality".to_string(),
            ],
        }
    }

    /// Documentation standards
    pub fn documentation() -> Self {
        Self {
            name: "documentation".to_string(),
            description: "Documentation best practices".to_string(),
            rules: vec![
                (RuleCategory::Documentation, 0, "Document all public APIs".to_string()),
                (RuleCategory::Documentation, 1, "Include examples in documentation".to_string()),
                (
                    RuleCategory::Documentation,
                    2,
                    "Document error conditions and panics".to_string(),
                ),
                (
                    RuleCategory::Documentation,
                    3,
                    "Keep documentation up to date with code".to_string(),
                ),
                (RuleCategory::Documentation, 4, "Use proper markdown formatting".to_string()),
                (RuleCategory::Documentation, 5, "Link to related documentation".to_string()),
            ],
            tags: vec![
                "documentation".to_string(),
                "docs".to_string(),
                "comments".to_string(),
            ],
        }
    }

    /// Git conventions
    pub fn git_conventions() -> Self {
        Self {
            name: "git-conventions".to_string(),
            description: "Git commit and branching conventions".to_string(),
            rules: vec![
                (
                    RuleCategory::Git,
                    0,
                    "Use conventional commit messages (feat:, fix:, docs:, etc.)".to_string(),
                ),
                (RuleCategory::Git, 1, "Keep commits atomic and focused".to_string()),
                (RuleCategory::Git, 2, "Write meaningful commit messages".to_string()),
                (RuleCategory::Git, 3, "Reference issues in commits when applicable".to_string()),
                (RuleCategory::Git, 4, "Use feature branches for development".to_string()),
                (RuleCategory::Git, 5, "Rebase to keep history clean".to_string()),
            ],
            tags: vec![
                "git".to_string(),
                "commits".to_string(),
                "branching".to_string(),
            ],
        }
    }
}

impl Template for StandardsTemplate {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn category(&self) -> TemplateCategory {
        TemplateCategory::Standards
    }

    fn expand(&self) -> Result<Vec<UnifiedRule>> {
        Ok(self
            .rules
            .iter()
            .map(|(category, priority, description)| UnifiedRule::Standard {
                category: *category,
                priority: *priority,
                description: description.clone(),
                pattern: None,
            })
            .collect())
    }

    fn tags(&self) -> Vec<&str> {
        self.tags.iter().map(|s| s.as_str()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rust_idioms() {
        let template = StandardsTemplate::rust_idioms();
        assert_eq!(template.name(), "rust-idioms");
        assert_eq!(template.category(), TemplateCategory::Standards);

        let rules = template.expand().unwrap();
        assert!(!rules.is_empty());
    }

    #[test]
    fn test_all_standards() {
        let standards = vec![
            StandardsTemplate::rust_idioms(),
            StandardsTemplate::error_handling(),
            StandardsTemplate::testing(),
            StandardsTemplate::documentation(),
            StandardsTemplate::git_conventions(),
        ];

        for standard in standards {
            let rules = standard.expand().unwrap();
            assert!(!rules.is_empty(), "Standard {} should have rules", standard.name());
        }
    }
}
