//! Rainbow animation effects for terminal UI
//!
//! This module provides high-performance rainbow text and border animations
//! using ANSI color codes.

use super::tokens::{RainbowColor, SolidColor};
use std::time::{Duration, Instant};

/// Rainbow animation state
#[derive(Debug, Clone)]
pub struct RainbowAnimation {
    /// Animation start time
    start: Instant,
    /// Animation speed multiplier
    speed: f32,
    /// Color saturation (0.0 to 1.0)
    saturation: f32,
    /// Color lightness (0.0 to 1.0)
    lightness: f32,
    /// Phase offset for multi-character animations
    phase_offset: f32,
}

impl Default for RainbowAnimation {
    fn default() -> Self {
        Self::new()
    }
}

impl RainbowAnimation {
    /// Create a new rainbow animation
    pub fn new() -> Self {
        Self {
            start: Instant::now(),
            speed: 1.0,
            saturation: 0.8,
            lightness: 0.6,
            phase_offset: 0.1,
        }
    }

    /// Set animation speed
    pub fn with_speed(mut self, speed: f32) -> Self {
        self.speed = speed;
        self
    }

    /// Set color saturation
    pub fn with_saturation(mut self, saturation: f32) -> Self {
        self.saturation = saturation.clamp(0.0, 1.0);
        self
    }

    /// Set color lightness
    pub fn with_lightness(mut self, lightness: f32) -> Self {
        self.lightness = lightness.clamp(0.0, 1.0);
        self
    }

    /// Set phase offset between characters
    pub fn with_phase_offset(mut self, offset: f32) -> Self {
        self.phase_offset = offset;
        self
    }

    /// Reset animation start time
    pub fn reset(&mut self) {
        self.start = Instant::now();
    }

    /// Get current animation time in seconds
    pub fn elapsed(&self) -> f32 {
        self.start.elapsed().as_secs_f32()
    }

    /// Get color at current time with optional character offset
    pub fn color_at(&self, char_index: usize) -> SolidColor {
        let t = self.elapsed() * self.speed + (char_index as f32 * self.phase_offset);
        let rainbow = RainbowColor {
            speed: 1.0,
            saturation: self.saturation,
            lightness: self.lightness,
        };
        rainbow.at(t)
    }

    /// Apply rainbow effect to a string, returning ANSI-escaped string
    pub fn apply(&self, text: &str) -> String {
        let mut result = String::with_capacity(text.len() * 20);

        for (i, c) in text.chars().enumerate() {
            if c.is_whitespace() {
                result.push(c);
            } else {
                let color = self.color_at(i);
                result.push_str(&format!(
                    "\x1b[38;2;{};{};{}m{}\x1b[0m",
                    color.r, color.g, color.b, c
                ));
            }
        }

        result
    }

    /// Apply rainbow effect with background color
    pub fn apply_with_bg(&self, text: &str, bg: SolidColor) -> String {
        let mut result = String::with_capacity(text.len() * 30);

        for (i, c) in text.chars().enumerate() {
            if c.is_whitespace() {
                result.push_str(&format!("\x1b[48;2;{};{};{}m{}\x1b[0m", bg.r, bg.g, bg.b, c));
            } else {
                let color = self.color_at(i);
                result.push_str(&format!(
                    "\x1b[38;2;{};{};{}m\x1b[48;2;{};{};{}m{}\x1b[0m",
                    color.r, color.g, color.b, bg.r, bg.g, bg.b, c
                ));
            }
        }

        result
    }

    /// Create a rainbow border string
    pub fn border(&self, width: usize, style: BorderStyle) -> String {
        let (left, fill, right) = match style {
            BorderStyle::Top => ('╭', '─', '╮'),
            BorderStyle::Bottom => ('╰', '─', '╯'),
            BorderStyle::TopSquare => ('┌', '─', '┐'),
            BorderStyle::BottomSquare => ('└', '─', '┘'),
            BorderStyle::Horizontal => ('─', '─', '─'),
        };

        let mut chars: Vec<char> = Vec::with_capacity(width);
        chars.push(left);
        for _ in 1..width.saturating_sub(1) {
            chars.push(fill);
        }
        if width > 1 {
            chars.push(right);
        }

        let mut result = String::with_capacity(width * 20);
        for (i, c) in chars.iter().enumerate() {
            let color = self.color_at(i);
            result.push_str(&format!("\x1b[38;2;{};{};{}m{}\x1b[0m", color.r, color.g, color.b, c));
        }

        result
    }

    /// Create vertical rainbow border character
    pub fn vertical_border(&self, row: usize) -> String {
        let color = self.color_at(row);
        format!("\x1b[38;2;{};{};{}m│\x1b[0m", color.r, color.g, color.b)
    }
}

/// Border style variants
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BorderStyle {
    /// Top border with rounded corners
    Top,
    /// Bottom border with rounded corners
    Bottom,
    /// Top border with square corners
    TopSquare,
    /// Bottom border with square corners
    BottomSquare,
    /// Simple horizontal line
    Horizontal,
}

/// Gradient animation between two colors
#[derive(Debug, Clone)]
pub struct GradientAnimation {
    /// Start color
    start: SolidColor,
    /// End color
    end: SolidColor,
    /// Animation duration
    duration: Duration,
    /// Animation start time
    start_time: Instant,
    /// Whether to ping-pong (reverse at end)
    ping_pong: bool,
}

impl GradientAnimation {
    /// Create a new gradient animation
    pub fn new(start: SolidColor, end: SolidColor, duration: Duration) -> Self {
        Self {
            start,
            end,
            duration,
            start_time: Instant::now(),
            ping_pong: true,
        }
    }

