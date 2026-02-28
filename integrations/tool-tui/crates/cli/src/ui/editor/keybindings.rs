//! Editor Keybindings Module
//!
//! Configurable keybinding system supporting Vim, Emacs, and Standard modes.
//! Keybindings are loaded from `editor.sr` config file.
//!
//! # Example
//!
//! ```rust,ignore
//! use dx_cli::ui::editor::{EditorKeybindings, KeybindingMode};
//!
//! let keybindings = EditorKeybindings::from_config("editor.sr")?;
//! let action = keybindings.map_key(KeyEvent::Char('j'));
//! ```

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// Keybinding mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum KeybindingMode {
    /// Vim-style keybindings (hjkl, modes)
    #[default]
    Vim,
    /// Emacs-style keybindings (Ctrl-based)
    Emacs,
    /// Standard arrow key navigation
    Standard,
}

/// Vim editor mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EditorMode {
    /// Normal mode (navigation)
    #[default]
    Normal,
    /// Insert mode (editing)
    Insert,
    /// Visual mode (selection)
    Visual,
    /// Command mode (ex commands)
    Command,
}

/// Vim motion types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VimMotion {
    /// Move up
    Up,
    /// Move down
    Down,
    /// Move left
    Left,
    /// Move right
    Right,
    /// Word forward
    WordForward,
    /// Word backward
    WordBackward,
    /// End of word
    WordEnd,
    /// Start of line
    LineStart,
    /// End of line
    LineEnd,
    /// First line
    FileStart,
    /// Last line
    FileEnd,
    /// Page up
    PageUp,
    /// Page down
    PageDown,
}

/// Pending operator in Vim mode (d, c, y, etc.)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PendingOperator {
    /// Delete
    Delete,
    /// Change (delete and enter insert)
    Change,
    /// Yank (copy)
    Yank,
}

/// Action to perform based on key input
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyAction {
    /// No action
    None,
    /// Move cursor
    Motion(VimMotion),
    /// Enter insert mode
    EnterInsert,
    /// Enter visual mode
    EnterVisual,
    /// Enter command mode
    EnterCommand,
    /// Exit to normal mode
    ExitToNormal,
    /// Delete
    Delete,
    /// Yank (copy)
    Yank,
    /// Paste
    Paste,
    /// Undo
    Undo,
    /// Redo
    Redo,
    /// Search forward
    SearchForward,
    /// Search backward
    SearchBackward,
    /// Next search match
    NextMatch,
    /// Previous search match
    PrevMatch,
    /// Toggle search mode (literal/regex)
    ToggleSearchMode,
    /// Save file
    Save,
    /// Quit
    Quit,
    /// Set pending operator
    SetOperator(PendingOperator),
}

/// Keybinding configuration loaded from editor.sr
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeybindingConfig {
    /// Keybinding mode
    pub mode: KeybindingMode,
    /// Custom keybindings (key -> action)
    #[serde(default)]
    pub custom: HashMap<String, String>,
    /// Enable Vim modes
    #[serde(default = "default_true")]
    pub vim_modes: bool,
    /// Enable relative line numbers in Vim mode
    #[serde(default = "default_true")]
    pub vim_relative_numbers: bool,
}

fn default_true() -> bool {
    true
}

impl Default for KeybindingConfig {
    fn default() -> Self {
        Self {
            mode: KeybindingMode::Vim,
            custom: HashMap::new(),
            vim_modes: true,
            vim_relative_numbers: true,
        }
    }
}

/// Editor keybindings manager
pub struct EditorKeybindings {
    /// Current configuration
    config: KeybindingConfig,
    /// Current Vim mode
    mode: EditorMode,
    /// Pending operator (for Vim motions like dw, cw)
    pending_operator: Option<PendingOperator>,
    /// Count prefix (for Vim commands like 5j)
    count: Option<usize>,
}

impl Default for EditorKeybindings {
    fn default() -> Self {
        Self::new()
    }
}

impl EditorKeybindings {
    /// Create new keybindings with default Vim mode
    pub fn new() -> Self {
        Self {
            config: KeybindingConfig::default(),
            mode: EditorMode::Normal,
            pending_operator: None,
            count: None,
        }
    }

    /// Load keybindings from config file
    pub fn from_config<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = std::fs::read_to_string(path.as_ref())
            .with_context(|| format!("Failed to read config: {}", path.as_ref().display()))?;

        let config: KeybindingConfig = toml::from_str(&content)
            .with_context(|| "Failed to parse editor.sr config")?;

