//! Windsurf .windsurfrules emitter

use super::{RuleEmitter, ensure_parent_dir, format_bullet_list, format_heading};
use crate::{Editor, Result, parser::UnifiedRule};
use std::path::Path;

/// Emitter for Windsurf .windsurfrules format
#[derive(Debug, Default)]
pub struct WindsurfEmitter;

impl WindsurfEmitter {
    /// Create a new Windsurf emitter
    pub fn new() -> Self {
        Self
    }
}

impl RuleEmitter for WindsurfEmitter {
    fn emit_file(&self, rules: &[UnifiedRule], path: &Path) -> Result<()> {
        ensure_parent_dir(path)?;
        let content = self.emit_string(rules)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    fn emit_string(&self, rules: &[UnifiedRule]) -> Result<String> {
        let mut output = String::new();

        // Windsurf format is similar to Cursor
        output.push_str(&format_heading(1, "Windsurf Rules"));

        let mut standards: Vec<String> = Vec::new();
        let mut context: Vec<String> = Vec::new();

        for rule in rules {
            match rule {
                UnifiedRule::Persona {
                    name, role, traits, ..
                } => {
                    output.push_str(&format_heading(2, &format!("Role: {}", name)));
                    output.push_str(role);
                    output.push_str("\n\n");

                    if !traits.is_empty() {
                        output.push_str(&format_bullet_list(traits));
                        output.push('\n');
                    }
                }
                UnifiedRule::Standard { description, .. } => {
                    standards.push(description.clone());
                }
                UnifiedRule::Context {
                    focus, includes, ..
                } => {
                    context.extend(focus.clone());
                    context.extend(includes.clone());
                }
                UnifiedRule::Workflow { name, steps } => {
                    output.push_str(&format_heading(2, name));
                    for step in steps {
                        output.push_str(&format!("- {}: {}\n", step.name, step.description));
                    }
                    output.push('\n');
                }
                UnifiedRule::Raw { content } => {
                    output.push_str(content);
                    output.push_str("\n\n");
                }
            }
        }

        if !context.is_empty() {
            output.push_str(&format_heading(2, "Project Context"));
            output.push_str(&format_bullet_list(&context));
            output.push('\n');
        }

        if !standards.is_empty() {
            output.push_str(&format_heading(2, "Coding Guidelines"));
            output.push_str(&format_bullet_list(&standards));
            output.push('\n');
        }

        Ok(output.trim().to_string())
    }

    fn editor(&self) -> Editor {
        Editor::Windsurf
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_emit_windsurf() {
        let rules = vec![UnifiedRule::Standard {
            category: crate::format::RuleCategory::Style,
            priority: 0,
            description: "Use TypeScript strict mode".to_string(),
            pattern: None,
        }];

        let emitter = WindsurfEmitter::new();
        let output = emitter.emit_string(&rules).unwrap();

        assert!(output.contains("Windsurf Rules"));
        assert!(output.contains("TypeScript strict mode"));
    }
}
