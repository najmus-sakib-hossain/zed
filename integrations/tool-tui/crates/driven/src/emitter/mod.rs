//! Target Format Generators
//!
//! Emits unified rules to editor-specific formats.

mod claude;
mod copilot;
mod cursor;
mod generic;
mod windsurf;

pub use claude::ClaudeEmitter;
pub use copilot::CopilotEmitter;
pub use cursor::CursorEmitter;
pub use generic::GenericEmitter;
pub use windsurf::WindsurfEmitter;

use crate::{DrivenError, Editor, Result, parser::UnifiedRule};
use std::path::Path;

/// Trait for emitting rules to a specific format
pub trait RuleEmitter {
    /// Emit rules to a file
    fn emit_file(&self, rules: &[UnifiedRule], path: &Path) -> Result<()>;

    /// Emit rules to a string
    fn emit_string(&self, rules: &[UnifiedRule]) -> Result<String>;

    /// Get the editor this emitter targets
    fn editor(&self) -> Editor;
}

/// Universal emitter that auto-selects format
pub struct Emitter {
    inner: Box<dyn RuleEmitter + Send + Sync>,
}

impl std::fmt::Debug for Emitter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Emitter").field("editor", &self.inner.editor()).finish()
    }
}

impl Emitter {
    /// Create an emitter for a specific editor
    pub fn for_editor(editor: Editor) -> Self {
        let inner: Box<dyn RuleEmitter + Send + Sync> = match editor {
            Editor::Cursor => Box::new(CursorEmitter::new()),
            Editor::Copilot => Box::new(CopilotEmitter::new()),
            Editor::Windsurf => Box::new(WindsurfEmitter::new()),
            Editor::Claude => Box::new(ClaudeEmitter::new()),
            Editor::Aider => Box::new(GenericEmitter::new()),
            Editor::Cline => Box::new(GenericEmitter::new()),
        };
        Self { inner }
    }

    /// Emit rules to a file
    pub fn emit(&self, rules: &[UnifiedRule], path: impl AsRef<Path>) -> Result<()> {
        self.inner.emit_file(rules, path.as_ref())
    }

    /// Emit rules to a string
    pub fn emit_string(&self, rules: &[UnifiedRule]) -> Result<String> {
        self.inner.emit_string(rules)
    }

    /// Get the editor this emitter targets
    pub fn editor(&self) -> Editor {
        self.inner.editor()
    }
}

/// Helper to ensure parent directory exists
pub(crate) fn ensure_parent_dir(path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        if !parent.exists() {
            std::fs::create_dir_all(parent).map_err(|e| {
                DrivenError::Io(std::io::Error::other(format!(
                    "Failed to create directory {}: {}",
                    parent.display(),
                    e
                )))
            })?;
        }
    }
    Ok(())
}

/// Format a section heading for markdown
pub(crate) fn format_heading(level: usize, text: &str) -> String {
    let hashes = "#".repeat(level.min(6));
    format!("{} {}\n\n", hashes, text)
}

/// Format bullet points for markdown
pub(crate) fn format_bullet_list(items: &[String]) -> String {
    items.iter().map(|item| format!("- {}\n", item)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_heading() {
        assert_eq!(format_heading(1, "Test"), "# Test\n\n");
        assert_eq!(format_heading(2, "Test"), "## Test\n\n");
    }

    #[test]
    fn test_format_bullet_list() {
        let items = vec!["Item 1".to_string(), "Item 2".to_string()];
        let result = format_bullet_list(&items);
        assert!(result.contains("- Item 1"));
        assert!(result.contains("- Item 2"));
    }
}
