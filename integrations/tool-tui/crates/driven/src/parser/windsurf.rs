//! Windsurf rules parser (.windsurfrules)

use super::{RuleParser, UnifiedRule, extract_bullet_points, parse_markdown_sections};
use crate::{DrivenError, Editor, Result};
use std::path::Path;

/// Parser for Windsurf .windsurfrules files
#[derive(Debug, Default)]
pub struct WindsurfParser;

impl WindsurfParser {
    /// Create a new Windsurf parser
    pub fn new() -> Self {
        Self
    }
}

impl RuleParser for WindsurfParser {
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

        // Windsurf format is similar to Cursor, reuse same logic
        for (heading, body) in sections {
            let lower = heading.to_lowercase();

            if lower.contains("rule") || lower.contains("guideline") || lower.contains("standard") {
                let points = extract_bullet_points(&body);
                for (i, point) in points.into_iter().enumerate() {
                    rules.push(UnifiedRule::Standard {
                        category: crate::format::RuleCategory::Style,
                        priority: i as u8,
                        description: point,
                        pattern: None,
                    });
                }
            } else if lower.contains("context") || lower.contains("project") {
                let points = extract_bullet_points(&body);
                rules.push(UnifiedRule::Context {
                    includes: Vec::new(),
                    excludes: Vec::new(),
                    focus: points,
                });
            } else {
                // Store as raw
                if !body.trim().is_empty() {
                    rules.push(UnifiedRule::raw(format!("# {}\n{}", heading, body)));
                }
            }
        }

        Ok(rules)
    }

    fn editor(&self) -> Editor {
        Editor::Windsurf
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_windsurfrules() {
        let content = r#"# Coding Rules

- Always use TypeScript strict mode
- Prefer functional components
- Use proper error boundaries

# Project Context

- This is a Next.js application
- Using Tailwind CSS for styling
"#;

        let parser = WindsurfParser::new();
        let rules = parser.parse_content(content).unwrap();

        assert!(!rules.is_empty());
    }
}
