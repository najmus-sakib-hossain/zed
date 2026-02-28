//! Editor experience configuration.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Editor visual and experience settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditorConfig {
    /// Tab size preference.
    #[serde(default = "default_tab_size")]
    pub tab_size: u8,

    /// Use spaces instead of tabs.
    #[serde(default = "default_true")]
    pub insert_spaces: bool,

    /// Recommended font family.
    #[serde(default)]
    pub font_family: Option<String>,

    /// Recommended font size.
    #[serde(default)]
    pub font_size: Option<u8>,

    /// Line height multiplier.
    #[serde(default)]
    pub line_height: Option<f32>,

    /// Recommended color theme.
    #[serde(default)]
    pub theme: Option<String>,

    /// Recommended icon theme.
    #[serde(default)]
    pub icon_theme: Option<String>,

    /// Keybinding style preference.
    #[serde(default)]
    pub keybinding_style: KeybindingStyle,

    /// Custom keybinding overrides.
    #[serde(default)]
    pub keybindings: Vec<KeybindingConfig>,

    /// Code snippet definitions.
    #[serde(default)]
    pub snippets: Vec<SnippetConfig>,

    /// Word wrap preference.
    #[serde(default)]
    pub word_wrap: WordWrap,

    /// Render whitespace.
    #[serde(default)]
    pub render_whitespace: RenderWhitespace,

    /// Minimap settings.
    #[serde(default)]
    pub minimap: MinimapConfig,

    /// Breadcrumbs settings.
    #[serde(default = "default_true")]
    pub breadcrumbs_enabled: bool,

    /// Additional editor-specific settings.
    #[serde(default)]
    pub extra: HashMap<String, serde_json::Value>,
}

fn default_tab_size() -> u8 {
    4
}

fn default_true() -> bool {
    true
}

impl Default for EditorConfig {
    fn default() -> Self {
        Self {
            tab_size: default_tab_size(),
            insert_spaces: default_true(),
            font_family: None,
            font_size: None,
            line_height: None,
            theme: None,
            icon_theme: None,
            keybinding_style: KeybindingStyle::default(),
            keybindings: Vec::new(),
            snippets: Vec::new(),
            word_wrap: WordWrap::default(),
            render_whitespace: RenderWhitespace::default(),
            minimap: MinimapConfig::default(),
            breadcrumbs_enabled: default_true(),
            extra: HashMap::new(),
        }
    }
}

impl EditorConfig {
    /// Validate editor configuration.
    pub fn validate(&self) -> crate::Result<()> {
        if self.tab_size == 0 || self.tab_size > 16 {
            return Err(crate::Error::validation("Tab size must be between 1 and 16"));
        }

        if let Some(size) = self.font_size
            && (!(6..=72).contains(&size))
        {
            return Err(crate::Error::validation("Font size must be between 6 and 72"));
        }

        Ok(())
    }
}

/// Keybinding style preferences.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum KeybindingStyle {
    /// Default VS Code style.
    #[default]
    Default,
    /// Vim keybindings.
    Vim,
    /// Emacs keybindings.
    Emacs,
    /// Sublime Text keybindings.
    Sublime,
    /// JetBrains/IntelliJ keybindings.
    JetBrains,
}

/// Custom keybinding configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeybindingConfig {
    /// Keyboard shortcut (e.g., "ctrl+shift+b").
    pub key: String,

    /// Command to execute.
    pub command: String,

    /// When clause for conditional activation.
    #[serde(default)]
    pub when: Option<String>,

    /// Arguments to pass to the command.
    #[serde(default)]
    pub args: Option<serde_json::Value>,
}

/// Code snippet configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnippetConfig {
    /// Snippet name/label.
    pub name: String,

    /// Trigger prefix for the snippet.
    pub prefix: String,

    /// Snippet body (with placeholder syntax).
    pub body: Vec<String>,

    /// Description of what the snippet does.
    #[serde(default)]
    pub description: Option<String>,

    /// Languages this snippet applies to.
    #[serde(default)]
    pub languages: Vec<String>,
}

/// Word wrap settings.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum WordWrap {
    /// No word wrap.
    #[default]
    Off,
    /// Wrap at viewport width.
    On,
    /// Wrap at specified column.
    WordWrapColumn,
    /// Wrap at minimum of viewport and column.
    Bounded,
}

/// Whitespace rendering options.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum RenderWhitespace {
    /// Never render whitespace.
    None,
    /// Render only boundary whitespace.
    Boundary,
    /// Render only selection whitespace.
    #[default]
    Selection,
    /// Render trailing whitespace.
    Trailing,
    /// Always render all whitespace.
    All,
}

/// Minimap configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MinimapConfig {
    /// Enable minimap.
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Minimap position (left or right).
    #[serde(default)]
    pub side: MinimapSide,

    /// Maximum column width.
    #[serde(default = "default_minimap_width")]
    pub max_column: u16,

    /// Render characters or blocks.
    #[serde(default)]
    pub render_characters: bool,
}

impl Default for MinimapConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            side: MinimapSide::default(),
            max_column: default_minimap_width(),
            render_characters: false,
        }
    }
}

fn default_minimap_width() -> u16 {
    120
}

/// Minimap position.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum MinimapSide {
    /// Left side of editor.
    Left,
    /// Right side of editor.
    #[default]
    Right,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_editor_config_defaults() {
        let config = EditorConfig::default();
        assert_eq!(config.tab_size, 4);
        assert!(config.insert_spaces);
    }

    #[test]
    fn test_editor_validation() {
        let mut config = EditorConfig::default();
        assert!(config.validate().is_ok());

        config.tab_size = 0;
        assert!(config.validate().is_err());

        config.tab_size = 4;
        config.font_size = Some(100);
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_keybinding_serialization() {
        let kb = KeybindingConfig {
            key: "ctrl+shift+b".to_string(),
            command: "dx.build".to_string(),
            when: Some("editorTextFocus".to_string()),
            args: None,
        };

        let json = serde_json::to_string(&kb).unwrap();
        assert!(json.contains("ctrl+shift+b"));
    }
}
