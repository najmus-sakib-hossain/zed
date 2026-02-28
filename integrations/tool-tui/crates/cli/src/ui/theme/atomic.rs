//! Atomic style system for composable, type-safe styling
//!
//! This module provides atomic style primitives that compose together
//! to create consistent, reusable styles across all CLI components.
//!
//! # Example
//!
//! ```rust
//! use dx_cli::ui::theme::atomic::{AtomicStyle, Fg, Bg, Border};
//!
//! let style = AtomicStyle::new()
//!     .fg(Fg::Primary)
//!     .bg(Bg::Card)
//!     .border(Border::Default)
//!     .padding(1);
//! ```

use super::tokens::{BorderRadius, Color, DesignTokens};
use console::Style;

/// Foreground color semantic tokens
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fg {
    /// Primary foreground (main text)
    Primary,
    /// Secondary foreground
    Secondary,
    /// Muted/dimmed text
    Muted,
    /// Card foreground
    Card,
    /// Popover foreground
    Popover,
    /// Primary button text
    PrimaryButton,
    /// Secondary button text
    SecondaryButton,
    /// Accent text
    Accent,
    /// Destructive/error text
    Destructive,
    /// Success text
    Success,
    /// Warning text
    Warning,
    /// Info text
    Info,
    /// Inherit (no change)
    Inherit,
}

impl Fg {
    /// Get the color from design tokens
    pub fn get<'a>(&self, tokens: &'a DesignTokens) -> Option<&'a Color> {
        match self {
            Fg::Primary => Some(&tokens.colors.foreground),
            Fg::Secondary => Some(&tokens.colors.secondary_foreground),
            Fg::Muted => Some(&tokens.colors.muted_foreground),
            Fg::Card => Some(&tokens.colors.card_foreground),
            Fg::Popover => Some(&tokens.colors.popover_foreground),
            Fg::PrimaryButton => Some(&tokens.colors.primary_foreground),
            Fg::SecondaryButton => Some(&tokens.colors.secondary_foreground),
            Fg::Accent => Some(&tokens.colors.accent_foreground),
            Fg::Destructive => Some(&tokens.colors.destructive_foreground),
            Fg::Success => Some(&tokens.colors.success),
            Fg::Warning => Some(&tokens.colors.warning),
            Fg::Info => Some(&tokens.colors.info),
            Fg::Inherit => None,
        }
    }
}

/// Background color semantic tokens
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Bg {
    /// Default background
    Default,
    /// Card background
    Card,
    /// Popover background
    Popover,
    /// Muted background
    Muted,
    /// Primary button background
    Primary,
    /// Secondary button background
    Secondary,
    /// Accent background
    Accent,
    /// Destructive background
    Destructive,
    /// Transparent (no background)
    Transparent,
}

impl Bg {
    /// Get the color from design tokens
    pub fn get<'a>(&self, tokens: &'a DesignTokens) -> Option<&'a Color> {
        match self {
            Bg::Default => Some(&tokens.colors.background),
            Bg::Card => Some(&tokens.colors.card),
            Bg::Popover => Some(&tokens.colors.popover),
            Bg::Muted => Some(&tokens.colors.muted),
            Bg::Primary => Some(&tokens.colors.primary),
            Bg::Secondary => Some(&tokens.colors.secondary),
            Bg::Accent => Some(&tokens.colors.accent),
            Bg::Destructive => Some(&tokens.colors.destructive),
            Bg::Transparent => None,
        }
    }
}

/// Border style tokens
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Border {
    /// No border
    None,
    /// Default border color
    Default,
    /// Input field border
    Input,
    /// Focus ring border
    Ring,
    /// Muted border
    Muted,
    /// Primary colored border
    Primary,
    /// Destructive/error border
    Destructive,
    /// Success border
    Success,
    /// Warning border
    Warning,
}

impl Border {
    /// Get the color from design tokens
    pub fn get<'a>(&self, tokens: &'a DesignTokens) -> Option<&'a Color> {
        match self {
            Border::None => None,
            Border::Default => Some(&tokens.colors.border),
            Border::Input => Some(&tokens.colors.input),
            Border::Ring => Some(&tokens.colors.ring),
            Border::Muted => Some(&tokens.colors.muted),
            Border::Primary => Some(&tokens.colors.primary),
            Border::Destructive => Some(&tokens.colors.destructive),
            Border::Success => Some(&tokens.colors.success),
            Border::Warning => Some(&tokens.colors.warning),
        }
    }
}

