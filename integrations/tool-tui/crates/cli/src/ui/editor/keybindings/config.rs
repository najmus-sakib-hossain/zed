//! Keybinding configuration

use super::modes::KeybindingMode;

/// Configuration for keybindings loaded from .sr file
#[derive(Debug, Clone, Default)]
pub struct KeybindingConfig {
    /// Binding mode
    pub mode: KeybindingMode,
    /// Custom key mappings
    pub mappings: Vec<(String, String, String)>, // (mode, key, action)
    /// Search is regex by default
    pub search_regex: bool,
    /// Leader key (for custom shortcuts)
    pub leader: char,
}

impl KeybindingConfig {
    /// Load from .sr file content
    pub fn from_sr(_content: &str) -> Self {
        // TODO: Parse .sr format
        Self::default()
    }
}
