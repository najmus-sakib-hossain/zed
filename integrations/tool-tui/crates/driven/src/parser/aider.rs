//! Aider rules parser (.aider files, aider.conf.yml)

use super::{RuleParser, UnifiedRule, extract_bullet_points};
use crate::{DrivenError, Editor, Result};
use std::path::Path;

/// Parser for Aider .aider files and aider.conf.yml
#[derive(Debug, Default)]
pub struct AiderParser;

impl AiderParser {
    /// Create a new Aider parser
    pub fn new() -> Self {
        Self
    }
}

impl RuleParser for AiderParser {
    fn parse_file(&self, path: &Path) -> Result<Vec<UnifiedRule>> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| DrivenError::Parse(format!("Failed to read {}: {}", path.display(), e)))?;

        // Check if it's YAML
        if path.extension().is_some_and(|ext| ext == "yml" || ext == "yaml") {
            return self.parse_yaml(&content);
        }

        self.parse_content(&content)
    }

    fn parse_content(&self, content: &str) -> Result<Vec<UnifiedRule>> {
        let mut rules = Vec::new();

        // Aider uses simple line-based format
        let points = extract_bullet_points(content);
        if !points.is_empty() {
            for (i, point) in points.into_iter().enumerate() {
                rules.push(UnifiedRule::Standard {
                    category: crate::format::RuleCategory::Style,
                    priority: i as u8,
                    description: point,
                    pattern: None,
                });
            }
        } else if !content.trim().is_empty() {
            // Treat as raw content
            rules.push(UnifiedRule::raw(content.trim()));
        }

        Ok(rules)
    }

    fn editor(&self) -> Editor {
        Editor::Aider
    }
}

impl AiderParser {
    fn parse_yaml(&self, content: &str) -> Result<Vec<UnifiedRule>> {
        let mut rules = Vec::new();

        // Parse YAML config
        if let Ok(yaml) = serde_yaml::from_str::<serde_yaml::Value>(content) {
            if let Some(mapping) = yaml.as_mapping() {
                // Look for common Aider config keys
                if let Some(conventions) = mapping.get("conventions") {
                    if let Some(seq) = conventions.as_sequence() {
                        for (i, item) in seq.iter().enumerate() {
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

                if let Some(read_only) = mapping.get("read_only_files") {
                    if let Some(seq) = read_only.as_sequence() {
                        let patterns: Vec<String> =
                            seq.iter().filter_map(|v| v.as_str().map(String::from)).collect();

                        if !patterns.is_empty() {
                            rules.push(UnifiedRule::Context {
                                includes: Vec::new(),
                                excludes: patterns,
                                focus: Vec::new(),
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
    fn test_parse_aider_simple() {
        let content = r#"
- Use descriptive variable names
- Write tests for all functions
- Keep functions small and focused
"#;

        let parser = AiderParser::new();
        let rules = parser.parse_content(content).unwrap();

        assert_eq!(rules.len(), 3);
    }

    #[test]
    fn test_parse_aider_yaml() {
        let content = r#"
conventions:
  - Use snake_case for functions
  - Use PascalCase for types
read_only_files:
  - "*.lock"
  - "vendor/**"
"#;

        let parser = AiderParser::new();
        let rules = parser.parse_yaml(content).unwrap();

        assert!(!rules.is_empty());
    }
}
