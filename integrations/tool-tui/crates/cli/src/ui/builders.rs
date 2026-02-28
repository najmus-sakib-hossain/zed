use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Borders},
};

/// Builder for creating layouts with fluent API
pub struct LayoutBuilder {
    direction: Direction,
    constraints: Vec<Constraint>,
}

impl LayoutBuilder {
    pub fn new() -> Self {
        Self {
            direction: Direction::Vertical,
            constraints: Vec::new(),
        }
    }

    pub fn vertical() -> Self {
        Self::new()
    }

    pub fn horizontal() -> Self {
        Self {
            direction: Direction::Horizontal,
            constraints: Vec::new(),
        }
    }

    pub fn direction(mut self, direction: Direction) -> Self {
        self.direction = direction;
        self
    }

    pub fn constraint(mut self, constraint: Constraint) -> Self {
        self.constraints.push(constraint);
        self
    }

    pub fn constraints(mut self, constraints: Vec<Constraint>) -> Self {
        self.constraints = constraints;
        self
    }

    pub fn min(self, size: u16) -> Self {
        self.constraint(Constraint::Min(size))
    }

    pub fn max(self, size: u16) -> Self {
        self.constraint(Constraint::Max(size))
    }

    pub fn length(self, size: u16) -> Self {
        self.constraint(Constraint::Length(size))
    }

    pub fn percentage(self, pct: u16) -> Self {
        self.constraint(Constraint::Percentage(pct))
    }

    pub fn split(self, area: Rect) -> Vec<Rect> {
        Layout::default()
            .direction(self.direction)
            .constraints(self.constraints)
            .split(area)
            .to_vec()
    }
}

impl Default for LayoutBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for creating blocks with fluent API
pub struct BlockBuilder {
    borders: Borders,
    title: Option<String>,
    border_style: Option<Style>,
    style: Option<Style>,
}

impl BlockBuilder {
    pub fn new() -> Self {
        Self {
            borders: Borders::NONE,
            title: None,
            border_style: None,
            style: None,
        }
    }

    pub fn borders(mut self, borders: Borders) -> Self {
        self.borders = borders;
        self
    }

    pub fn all_borders(mut self) -> Self {
        self.borders = Borders::ALL;
        self
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn border_style(mut self, style: Style) -> Self {
        self.border_style = Some(style);
        self
    }

    pub fn border_color(mut self, color: Color) -> Self {
        self.border_style = Some(Style::default().fg(color));
        self
    }

    pub fn style(mut self, style: Style) -> Self {
        self.style = Some(style);
        self
    }

    pub fn build(self) -> Block<'static> {
        let mut block = Block::default().borders(self.borders);

        if let Some(title) = self.title {
            block = block.title(title);
        }

        if let Some(style) = self.border_style {
            block = block.border_style(style);
        }

        if let Some(style) = self.style {
            block = block.style(style);
        }

        block
    }
}

impl Default for BlockBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_layout_builder_vertical() {
        let builder = LayoutBuilder::vertical().min(10).length(5);
        assert_eq!(builder.direction, Direction::Vertical);
        assert_eq!(builder.constraints.len(), 2);
    }

    #[test]
    fn test_layout_builder_horizontal() {
        let builder = LayoutBuilder::horizontal().percentage(50).percentage(50);
        assert_eq!(builder.direction, Direction::Horizontal);
        assert_eq!(builder.constraints.len(), 2);
    }

    #[test]
    fn test_block_builder() {
        let block = BlockBuilder::new()
            .all_borders()
            .title("Test")
            .border_color(Color::Cyan)
            .build();

        // Just verify it builds without panicking
        let _ = block;
    }
}
