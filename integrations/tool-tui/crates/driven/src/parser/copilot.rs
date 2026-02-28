//! GitHub Copilot instructions parser (copilot-instructions.md)

use super::{RuleParser, UnifiedRule, extract_bullet_points, parse_markdown_sections};
use crate::{DrivenError, Editor, Result};
use std::path::Path;

/// Parser for GitHub Copilot copilot-instructions.md files
#[derive(Debug, Default)]
pub struct CopilotParser;

impl CopilotParser {
    /// Create a new Copilot parser
    pub fn new() -> Self {
        Self
    }
}

impl RuleParser for CopilotParser {
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
            let lower_heading = heading.to_lowercase();

            if lower_heading.contains("project")
                || lower_heading.contains("overview")
                || lower_heading.contains("context")
            {
                // Project context section
                let points = extract_bullet_points(&body);
                if !points.is_empty() {
                    rules.push(UnifiedRule::Context {
                        includes: Vec::new(),
                        excludes: Vec::new(),
                        focus: points,
                    });
                }
            } else if lower_heading.contains("code")
                || lower_heading.contains("style")
                || lower_heading.contains("convention")
                || lower_heading.contains("quality")
            {
                // Coding standards section
                let points = extract_bullet_points(&body);
                for (i, point) in points.into_iter().enumerate() {
                    rules.push(UnifiedRule::Standard {
                        category: crate::format::RuleCategory::Style,
                        priority: i as u8,
                        description: point,
                        pattern: None,
                    });
                }
            } else if lower_heading.contains("structure") || lower_heading.contains("architecture")
            {
                // Architecture standards
                let points = extract_bullet_points(&body);
                for (i, point) in points.into_iter().enumerate() {
                    rules.push(UnifiedRule::Standard {
                        category: crate::format::RuleCategory::Architecture,
                        priority: i as u8,
                        description: point,
                        pattern: None,
                    });
                }
            } else if lower_heading.contains("test") {
                // Testing standards
                let points = extract_bullet_points(&body);
                for (i, point) in points.into_iter().enumerate() {
                    rules.push(UnifiedRule::Standard {
                        category: crate::format::RuleCategory::Testing,
                        priority: i as u8,
                        description: point,
                        pattern: None,
                    });
                }
            } else if lower_heading.contains("doc") {
                // Documentation standards
                let points = extract_bullet_points(&body);
                for (i, point) in points.into_iter().enumerate() {
                    rules.push(UnifiedRule::Standard {
                        category: crate::format::RuleCategory::Documentation,
                        priority: i as u8,
                        description: point,
                        pattern: None,
                    });
                }
            } else {
                // Other sections as raw content
                if !body.trim().is_empty() {
                    rules.push(UnifiedRule::raw(format!("# {}\n{}", heading, body)));
                }
            }
        }

        Ok(rules)
    }

    fn editor(&self) -> Editor {
        Editor::Copilot
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_copilot_instructions() {
        let content = r#"# Project Overview

This is a Rust project for binary-first web development.

- Focus on performance
- Zero-copy operations

# Code Quality

- Use `cargo fmt` before commits
- Run `cargo clippy` and fix warnings
- Write unit tests for all public functions

# Testing Standards

- Use property-based testing where applicable
- Aim for 80% code coverage
"#;

        let parser = CopilotParser::new();
        let rules = parser.parse_content(content).unwrap();

        assert!(!rules.is_empty());

        // Should have context, style standards, and testing standards
        let has_context = rules.iter().any(|r| matches!(r, UnifiedRule::Context { .. }));
        let has_testing = rules.iter().any(|r| {
            matches!(
                r,
                UnifiedRule::Standard {
                    category: crate::format::RuleCategory::Testing,
                    ..
                }
            )
        });

        assert!(has_context);
        assert!(has_testing);
    }

    #[test]
    fn test_parse_empty() {
        let parser = CopilotParser::new();
        let rules = parser.parse_content("").unwrap();
        assert!(rules.is_empty());
    }
}
