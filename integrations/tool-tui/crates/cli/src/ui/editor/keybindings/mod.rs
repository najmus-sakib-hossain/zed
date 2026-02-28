//! Editor Keybindings
//!
//! Configurable keybinding system supporting Vim, Emacs, and standard modes
//! with regex-based search and configurable via editor.sr files.
//!
//! # Features
//!
//! - Multiple binding modes (Vim, Emacs, Standard)
//! - Vim modes: Normal, Insert, Visual, Command
//! - Regex search support
//! - Configurable via .sr files
//! - Motion composition (e.g., 5j, d2w)
//!
//! # Example
//!
//! ```rust,ignore
//! use dx_cli::ui::editor::{EditorKeybindings, EditorMode, KeyAction};
//!
//! let mut bindings = EditorKeybindings::vim();
//! bindings.set_mode(EditorMode::Insert);
//!
//! let action = bindings.handle_key('j');
//! ```

mod actions;
mod config;
mod emacs;
mod modes;
mod motions;
mod standard;
mod vim;

pub use actions::{KeyAction, PendingOperator};
pub use config::KeybindingConfig;
pub use modes::{EditorMode, KeybindingMode};
pub use motions::VimMotion;

use crate::ui::components::traits::KeyEvent;
use std::collections::HashMap;

/// Keybinding configuration and state
pub struct EditorKeybindings {
    /// Binding mode
    mode: KeybindingMode,
    /// Current editor mode (for Vim)
    editor_mode: EditorMode,
    /// Pending count prefix
    count: Option<u32>,
    /// Pending operator
    pending_operator: Option<PendingOperator>,
    /// Pending find character (for f, F, t, T)
    pending_find: Option<char>,
    /// Custom key mappings (mode -> key -> action)
    custom_mappings: HashMap<EditorMode, HashMap<String, KeyAction>>,
    /// Command buffer (for : commands)
    command_buffer: String,
    /// Search buffer
    search_buffer: String,
    /// Search is regex
    search_regex: bool,
    /// Search direction (true = forward)
    search_forward: bool,
    /// Last action (for repeat)
    last_action: Option<KeyAction>,
    /// Recording macro
    recording_macro: Option<char>,
    /// Macro buffer
    macro_buffer: Vec<KeyEvent>,
    /// Stored macros
    macros: HashMap<char, Vec<KeyEvent>>,
}

impl Default for EditorKeybindings {
    fn default() -> Self {
        Self::vim()
    }
}

impl EditorKeybindings {
    /// Create Vim-style keybindings
    pub fn vim() -> Self {
        Self {
            mode: KeybindingMode::Vim,
            editor_mode: EditorMode::Normal,
            count: None,
            pending_operator: None,
            pending_find: None,
            custom_mappings: HashMap::new(),
            command_buffer: String::new(),
            search_buffer: String::new(),
            search_regex: true,
            search_forward: true,
            last_action: None,
            recording_macro: None,
            macro_buffer: Vec::new(),
            macros: HashMap::new(),
        }
    }

    /// Create Emacs-style keybindings
    pub fn emacs() -> Self {
        Self {
            mode: KeybindingMode::Emacs,
            editor_mode: EditorMode::Insert,
            ..Self::vim()
        }
    }

    /// Create standard keybindings (arrow keys)
    pub fn standard() -> Self {
        Self {
            mode: KeybindingMode::Standard,
            editor_mode: EditorMode::Insert,
            ..Self::vim()
        }
    }

    /// Get current editor mode
    pub fn editor_mode(&self) -> EditorMode {
        self.editor_mode
    }

    /// Set editor mode
    pub fn set_mode(&mut self, mode: EditorMode) {
        self.editor_mode = mode;
        self.count = None;
        self.pending_operator = None;
    }

    /// Get current count (or 1 if none)
    pub fn count_or_default(&self) -> u32 {
        self.count.unwrap_or(1)
    }

    /// Get command buffer
    pub fn command_buffer(&self) -> &str {
        &self.command_buffer
    }

    /// Get search buffer
    pub fn search_buffer(&self) -> &str {
        &self.search_buffer
    }

    /// Is search regex mode
    pub fn is_search_regex(&self) -> bool {
        self.search_regex
    }

    /// Toggle search regex mode
    pub fn toggle_search_regex(&mut self) {
        self.search_regex = !self.search_regex;
    }

    /// Add custom key mapping
    pub fn map(&mut self, mode: EditorMode, key: impl Into<String>, action: KeyAction) {
        self.custom_mappings
            .entry(mode)
            .or_insert_with(HashMap::new)
            .insert(key.into(), action);
    }

    /// Handle a key event and return the resulting action
    pub fn handle_key(&mut self, key: KeyEvent) -> KeyAction {
        // Record macro if active
        if self.recording_macro.is_some() {
            self.macro_buffer.push(key.clone());
        }

        match self.mode {
            KeybindingMode::Vim => vim::handle_vim_key(self, key),
            KeybindingMode::Emacs => emacs::handle_emacs_key(self, key),
            KeybindingMode::Standard => standard::handle_standard_key(self, key),
        }
    }
}