        Ok(Self {
            config,
            mode: EditorMode::Normal,
            pending_operator: None,
            count: None,
        })
    }

    /// Get current keybinding mode
    pub fn mode(&self) -> KeybindingMode {
        self.config.mode
    }

    /// Get current editor mode (Vim only)
    pub fn editor_mode(&self) -> EditorMode {
        self.mode
    }

    /// Set editor mode
    pub fn set_mode(&mut self, mode: EditorMode) {
        self.mode = mode;
        if mode != EditorMode::Normal {
            self.pending_operator = None;
            self.count = None;
        }
    }

    /// Get pending operator
    pub fn pending_operator(&self) -> Option<PendingOperator> {
        self.pending_operator
    }

    /// Get count prefix
    pub fn count(&self) -> Option<usize> {
        self.count
    }

    /// Map a key event to an action based on current mode
    pub fn map_key(&mut self, key: char) -> KeyAction {
        match self.config.mode {
            KeybindingMode::Vim => self.map_vim_key(key),
            KeybindingMode::Emacs => self.map_emacs_key(key),
            KeybindingMode::Standard => self.map_standard_key(key),
        }
    }

    /// Map Vim keybinding
    fn map_vim_key(&mut self, key: char) -> KeyAction {
        match self.mode {
            EditorMode::Normal => self.map_vim_normal(key),
            EditorMode::Insert => self.map_vim_insert(key),
            EditorMode::Visual => self.map_vim_visual(key),
            EditorMode::Command => KeyAction::None,
        }
    }

    /// Map Vim normal mode key
    fn map_vim_normal(&mut self, key: char) -> KeyAction {
        // Handle count prefix (1-9)
        if key.is_ascii_digit() && key != '0' {
            let digit = key.to_digit(10).unwrap() as usize;
            self.count = Some(self.count.unwrap_or(0) * 10 + digit);
            return KeyAction::None;
        }

        let action = match key {
            // Motion
            'h' => KeyAction::Motion(VimMotion::Left),
            'j' => KeyAction::Motion(VimMotion::Down),
            'k' => KeyAction::Motion(VimMotion::Up),
            'l' => KeyAction::Motion(VimMotion::Right),
            'w' => KeyAction::Motion(VimMotion::WordForward),
            'b' => KeyAction::Motion(VimMotion::WordBackward),
            'e' => KeyAction::Motion(VimMotion::WordEnd),
            '0' => KeyAction::Motion(VimMotion::LineStart),
            '$' => KeyAction::Motion(VimMotion::LineEnd),
            'g' => KeyAction::Motion(VimMotion::FileStart),
            'G' => KeyAction::Motion(VimMotion::FileEnd),

            // Mode changes
            'i' => KeyAction::EnterInsert,
            'a' => KeyAction::EnterInsert, // TODO: move right first
            'I' => KeyAction::EnterInsert, // TODO: go to line start
            'A' => KeyAction::EnterInsert, // TODO: go to line end
            'v' => KeyAction::EnterVisual,
            ':' => KeyAction::EnterCommand,

            // Operators
            'd' => KeyAction::SetOperator(PendingOperator::Delete),
            'c' => KeyAction::SetOperator(PendingOperator::Change),
            'y' => KeyAction::SetOperator(PendingOperator::Yank),
            'p' => KeyAction::Paste,

            // Undo/Redo
            'u' => KeyAction::Undo,
            'r' => KeyAction::Redo, // Ctrl-r in real Vim

            // Search
            '/' => KeyAction::SearchForward,
            '?' => KeyAction::SearchBackward,
            'n' => KeyAction::NextMatch,
            'N' => KeyAction::PrevMatch,

            _ => KeyAction::None,
        };

        // Clear count after action
        if action != KeyAction::None {
            self.count = None;
        }

        action
    }

    /// Map Vim insert mode key
    fn map_vim_insert(&mut self, key: char) -> KeyAction {
        match key {
            '\x1b' => KeyAction::ExitToNormal, // ESC
            _ => KeyAction::None,
        }
    }

    /// Map Vim visual mode key
    fn map_vim_visual(&mut self, key: char) -> KeyAction {
        match key {
            '\x1b' => KeyAction::ExitToNormal, // ESC
            'd' => KeyAction::Delete,
            'y' => KeyAction::Yank,
            _ => self.map_vim_normal(key), // Reuse normal mode motions
        }
    }

    /// Map Emacs keybinding
    fn map_emacs_key(&mut self, _key: char) -> KeyAction {
        // TODO: Implement Emacs keybindings
        // Ctrl-n/p for up/down, Ctrl-f/b for left/right, etc.
        KeyAction::None
    }

    /// Map standard keybinding
    fn map_standard_key(&mut self, _key: char) -> KeyAction {
        // Standard mode uses arrow keys, handled separately
        KeyAction::None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_keybindings() {
        let kb = EditorKeybindings::new();
        assert_eq!(kb.mode(), KeybindingMode::Vim);
        assert_eq!(kb.editor_mode(), EditorMode::Normal);
    }

    #[test]
    fn test_vim_motion_keys() {
        let mut kb = EditorKeybindings::new();

        assert_eq!(kb.map_key('j'), KeyAction::Motion(VimMotion::Down));
        assert_eq!(kb.map_key('k'), KeyAction::Motion(VimMotion::Up));
        assert_eq!(kb.map_key('h'), KeyAction::Motion(VimMotion::Left));
        assert_eq!(kb.map_key('l'), KeyAction::Motion(VimMotion::Right));
    }

    #[test]
    fn test_vim_mode_changes() {
        let mut kb = EditorKeybindings::new();

        assert_eq!(kb.map_key('i'), KeyAction::EnterInsert);
        kb.set_mode(EditorMode::Insert);
        assert_eq!(kb.editor_mode(), EditorMode::Insert);

        assert_eq!(kb.map_key('\x1b'), KeyAction::ExitToNormal);
    }

    #[test]
    fn test_vim_count_prefix() {
        let mut kb = EditorKeybindings::new();

        kb.map_key('5');
        assert_eq!(kb.count(), Some(5));

        kb.map_key('j');
        assert_eq!(kb.count(), None); // Cleared after action
    }

    #[test]
    fn test_vim_operators() {
        let mut kb = EditorKeybindings::new();

        assert_eq!(
            kb.map_key('d'),
            KeyAction::SetOperator(PendingOperator::Delete)
        );
        assert_eq!(
            kb.map_key('y'),
            KeyAction::SetOperator(PendingOperator::Yank)
        );
    }

    #[test]
    fn test_config_parsing() {
        let config_str = r#"
mode = "vim"
vim_modes = true
vim_relative_numbers = true

[custom]
"Ctrl-s" = "save"
"#;

        let config: KeybindingConfig = toml::from_str(config_str).unwrap();
        assert_eq!(config.mode, KeybindingMode::Vim);
        assert!(config.vim_modes);
        assert!(config.vim_relative_numbers);
    }
}
