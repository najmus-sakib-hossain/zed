//! Scrollable area component
//!
//! Provides a simple scrollable region with optional scrollbar.
//!
//! # Example
//!
//! ```rust,ignore
//! use ratatui::text::Line;
//! use dx_cli::ui::components::scroll::ScrollArea;
//!
//! let content = vec![Line::from("Line 1"), Line::from("Line 2")];
//! let scroll = ScrollArea::new(content).show_scrollbar(true);
//! ```

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

/// Scrollable content area with optional scrollbar
pub struct ScrollArea {
    content: Vec<Line<'static>>,
    scroll_position: usize,
    show_scrollbar: bool,
    scrollbar_width: u16,
    title: Option<String>,
}

impl ScrollArea {
    /// Create a new scroll area with content
    pub fn new(content: Vec<Line<'static>>) -> Self {
        Self {
            content,
            scroll_position: 0,
            show_scrollbar: true,
            scrollbar_width: 3,
            title: None,
        }
    }

    /// Set initial scroll position
    pub fn scroll_position(mut self, position: usize) -> Self {
        self.scroll_position = position;
        self
    }

    /// Toggle scrollbar visibility
    pub fn show_scrollbar(mut self, show: bool) -> Self {
        self.show_scrollbar = show;
        self
    }

    /// Set scrollbar width
    pub fn scrollbar_width(mut self, width: u16) -> Self {
        self.scrollbar_width = width;
        self
    }

    /// Set optional title
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Scroll up by one line
    pub fn scroll_up(&mut self) {
        self.scroll_position = self.scroll_position.saturating_sub(1);
    }

    /// Scroll down by one line
    pub fn scroll_down(&mut self, visible_height: usize) {
        let max_scroll = self.content.len().saturating_sub(visible_height);
        self.scroll_position = (self.scroll_position + 1).min(max_scroll);
    }

    /// Render the scroll area
    pub fn render(&self, f: &mut Frame, area: Rect) {
        let (content_area, scrollbar_area) = if self.show_scrollbar {
            let chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Min(0), Constraint::Length(self.scrollbar_width)])
                .split(area);
            (chunks[0], Some(chunks[1]))
        } else {
            (area, None)
        };

        // Calculate visible content
        let content_height = (content_area.height as usize).saturating_sub(2); // Account for borders
        let start_line = self.scroll_position.min(self.content.len());
        let end_line = (start_line + content_height).min(self.content.len());
        let visible_content: Vec<Line> = self.content[start_line..end_line].to_vec();

        // Render content
        let mut block = Block::default().borders(Borders::ALL);
        if let Some(title) = &self.title {
            block = block.title(title.as_str());
        }

        let paragraph = Paragraph::new(visible_content).block(block);
        f.render_widget(paragraph, content_area);

        // Render scrollbar
        if let Some(scrollbar_rect) = scrollbar_area {
            let scrollbar_height = scrollbar_rect.height.saturating_sub(2);
            let total_content = self.content.len() as f32;
            let visible_ratio = content_height as f32 / total_content;
            let thumb_height = (scrollbar_height as f32 * visible_ratio).max(1.0) as u16;
            let scroll_ratio =
                self.scroll_position as f32 / (total_content - content_height as f32).max(1.0);
            let thumb_position = (scroll_ratio * (scrollbar_height - thumb_height) as f32) as u16;

            let mut scrollbar_lines = Vec::new();
            for i in 0..scrollbar_height {
                if i >= thumb_position && i < thumb_position + thumb_height {
                    scrollbar_lines
                        .push(Line::from(Span::styled("█", Style::default().fg(Color::Cyan))));
                } else {
                    scrollbar_lines
                        .push(Line::from(Span::styled("│", Style::default().fg(Color::DarkGray))));
                }
            }

            let scrollbar =
                Paragraph::new(scrollbar_lines).block(Block::default().borders(Borders::ALL));
            f.render_widget(scrollbar, scrollbar_rect);
        }
    }
}
