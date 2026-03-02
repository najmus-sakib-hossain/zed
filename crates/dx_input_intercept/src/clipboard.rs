//! Cross-platform clipboard integration.
//!
//! Uses `arboard` for clipboard access and `get-selected-text` patterns
//! for accessing the currently selected text in any application.

use anyhow::Result;
use std::time::Instant;

/// Manages clipboard operations for DX input interception.
pub struct ClipboardManager {
    /// Last clipboard content we set (to detect external changes).
    last_set_content: Option<String>,
    /// Timestamp of last clipboard read.
    last_read_at: Option<Instant>,
}

impl ClipboardManager {
    pub fn new() -> Self {
        Self {
            last_set_content: None,
            last_read_at: None,
        }
    }

    /// Read the current clipboard text content.
    pub fn read_text(&mut self) -> Result<String> {
        // Using arboard crate pattern:
        // let mut clipboard = arboard::Clipboard::new()?;
        // let text = clipboard.get_text()?;
        self.last_read_at = Some(Instant::now());
        log::debug!("Clipboard read (placeholder)");
        Ok(String::new())
    }

    /// Write text to the clipboard.
    pub fn write_text(&mut self, text: &str) -> Result<()> {
        // let mut clipboard = arboard::Clipboard::new()?;
        // clipboard.set_text(text)?;
        self.last_set_content = Some(text.to_string());
        log::debug!("Clipboard write: {} chars", text.len());
        Ok(())
    }

    /// Get the currently selected text in the foreground application.
    ///
    /// Uses a Cmd+C / Ctrl+C simulation approach as fallback when
    /// accessibility APIs don't provide selected text directly.
    pub fn get_selected_text(&mut self) -> Result<Option<String>> {
        // Strategy:
        // 1. Try accessibility API first (TextFieldReader::read_focused)
        // 2. If no selection from accessibility, use clipboard:
        //    a. Save current clipboard content
        //    b. Simulate Cmd+C / Ctrl+C
        //    c. Read clipboard
        //    d. Restore original clipboard content
        // This is the `get-selected-text` crate pattern.
        log::debug!("Getting selected text (placeholder)");
        Ok(None)
    }

    /// Check if clipboard content has changed since we last set it
    /// (indicates user copied something externally).
    pub fn has_external_change(&mut self) -> Result<bool> {
        let current = self.read_text()?;
        Ok(self
            .last_set_content
            .as_ref()
            .map_or(true, |last| *last != current))
    }
}

impl Default for ClipboardManager {
    fn default() -> Self {
        Self::new()
    }
}
