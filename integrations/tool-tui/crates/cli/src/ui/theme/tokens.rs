//! Design tokens for the DX CLI theme system
//!
//! This module defines shadcn-ui inspired design tokens that provide
//! consistent styling across all CLI components.
//!
//! # Example
//!
//! ```rust
//! use dx_cli::ui::theme::tokens::{DesignTokens, ColorPalette};
//!
//! let tokens = DesignTokens::dark();
//! println!("Primary color: {:?}", tokens.colors.primary);
//! ```

use console::Style;

/// Color palette following shadcn-ui conventions
#[derive(Debug, Clone)]
pub struct ColorPalette {
    /// Background color for the main content area
    pub background: Color,
    /// Foreground (text) color
    pub foreground: Color,
    /// Card/panel background
    pub card: Color,
    /// Card foreground text
    pub card_foreground: Color,
    /// Popover/dropdown background
    pub popover: Color,
    /// Popover foreground text
    pub popover_foreground: Color,
    /// Primary brand color
    pub primary: Color,
    /// Primary foreground (text on primary)
    pub primary_foreground: Color,
    /// Secondary/muted brand color
    pub secondary: Color,
    /// Secondary foreground
    pub secondary_foreground: Color,
    /// Muted backgrounds
    pub muted: Color,
    /// Muted foreground text
    pub muted_foreground: Color,
    /// Accent highlights
    pub accent: Color,
    /// Accent foreground
    pub accent_foreground: Color,
    /// Destructive/error color
    pub destructive: Color,
    /// Destructive foreground
    pub destructive_foreground: Color,
    /// Border color
    pub border: Color,
    /// Input field border
    pub input: Color,
    /// Focus ring color
    pub ring: Color,
    /// Success state
    pub success: Color,
    /// Warning state
    pub warning: Color,
    /// Info state
    pub info: Color,
}

/// Represents a color that can be solid, gradient, or rainbow animated
#[derive(Debug, Clone)]
pub enum Color {
    /// Single solid color
    Solid(SolidColor),
    /// Linear gradient between colors
    Gradient(GradientColor),
    /// Animated rainbow effect
    Rainbow(RainbowColor),
}

impl Color {
    /// Create a solid color from RGB values
    pub fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self::Solid(SolidColor { r, g, b })
    }

    /// Create a solid color from hex string (e.g., "#ff0000" or "ff0000")
    pub fn hex(hex: &str) -> Self {
        let hex = hex.trim_start_matches('#');
        let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0);
        let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0);
        let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0);
        Self::rgb(r, g, b)
    }

    /// Create a gradient between two colors
    pub fn gradient(start: SolidColor, end: SolidColor) -> Self {
        Self::Gradient(GradientColor { start, end })
    }

    /// Create a rainbow animated color
    pub fn rainbow() -> Self {
        Self::Rainbow(RainbowColor {
            speed: 1.0,
            saturation: 1.0,
            lightness: 0.5,
        })
    }

    /// Convert to console Style
    pub fn to_style(&self) -> Style {
        match self {
            Color::Solid(s) => Style::new().color256(s.to_ansi256()),
            Color::Gradient(g) => {
                // Use start color for static rendering
                Style::new().color256(g.start.to_ansi256())
            }
            Color::Rainbow(_) => {
                // Use cyan as default rainbow color for static
                Style::new().cyan()
            }
        }
    }
}

/// A solid RGB color
#[derive(Debug, Clone, Copy)]
pub struct SolidColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl SolidColor {
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    /// Convert to nearest ANSI 256 color
    pub fn to_ansi256(&self) -> u8 {
        // Use the 6x6x6 color cube (colors 16-231)
        let r = (self.r as u16 * 5 / 255) as u8;
        let g = (self.g as u16 * 5 / 255) as u8;
        let b = (self.b as u16 * 5 / 255) as u8;
        16 + 36 * r + 6 * g + b
    }

    /// Interpolate between two colors
    pub fn lerp(&self, other: &Self, t: f32) -> Self {
        let t = t.clamp(0.0, 1.0);
        Self {
            r: (self.r as f32 + (other.r as f32 - self.r as f32) * t) as u8,
            g: (self.g as f32 + (other.g as f32 - self.g as f32) * t) as u8,
            b: (self.b as f32 + (other.b as f32 - self.b as f32) * t) as u8,
        }
    }
}

/// A gradient between two colors
#[derive(Debug, Clone)]
pub struct GradientColor {
    pub start: SolidColor,
    pub end: SolidColor,
}

impl GradientColor {
    /// Get color at position t (0.0 to 1.0)
    pub fn at(&self, t: f32) -> SolidColor {
        self.start.lerp(&self.end, t)
    }
}

/// Rainbow animated color settings
#[derive(Debug, Clone)]
pub struct RainbowColor {
    /// Animation speed multiplier
    pub speed: f32,
    /// Color saturation (0.0 to 1.0)
    pub saturation: f32,
    /// Color lightness (0.0 to 1.0)
    pub lightness: f32,
}

