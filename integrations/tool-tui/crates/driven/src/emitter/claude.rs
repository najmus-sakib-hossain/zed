//! Claude Code emitter (.claude/, CLAUDE.md)

use super::{RuleEmitter, ensure_parent_dir, format_bullet_list, format_heading};
use crate::{Editor, Result, parser::UnifiedRule};
use std::path::Path;

/// Emitter for Claude Code format
#[derive(Debug, Default)]
pub struct ClaudeEmitter;

impl ClaudeEmitter {
    /// Create a new Claude emitter
    pub fn new() -> Self {
        Self
    }
}

impl RuleEmitter for ClaudeEmitter {
    fn emit_file(&self, rules: &[UnifiedRule], path: &Path) -> Result<()> {
        ensure_parent_dir(path)?;
        let content = self.emit_string(rules)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    fn emit_string(&self, rules: &[UnifiedRule]) -> Result<String> {
        let mut output = String::new();

        output.push_str(&format_heading(1, "CLAUDE.md"));
        output.push_str("Instructions for Claude Code assistant.\n\n");

        let mut instructions: Vec<String> = Vec::new();
        let mut context: Vec<String> = Vec::new();

        for rule in rules {
            match rule {
                UnifiedRule::Persona {
                    role, principles, ..
                } => {
                    output.push_str(&format_heading(2, "Role"));
                    output.push_str(role);
                    output.push_str("\n\n");

                    if !principles.is_empty() {
                        instructions.extend(principles.clone());
                    }
                }
                UnifiedRule::Standard { description, .. } => {
                    instructions.push(description.clone());
                }
                UnifiedRule::Context {
                    focus, includes, ..
                } => {
                    context.extend(focus.clone());
                    context.extend(includes.clone());
                }
                UnifiedRule::Workflow { name, steps } => {
                    output.push_str(&format_heading(2, &format!("Workflow: {}", name)));
                    for (i, step) in steps.iter().enumerate() {
                        output.push_str(&format!(
                            "{}. **{}**: {}\n",
                            i + 1,
                            step.name,
                            step.description
                        ));
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
            output.push_str(&format_heading(2, "About This Project"));
            output.push_str(&format_bullet_list(&context));
            output.push('\n');
        }

        if !instructions.is_empty() {
            output.push_str(&format_heading(2, "Instructions"));
            output.push_str(&format_bullet_list(&instructions));
            output.push('\n');
        }

        Ok(output.trim().to_string())
    }

    fn editor(&self) -> Editor {
        Editor::Claude
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_emit_claude() {
        let rules = vec![UnifiedRule::Standard {
            category: crate::format::RuleCategory::Style,
            priority: 0,
            description: "Always explain your reasoning".to_string(),
            pattern: None,
        }];

        let emitter = ClaudeEmitter::new();
        let output = emitter.emit_string(&rules).unwrap();

        assert!(output.contains("CLAUDE.md"));
        assert!(output.contains("explain your reasoning"));
    }
}
