//! Context provider for AI agents

use super::{ProjectContext, ProjectScanner};
use crate::{Result, parser::UnifiedRule};
use std::path::Path;

/// Provides context to AI agents
#[derive(Debug, Default)]
pub struct ContextProvider {
    context: Option<ProjectContext>,
}

impl ContextProvider {
    /// Create a new context provider
    pub fn new() -> Self {
        Self { context: None }
    }

    /// Create with pre-loaded context
    pub fn with_context(context: ProjectContext) -> Self {
        Self {
            context: Some(context),
        }
    }

    /// Set the project context
    pub fn set_context(&mut self, context: ProjectContext) {
        self.context = Some(context);
    }

    /// Get the current context
    pub fn get_context(&self) -> Option<&ProjectContext> {
        self.context.as_ref()
    }

    /// Generate context from a project path
    pub fn generate(&self, project_root: &Path) -> Result<String> {
        let scanner = ProjectScanner::new();
        let result = scanner.scan(project_root)?;

        let mut output = String::new();
        output.push_str("# Project Context\n\n");

        output.push_str("## Languages\n");
        for lang in &result.languages {
            output.push_str(&format!("- {}\n", lang));
        }

        output.push_str("\n## Frameworks\n");
        for framework in &result.frameworks {
            output.push_str(&format!("- {}\n", framework));
        }

        if !result.key_directories.is_empty() {
            output.push_str("\n## Key Directories\n");
            for dir in &result.key_directories {
                output.push_str(&format!("- {}\n", dir));
            }
        }

        Ok(output)
    }

    /// Generate context rules for AI agents
    pub fn generate_rules(&self) -> Result<Vec<UnifiedRule>> {
        let Some(context) = &self.context else {
            return Ok(Vec::new());
        };

        let mut rules = Vec::new();

        // Add project type context
        let mut focus = Vec::new();

        if let Some(project_type) = &context.project_type {
            focus.push(format!("This is a {} project", project_type));
        }

        if !context.languages.is_empty() {
            focus.push(format!("Languages: {}", context.languages.join(", ")));
        }

        if !context.frameworks.is_empty() {
            focus.push(format!("Frameworks: {}", context.frameworks.join(", ")));
        }

        // Add naming conventions
        if let Some(style) = &context.naming_conventions.functions {
            focus.push(format!("Use {} for functions", style));
        }

        if let Some(style) = &context.naming_conventions.types {
            focus.push(format!("Use {} for types", style));
        }

        // Add patterns
        focus.extend(context.patterns.clone());

        if !focus.is_empty() {
            rules.push(UnifiedRule::Context {
                includes: context.directories.clone(),
                excludes: Vec::new(),
                focus,
            });
        }

        Ok(rules)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_new() {
        let provider = ContextProvider::new();
        assert!(provider.get_context().is_none());
    }

    #[test]
    fn test_provider_with_context() {
        let context = ProjectContext {
            project_type: Some("Rust Project".to_string()),
            ..Default::default()
        };

        let provider = ContextProvider::with_context(context);
        assert!(provider.get_context().is_some());
    }

    #[test]
    fn test_generate_rules_empty() {
        let provider = ContextProvider::new();
        let rules = provider.generate_rules().unwrap();
        assert!(rules.is_empty());
    }

    #[test]
    fn test_generate_rules_with_context() {
        let context = ProjectContext {
            project_type: Some("Rust Project".to_string()),
            languages: vec!["Rust".to_string()],
            ..Default::default()
        };

        let provider = ContextProvider::with_context(context);
        let rules = provider.generate_rules().unwrap();
        assert!(!rules.is_empty());
    }
}
