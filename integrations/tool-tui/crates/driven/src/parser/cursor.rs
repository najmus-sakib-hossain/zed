//! Cursor rules parser (.cursorrules)

use super::{RuleParser, UnifiedRule, extract_bullet_points, parse_markdown_sections};
use crate::{DrivenError, Editor, Result};
use std::path::Path;

/// Parser for Cursor .cursorrules files
#[derive(Debug, Default)]
pub struct CursorParser;

impl CursorParser {
    /// Create a new Cursor parser
    pub fn new() -> Self {
        Self
    }

    /// Detect rule type from section heading
    fn detect_section_type(heading: &str) -> SectionKind {
        let lower = heading.to_lowercase();

        if lower.contains("persona")
            || lower.contains("role")
            || lower.contains("who you are")
            || lower.contains("identity")
        {
            SectionKind::Persona
        } else if lower.contains("context")
            || lower.contains("project")
            || lower.contains("codebase")
        {
            SectionKind::Context
        } else if lower.contains("workflow") || lower.contains("process") || lower.contains("steps")
        {
            SectionKind::Workflow
        } else if lower.contains("style")
            || lower.contains("convention")
            || lower.contains("standard")
            || lower.contains("rule")
            || lower.contains("guideline")
        {
            SectionKind::Standards
        } else {
            SectionKind::Other
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SectionKind {
    Persona,
    Context,
    Workflow,
    Standards,
    Other,
}

impl RuleParser for CursorParser {
    fn parse_file(&self, path: &Path) -> Result<Vec<UnifiedRule>> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| DrivenError::Parse(format!("Failed to read {}: {}", path.display(), e)))?;
        self.parse_content(&content)
    }

    fn parse_content(&self, content: &str) -> Result<Vec<UnifiedRule>> {
        let mut rules = Vec::new();
        let sections = parse_markdown_sections(content);

        if sections.is_empty() {
            // No markdown structure, treat as raw content
            if !content.trim().is_empty() {
                rules.push(UnifiedRule::raw(content.trim()));
            }
            return Ok(rules);
        }

        for (heading, body) in sections {
            let kind = Self::detect_section_type(&heading);

            match kind {
                SectionKind::Persona => {
                    let points = extract_bullet_points(&body);
                    let (traits, principles): (Vec<_>, Vec<_>) =
                        points.into_iter().partition(|p| !p.to_lowercase().contains("principle"));

                    rules.push(UnifiedRule::Persona {
                        name: heading.clone(),
                        role: body.lines().next().unwrap_or("").to_string(),
                        identity: None,
                        style: None,
                        traits,
                        principles,
                    });
                }
                SectionKind::Context => {
                    let points = extract_bullet_points(&body);
                    let (includes, rest): (Vec<_>, Vec<_>) =
                        points.into_iter().partition(|p| p.contains("**") || p.contains("src/"));

                    rules.push(UnifiedRule::Context {
                        includes,
                        excludes: Vec::new(),
                        focus: rest,
                    });
                }
                SectionKind::Workflow => {
                    let points = extract_bullet_points(&body);
                    let steps = points
                        .into_iter()
                        .map(|p| super::WorkflowStepData {
                            name: p.clone(),
                            description: p,
                            condition: None,
                            actions: Vec::new(),
                        })
                        .collect();

                    rules.push(UnifiedRule::Workflow {
                        name: heading,
                        steps,
                    });
                }
                SectionKind::Standards => {
                    let points = extract_bullet_points(&body);
                    for (i, point) in points.into_iter().enumerate() {
                        rules.push(UnifiedRule::Standard {
                            category: crate::format::RuleCategory::Style,
                            priority: i as u8,
                            description: point,
                            pattern: None,
                        });
                    }
                }
                SectionKind::Other => {
                    // Store as raw content
                    if !body.trim().is_empty() {
                        rules.push(UnifiedRule::raw(format!("# {}\n{}", heading, body)));
                    }
                }
            }
        }

        Ok(rules)
    }

    fn editor(&self) -> Editor {
        Editor::Cursor
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_cursorrules() {
        let content = r#"# Coding Guidelines

- Use snake_case for functions
- Use PascalCase for types
- Keep functions under 50 lines

# Project Context

- Focus on src/ directory
- This is a Rust project
"#;

        let parser = CursorParser::new();
        let rules = parser.parse_content(content).unwrap();

        assert!(!rules.is_empty());
    }

    #[test]
    fn test_parse_empty() {
        let parser = CursorParser::new();
        let rules = parser.parse_content("").unwrap();
        assert!(rules.is_empty());
    }

    #[test]
    fn test_parse_no_structure() {
        let content = "Just some plain text without markdown structure.";
        let parser = CursorParser::new();
        let rules = parser.parse_content(content).unwrap();

        assert_eq!(rules.len(), 1);
        assert!(matches!(rules[0], UnifiedRule::Raw { .. }));
    }

    #[test]
    fn test_detect_section_types() {
        assert_eq!(CursorParser::detect_section_type("AI Persona"), SectionKind::Persona);
        assert_eq!(CursorParser::detect_section_type("Coding Conventions"), SectionKind::Standards);
        assert_eq!(CursorParser::detect_section_type("Project Context"), SectionKind::Context);
        assert_eq!(
            CursorParser::detect_section_type("Development Workflow"),
            SectionKind::Workflow
        );
    }
}
