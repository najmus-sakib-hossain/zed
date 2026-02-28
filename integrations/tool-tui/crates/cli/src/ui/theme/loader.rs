//! Configuration loader for theme.sr files
//!
//! This module handles loading and parsing DX Serializer theme configuration
//! files for runtime theme customization.
//!
//! # Example
//!
//! ```rust,ignore
//! use dx_cli::ui::theme::loader::ThemeLoader;
//!
//! let theme = ThemeLoader::load_from_file("~/.dx/config/theme.sr")?;
//! ```

use super::tokens::{BorderRadius, Color, DesignTokens};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Theme configuration loaded from .sr files
#[derive(Debug, Clone)]
pub struct ThemeConfig {
    /// Theme name
    pub name: String,
    /// Base theme to extend (dark, light, high-contrast)
    pub extends: Option<String>,
    /// Color overrides
    pub colors: HashMap<String, String>,
    /// Border radius override
    pub border_radius: Option<String>,
    /// Custom properties
    pub custom: HashMap<String, String>,
}

impl Default for ThemeConfig {
    fn default() -> Self {
        Self {
            name: "default".to_string(),
            extends: Some("dark".to_string()),
            colors: HashMap::new(),
            border_radius: None,
            custom: HashMap::new(),
        }
    }
}

/// Theme loader for .sr configuration files
pub struct ThemeLoader;

impl ThemeLoader {
    /// Load theme from a .sr file path
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<DesignTokens, ThemeLoadError> {
        let content = fs::read_to_string(path.as_ref())
            .map_err(|e| ThemeLoadError::IoError(e.to_string()))?;
        Self::load_from_str(&content)
    }

    /// Load theme from a string
    pub fn load_from_str(content: &str) -> Result<DesignTokens, ThemeLoadError> {
        let config = Self::parse_sr(content)?;
        Self::build_tokens(config)
    }

    /// Parse .sr format into ThemeConfig
    fn parse_sr(content: &str) -> Result<ThemeConfig, ThemeLoadError> {
        let mut config = ThemeConfig::default();
        let mut current_section = String::new();

        for line in content.lines() {
            let line = line.trim();

            // Skip empty lines and comments
            if line.is_empty() || line.starts_with('#') || line.starts_with("//") {
                continue;
            }

            // Section header
            if line.starts_with('[') && line.ends_with(']') {
                current_section = line[1..line.len() - 1].to_string();
                continue;
            }

            // Key-value pair
            if let Some(eq_pos) = line.find('=') {
                let key = line[..eq_pos].trim();
                let value = line[eq_pos + 1..].trim().trim_matches('"');

                match current_section.as_str() {
                    "theme" | "" => match key {
                        "name" => config.name = value.to_string(),
                        "extends" => config.extends = Some(value.to_string()),
                        "border_radius" => config.border_radius = Some(value.to_string()),
                        _ => {}
                    },
                    "colors" => {
                        config.colors.insert(key.to_string(), value.to_string());
                    }
                    "custom" => {
                        config.custom.insert(key.to_string(), value.to_string());
                    }
                    _ => {}
                }
            }
        }

        Ok(config)
    }

    /// Build DesignTokens from ThemeConfig
    fn build_tokens(config: ThemeConfig) -> Result<DesignTokens, ThemeLoadError> {
        // Start with base theme
        let mut tokens = match config.extends.as_deref() {
            Some("light") => DesignTokens::light(),
            Some("high-contrast") | Some("high_contrast") => DesignTokens::high_contrast(),
            _ => DesignTokens::dark(),
        };

        // Apply color overrides
        for (key, value) in &config.colors {
            let color = Color::hex(value);
            match key.as_str() {
                "background" => tokens.colors.background = color,
                "foreground" => tokens.colors.foreground = color,
                "card" => tokens.colors.card = color,
                "card_foreground" => tokens.colors.card_foreground = color,
                "popover" => tokens.colors.popover = color,
                "popover_foreground" => tokens.colors.popover_foreground = color,
                "primary" => tokens.colors.primary = color,
                "primary_foreground" => tokens.colors.primary_foreground = color,
                "secondary" => tokens.colors.secondary = color,
                "secondary_foreground" => tokens.colors.secondary_foreground = color,
                "muted" => tokens.colors.muted = color,
                "muted_foreground" => tokens.colors.muted_foreground = color,
                "accent" => tokens.colors.accent = color,
                "accent_foreground" => tokens.colors.accent_foreground = color,
                "destructive" => tokens.colors.destructive = color,
                "destructive_foreground" => tokens.colors.destructive_foreground = color,
                "border" => tokens.colors.border = color,
                "input" => tokens.colors.input = color,
                "ring" => tokens.colors.ring = color,
                "success" => tokens.colors.success = color,
                "warning" => tokens.colors.warning = color,
                "info" => tokens.colors.info = color,
                _ => {} // Ignore unknown colors
            }
        }

        // Apply border radius override
        if let Some(radius) = &config.border_radius {
            tokens.border_radius = match radius.as_str() {
                "none" => BorderRadius::None,
                "small" => BorderRadius::Small,
                "full" => BorderRadius::Full,
                _ => BorderRadius::Small,
            };
        }

        Ok(tokens)
    }

