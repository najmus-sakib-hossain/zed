//! Copilot copilot-instructions.md emitter

use super::{RuleEmitter, ensure_parent_dir, format_bullet_list, format_heading};
use crate::{Editor, Result, parser::UnifiedRule};
use std::path::Path;

/// Emitter for GitHub Copilot copilot-instructions.md format
#[derive(Debug, Default)]
pub struct CopilotEmitter;

impl CopilotEmitter {
    /// Create a new Copilot emitter
    pub fn new() -> Self {
        Self
    }
}

impl RuleEmitter for CopilotEmitter {
    fn emit_file(&self, rules: &[UnifiedRule], path: &Path) -> Result<()> {
        ensure_parent_dir(path)?;
        let content = self.emit_string(rules)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    fn emit_string(&self, rules: &[UnifiedRule]) -> Result<String> {
        let mut output = String::new();

        // Copilot format uses clear section headers
        output.push_str(&format_heading(1, "Copilot Instructions"));

        // Collect all context info
        let mut focus_areas: Vec<String> = Vec::new();
        let mut all_standards: Vec<(String, String)> = Vec::new(); // (category, description)
        let mut raw_content: Vec<String> = Vec::new();

        for rule in rules {
            match rule {
                UnifiedRule::Persona {
                    role,
                    traits,
                    principles,
                    ..
                } => {
                    output.push_str(&format_heading(2, "AI Role"));
                    output.push_str(role);
                    output.push_str("\n\n");

                    if !traits.is_empty() {
                        output.push_str(&format_bullet_list(traits));
                        output.push('\n');
                    }

                    if !principles.is_empty() {
                        output.push_str(&format_heading(3, "Core Principles"));
                        output.push_str(&format_bullet_list(principles));
                        output.push('\n');
                    }
                }
                UnifiedRule::Context {
                    focus, includes, ..
                } => {
                    focus_areas.extend(focus.clone());
                    focus_areas.extend(includes.clone());
                }
                UnifiedRule::Standard {
                    category,
                    description,
                    ..
                } => {
                    all_standards.push((format!("{:?}", category), description.clone()));
                }
                UnifiedRule::Workflow { name, steps } => {
                    output.push_str(&format_heading(2, &format!("Workflow: {}", name)));
                    for step in steps {
                        output.push_str(&format!("1. **{}**: {}\n", step.name, step.description));
                    }
                    output.push('\n');
                }
                UnifiedRule::Raw { content } => {
                    raw_content.push(content.clone());
                }
            }
        }

        // Emit focus areas
        if !focus_areas.is_empty() {
            output.push_str(&format_heading(2, "Project Context"));
            output.push_str(&format_bullet_list(&focus_areas));
            output.push('\n');
        }

        // Emit standards grouped by category
        if !all_standards.is_empty() {
            output.push_str(&format_heading(2, "Code Quality"));

            use std::collections::HashMap;
            let mut by_category: HashMap<String, Vec<String>> = HashMap::new();

            for (category, description) in all_standards {
                by_category.entry(category).or_default().push(description);
            }

            for (category, descriptions) in by_category {
                output.push_str(&format_heading(3, &category));
                output.push_str(&format_bullet_list(&descriptions));
                output.push('\n');
            }
        }

        // Emit raw content
        for content in raw_content {
            output.push_str(&content);
            output.push_str("\n\n");
        }

        Ok(output.trim().to_string())
    }

    fn editor(&self) -> Editor {
        Editor::Copilot
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_emit_basic() {
        let rules = vec![UnifiedRule::Context {
            includes: vec!["src/**".to_string()],
            excludes: vec!["target/**".to_string()],
            focus: vec!["Focus on performance".to_string()],
        }];

        let emitter = CopilotEmitter::new();
        let output = emitter.emit_string(&rules).unwrap();

        assert!(output.contains("Copilot Instructions"));
        assert!(output.contains("Project Context"));
        assert!(output.contains("Focus on performance"));
    }
}