/// Text weight
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Weight {
    #[default]
    Normal,
    Bold,
}

/// Text decoration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Decoration {
    #[default]
    None,
    Underline,
    Strikethrough,
}

/// Atomic style builder for composable styles
#[derive(Debug, Clone, Default)]
pub struct AtomicStyle {
    pub fg: Option<Fg>,
    pub bg: Option<Bg>,
    pub border: Option<Border>,
    pub border_radius: Option<BorderRadius>,
    pub weight: Weight,
    pub decoration: Decoration,
    pub dim: bool,
    pub italic: bool,
    pub padding_x: u16,
    pub padding_y: u16,
    pub margin_x: u16,
    pub margin_y: u16,
}

impl AtomicStyle {
    /// Create a new empty atomic style
    pub const fn new() -> Self {
        Self {
            fg: None,
            bg: None,
            border: None,
            border_radius: None,
            weight: Weight::Normal,
            decoration: Decoration::None,
            dim: false,
            italic: false,
            padding_x: 0,
            padding_y: 0,
            margin_x: 0,
            margin_y: 0,
        }
    }

    /// Set foreground color
    pub const fn fg(mut self, fg: Fg) -> Self {
        self.fg = Some(fg);
        self
    }

    /// Set background color
    pub const fn bg(mut self, bg: Bg) -> Self {
        self.bg = Some(bg);
        self
    }

    /// Set border style
    pub const fn border(mut self, border: Border) -> Self {
        self.border = Some(border);
        self
    }

    /// Set border radius
    pub const fn radius(mut self, radius: BorderRadius) -> Self {
        self.border_radius = Some(radius);
        self
    }

    /// Set font weight to bold
    pub const fn bold(mut self) -> Self {
        self.weight = Weight::Bold;
        self
    }

    /// Set text to dim
    pub const fn dim(mut self) -> Self {
        self.dim = true;
        self
    }

    /// Set text to italic
    pub const fn italic(mut self) -> Self {
        self.italic = true;
        self
    }

    /// Set text decoration to underline
    pub const fn underline(mut self) -> Self {
        self.decoration = Decoration::Underline;
        self
    }

    /// Set text decoration to strikethrough
    pub const fn strikethrough(mut self) -> Self {
        self.decoration = Decoration::Strikethrough;
        self
    }

    /// Set horizontal padding (left and right)
    pub const fn px(mut self, padding: u16) -> Self {
        self.padding_x = padding;
        self
    }

    /// Set vertical padding (top and bottom)
    pub const fn py(mut self, padding: u16) -> Self {
        self.padding_y = padding;
        self
    }

    /// Set all padding
    pub const fn padding(mut self, padding: u16) -> Self {
        self.padding_x = padding;
        self.padding_y = padding;
        self
    }

    /// Set horizontal margin
    pub const fn mx(mut self, margin: u16) -> Self {
        self.margin_x = margin;
        self
    }

    /// Set vertical margin
    pub const fn my(mut self, margin: u16) -> Self {
        self.margin_y = margin;
        self
    }

    /// Set all margin
    pub const fn margin(mut self, margin: u16) -> Self {
        self.margin_x = margin;
        self.margin_y = margin;
        self
    }

    /// Merge another style on top of this one
    pub fn merge(mut self, other: &AtomicStyle) -> Self {
        if other.fg.is_some() {
            self.fg = other.fg;
        }
        if other.bg.is_some() {
            self.bg = other.bg;
        }
        if other.border.is_some() {
            self.border = other.border;
        }
        if other.border_radius.is_some() {
            self.border_radius = other.border_radius;
        }
        if other.weight != Weight::Normal {
            self.weight = other.weight;
        }
        if other.decoration != Decoration::None {
            self.decoration = other.decoration;
        }
        if other.dim {
            self.dim = true;
        }
        if other.italic {
            self.italic = true;
        }
        if other.padding_x > 0 {
            self.padding_x = other.padding_x;
        }
        if other.padding_y > 0 {
            self.padding_y = other.padding_y;
        }
        if other.margin_x > 0 {
            self.margin_x = other.margin_x;
        }
        if other.margin_y > 0 {
            self.margin_y = other.margin_y;
        }
        self
    }

