//! Development workflow templates

use super::{Template, TemplateCategory};
use crate::{
    Result,
    parser::{UnifiedRule, WorkflowStepData},
};

/// Workflow template definition
#[derive(Debug, Clone)]
pub struct WorkflowTemplate {
    name: String,
    description: String,
    steps: Vec<WorkflowStepData>,
    tags: Vec<String>,
}

impl WorkflowTemplate {
    /// Test-Driven Development workflow
    pub fn tdd() -> Self {
        Self {
            name: "tdd".to_string(),
            description: "Test-Driven Development workflow".to_string(),
            steps: vec![
                WorkflowStepData {
                    name: "Write Failing Test".to_string(),
                    description: "Write a test that describes the expected behavior. The test should fail initially.".to_string(),
                    condition: None,
                    actions: vec![
                        "Identify the behavior to implement".to_string(),
                        "Write a test case for that behavior".to_string(),
                        "Run tests to confirm failure".to_string(),
                    ],
                },
                WorkflowStepData {
                    name: "Write Minimal Code".to_string(),
                    description: "Write just enough code to make the test pass.".to_string(),
                    condition: Some("Test is failing".to_string()),
                    actions: vec![
                        "Implement the minimum code needed".to_string(),
                        "Run tests to confirm pass".to_string(),
                    ],
                },
                WorkflowStepData {
                    name: "Refactor".to_string(),
                    description: "Improve the code while keeping tests green.".to_string(),
                    condition: Some("Tests are passing".to_string()),
                    actions: vec![
                        "Look for code smells".to_string(),
                        "Apply refactoring patterns".to_string(),
                        "Run tests after each change".to_string(),
                    ],
                },
            ],
            tags: vec!["tdd".to_string(), "testing".to_string(), "development".to_string()],
        }
    }

    /// Feature development workflow
    pub fn feature_development() -> Self {
        Self {
            name: "feature-development".to_string(),
            description: "Standard feature development workflow".to_string(),
            steps: vec![
                WorkflowStepData {
                    name: "Understand Requirements".to_string(),
                    description: "Clarify what needs to be built and why.".to_string(),
                    condition: None,
                    actions: vec![
                        "Read the feature specification".to_string(),
                        "Ask clarifying questions".to_string(),
                        "Identify acceptance criteria".to_string(),
                    ],
                },
                WorkflowStepData {
                    name: "Design Solution".to_string(),
                    description: "Plan the technical approach.".to_string(),
                    condition: Some("Requirements are clear".to_string()),
                    actions: vec![
                        "Identify affected components".to_string(),
                        "Consider edge cases".to_string(),
                        "Document design decisions".to_string(),
                    ],
                },
                WorkflowStepData {
                    name: "Implement".to_string(),
                    description: "Write the code.".to_string(),
                    condition: Some("Design is approved".to_string()),
                    actions: vec![
                        "Create feature branch".to_string(),
                        "Write code following standards".to_string(),
                        "Write tests".to_string(),
                        "Update documentation".to_string(),
                    ],
                },
                WorkflowStepData {
                    name: "Review & Merge".to_string(),
                    description: "Get code reviewed and merged.".to_string(),
                    condition: Some("Implementation is complete".to_string()),
                    actions: vec![
                        "Create pull request".to_string(),
                        "Address review feedback".to_string(),
                        "Merge to main branch".to_string(),
                    ],
                },
            ],
            tags: vec![
                "feature".to_string(),
                "development".to_string(),
                "process".to_string(),
            ],
        }
    }

    /// Bug fixing workflow
    pub fn bug_fixing() -> Self {
        Self {
            name: "bug-fixing".to_string(),
            description: "Systematic bug fixing workflow".to_string(),
            steps: vec![
                WorkflowStepData {
                    name: "Reproduce".to_string(),
                    description: "Confirm and reproduce the bug.".to_string(),
                    condition: None,
                    actions: vec![
                        "Read the bug report".to_string(),
                        "Reproduce the issue".to_string(),
                        "Identify exact steps to reproduce".to_string(),
                    ],
                },
                WorkflowStepData {
                    name: "Diagnose".to_string(),
                    description: "Find the root cause.".to_string(),
                    condition: Some("Bug is reproducible".to_string()),
                    actions: vec![
                        "Add logging if needed".to_string(),
                        "Use debugger".to_string(),
                        "Trace through the code".to_string(),
                        "Identify root cause".to_string(),
                    ],
                },
                WorkflowStepData {
                    name: "Fix".to_string(),
                    description: "Implement the fix.".to_string(),
                    condition: Some("Root cause is identified".to_string()),
                    actions: vec![
                        "Write a test that fails".to_string(),
                        "Implement the fix".to_string(),
                        "Verify test passes".to_string(),
                        "Check for similar issues".to_string(),
                    ],
                },
                WorkflowStepData {
                    name: "Verify".to_string(),
                    description: "Confirm the fix works.".to_string(),
                    condition: Some("Fix is implemented".to_string()),
                    actions: vec![
                        "Run all tests".to_string(),
                        "Manually verify fix".to_string(),
                        "Check for regressions".to_string(),
                    ],
                },
            ],
            tags: vec![
                "bug".to_string(),
                "fix".to_string(),
                "debugging".to_string(),
            ],
        }
    }

