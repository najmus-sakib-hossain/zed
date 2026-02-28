//! Minimap Preview Component
//!
//! Displays a compact overview of the entire file with:
//! - Syntax highlighting (simplified)
//! - Current viewport indicator
//! - Clickable navigation
//!
//! # Example
//!
//! ```rust,ignore
//! use dx_cli::ui::editor::{Minimap, MinimapConfig};
//!
//! let minimap = Minimap::new()
//!     .width(15)
//!     .show_viewport(true);
//! ```

use std::any::Any;

use syntect::highlighting::{Color, Style, Theme, ThemeSet};
use syntect::parsing::{SyntaxReference, SyntaxSet};

use crate::ui::components::traits::{
    Bounds, ComponentResult, DxComponent, KeyEvent, MouseEvent, RenderContext,
};

/// Configuration for minimap display
#[derive(Debug, Clone)]
pub struct MinimapConfig {
    /// Width in characters (default: 15)
    pub width: u16,
    /// Show viewport indicator (default: true)
    pub show_viewport: bool,
    /// Characters per line in minimap (default: 1)
    pub chars_per_line: usize,
    /// Lines to compress into one minimap line (default: 4)
    pub lines_per_row: usize,
    /// Show syntax highlighting (default: true)
    pub show_syntax: bool,
}

impl Default for MinimapConfig {
    fn default() -> Self {
        Self {
            width: 15,
            show_viewport: true,
            chars_per_line: 1,
            lines_per_row: 4,
            show_syntax: true,
        }
    }
}

/// Minimap component showing file overview
pub struct Minimap {
    /// File content (lines)
    lines: Vec<String>,
    /// Current viewport start line
    viewport_start: usize,
    /// Current viewport end line
    viewport_end: usize,
    /// Total visible lines in main editor
    editor_visible_lines: usize,
    /// Syntax set for highlighting
    syntax_set: SyntaxSet,
    /// Theme set
    theme_set: ThemeSet,
    /// Current syntax
    current_syntax: Option<usize>,
    /// Configuration
    config: MinimapConfig,
    /// Component bounds
    bounds: Bounds,
    /// Whether focused
    focused: bool,
    /// Theme name
    theme_name: String,
}

impl Minimap {
    /// Create a new minimap
    pub fn new() -> Self {
        Self {
            lines: Vec::new(),
            viewport_start: 0,
            viewport_end: 0,
            editor_visible_lines: 24,
            syntax_set: SyntaxSet::load_defaults_newlines(),
            theme_set: ThemeSet::load_defaults(),
            current_syntax: None,
            config: MinimapConfig::default(),
            bounds: Bounds::default(),
            focused: false,
            theme_name: "base16-ocean.dark".to_string(),
        }
    }

    /// Set minimap width
    pub fn width(mut self, width: u16) -> Self {
        self.config.width = width;
        self
    }

    /// Set whether to show viewport indicator
    pub fn show_viewport(mut self, show: bool) -> Self {
        self.config.show_viewport = show;
        self
    }

    /// Set configuration
    pub fn config(mut self, config: MinimapConfig) -> Self {
        self.config = config;
        self
    }

    /// Set theme name
    pub fn theme(mut self, name: impl Into<String>) -> Self {
        self.theme_name = name.into();
        self
    }

    /// Update content from viewer
    pub fn update_content(
        &mut self,
        lines: &[String],
        viewport_start: usize,
        viewport_end: usize,
        syntax_idx: Option<usize>,
    ) {
        self.lines = lines.to_vec();
        self.viewport_start = viewport_start;
        self.viewport_end = viewport_end;
        self.current_syntax = syntax_idx;
        self.editor_visible_lines = viewport_end.saturating_sub(viewport_start);
    }

    /// Get the current theme
    fn get_theme(&self) -> &Theme {
        self.theme_set
            .themes
            .get(&self.theme_name)
            .unwrap_or_else(|| &self.theme_set.themes["base16-ocean.dark"])
    }

    /// Get the current syntax reference
    fn get_syntax(&self) -> &SyntaxReference {
        self.current_syntax
            .and_then(|i| self.syntax_set.syntaxes().get(i))
            .unwrap_or_else(|| self.syntax_set.find_syntax_plain_text())
    }

    /// Compress multiple lines into a single character representation
    fn compress_lines(&self, start_line: usize) -> char {
        let end_line = (start_line + self.config.lines_per_row).min(self.lines.len());
        let mut char_count = 0;
        let mut total_chars = 0;

        for i in start_line..end_line {
            if let Some(line) = self.lines.get(i) {
                let trimmed = line.trim();
                if !trimmed.is_empty() {
                    char_count += trimmed.len();
                    total_chars += 1;
                }
            }
        }

        // Represent density with different characters
        if total_chars == 0 {
            ' '
        } else {
            let avg_density = char_count / total_chars.max(1);
            match avg_density {
                0..=10 => '░',
                11..=30 => '▒',
                31..=60 => '▓',
                _ => '█',
            }
        }
    }

    /// Get color for a line based on syntax highlighting
    fn get_line_color(&self, line_idx: usize) -> Option<Color> {
        if !self.config.show_syntax {
            return None;
        }

        let line = self.lines.get(line_idx)?;
        let theme = self.get_theme();
        let syntax = self.get_syntax();

        // Simple heuristic: check first non-whitespace token
        let trimmed = line.trim();
        if trimmed.is_empty() {
            return None;
        }

        // Use syntect to get highlighting for first token
        use syntect::easy::HighlightLines;
        let mut highlighter = HighlightLines::new(syntax, theme);
        
        if let Ok(ranges) = highlighter.highlight_line(line, &self.syntax_set) {
            if let Some((style, _)) = ranges.first() {
                return Some(style.foreground);
            }
        }

        None
    }

