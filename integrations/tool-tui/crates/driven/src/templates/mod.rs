//! Template System
//!
//! Built-in library of professional AI agent templates.

mod composer;
mod persona;
mod project;
mod registry;
mod standards;
mod task;
mod workflow;

pub use composer::TemplateComposer;
pub use persona::PersonaTemplate;
pub use project::ProjectTemplate;
pub use registry::TemplateRegistry;
pub use standards::StandardsTemplate;
pub use task::TaskTemplate;
pub use workflow::WorkflowTemplate;

use crate::{Result, parser::UnifiedRule};

/// A loadable template
pub trait Template: Send + Sync {
    /// Get the template name
    fn name(&self) -> &str;

    /// Get a description of this template
    fn description(&self) -> &str;

    /// Get the category of this template
    fn category(&self) -> TemplateCategory;

    /// Expand this template to rules
    fn expand(&self) -> Result<Vec<UnifiedRule>>;

    /// Get template tags for discovery
    fn tags(&self) -> Vec<&str> {
        Vec::new()
    }

    /// Render this template to a string
    fn render(&self, _variables: &std::collections::HashMap<String, String>) -> Result<String> {
        // Default implementation: expand to rules and format as markdown
        let rules = self.expand()?;
        let mut output = format!("# {} Template\n\n", self.name());
        output.push_str(&format!("{}\n\n", self.description()));

        for rule in rules {
            match rule {
                UnifiedRule::Persona {
                    name,
                    role,
                    identity,
                    style,
                    traits,
                    principles,
                } => {
                    output.push_str("## Persona\n\n");
                    output.push_str(&format!("**{}** - {}\n\n", name, role));
                    if let Some(id) = identity {
                        output.push_str(&format!("{}\n\n", id));
                    }
                    if let Some(s) = style {
                        output.push_str(&format!("Style: {}\n\n", s));
                    }
                    if !traits.is_empty() {
                        output.push_str("### Traits\n");
                        for t in traits {
                            output.push_str(&format!("- {}\n", t));
                        }
                        output.push('\n');
                    }
                    if !principles.is_empty() {
                        output.push_str("### Principles\n");
                        for p in principles {
                            output.push_str(&format!("- {}\n", p));
                        }
                        output.push('\n');
                    }
                }
                UnifiedRule::Standard {
                    category,
                    description,
                    ..
                } => {
                    output.push_str(&format!("### {:?}\n", category));
                    output.push_str(&format!("- {}\n\n", description));
                }
                UnifiedRule::Context {
                    includes,
                    excludes,
                    focus,
                } => {
                    output.push_str("## Context\n\n");
                    if !includes.is_empty() {
                        output.push_str("### Include\n");
                        for inc in includes {
                            output.push_str(&format!("- {}\n", inc));
                        }
                        output.push('\n');
                    }
                    if !excludes.is_empty() {
                        output.push_str("### Exclude\n");
                        for exc in excludes {
                            output.push_str(&format!("- {}\n", exc));
                        }
                        output.push('\n');
                    }
                    if !focus.is_empty() {
                        output.push_str("### Focus\n");
                        for f in focus {
                            output.push_str(&format!("- {}\n", f));
                        }
                        output.push('\n');
                    }
                }
                UnifiedRule::Workflow { name, steps } => {
                    output.push_str(&format!("## Workflow: {}\n\n", name));
                    for step in steps {
                        output.push_str(&format!("### Step: {}\n", step.name));
                        output.push_str(&format!("{}\n\n", step.description));
                    }
                }
                UnifiedRule::Raw { content } => {
                    output.push_str(&content);
                    output.push('\n');
                }
            }
        }

        Ok(output)
    }
}

/// Template categories
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TemplateCategory {
    /// AI persona definitions
    Persona,
    /// Project structure templates
    Project,
    /// Coding standards
    Standards,
    /// Development workflows
    Workflow,
    /// Task-specific guidance
    Task,
}

impl std::fmt::Display for TemplateCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TemplateCategory::Persona => write!(f, "Persona"),
            TemplateCategory::Project => write!(f, "Project"),
            TemplateCategory::Standards => write!(f, "Standards"),
            TemplateCategory::Workflow => write!(f, "Workflow"),
            TemplateCategory::Task => write!(f, "Task"),
        }
    }
}

/// Built-in template definitions
pub mod builtin {
    use super::*;

    /// Get all built-in persona templates
    pub fn personas() -> Vec<Box<dyn Template>> {
        vec![
            Box::new(PersonaTemplate::architect()),
            Box::new(PersonaTemplate::reviewer()),
            Box::new(PersonaTemplate::documenter()),
            Box::new(PersonaTemplate::security()),
            Box::new(PersonaTemplate::performance()),
            Box::new(PersonaTemplate::teacher()),
        ]
    }

    /// Get all built-in project templates
    pub fn projects() -> Vec<Box<dyn Template>> {
        vec![
            Box::new(ProjectTemplate::rust_workspace()),
            Box::new(ProjectTemplate::typescript_monorepo()),
            Box::new(ProjectTemplate::fullstack()),
            Box::new(ProjectTemplate::cli_tool()),
            Box::new(ProjectTemplate::library()),
        ]
    }

    /// Get all built-in standards templates
    pub fn standards() -> Vec<Box<dyn Template>> {
        vec![
            Box::new(StandardsTemplate::rust_idioms()),
            Box::new(StandardsTemplate::error_handling()),
            Box::new(StandardsTemplate::testing()),
            Box::new(StandardsTemplate::documentation()),
            Box::new(StandardsTemplate::git_conventions()),
        ]
    }

    /// Get all built-in workflow templates
    pub fn workflows() -> Vec<Box<dyn Template>> {
        vec![
            Box::new(WorkflowTemplate::tdd()),
            Box::new(WorkflowTemplate::feature_development()),
            Box::new(WorkflowTemplate::bug_fixing()),
            Box::new(WorkflowTemplate::refactoring()),
            Box::new(WorkflowTemplate::code_review()),
        ]
    }

    /// Get all built-in task templates
    pub fn tasks() -> Vec<Box<dyn Template>> {
        vec![
            Box::new(TaskTemplate::implement_feature()),
            Box::new(TaskTemplate::write_tests()),
            Box::new(TaskTemplate::fix_bug()),
            Box::new(TaskTemplate::optimize()),
            Box::new(TaskTemplate::document()),
        ]
    }

    /// Get all built-in templates
    pub fn all() -> Vec<Box<dyn Template>> {
        let mut all = Vec::new();
        all.extend(personas());
        all.extend(projects());
        all.extend(standards());
        all.extend(workflows());
        all.extend(tasks());
        all
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_category_display() {
        assert_eq!(format!("{}", TemplateCategory::Persona), "Persona");
        assert_eq!(format!("{}", TemplateCategory::Workflow), "Workflow");
    }

    #[test]
    fn test_builtin_templates() {
        let all = builtin::all();
        assert!(!all.is_empty());

        // Check we have templates in each category
        let personas = builtin::personas();
        assert!(!personas.is_empty());
    }
}