    /// Set ping-pong mode
    pub fn with_ping_pong(mut self, enabled: bool) -> Self {
        self.ping_pong = enabled;
        self
    }

    /// Reset animation
    pub fn reset(&mut self) {
        self.start_time = Instant::now();
    }

    /// Get current color
    pub fn current_color(&self) -> SolidColor {
        let elapsed = self.start_time.elapsed();
        let cycle_time = self.duration.as_secs_f32();

        let mut t = (elapsed.as_secs_f32() % (cycle_time * 2.0)) / cycle_time;

        if self.ping_pong && t > 1.0 {
            t = 2.0 - t;
        } else {
            t = t % 1.0;
        }

        self.start.lerp(&self.end, t)
    }

    /// Apply gradient to text
    pub fn apply(&self, text: &str) -> String {
        let color = self.current_color();
        format!("\x1b[38;2;{};{};{}m{}\x1b[0m", color.r, color.g, color.b, text)
    }
}

/// Pulse animation (fade in/out)
#[derive(Debug, Clone)]
pub struct PulseAnimation {
    /// Base color
    color: SolidColor,
    /// Minimum brightness (0.0 to 1.0)
    min_brightness: f32,
    /// Pulse duration
    duration: Duration,
    /// Animation start time
    start_time: Instant,
}

impl PulseAnimation {
    /// Create a new pulse animation
    pub fn new(color: SolidColor, duration: Duration) -> Self {
        Self {
            color,
            min_brightness: 0.3,
            duration,
            start_time: Instant::now(),
        }
    }

    /// Set minimum brightness
    pub fn with_min_brightness(mut self, brightness: f32) -> Self {
        self.min_brightness = brightness.clamp(0.0, 1.0);
        self
    }

    /// Reset animation
    pub fn reset(&mut self) {
        self.start_time = Instant::now();
    }

    /// Get current brightness multiplier
    pub fn current_brightness(&self) -> f32 {
        let elapsed = self.start_time.elapsed();
        let t = (elapsed.as_secs_f32() / self.duration.as_secs_f32()) % 1.0;

        // Smooth sine wave
        let sine = (t * std::f32::consts::PI * 2.0).sin();
        let normalized = (sine + 1.0) / 2.0; // 0.0 to 1.0

        self.min_brightness + normalized * (1.0 - self.min_brightness)
    }

    /// Get current color
    pub fn current_color(&self) -> SolidColor {
        let brightness = self.current_brightness();
        SolidColor {
            r: (self.color.r as f32 * brightness) as u8,
            g: (self.color.g as f32 * brightness) as u8,
            b: (self.color.b as f32 * brightness) as u8,
        }
    }

    /// Apply pulse to text
    pub fn apply(&self, text: &str) -> String {
        let color = self.current_color();
        format!("\x1b[38;2;{};{};{}m{}\x1b[0m", color.r, color.g, color.b, text)
    }
}

/// Typing animation effect
#[derive(Debug, Clone)]
pub struct TypeAnimation {
    /// Full text to animate
    text: String,
    /// Characters per second
    chars_per_second: f32,
    /// Animation start time
    start_time: Instant,
}

impl TypeAnimation {
    /// Create a new typing animation
    pub fn new(text: impl Into<String>, chars_per_second: f32) -> Self {
        Self {
            text: text.into(),
            chars_per_second,
            start_time: Instant::now(),
        }
    }

    /// Reset animation
    pub fn reset(&mut self) {
        self.start_time = Instant::now();
    }

    /// Get currently visible text
    pub fn current_text(&self) -> &str {
        let elapsed = self.start_time.elapsed().as_secs_f32();
        let char_count = (elapsed * self.chars_per_second) as usize;
        let char_count = char_count.min(self.text.len());

        // Find char boundary
        let mut end = 0;
        for (i, (idx, _)) in self.text.char_indices().enumerate() {
            if i >= char_count {
                break;
            }
            end = idx + self.text[idx..].chars().next().map(|c| c.len_utf8()).unwrap_or(0);
        }

        &self.text[..end]
    }

    /// Check if animation is complete
    pub fn is_complete(&self) -> bool {
        let elapsed = self.start_time.elapsed().as_secs_f32();
        let char_count = (elapsed * self.chars_per_second) as usize;
        char_count >= self.text.chars().count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rainbow_animation() {
        let anim = RainbowAnimation::new();
        let result = anim.apply("Hello");
        assert!(result.contains("\x1b[38;2;"));
        assert!(result.contains("H"));
    }

    #[test]
    fn test_rainbow_border() {
        let anim = RainbowAnimation::new();
        let border = anim.border(10, BorderStyle::Top);
        assert!(border.contains("╭"));
        assert!(border.contains("╮"));
    }

    #[test]
    fn test_gradient_animation() {
        let start = SolidColor::new(255, 0, 0);
        let end = SolidColor::new(0, 0, 255);
        let anim = GradientAnimation::new(start, end, Duration::from_secs(1));
        let color = anim.current_color();
        // Should be somewhere between red and blue
        assert!(color.r > 0 || color.b > 0);
    }

    #[test]
    fn test_pulse_animation() {
        let color = SolidColor::new(255, 255, 255);
        let anim = PulseAnimation::new(color, Duration::from_secs(1));
        let brightness = anim.current_brightness();
        assert!(brightness >= 0.3 && brightness <= 1.0);
    }

    #[test]
    fn test_type_animation() {
        let anim = TypeAnimation::new("Hello World", 100.0);
        std::thread::sleep(Duration::from_millis(50));
        let text = anim.current_text();
        assert!(!text.is_empty());
    }
}
