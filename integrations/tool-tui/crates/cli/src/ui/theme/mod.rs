//! Theme definitions for consistent CLI styling
//!
//! This module provides a comprehensive theme system inspired by shadcn-ui,
//! with support for design tokens, atomic styles, animations, and runtime
//! configuration via .sr files.
//!
//! # Architecture
//!
//! The theme system is organized into several layers:
//!
//! - **Tokens** ([`tokens`]): Design tokens defining colors, spacing, typography
//! - **Atomic** ([`atomic`]): Composable style primitives (Fg, Bg, Border, etc.)
//! - **Animation** ([`animation`]): Rainbow, gradient, and pulse effects
//! - **Loader** ([`loader`]): Runtime configuration from .sr files
//!
//! # Example
//!
//! ```rust,ignore
//! use dx_cli::ui::theme::{DxTheme, atomic::presets};
//!
//! // Get the global theme
//! let theme = DxTheme::global();
//!
//! // Use atomic style presets
//! let style = presets::BUTTON_PRIMARY.to_style(&theme.tokens);
//! println!("{}", style.apply_to("Click me"));
//! ```

pub mod animation;
pub mod atomic;
pub mod icons;
pub mod loader;
mod styles;
pub mod tokens;
mod types;

pub use atomic::AtomicStyle;
pub use loader::ThemeLoader;
pub use styles::Theme;
pub use tokens::DesignTokens;
pub use types::ColorMode;

use once_cell::sync::Lazy;
use std::sync::{Arc, RwLock};

/// ASCII art logo for splash screens
pub const LOGO_SMALL: &str = r"
    ◆  DX
    Binary-First Development
";

/// Compact logo for inline use
pub const LOGO_INLINE: &str = "◆ DX";

/// Logo with tagline
pub const LOGO_TAGLINE: &str = "◆ DX — Binary-First Development";

/// Minimal logo mark (diamond symbol)
pub const LOGO_MARK: &str = "◆";

/// The unified DX theme providing access to all styling functionality
#[derive(Debug, Clone)]
pub struct DxTheme {
    /// Design tokens (colors, spacing, etc.)
    pub tokens: DesignTokens,
    /// Color mode setting
    pub color_mode: ColorMode,
    /// Whether colors are enabled
    pub colors_enabled: bool,
}

impl Default for DxTheme {
    fn default() -> Self {
        Self::new()
    }
}

impl DxTheme {
    /// Create a new theme with default settings
    pub fn new() -> Self {
        let colors_enabled = Self::detect_color_support();
        Self {
            tokens: ThemeLoader::load_default(),
            color_mode: ColorMode::Auto,
            colors_enabled,
        }
    }

    /// Create a theme with specific color mode
    pub fn with_color_mode(mode: ColorMode) -> Self {
        let colors_enabled = match mode {
            ColorMode::Always => true,
            ColorMode::Never => false,
            ColorMode::Auto => Self::detect_color_support(),
        };

        Self {
            tokens: ThemeLoader::load_default(),
            color_mode: mode,
            colors_enabled,
        }
    }

    /// Create a theme with specific tokens
    pub fn with_tokens(tokens: DesignTokens) -> Self {
        Self {
            tokens,
            color_mode: ColorMode::Auto,
            colors_enabled: Self::detect_color_support(),
        }
    }

    /// Get the global theme instance
    pub fn global() -> Arc<RwLock<DxTheme>> {
        GLOBAL_THEME.clone()
    }

    /// Set the global theme
    pub fn set_global(theme: DxTheme) {
        if let Ok(mut global) = GLOBAL_THEME.write() {
            *global = theme;
        }
    }

    /// Reload theme from configuration file
    pub fn reload(&mut self) {
        self.tokens = ThemeLoader::load_default();
    }

    /// Detect terminal color support
    fn detect_color_support() -> bool {
        // Check for NO_COLOR environment variable
        if std::env::var("NO_COLOR").is_ok() {
            return false;
        }

        // Check for FORCE_COLOR environment variable
        if std::env::var("FORCE_COLOR").is_ok() {
            return true;
        }

        // Check if stderr is a TTY
        atty::is(atty::Stream::Stderr)
    }

    /// Create an atomic style and resolve it with current tokens
    pub fn style(&self, atomic: &AtomicStyle) -> console::Style {
        if self.colors_enabled {
            atomic.to_style(&self.tokens)
        } else {
            console::Style::new()
        }
    }

    /// Get a rainbow animation
    pub fn rainbow(&self) -> animation::RainbowAnimation {
        animation::RainbowAnimation::new()
    }

    /// Apply a style preset
    pub fn apply_preset<S: AsRef<str>>(&self, preset: &AtomicStyle, text: S) -> String {
        if self.colors_enabled {
            let style = preset.to_style(&self.tokens);
            style.apply_to(text.as_ref()).to_string()
        } else {
            text.as_ref().to_string()
        }
    }
}

/// Global theme instance
static GLOBAL_THEME: Lazy<Arc<RwLock<DxTheme>>> =
    Lazy::new(|| Arc::new(RwLock::new(DxTheme::new())));

/// Set the global theme (convenience function)
pub fn set_theme(theme: DxTheme) {
    DxTheme::set_global(theme);
}

/// Get the global theme (convenience function)
pub fn get_theme() -> DxTheme {
    GLOBAL_THEME.read().map(|t| t.clone()).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_theme_creation() {
        let theme = DxTheme::new();
        assert!(theme.tokens.spacing.md == 4);
    }

    #[test]
    fn test_global_theme() {
        let theme = get_theme();
        assert!(theme.tokens.spacing.md == 4);
    }

    #[test]
    fn test_atomic_style_resolution() {
        let theme = DxTheme::new();
        let style = atomic::AtomicStyle::new().fg(Fg::Primary).bold();
        let resolved = theme.style(&style);
        let _ = resolved.apply_to("test");
    }
}