    /// Convert to console Style using design tokens
    pub fn to_style(&self, tokens: &DesignTokens) -> Style {
        let mut style = Style::new();

        // Apply foreground color
        if let Some(fg) = &self.fg {
            if let Some(color) = fg.get(tokens) {
                style = apply_color_to_style(style, color, false);
            }
        }

        // Apply background color
        if let Some(bg) = &self.bg {
            if let Some(color) = bg.get(tokens) {
                style = apply_color_to_style(style, color, true);
            }
        }

        // Apply weight
        if self.weight == Weight::Bold {
            style = style.bold();
        }

        // Apply dim
        if self.dim {
            style = style.dim();
        }

        // Apply italic
        if self.italic {
            style = style.italic();
        }

        // Apply decoration
        match self.decoration {
            Decoration::None => {}
            Decoration::Underline => style = style.underlined(),
            Decoration::Strikethrough => style = style.strikethrough(),
        }

        style
    }

    /// Get border radius, falling back to theme default
    pub fn get_border_radius(&self, tokens: &DesignTokens) -> BorderRadius {
        self.border_radius.unwrap_or(tokens.border_radius)
    }
}

/// Apply a color to a console style
fn apply_color_to_style(style: Style, color: &Color, is_background: bool) -> Style {
    match color {
        Color::Solid(s) => {
            let ansi = s.to_ansi256();
            if is_background {
                style.on_color256(ansi)
            } else {
                style.color256(ansi)
            }
        }
        Color::Gradient(g) => {
            // Use start color for static rendering
            let ansi = g.start.to_ansi256();
            if is_background {
                style.on_color256(ansi)
            } else {
                style.color256(ansi)
            }
        }
        Color::Rainbow(_) => {
            // Use cyan as default for static rainbow
            if is_background {
                style.on_cyan()
            } else {
                style.cyan()
            }
        }
    }
}

// ============================================================================
// Style Presets
// ============================================================================

/// Common style presets for quick usage
pub mod presets {
    use super::*;

    /// Primary button style
    pub const BUTTON_PRIMARY: AtomicStyle =
        AtomicStyle::new().fg(Fg::PrimaryButton).bg(Bg::Primary).bold().px(2);

    /// Secondary button style
    pub const BUTTON_SECONDARY: AtomicStyle =
        AtomicStyle::new().fg(Fg::SecondaryButton).bg(Bg::Secondary).px(2);

    /// Destructive button style
    pub const BUTTON_DESTRUCTIVE: AtomicStyle =
        AtomicStyle::new().fg(Fg::Destructive).bg(Bg::Destructive).bold().px(2);

    /// Ghost button style (no background)
    pub const BUTTON_GHOST: AtomicStyle = AtomicStyle::new().fg(Fg::Primary).bg(Bg::Transparent);

    /// Card container style
    pub const CARD: AtomicStyle =
        AtomicStyle::new().fg(Fg::Card).bg(Bg::Card).border(Border::Default).padding(1);

    /// Panel container style (subtle background)
    pub const PANEL: AtomicStyle = AtomicStyle::new()
        .fg(Fg::Primary)
        .bg(Bg::Muted)
        .border(Border::Muted)
        .padding(1);

    /// Muted text style
    pub const TEXT_MUTED: AtomicStyle = AtomicStyle::new().fg(Fg::Muted).dim();

    /// Success text style
    pub const TEXT_SUCCESS: AtomicStyle = AtomicStyle::new().fg(Fg::Success);

    /// Warning text style
    pub const TEXT_WARNING: AtomicStyle = AtomicStyle::new().fg(Fg::Warning);

    /// Error text style
    pub const TEXT_ERROR: AtomicStyle = AtomicStyle::new().fg(Fg::Destructive);

    /// Info text style
    pub const TEXT_INFO: AtomicStyle = AtomicStyle::new().fg(Fg::Info);

    /// Input field style
    pub const INPUT: AtomicStyle =
        AtomicStyle::new().fg(Fg::Primary).bg(Bg::Default).border(Border::Input).px(1);

    /// Input field focused style
    pub const INPUT_FOCUSED: AtomicStyle =
        AtomicStyle::new().fg(Fg::Primary).bg(Bg::Default).border(Border::Ring);

    /// Menu item style
    pub const MENU_ITEM: AtomicStyle = AtomicStyle::new().fg(Fg::Primary).bg(Bg::Transparent).px(2);

