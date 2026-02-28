//! Task-specific templates

use super::{Template, TemplateCategory};
use crate::{Result, format::RuleCategory, parser::UnifiedRule};

/// Task template definition
#[derive(Debug, Clone)]
pub struct TaskTemplate {
    name: String,
    description: String,
    standards: Vec<(RuleCategory, String)>,
    tags: Vec<String>,
}

impl TaskTemplate {
    /// Implement feature task
    pub fn implement_feature() -> Self {
        Self {
            name: "implement-feature".to_string(),
            description: "Guidance for implementing a new feature".to_string(),
            standards: vec![
                (
                    RuleCategory::Architecture,
                    "Consider how this feature fits into the overall architecture".to_string(),
                ),
                (RuleCategory::Style, "Follow existing code patterns in the codebase".to_string()),
                (RuleCategory::Testing, "Write tests alongside the implementation".to_string()),
                (RuleCategory::Documentation, "Update relevant documentation".to_string()),
                (RuleCategory::Other, "Consider backward compatibility".to_string()),
            ],
            tags: vec![
                "feature".to_string(),
                "implement".to_string(),
                "development".to_string(),
            ],
        }
    }

    /// Write tests task
    pub fn write_tests() -> Self {
        Self {
            name: "write-tests".to_string(),
            description: "Guidance for writing comprehensive tests".to_string(),
            standards: vec![
                (RuleCategory::Testing, "Test both happy path and error cases".to_string()),
                (RuleCategory::Testing, "Use descriptive test names".to_string()),
                (RuleCategory::Testing, "Keep tests isolated and independent".to_string()),
                (RuleCategory::Testing, "Mock external dependencies".to_string()),
                (RuleCategory::Testing, "Test edge cases and boundary conditions".to_string()),
                (
                    RuleCategory::Testing,
                    "Aim for meaningful coverage, not just high numbers".to_string(),
                ),
            ],
            tags: vec![
                "testing".to_string(),
                "tests".to_string(),
                "quality".to_string(),
            ],
        }
    }

    /// Fix bug task
    pub fn fix_bug() -> Self {
        Self {
            name: "fix-bug".to_string(),
            description: "Guidance for fixing bugs systematically".to_string(),
            standards: vec![
                (RuleCategory::Other, "First reproduce the bug".to_string()),
                (RuleCategory::Other, "Understand the root cause before fixing".to_string()),
                (
                    RuleCategory::Testing,
                    "Write a failing test that reproduces the bug".to_string(),
                ),
                (RuleCategory::Other, "Check for similar issues elsewhere".to_string()),
                (RuleCategory::Other, "Consider if this indicates a design issue".to_string()),
            ],
            tags: vec![
                "bug".to_string(),
                "fix".to_string(),
                "debugging".to_string(),
            ],
        }
    }

    /// Optimize task
    pub fn optimize() -> Self {
        Self {
            name: "optimize".to_string(),
            description: "Guidance for performance optimization".to_string(),
            standards: vec![
                (RuleCategory::Performance, "Measure before optimizing".to_string()),
                (RuleCategory::Performance, "Identify the bottleneck first".to_string()),
                (
                    RuleCategory::Performance,
                    "Consider algorithmic improvements before micro-optimizations".to_string(),
                ),
                (
                    RuleCategory::Performance,
                    "Document the optimization and its impact".to_string(),
                ),
                (RuleCategory::Testing, "Ensure functionality is preserved".to_string()),
                (RuleCategory::Performance, "Measure after to confirm improvement".to_string()),
            ],
            tags: vec![
                "performance".to_string(),
                "optimization".to_string(),
                "speed".to_string(),
            ],
        }
    }

    /// Document task
    pub fn document() -> Self {
        Self {
            name: "document".to_string(),
            description: "Guidance for writing documentation".to_string(),
            standards: vec![
                (RuleCategory::Documentation, "Write for the reader, not the writer".to_string()),
                (RuleCategory::Documentation, "Include practical examples".to_string()),
                (RuleCategory::Documentation, "Keep it concise but complete".to_string()),
                (RuleCategory::Documentation, "Use proper formatting and structure".to_string()),
                (
                    RuleCategory::Documentation,
                    "Explain the 'why', not just the 'what'".to_string(),
                ),
                (
                    RuleCategory::Documentation,
                    "Consider different audience skill levels".to_string(),
                ),
            ],
            tags: vec![
                "documentation".to_string(),
                "docs".to_string(),
                "writing".to_string(),
            ],
        }
    }
}

impl Template for TaskTemplate {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn category(&self) -> TemplateCategory {
        TemplateCategory::Task
    }

    fn expand(&self) -> Result<Vec<UnifiedRule>> {
        Ok(self
            .standards
            .iter()
            .enumerate()
            .map(|(i, (category, description))| UnifiedRule::Standard {
                category: *category,
                priority: i as u8,
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
    fn test_implement_feature() {
        let template = TaskTemplate::implement_feature();
        assert_eq!(template.name(), "implement-feature");
        assert_eq!(template.category(), TemplateCategory::Task);

        let rules = template.expand().unwrap();
        assert!(!rules.is_empty());
    }
}