/// Convert key event to string representation
pub(crate) fn key_to_string(key: &KeyEvent) -> String {
    match key {
        KeyEvent::Char(c) => c.to_string(),
        KeyEvent::Ctrl(c) => format!("C-{}", c),
        KeyEvent::Alt(c) => format!("M-{}", c),
        KeyEvent::F(n) => format!("F{}", n),
        KeyEvent::Enter => "Enter".to_string(),
        KeyEvent::Escape => "Escape".to_string(),
        KeyEvent::Tab => "Tab".to_string(),
        KeyEvent::BackTab => "BackTab".to_string(),
        KeyEvent::Backspace => "Backspace".to_string(),
        KeyEvent::Delete => "Delete".to_string(),
        KeyEvent::Up => "Up".to_string(),
        KeyEvent::Down => "Down".to_string(),
        KeyEvent::Left => "Left".to_string(),
        KeyEvent::Right => "Right".to_string(),
        KeyEvent::Home => "Home".to_string(),
        KeyEvent::End => "End".to_string(),
        KeyEvent::PageUp => "PageUp".to_string(),
        KeyEvent::PageDown => "PageDown".to_string(),
        KeyEvent::Unknown => "Unknown".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vim_mode_changes() {
        let mut bindings = EditorKeybindings::vim();
        assert_eq!(bindings.editor_mode(), EditorMode::Normal);

        let action = bindings.handle_key(KeyEvent::Char('i'));
        assert_eq!(action, KeyAction::ChangeMode(EditorMode::Insert));
        assert_eq!(bindings.editor_mode(), EditorMode::Insert);

        let action = bindings.handle_key(KeyEvent::Escape);
        assert_eq!(action, KeyAction::ChangeMode(EditorMode::Normal));
        assert_eq!(bindings.editor_mode(), EditorMode::Normal);
    }

    #[test]
    fn test_vim_count() {
        let mut bindings = EditorKeybindings::vim();

        bindings.handle_key(KeyEvent::Char('5'));
        let action = bindings.handle_key(KeyEvent::Char('j'));
        assert_eq!(action, KeyAction::MoveCount(VimMotion::Down, 5));
    }

    #[test]
    fn test_vim_motions() {
        let mut bindings = EditorKeybindings::vim();

        assert_eq!(
            bindings.handle_key(KeyEvent::Char('h')),
            KeyAction::MoveCount(VimMotion::Left, 1)
        );
        assert_eq!(
            bindings.handle_key(KeyEvent::Char('j')),
            KeyAction::MoveCount(VimMotion::Down, 1)
        );
        assert_eq!(
            bindings.handle_key(KeyEvent::Char('k')),
            KeyAction::MoveCount(VimMotion::Up, 1)
        );
        assert_eq!(
            bindings.handle_key(KeyEvent::Char('l')),
            KeyAction::MoveCount(VimMotion::Right, 1)
        );
    }

    #[test]
    fn test_vim_operators() {
        let mut bindings = EditorKeybindings::vim();

        // dd - delete line
        bindings.handle_key(KeyEvent::Char('d'));
        let action = bindings.handle_key(KeyEvent::Char('d'));
        assert_eq!(action, KeyAction::Delete(VimMotion::Down));
    }

    #[test]
    fn test_vim_command_mode() {
        let mut bindings = EditorKeybindings::vim();

        bindings.handle_key(KeyEvent::Char(':'));
        assert_eq!(bindings.editor_mode(), EditorMode::Command);

        bindings.handle_key(KeyEvent::Char('w'));
        let action = bindings.handle_key(KeyEvent::Enter);
        assert_eq!(action, KeyAction::Save);
    }

    #[test]
    fn test_emacs_navigation() {
        let mut bindings = EditorKeybindings::emacs();

        assert_eq!(
            bindings.handle_key(KeyEvent::Ctrl('f')),
            KeyAction::Move(VimMotion::Right)
        );
        assert_eq!(
            bindings.handle_key(KeyEvent::Ctrl('b')),
            KeyAction::Move(VimMotion::Left)
        );
        assert_eq!(
            bindings.handle_key(KeyEvent::Ctrl('n')),
            KeyAction::Move(VimMotion::Down)
        );
        assert_eq!(
            bindings.handle_key(KeyEvent::Ctrl('p')),
            KeyAction::Move(VimMotion::Up)
        );
    }

    #[test]
    fn test_standard_editing() {
        let mut bindings = EditorKeybindings::standard();

        assert_eq!(bindings.handle_key(KeyEvent::Ctrl('c')), KeyAction::Copy);
        assert_eq!(bindings.handle_key(KeyEvent::Ctrl('v')), KeyAction::Paste);
        assert_eq!(bindings.handle_key(KeyEvent::Ctrl('z')), KeyAction::Undo);
    }

    #[test]
    fn test_mode_indicator() {
        assert_eq!(EditorMode::Normal.indicator(), "NORMAL");
        assert_eq!(EditorMode::Insert.indicator(), "INSERT");
        assert_eq!(EditorMode::Visual.indicator(), "VISUAL");
    }
}