    /// Menu item selected style
    pub const MENU_ITEM_SELECTED: AtomicStyle =
        AtomicStyle::new().fg(Fg::Accent).bg(Bg::Accent).px(2);

    /// Sidebar item style
    pub const SIDEBAR_ITEM: AtomicStyle =
        AtomicStyle::new().fg(Fg::Secondary).bg(Bg::Transparent).px(1);

    /// Sidebar item selected style
    pub const SIDEBAR_ITEM_SELECTED: AtomicStyle =
        AtomicStyle::new().fg(Fg::Primary).bg(Bg::Muted).bold().px(1);

    /// Badge style
    pub const BADGE: AtomicStyle =
        AtomicStyle::new().fg(Fg::SecondaryButton).bg(Bg::Secondary).px(1);

    /// Badge success style
    pub const BADGE_SUCCESS: AtomicStyle =
        AtomicStyle::new().fg(Fg::PrimaryButton).bg(Bg::Primary).px(1);

    /// Badge warning style
    pub const BADGE_WARNING: AtomicStyle = AtomicStyle::new().fg(Fg::Warning).bg(Bg::Muted).px(1);

    /// Badge error style
    pub const BADGE_ERROR: AtomicStyle = AtomicStyle::new().fg(Fg::Destructive).bg(Bg::Muted).px(1);

    /// Code inline style
    pub const CODE: AtomicStyle = AtomicStyle::new().fg(Fg::Primary).bg(Bg::Muted).px(1);

    /// Heading style
    pub const HEADING: AtomicStyle = AtomicStyle::new().fg(Fg::Primary).bold();

    /// Subheading style
    pub const SUBHEADING: AtomicStyle = AtomicStyle::new().fg(Fg::Muted).bold();

    /// Caption style
    pub const CAPTION: AtomicStyle = AtomicStyle::new().fg(Fg::Muted).dim();

    /// Link style
    pub const LINK: AtomicStyle = AtomicStyle::new().fg(Fg::Info).underline();

    /// Separator/divider style
    pub const SEPARATOR: AtomicStyle = AtomicStyle::new().fg(Fg::Muted).dim();

    /// List item style
    pub const LIST_ITEM: AtomicStyle = AtomicStyle::new().fg(Fg::Primary).bg(Bg::Transparent).px(1);

    /// List item selected style
    pub const LIST_ITEM_SELECTED: AtomicStyle =
        AtomicStyle::new().fg(Fg::Primary).bg(Bg::Muted).bold().px(1);

    /// Progress bar track style
    pub const PROGRESS_TRACK: AtomicStyle = AtomicStyle::new().fg(Fg::Muted).dim();

    /// Progress bar fill style
    pub const PROGRESS_FILL: AtomicStyle = AtomicStyle::new().fg(Fg::Primary);

    /// Spinner style
    pub const SPINNER: AtomicStyle = AtomicStyle::new().fg(Fg::Secondary).dim();

    /// Toast/notification style
    pub const TOAST: AtomicStyle =
        AtomicStyle::new().fg(Fg::Card).bg(Bg::Card).border(Border::Default).padding(1);

    /// Success toast style
    pub const TOAST_SUCCESS: AtomicStyle = AtomicStyle::new()
        .fg(Fg::Success)
        .bg(Bg::Card)
        .border(Border::Success)
        .padding(1);

    /// Warning toast style
    pub const TOAST_WARNING: AtomicStyle = AtomicStyle::new()
        .fg(Fg::Warning)
        .bg(Bg::Card)
        .border(Border::Warning)
        .padding(1);

    /// Error toast style
    pub const TOAST_ERROR: AtomicStyle = AtomicStyle::new()
        .fg(Fg::Destructive)
        .bg(Bg::Card)
        .border(Border::Destructive)
        .padding(1);

    /// Tooltip style
    pub const TOOLTIP: AtomicStyle =
        AtomicStyle::new().fg(Fg::Popover).bg(Bg::Popover).border(Border::Muted).px(1);

    /// Dialog title style
    pub const DIALOG_TITLE: AtomicStyle = AtomicStyle::new().fg(Fg::Primary).bold();

    /// Dialog body style
    pub const DIALOG_BODY: AtomicStyle = AtomicStyle::new().fg(Fg::Secondary);

    /// Dialog warning style
    pub const DIALOG_WARNING: AtomicStyle = AtomicStyle::new()
        .fg(Fg::Warning)
        .bg(Bg::Muted)
        .border(Border::Warning)
        .padding(1);

