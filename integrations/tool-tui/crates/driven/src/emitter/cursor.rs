//! Cursor .cursorrules emitter

use super::{RuleEmitter, ensure_parent_dir, format_bullet_list, format_heading};
use crate::{Editor, Result, parser::UnifiedRule};
use std::path::Path;

/// Emitter for Cursor .cursorrules format
#[derive(Debug, Default)]
pub struct CursorEmitter;

impl CursorEmitter {
    /// Create a new Cursor emitter
    pub fn new() -> Self {
        Self
    }
}

impl RuleEmitter for CursorEmitter {
    fn emit_file(&self, rules: &[UnifiedRule], path: &Path) -> Result<()> {
        ensure_parent_dir(path)?;
        let content = self.emit_string(rules)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    fn emit_string(&self, rules: &[UnifiedRule]) -> Result<String> {
        let mut output = String::new();

        // Group rules by type
        let mut personas: Vec<&UnifiedRule> = Vec::new();
        let mut standards: Vec<&UnifiedRule> = Vec::new();
        let mut contexts: Vec<&UnifiedRule> = Vec::new();
        let mut workflows: Vec<&UnifiedRule> = Vec::new();
        let mut raw: Vec<&UnifiedRule> = Vec::new();

        for rule in rules {
            match rule {
                UnifiedRule::Persona { .. } => personas.push(rule),
                UnifiedRule::Standard { .. } => standards.push(rule),
                UnifiedRule::Context { .. } => contexts.push(rule),
                UnifiedRule::Workflow { .. } => workflows.push(rule),
                UnifiedRule::Raw { .. } => raw.push(rule),
            }
        }

        // Emit personas
        for rule in personas {
            if let UnifiedRule::Persona {
                name,
                role,
                identity,
                style,
                traits,
                principles,
            } = rule
            {
                output.push_str(&format_heading(1, &format!("AI Persona: {}", name)));
                output.push_str(role);
                output.push_str("\n\n");

                if let Some(id) = identity {
                    output.push_str(&format!("**Identity:** {}\n\n", id));
                }

                if let Some(s) = style {
                    output.push_str(&format!("**Communication Style:** {}\n\n", s));
                }

                if !traits.is_empty() {
                    output.push_str(&format_heading(2, "Traits"));
                    output.push_str(&format_bullet_list(traits));
                    output.push('\n');
                }

                if !principles.is_empty() {
                    output.push_str(&format_heading(2, "Principles"));
                    output.push_str(&format_bullet_list(principles));
                    output.push('\n');
                }
            }
        }

        // Emit context
        if !contexts.is_empty() {
            output.push_str(&format_heading(1, "Project Context"));
            for rule in contexts {
                if let UnifiedRule::Context {
                    includes,
                    excludes,
                    focus,
                } = rule
                {
                    if !includes.is_empty() {
                        output.push_str(&format_heading(2, "Include"));
                        output.push_str(&format_bullet_list(includes));
                        output.push('\n');
                    }

                    if !excludes.is_empty() {
                        output.push_str(&format_heading(2, "Exclude"));
                        output.push_str(&format_bullet_list(excludes));
                        output.push('\n');
                    }

                    if !focus.is_empty() {
                        output.push_str(&format_heading(2, "Focus Areas"));
                        output.push_str(&format_bullet_list(focus));
                        output.push('\n');
                    }
                }
            }
        }

        // Emit standards
        if !standards.is_empty() {
            output.push_str(&format_heading(1, "Coding Standards"));

            // Group by category
            use std::collections::HashMap;
            let mut by_category: HashMap<String, Vec<&str>> = HashMap::new();

            for rule in &standards {
                if let UnifiedRule::Standard {
                    category,
                    description,
                    ..
                } = rule
                {
                    let cat_name = format!("{:?}", category);
                    by_category.entry(cat_name).or_default().push(description);
                }
            }

            for (category, descriptions) in by_category {
                output.push_str(&format_heading(2, &category));
                let items: Vec<String> = descriptions.iter().map(|s| s.to_string()).collect();
                output.push_str(&format_bullet_list(&items));
                output.push('\n');
            }
        }

        // Emit workflows
        for rule in workflows {
            if let UnifiedRule::Workflow { name, steps } = rule {
                output.push_str(&format_heading(1, &format!("Workflow: {}", name)));

                for (i, step) in steps.iter().enumerate() {
                    output.push_str(&format!("### Step {}: {}\n\n", i + 1, step.name));
                    output.push_str(&step.description);
                    output.push_str("\n\n");

                    if let Some(condition) = &step.condition {
                        output.push_str(&format!("**Condition:** {}\n\n", condition));
                    }

                    if !step.actions.is_empty() {
                        output.push_str("**Actions:**\n");
                        output.push_str(&format_bullet_list(&step.actions));
                        output.push('\n');
                    }
                }
            }
        }

        // Emit raw content
        for rule in raw {
            if let UnifiedRule::Raw { content } = rule {
                output.push_str(content);
                output.push_str("\n\n");
            }
        }

        Ok(output.trim().to_string())
    }

    fn editor(&self) -> Editor {
        Editor::Cursor
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::format::RuleCategory;

    #[test]
    fn test_emit_standards() {
        let rules = vec![
            UnifiedRule::Standard {
                category: RuleCategory::Style,
                priority: 0,
                description: "Use snake_case for functions".to_string(),
                pattern: None,
            },
            UnifiedRule::Standard {
                category: RuleCategory::Style,
                priority: 1,
                description: "Use PascalCase for types".to_string(),
                pattern: None,
            },
        ];

        let emitter = CursorEmitter::new();
        let output = emitter.emit_string(&rules).unwrap();

        assert!(output.contains("Coding Standards"));
        assert!(output.contains("snake_case"));
        assert!(output.contains("PascalCase"));
    }

    #[test]
    fn test_emit_persona() {
        let rules = vec![UnifiedRule::Persona {
            name: "Architect".to_string(),
            role: "Senior system architect".to_string(),
            identity: Some("Expert in distributed systems".to_string()),
            style: Some("Direct and technical".to_string()),
            traits: vec!["Analytical".to_string(), "Detail-oriented".to_string()],
            principles: vec!["Simplicity first".to_string()],
        }];

        let emitter = CursorEmitter::new();
        let output = emitter.emit_string(&rules).unwrap();

        assert!(output.contains("AI Persona: Architect"));
        assert!(output.contains("Senior system architect"));
        assert!(output.contains("Analytical"));
    }
}