    /// Convert syntect color to ANSI color code
    fn color_to_ansi(&self, color: Color) -> String {
        format!("\x1b[38;2;{};{};{}m", color.r, color.g, color.b)
    }

    /// Render the minimap
    fn render_internal(&self, ctx: &RenderContext<'_>) -> Vec<String> {
        let mut output = Vec::new();
        let height = ctx.bounds.height as usize;
        let width = self.config.width as usize;

        if self.lines.is_empty() {
            return vec![" ".repeat(width); height];
        }

        let total_lines = self.lines.len();
        let lines_per_minimap_row = (total_lines as f32 / height as f32).ceil() as usize;
        let lines_per_minimap_row = lines_per_minimap_row.max(1);

        for row in 0..height {
            let start_line = row * lines_per_minimap_row;
            let end_line = ((row + 1) * lines_per_minimap_row).min(total_lines);

            // Check if this row is in viewport
            let in_viewport = self.config.show_viewport
                && start_line <= self.viewport_end
                && end_line >= self.viewport_start;

            let mut line_str = String::new();

            // Viewport indicator (left border)
            if in_viewport {
                line_str.push('│');
            } else {
                line_str.push(' ');
            }

            // Content representation
            for _ in 0..width.saturating_sub(2) {
                let ch = self.compress_lines(start_line);
                
                // Apply color if syntax highlighting is enabled
                if self.config.show_syntax {
                    if let Some(color) = self.get_line_color(start_line) {
                        line_str.push_str(&self.color_to_ansi(color));
                        line_str.push(ch);
                        line_str.push_str("\x1b[0m"); // Reset
                    } else {
                        line_str.push(ch);
                    }
                } else {
                    line_str.push(ch);
                }
            }

            // Viewport indicator (right border)
            if in_viewport {
                line_str.push('│');
            } else {
                line_str.push(' ');
            }

            // Pad to width
            while line_str.len() < width {
                line_str.push(' ');
            }

            output.push(line_str);
        }

        output
    }

    /// Handle click on minimap to jump to location
    fn handle_click(&self, y: u16) -> Option<usize> {
        let height = self.bounds.height as usize;
        let total_lines = self.lines.len();
        
        if total_lines == 0 || height == 0 {
            return None;
        }

        let relative_y = y.saturating_sub(self.bounds.y) as usize;
        let lines_per_row = (total_lines as f32 / height as f32).ceil() as usize;
        let target_line = relative_y * lines_per_row.max(1);

        Some(target_line.min(total_lines.saturating_sub(1)))
    }
}

impl Default for Minimap {
    fn default() -> Self {
        Self::new()
    }
}

impl DxComponent for Minimap {
    fn handle_key(&mut self, _key: KeyEvent) -> ComponentResult {
        // Minimap doesn't handle keyboard input directly
        ComponentResult::Ignored
    }

    fn handle_mouse(&mut self, event: MouseEvent) -> ComponentResult {
        match event {
            MouseEvent::Click { x, y } => {
                // Check if click is within bounds
                if x >= self.bounds.x
                    && x < self.bounds.x + self.bounds.width
                    && y >= self.bounds.y
                    && y < self.bounds.y + self.bounds.height
                {
                    if let Some(target_line) = self.handle_click(y) {
                        return ComponentResult::ActionWithData(
                            "jump_to_line".to_string(),
                            target_line.to_string(),
                        );
                    }
                }
                ComponentResult::Ignored
            }
            _ => ComponentResult::Ignored,
        }
    }

    fn render(&self, ctx: &RenderContext<'_>) -> Vec<String> {
        self.render_internal(ctx)
    }

    fn is_focusable(&self) -> bool {
        false // Minimap is not directly focusable
    }

    fn set_focused(&mut self, focused: bool) {
        self.focused = focused;
    }

    fn is_focused(&self) -> bool {
        self.focused
    }

    fn id(&self) -> Option<&str> {
        Some("minimap")
    }

    fn bounds(&self) -> Bounds {
        self.bounds
    }

    fn set_bounds(&mut self, bounds: Bounds) {
        self.bounds = bounds;
    }

    fn min_size(&self) -> (u16, u16) {
        (10, 10)
    }

    fn preferred_size(&self) -> (u16, u16) {
        (self.config.width, 24)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_minimap_creation() {
        let minimap = Minimap::new();
        assert_eq!(minimap.config.width, 15);
        assert!(minimap.config.show_viewport);
    }

    #[test]
    fn test_minimap_config() {
        let minimap = Minimap::new().width(20).show_viewport(false);
        assert_eq!(minimap.config.width, 20);
        assert!(!minimap.config.show_viewport);
    }

    #[test]
    fn test_compress_lines() {
        let mut minimap = Minimap::new();
        let lines = vec![
            "".to_string(),
            "short".to_string(),
            "this is a much longer line with more content".to_string(),
            "".to_string(),
        ];
        minimap.update_content(&lines, 0, 10, None);

        let ch = minimap.compress_lines(0);
        assert!(ch != ' '); // Should have some content
    }

    #[test]
    fn test_handle_click() {
        let mut minimap = Minimap::new();
        minimap.set_bounds(Bounds::new(0, 0, 15, 24));
        
        let lines: Vec<String> = (0..100).map(|i| format!("line {}", i)).collect();
        minimap.update_content(&lines, 0, 24, None);

        // Click in middle should jump to middle of file
        let target = minimap.handle_click(12);
        assert!(target.is_some());
        let line = target.unwrap();
        assert!(line > 40 && line < 60); // Roughly middle
    }
}
