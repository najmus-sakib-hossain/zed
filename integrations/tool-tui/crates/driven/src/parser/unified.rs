//! Unified rule representation

use crate::format::RuleCategory;

/// Parsed rule from any source
#[derive(Debug, Clone)]
pub struct ParsedRule {
    /// Source file
    pub source: Option<std::path::PathBuf>,
    /// Line number in source
    pub line: Option<usize>,
    /// The unified rule
    pub rule: UnifiedRule,
}

/// Workflow step data
#[derive(Debug, Clone)]
pub struct WorkflowStepData {
    /// Step name
    pub name: String,
    /// Step description
    pub description: String,
    /// Condition for this step (optional)
    pub condition: Option<String>,
    /// Actions to perform
    pub actions: Vec<String>,
}

/// Unified representation of AI coding rules
#[derive(Debug, Clone)]
pub enum UnifiedRule {
    /// AI persona definition
    Persona {
        /// Persona name
        name: String,
        /// Role description
        role: String,
        /// Identity/expertise description
        identity: Option<String>,
        /// Communication style
        style: Option<String>,
        /// Personality traits
        traits: Vec<String>,
        /// Core principles
        principles: Vec<String>,
    },

    /// Coding standard rule
    Standard {
        /// Category of the rule
        category: RuleCategory,
        /// Priority (0 = highest)
        priority: u8,
        /// Description of the rule
        description: String,
        /// Example pattern (optional)
        pattern: Option<String>,
    },

    /// Project context
    Context {
        /// Include patterns
        includes: Vec<String>,
        /// Exclude patterns
        excludes: Vec<String>,
        /// Focus areas
        focus: Vec<String>,
    },

    /// Development workflow
    Workflow {
        /// Workflow name
        name: String,
        /// Workflow steps
        steps: Vec<WorkflowStepData>,
    },

    /// Raw content (unparsed)
    Raw {
        /// Raw content
        content: String,
    },
}

impl UnifiedRule {
    /// Create a persona rule
    pub fn persona(name: impl Into<String>, role: impl Into<String>) -> Self {
        Self::Persona {
            name: name.into(),
            role: role.into(),
            identity: None,
            style: None,
            traits: Vec::new(),
            principles: Vec::new(),
        }
    }

    /// Create a standard rule
    pub fn standard(category: RuleCategory, priority: u8, description: impl Into<String>) -> Self {
        Self::Standard {
            category,
            priority,
            description: description.into(),
            pattern: None,
        }
    }

    /// Create a context rule
    pub fn context(includes: Vec<String>, excludes: Vec<String>) -> Self {
        Self::Context {
            includes,
            excludes,
            focus: Vec::new(),
        }
    }

    /// Create a workflow rule
    pub fn workflow(name: impl Into<String>, steps: Vec<WorkflowStepData>) -> Self {
        Self::Workflow {
            name: name.into(),
            steps,
        }
    }

    /// Create a raw content rule
    pub fn raw(content: impl Into<String>) -> Self {
        Self::Raw {
            content: content.into(),
        }
    }

    /// Get the type name of this rule
    pub fn type_name(&self) -> &'static str {
        match self {
            UnifiedRule::Persona { .. } => "persona",
            UnifiedRule::Standard { .. } => "standard",
            UnifiedRule::Context { .. } => "context",
            UnifiedRule::Workflow { .. } => "workflow",
            UnifiedRule::Raw { .. } => "raw",
        }
    }
}

impl std::fmt::Display for UnifiedRule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UnifiedRule::Persona { name, role, .. } => {
                write!(f, "Persona[{}]: {}", name, role)
            }
            UnifiedRule::Standard {
                category,
                description,
                ..
            } => {
                write!(f, "Standard[{:?}]: {}", category, description)
            }
            UnifiedRule::Context { includes, .. } => {
                write!(f, "Context: {} patterns", includes.len())
            }
            UnifiedRule::Workflow { name, steps } => {
                write!(f, "Workflow[{}]: {} steps", name, steps.len())
            }
            UnifiedRule::Raw { content } => {
                let preview: String = content.chars().take(50).collect();
                write!(f, "Raw: {}...", preview)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_persona_creation() {
        let rule = UnifiedRule::persona("Architect", "Senior system architect");
        assert_eq!(rule.type_name(), "persona");

        if let UnifiedRule::Persona { name, role, .. } = rule {
            assert_eq!(name, "Architect");
            assert_eq!(role, "Senior system architect");
        } else {
            panic!("Expected Persona variant");
        }
    }

    #[test]
    fn test_standard_creation() {
        let rule = UnifiedRule::standard(RuleCategory::Naming, 1, "Use snake_case for functions");
        assert_eq!(rule.type_name(), "standard");
    }

    #[test]
    fn test_display() {
        let rule = UnifiedRule::persona("Test", "Tester");
        let display = format!("{}", rule);
        assert!(display.contains("Persona"));
        assert!(display.contains("Test"));
    }
}
