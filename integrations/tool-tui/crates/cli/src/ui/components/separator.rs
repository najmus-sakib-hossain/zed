//! Separator component
//!
//! Renders a horizontal separator line.
//!
//! # Example
//!
//! ```rust,ignore
//! use dx_cli::ui::components::separator::Separator;
//!
//! let separator = Separator::new();
//! ```

use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::Paragraph,
};

/// Horizontal separator line
pub struct Separator {
    style: Style,
    char: char,
}

impl Separator {
    /// Create a new separator with default style
    pub fn new() -> Self {
        Self {
            style: Style::default().fg(Color::DarkGray),
            char: '─',
        }
    }

    /// Set the style
    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    /// Set the separator character
    pub fn char(mut self, c: char) -> Self {
        self.char = c;
        self
    }

    /// Render the separator
    pub fn render(&self, f: &mut Frame, area: Rect) {
        let line =
            Line::from(Span::styled(self.char.to_string().repeat(area.width as usize), self.style));
        let paragraph = Paragraph::new(vec![line]);
        f.render_widget(paragraph, area);
    }
}

impl Default for Separator {
    fn default() -> Self {
        Self::new()
    }
}