impl RainbowColor {
    /// Get color at time t (in seconds)
    pub fn at(&self, t: f32) -> SolidColor {
        let hue = (t * self.speed * 360.0) % 360.0;
        hsl_to_rgb(hue, self.saturation, self.lightness)
    }
}

/// Convert HSL to RGB
fn hsl_to_rgb(h: f32, s: f32, l: f32) -> SolidColor {
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = l - c / 2.0;

    let (r, g, b) = match h as u32 {
        0..=59 => (c, x, 0.0),
        60..=119 => (x, c, 0.0),
        120..=179 => (0.0, c, x),
        180..=239 => (0.0, x, c),
        240..=299 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };

    SolidColor {
        r: ((r + m) * 255.0) as u8,
        g: ((g + m) * 255.0) as u8,
        b: ((b + m) * 255.0) as u8,
    }
}

/// Spacing scale (in terminal cells)
#[derive(Debug, Clone, Copy)]
pub struct SpacingScale {
    pub xs: u16,  // 1 cell
    pub sm: u16,  // 2 cells
    pub md: u16,  // 4 cells
    pub lg: u16,  // 6 cells
    pub xl: u16,  // 8 cells
    pub xxl: u16, // 12 cells
}

impl Default for SpacingScale {
    fn default() -> Self {
        Self {
            xs: 1,
            sm: 2,
            md: 4,
            lg: 6,
            xl: 8,
            xxl: 12,
        }
    }
}

/// Border radius presets (for box-drawing characters)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BorderRadius {
    /// No rounding (sharp corners): ┌┐└┘
    None,
    /// Slight rounding: ╭╮╰╯
    Small,
    /// Full rounding (pill shape)
    Full,
}

impl BorderRadius {
    /// Get corner characters for this radius
    pub fn corners(&self) -> BoxCorners {
        match self {
            BorderRadius::None => BoxCorners {
                top_left: '┌',
                top_right: '┐',
                bottom_left: '└',
                bottom_right: '┘',
            },
            BorderRadius::Small | BorderRadius::Full => BoxCorners {
                top_left: '╭',
                top_right: '╮',
                bottom_left: '╰',
                bottom_right: '╯',
            },
        }
    }
}

/// Box corner characters
#[derive(Debug, Clone, Copy)]
pub struct BoxCorners {
    pub top_left: char,
    pub top_right: char,
    pub bottom_left: char,
    pub bottom_right: char,
}

/// Typography settings
#[derive(Debug, Clone)]
pub struct Typography {
    /// Bold text style
    pub bold: Style,
    /// Italic/dim text style
    pub italic: Style,
    /// Underlined text style
    pub underline: Style,
    /// Strikethrough text style
    pub strikethrough: Style,
    /// Code/monospace style
    pub code: Style,
}

impl Default for Typography {
    fn default() -> Self {
        Self {
            bold: Style::new().bold(),
            italic: Style::new().dim(),
            underline: Style::new().underlined(),
            strikethrough: Style::new().strikethrough(),
            code: Style::new().cyan(),
        }
    }
}

/// Animation timing presets
#[derive(Debug, Clone, Copy)]
pub struct AnimationTiming {
    /// Fast animations (50ms)
    pub fast: u64,
    /// Normal animations (100ms)
    pub normal: u64,
    /// Slow animations (200ms)
    pub slow: u64,
    /// Extra slow animations (300ms)
    pub extra_slow: u64,
}

impl Default for AnimationTiming {
    fn default() -> Self {
        Self {
            fast: 50,
            normal: 100,
            slow: 200,
            extra_slow: 300,
        }
    }
}

/// Complete design token set
#[derive(Debug, Clone)]
pub struct DesignTokens {
    /// Color palette
    pub colors: ColorPalette,
    /// Spacing scale
    pub spacing: SpacingScale,
    /// Default border radius
    pub border_radius: BorderRadius,
    /// Typography styles
    pub typography: Typography,
    /// Animation timing
    pub animation: AnimationTiming,
}

impl DesignTokens {
    /// Create dark theme tokens (default)
    pub fn dark() -> Self {
        Self {
            colors: ColorPalette {
                background: Color::hex("09090b"),
                foreground: Color::hex("fafafa"),
                card: Color::hex("09090b"),
                card_foreground: Color::hex("fafafa"),
                popover: Color::hex("09090b"),
                popover_foreground: Color::hex("fafafa"),
                primary: Color::hex("fafafa"),
                primary_foreground: Color::hex("18181b"),
                secondary: Color::hex("27272a"),
                secondary_foreground: Color::hex("fafafa"),
                muted: Color::hex("27272a"),
                muted_foreground: Color::hex("a1a1aa"),
                accent: Color::hex("27272a"),
                accent_foreground: Color::hex("fafafa"),
                destructive: Color::hex("7f1d1d"),
                destructive_foreground: Color::hex("fafafa"),
                border: Color::hex("27272a"),
                input: Color::hex("27272a"),
                ring: Color::hex("d4d4d8"),
                success: Color::hex("22c55e"),
                warning: Color::hex("eab308"),
                info: Color::hex("3b82f6"),
            },
            spacing: SpacingScale::default(),
            border_radius: BorderRadius::Small,
            typography: Typography::default(),
            animation: AnimationTiming::default(),
        }
    }

