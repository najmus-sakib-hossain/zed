//! Universal Rule Parser
//!
//! Parses AI coding rules from various editor formats into a unified AST.
//!
//! Supported formats:
//! - Cursor: `.cursorrules`
//! - Copilot: `copilot-instructions.md`
//! - Windsurf: `.windsurfrules`
//! - Claude: `.claude/` folder
//! - Aider: `.aider` files

mod aider;
mod claude;
mod copilot;
mod cursor;
mod unified;
mod windsurf;

pub use aider::AiderParser;
pub use claude::ClaudeParser;
pub use copilot::CopilotParser;
pub use cursor::CursorParser;
pub use unified::{ParsedRule, UnifiedRule, WorkflowStepData};
pub use windsurf::WindsurfParser;

use crate::{DrivenError, Editor, Result};
use std::path::Path;

/// Trait for parsing editor-specific rule formats
pub trait RuleParser {
    /// Parse rules from a file
    fn parse_file(&self, path: &Path) -> Result<Vec<UnifiedRule>>;

    /// Parse rules from content
    fn parse_content(&self, content: &str) -> Result<Vec<UnifiedRule>>;

    /// Get the editor this parser handles
    fn editor(&self) -> Editor;
}

/// Universal parser that auto-detects format
pub struct Parser {
    inner: Box<dyn RuleParser + Send + Sync>,
}

impl std::fmt::Debug for Parser {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Parser").field("editor", &self.inner.editor()).finish()
    }
}

impl Parser {
    /// Create a parser for a specific editor
    pub fn for_editor(editor: Editor) -> Self {
        let inner: Box<dyn RuleParser + Send + Sync> = match editor {
            Editor::Cursor => Box::new(CursorParser::new()),
            Editor::Copilot => Box::new(CopilotParser::new()),
            Editor::Windsurf => Box::new(WindsurfParser::new()),
            Editor::Claude => Box::new(ClaudeParser::new()),
            Editor::Aider => Box::new(AiderParser::new()),
            Editor::Cline => Box::new(CursorParser::new()), // Similar format
        };
        Self { inner }
    }

    /// Auto-detect parser from file path
    pub fn detect(path: &Path) -> Result<Self> {
        let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

        let editor = if file_name == ".cursorrules" || file_name.ends_with(".cursorrules") {
            Editor::Cursor
        } else if file_name == "copilot-instructions.md"
            || path.to_string_lossy().contains(".github")
        {
            Editor::Copilot
        } else if file_name == ".windsurfrules" || file_name.ends_with(".windsurfrules") {
            Editor::Windsurf
        } else if file_name == "CLAUDE.md" || path.to_string_lossy().contains(".claude") {
            Editor::Claude
        } else if file_name.contains(".aider") || file_name == "aider.conf.yml" {
            Editor::Aider
        } else if file_name.ends_with(".md") {
            // Default to Copilot for markdown
            Editor::Copilot
        } else {
            return Err(DrivenError::UnsupportedFormat(format!(
                "Cannot detect format for: {}",
                path.display()
            )));
        };

        Ok(Self::for_editor(editor))
    }

    /// Parse a file
    pub fn parse_file(&self, path: &Path) -> Result<Vec<UnifiedRule>> {
        self.inner.parse_file(path)
    }

    /// Parse content
    pub fn parse_content(&self, content: &str) -> Result<Vec<UnifiedRule>> {
        self.inner.parse_content(content)
    }

    /// Get the detected editor
    pub fn editor(&self) -> Editor {
        self.inner.editor()
    }
}

/// Common markdown section parser
pub(crate) fn parse_markdown_sections(content: &str) -> Vec<(String, String)> {
    let mut sections = Vec::new();
    let mut current_heading = String::new();
    let mut current_content = String::new();

    for line in content.lines() {
        if let Some(stripped) = line.strip_prefix('#') {
            // Save previous section if any
            if !current_heading.is_empty() {
                sections.push((current_heading.clone(), current_content.trim().to_string()));
            }

            // Start new section
            current_heading = stripped.trim_start_matches('#').trim().to_string();
            current_content.clear();
        } else {
            current_content.push_str(line);
            current_content.push('\n');
        }
    }

    // Save last section
    if !current_heading.is_empty() {
        sections.push((current_heading, current_content.trim().to_string()));
    }

    sections
}

/// Extract bullet points from markdown content
pub(crate) fn extract_bullet_points(content: &str) -> Vec<String> {
    content
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            if let Some(rest) = trimmed.strip_prefix("- ") {
                Some(rest.to_string())
            } else if let Some(rest) = trimmed.strip_prefix("* ") {
                Some(rest.to_string())
            } else {
                trimmed.strip_prefix("• ").map(|rest| rest.to_string())
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_cursor() {
        let parser = Parser::detect(Path::new(".cursorrules")).unwrap();
        assert_eq!(parser.editor(), Editor::Cursor);
    }

    #[test]
    fn test_detect_copilot() {
        let parser = Parser::detect(Path::new(".github/copilot-instructions.md")).unwrap();
        assert_eq!(parser.editor(), Editor::Copilot);
    }

    #[test]
    fn test_detect_windsurf() {
        let parser = Parser::detect(Path::new(".windsurfrules")).unwrap();
        assert_eq!(parser.editor(), Editor::Windsurf);
    }

    #[test]
    fn test_parse_markdown_sections() {
        let content = r#"# Section One
Content for section one.

## Section Two
Content for section two.
"#;

        let sections = parse_markdown_sections(content);
        assert_eq!(sections.len(), 2);
        assert_eq!(sections[0].0, "Section One");
        assert_eq!(sections[1].0, "Section Two");
    }

    #[test]
    fn test_extract_bullet_points() {
        let content = r#"Some text
- Point one
- Point two
* Point three
• Point four
Not a point
"#;

        let points = extract_bullet_points(content);
        assert_eq!(points.len(), 4);
        assert_eq!(points[0], "Point one");
        assert_eq!(points[3], "Point four");
    }
}