    /// Get default theme configuration directory
    pub fn default_config_dir() -> Option<std::path::PathBuf> {
        dirs::home_dir().map(|h| h.join(".dx").join("config"))
    }

    /// Load theme from default location or return default tokens
    pub fn load_default() -> DesignTokens {
        if let Some(config_dir) = Self::default_config_dir() {
            let theme_path = config_dir.join("theme.sr");
            if theme_path.exists() {
                if let Ok(tokens) = Self::load_from_file(&theme_path) {
                    return tokens;
                }
            }
        }

        // Auto-detect dark/light mode
        if Self::detect_light_mode() {
            DesignTokens::light()
        } else {
            DesignTokens::dark()
        }
    }

    /// Detect if the terminal is in light mode
    fn detect_light_mode() -> bool {
        // Check environment variables for light mode hints
        if let Ok(colorfgbg) = std::env::var("COLORFGBG") {
            // Format: "foreground;background" - high background number = light mode
            if let Some(bg) = colorfgbg.split(';').last() {
                if let Ok(bg_num) = bg.parse::<u8>() {
                    return bg_num > 8;
                }
            }
        }

        // Check for explicit light mode setting
        if let Ok(term_bg) = std::env::var("TERM_BACKGROUND") {
            return term_bg == "light";
        }

        // Default to dark mode
        false
    }
}

/// Theme loading error types
#[derive(Debug, Clone)]
pub enum ThemeLoadError {
    /// File I/O error
    IoError(String),
    /// Parse error
    ParseError(String),
    /// Invalid configuration
    InvalidConfig(String),
}

impl std::fmt::Display for ThemeLoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ThemeLoadError::IoError(e) => write!(f, "IO error: {}", e),
            ThemeLoadError::ParseError(e) => write!(f, "Parse error: {}", e),
            ThemeLoadError::InvalidConfig(e) => write!(f, "Invalid config: {}", e),
        }
    }
}

impl std::error::Error for ThemeLoadError {}

/// Generate a default theme.sr file content
pub fn generate_default_theme_sr() -> String {
    let mut content = String::new();
    content.push_str("# DX CLI Theme Configuration\n");
    content.push_str("# This file uses DX Serializer (.sr) format\n\n");
    content.push_str("[theme]\n");
    content.push_str("name = \"custom\"\n");
    content.push_str("extends = \"dark\"\n");
    content.push_str("border_radius = \"small\"\n\n");
    content.push_str("[colors]\n");
    content.push_str("# Uncomment and modify to customize colors (hex format)\n");
    content.push_str("# primary = \"fafafa\"\n");
    content.push_str("# secondary = \"27272a\"\n");
    content.push_str("# accent = \"22c55e\"\n");
    content.push_str("# success = \"22c55e\"\n");
    content.push_str("# warning = \"eab308\"\n");
    content.push_str("# destructive = \"ef4444\"\n");
    content.push_str("# info = \"3b82f6\"\n");
    content.push_str("# muted = \"27272a\"\n");
    content.push_str("# muted_foreground = \"a1a1aa\"\n");
    content.push_str("# border = \"27272a\"\n\n");
    content.push_str("[custom]\n");
    content.push_str("# Add custom properties here\n");
    content.push_str("# logo_color = \"rainbow\"\n");
    content
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_sr_basic() {
        let content = r#"
[theme]
name = "test"
extends = "dark"

[colors]
primary = "ff0000"
success = "00ff00"
"#;

        let tokens = ThemeLoader::load_from_str(content).unwrap();
        // Primary should be overridden to red
        if let Color::Solid(s) = &tokens.colors.primary {
            assert_eq!(s.r, 255);
            assert_eq!(s.g, 0);
            assert_eq!(s.b, 0);
        }
    }

    #[test]
    fn test_parse_sr_light_theme() {
        let content = r#"
[theme]
extends = "light"
"#;

        let tokens = ThemeLoader::load_from_str(content).unwrap();
        // Light theme should have white-ish background
        if let Color::Solid(s) = &tokens.colors.background {
            assert!(s.r > 200);
        }
    }

    #[test]
    fn test_parse_sr_border_radius() {
        let content = r#"
[theme]
border_radius = "none"
"#;

        let tokens = ThemeLoader::load_from_str(content).unwrap();
        assert_eq!(tokens.border_radius, BorderRadius::None);
    }

    #[test]
    fn test_default_theme_sr_generation() {
        let content = generate_default_theme_sr();
        assert!(content.contains("[theme]"));
        assert!(content.contains("[colors]"));
        assert!(content.contains("extends"));
    }
}
