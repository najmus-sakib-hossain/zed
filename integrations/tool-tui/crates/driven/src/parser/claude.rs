//! Claude Code rules parser (.claude/ folder, CLAUDE.md)

use super::{RuleParser, UnifiedRule, extract_bullet_points, parse_markdown_sections};
use crate::{DrivenError, Editor, Result};
use std::path::Path;

/// Parser for Claude Code .claude/ folder and CLAUDE.md files
#[derive(Debug, Default)]
pub struct ClaudeParser;

impl ClaudeParser {
    /// Create a new Claude parser
    pub fn new() -> Self {
        Self
    }
}

impl RuleParser for ClaudeParser {
    fn parse_file(&self, path: &Path) -> Result<Vec<UnifiedRule>> {
        // Check if it's a directory
        if path.is_dir() {
            return self.parse_directory(path);
        }

        let content = std::fs::read_to_string(path)
            .map_err(|e| DrivenError::Parse(format!("Failed to read {}: {}", path.display(), e)))?;
        self.parse_content(&content)
    }

    fn parse_content(&self, content: &str) -> Result<Vec<UnifiedRule>> {
        let mut rules = Vec::new();
        let sections = parse_markdown_sections(content);

        if sections.is_empty() {
            if !content.trim().is_empty() {
                rules.push(UnifiedRule::raw(content.trim()));
            }
            return Ok(rules);
        }

        for (heading, body) in sections {
            let lower = heading.to_lowercase();

            if lower.contains("instructions") || lower.contains("guidelines") {
                let points = extract_bullet_points(&body);
                for (i, point) in points.into_iter().enumerate() {
                    rules.push(UnifiedRule::Standard {
                        category: crate::format::RuleCategory::Style,
                        priority: i as u8,
                        description: point,
                        pattern: None,
                    });
                }
            } else if lower.contains("context") || lower.contains("about") {
                let points = extract_bullet_points(&body);
                rules.push(UnifiedRule::Context {
                    includes: Vec::new(),
                    excludes: Vec::new(),
                    focus: points,
                });
            } else if lower.contains("memory") || lower.contains("remember") {
                // Claude's memory/persistence section
                let points = extract_bullet_points(&body);
                for (i, point) in points.into_iter().enumerate() {
                    rules.push(UnifiedRule::Standard {
                        category: crate::format::RuleCategory::Other,
                        priority: (100 + i) as u8,
                        description: point,
                        pattern: None,
                    });
                }
            } else if !body.trim().is_empty() {
                rules.push(UnifiedRule::raw(format!("# {}\n{}", heading, body)));
            }
        }

        Ok(rules)
    }

    fn editor(&self) -> Editor {
        Editor::Claude
    }
}

impl ClaudeParser {
    fn parse_directory(&self, path: &Path) -> Result<Vec<UnifiedRule>> {
        let mut rules = Vec::new();

        // Look for common files
        let files_to_check = ["settings.json", "CLAUDE.md", "instructions.md"];

        for file_name in files_to_check {
            let file_path = path.join(file_name);
            if file_path.exists() {
                if file_name.ends_with(".json") {
                    // Parse JSON settings
                    let content = std::fs::read_to_string(&file_path)?;
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                        rules.extend(self.parse_json_settings(&json)?);
                    }
                } else {
                    // Parse markdown
                    let content = std::fs::read_to_string(&file_path)?;
                    rules.extend(self.parse_content(&content)?);
                }
            }
        }

        Ok(rules)
    }

    fn parse_json_settings(&self, json: &serde_json::Value) -> Result<Vec<UnifiedRule>> {
        let mut rules = Vec::new();

        if let Some(obj) = json.as_object() {
            if let Some(instructions) = obj.get("instructions") {
                if let Some(text) = instructions.as_str() {
                    rules.push(UnifiedRule::raw(text));
                } else if let Some(arr) = instructions.as_array() {
                    for (i, item) in arr.iter().enumerate() {
                        if let Some(text) = item.as_str() {
                            rules.push(UnifiedRule::Standard {
                                category: crate::format::RuleCategory::Style,
                                priority: i as u8,
                                description: text.to_string(),
                                pattern: None,
                            });
                        }
                    }
                }
            }
        }

        Ok(rules)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_claude_md() {
        let content = r#"# CLAUDE.md

## Instructions

- Always explain your reasoning
- Use clear, concise language
- Follow the coding standards

## Project Context

- This is a Rust project
- Focus on performance
"#;

        let parser = ClaudeParser::new();
        let rules = parser.parse_content(content).unwrap();

        assert!(!rules.is_empty());
    }
}