    /// Dialog error style
    pub const DIALOG_ERROR: AtomicStyle = AtomicStyle::new()
        .fg(Fg::Destructive)
        .bg(Bg::Muted)
        .border(Border::Destructive)
        .padding(1);

    /// Dialog success style
    pub const DIALOG_SUCCESS: AtomicStyle = AtomicStyle::new()
        .fg(Fg::Success)
        .bg(Bg::Muted)
        .border(Border::Success)
        .padding(1);

    /// Table header style
    pub const TABLE_HEADER: AtomicStyle = AtomicStyle::new().fg(Fg::Primary).bold();

    /// Table row style
    pub const TABLE_ROW: AtomicStyle = AtomicStyle::new().fg(Fg::Primary).bg(Bg::Transparent);

    /// Table row selected style
    pub const TABLE_ROW_SELECTED: AtomicStyle =
        AtomicStyle::new().fg(Fg::Primary).bg(Bg::Muted).bold();

    /// Tab style
    pub const TAB: AtomicStyle = AtomicStyle::new().fg(Fg::Secondary).bg(Bg::Transparent).px(1);

    /// Active tab style
    pub const TAB_ACTIVE: AtomicStyle =
        AtomicStyle::new().fg(Fg::Primary).bg(Bg::Muted).bold().px(1);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_atomic_style_builder() {
        let style = AtomicStyle::new().fg(Fg::Primary).bg(Bg::Card).bold().padding(2);

        assert_eq!(style.fg, Some(Fg::Primary));
        assert_eq!(style.bg, Some(Bg::Card));
        assert_eq!(style.weight, Weight::Bold);
        assert_eq!(style.padding_x, 2);
        assert_eq!(style.padding_y, 2);
    }

    #[test]
    fn test_style_merge() {
        let base = AtomicStyle::new().fg(Fg::Primary).bg(Bg::Default);

        let overlay = AtomicStyle::new().bg(Bg::Card).bold();

        let merged = base.merge(&overlay);

        assert_eq!(merged.fg, Some(Fg::Primary)); // From base
        assert_eq!(merged.bg, Some(Bg::Card)); // From overlay
        assert_eq!(merged.weight, Weight::Bold); // From overlay
    }

    #[test]
    fn test_to_console_style() {
        let tokens = DesignTokens::dark();
        let style = AtomicStyle::new().fg(Fg::Success).bold();

        let console_style = style.to_style(&tokens);
        // Should not panic and produce a valid style
        let _ = console_style.apply_to("test");
    }

    #[test]
    fn test_presets() {
        let tokens = DesignTokens::dark();

        // Test that presets compile and work
        let _ = presets::BUTTON_PRIMARY.to_style(&tokens);
        let _ = presets::CARD.to_style(&tokens);
        let _ = presets::TEXT_MUTED.to_style(&tokens);
    }

    #[test]
    fn test_preset_fields() {
        assert_eq!(presets::BUTTON_PRIMARY.fg, Some(Fg::PrimaryButton));
        assert_eq!(presets::BUTTON_PRIMARY.bg, Some(Bg::Primary));
        assert_eq!(presets::BUTTON_PRIMARY.weight, Weight::Bold);

        assert_eq!(presets::BUTTON_SECONDARY.fg, Some(Fg::SecondaryButton));
        assert_eq!(presets::BUTTON_SECONDARY.bg, Some(Bg::Secondary));

        assert_eq!(presets::BUTTON_DESTRUCTIVE.fg, Some(Fg::Destructive));
        assert_eq!(presets::BUTTON_DESTRUCTIVE.bg, Some(Bg::Destructive));
        assert_eq!(presets::BUTTON_DESTRUCTIVE.weight, Weight::Bold);

        assert_eq!(presets::INPUT.border, Some(Border::Input));
        assert_eq!(presets::INPUT_FOCUSED.border, Some(Border::Ring));

        assert_eq!(presets::MENU_ITEM_SELECTED.fg, Some(Fg::Accent));
        assert_eq!(presets::MENU_ITEM_SELECTED.bg, Some(Bg::Accent));

        assert_eq!(presets::TOOLTIP.bg, Some(Bg::Popover));
        assert_eq!(presets::TOOLTIP.border, Some(Border::Muted));
    }
}