    /// Create light theme tokens
    pub fn light() -> Self {
        Self {
            colors: ColorPalette {
                background: Color::hex("ffffff"),
                foreground: Color::hex("09090b"),
                card: Color::hex("ffffff"),
                card_foreground: Color::hex("09090b"),
                popover: Color::hex("ffffff"),
                popover_foreground: Color::hex("09090b"),
                primary: Color::hex("18181b"),
                primary_foreground: Color::hex("fafafa"),
                secondary: Color::hex("f4f4f5"),
                secondary_foreground: Color::hex("18181b"),
                muted: Color::hex("f4f4f5"),
                muted_foreground: Color::hex("71717a"),
                accent: Color::hex("f4f4f5"),
                accent_foreground: Color::hex("18181b"),
                destructive: Color::hex("ef4444"),
                destructive_foreground: Color::hex("fafafa"),
                border: Color::hex("e4e4e7"),
                input: Color::hex("e4e4e7"),
                ring: Color::hex("a1a1aa"),
                success: Color::hex("22c55e"),
                warning: Color::hex("eab308"),
                info: Color::hex("3b82f6"),
            },
            spacing: SpacingScale::default(),
            border_radius: BorderRadius::Small,
            typography: Typography::default(),
            animation: AnimationTiming::default(),
        }
    }

    /// Create high contrast theme tokens
    pub fn high_contrast() -> Self {
        Self {
            colors: ColorPalette {
                background: Color::hex("000000"),
                foreground: Color::hex("ffffff"),
                card: Color::hex("000000"),
                card_foreground: Color::hex("ffffff"),
                popover: Color::hex("000000"),
                popover_foreground: Color::hex("ffffff"),
                primary: Color::hex("ffffff"),
                primary_foreground: Color::hex("000000"),
                secondary: Color::hex("1a1a1a"),
                secondary_foreground: Color::hex("ffffff"),
                muted: Color::hex("1a1a1a"),
                muted_foreground: Color::hex("cccccc"),
                accent: Color::hex("00ffff"),
                accent_foreground: Color::hex("000000"),
                destructive: Color::hex("ff0000"),
                destructive_foreground: Color::hex("ffffff"),
                border: Color::hex("ffffff"),
                input: Color::hex("ffffff"),
                ring: Color::hex("00ffff"),
                success: Color::hex("00ff00"),
                warning: Color::hex("ffff00"),
                info: Color::hex("00ffff"),
            },
            spacing: SpacingScale::default(),
            border_radius: BorderRadius::None,
            typography: Typography::default(),
            animation: AnimationTiming::default(),
        }
    }
}

/// Semantic color aliases for common use cases
#[derive(Debug, Clone, Copy)]
pub enum SemanticColor {
    /// Primary brand color
    Primary,
    /// Secondary/muted color
    Secondary,
    /// Success/positive state
    Success,
    /// Warning/caution state
    Warning,
    /// Error/destructive state
    Error,
    /// Informational state
    Info,
    /// Muted/disabled state
    Muted,
    /// Accent/highlight
    Accent,
}

impl SemanticColor {
    /// Get the color from tokens
    pub fn get<'a>(&self, tokens: &'a DesignTokens) -> &'a Color {
        match self {
            SemanticColor::Primary => &tokens.colors.primary,
            SemanticColor::Secondary => &tokens.colors.secondary,
            SemanticColor::Success => &tokens.colors.success,
            SemanticColor::Warning => &tokens.colors.warning,
            SemanticColor::Error => &tokens.colors.destructive,
            SemanticColor::Info => &tokens.colors.info,
            SemanticColor::Muted => &tokens.colors.muted,
            SemanticColor::Accent => &tokens.colors.accent,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_color_hex_parsing() {
        let color = Color::hex("#ff5500");
        if let Color::Solid(s) = color {
            assert_eq!(s.r, 255);
            assert_eq!(s.g, 85);
            assert_eq!(s.b, 0);
        } else {
            panic!("Expected solid color");
        }
    }

    #[test]
    fn test_color_lerp() {
        let start = SolidColor::new(0, 0, 0);
        let end = SolidColor::new(255, 255, 255);
        let mid = start.lerp(&end, 0.5);
        assert_eq!(mid.r, 127);
        assert_eq!(mid.g, 127);
        assert_eq!(mid.b, 127);
    }

    #[test]
    fn test_design_tokens_dark() {
        let tokens = DesignTokens::dark();
        assert_eq!(tokens.spacing.md, 4);
        assert_eq!(tokens.border_radius, BorderRadius::Small);
    }

    #[test]
    fn test_rainbow_color() {
        let rainbow = RainbowColor {
            speed: 1.0,
            saturation: 1.0,
            lightness: 0.5,
        };
        let color = rainbow.at(0.0);
        // At hue 0, should be red-ish
        assert!(color.r > color.g && color.r > color.b);
    }
}