    /// Refactoring workflow
    pub fn refactoring() -> Self {
        Self {
            name: "refactoring".to_string(),
            description: "Safe refactoring workflow".to_string(),
            steps: vec![
                WorkflowStepData {
                    name: "Ensure Tests".to_string(),
                    description: "Make sure adequate test coverage exists.".to_string(),
                    condition: None,
                    actions: vec![
                        "Check existing test coverage".to_string(),
                        "Add tests for uncovered code".to_string(),
                        "Run tests to establish baseline".to_string(),
                    ],
                },
                WorkflowStepData {
                    name: "Small Steps".to_string(),
                    description: "Make small, incremental changes.".to_string(),
                    condition: Some("Tests are green".to_string()),
                    actions: vec![
                        "Make one small change".to_string(),
                        "Run tests".to_string(),
                        "Commit if green".to_string(),
                        "Repeat".to_string(),
                    ],
                },
                WorkflowStepData {
                    name: "Review".to_string(),
                    description: "Review the refactored code.".to_string(),
                    condition: Some("Refactoring is complete".to_string()),
                    actions: vec![
                        "Compare before/after".to_string(),
                        "Check for improvements".to_string(),
                        "Verify behavior is unchanged".to_string(),
                    ],
                },
            ],
            tags: vec![
                "refactoring".to_string(),
                "cleanup".to_string(),
                "improvement".to_string(),
            ],
        }
    }

    /// Code review workflow
    pub fn code_review() -> Self {
        Self {
            name: "code-review".to_string(),
            description: "Thorough code review workflow".to_string(),
            steps: vec![
                WorkflowStepData {
                    name: "Understand Context".to_string(),
                    description: "Read the PR description and linked issues.".to_string(),
                    condition: None,
                    actions: vec![
                        "Read PR description".to_string(),
                        "Check linked issues".to_string(),
                        "Understand the goal".to_string(),
                    ],
                },
                WorkflowStepData {
                    name: "Review Code".to_string(),
                    description: "Examine the code changes.".to_string(),
                    condition: Some("Context is understood".to_string()),
                    actions: vec![
                        "Check for correctness".to_string(),
                        "Check for security issues".to_string(),
                        "Check for performance issues".to_string(),
                        "Check for readability".to_string(),
                        "Check test coverage".to_string(),
                    ],
                },
                WorkflowStepData {
                    name: "Provide Feedback".to_string(),
                    description: "Give constructive feedback.".to_string(),
                    condition: Some("Review is complete".to_string()),
                    actions: vec![
                        "Praise good patterns".to_string(),
                        "Suggest improvements".to_string(),
                        "Distinguish critical from minor".to_string(),
                        "Approve or request changes".to_string(),
                    ],
                },
            ],
            tags: vec![
                "review".to_string(),
                "code-review".to_string(),
                "pr".to_string(),
            ],
        }
    }
}

impl Template for WorkflowTemplate {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn category(&self) -> TemplateCategory {
        TemplateCategory::Workflow
    }

    fn expand(&self) -> Result<Vec<UnifiedRule>> {
        Ok(vec![UnifiedRule::Workflow {
            name: self.name.clone(),
            steps: self.steps.clone(),
        }])
    }

    fn tags(&self) -> Vec<&str> {
        self.tags.iter().map(|s| s.as_str()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tdd_workflow() {
        let template = WorkflowTemplate::tdd();
        assert_eq!(template.name(), "tdd");
        assert_eq!(template.category(), TemplateCategory::Workflow);

        let rules = template.expand().unwrap();
        assert_eq!(rules.len(), 1);

        if let UnifiedRule::Workflow { steps, .. } = &rules[0] {
            assert_eq!(steps.len(), 3);
        } else {
            panic!("Expected Workflow rule");
        }
    }
}
