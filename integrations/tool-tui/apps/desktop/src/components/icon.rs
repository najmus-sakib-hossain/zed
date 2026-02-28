use crate::theme::Theme;
use gpui::{prelude::*, px, svg, IntoElement, SharedString};

/// Icon component for rendering SVG icons
pub struct Icon {
    name: String,
    size: gpui::Pixels,
    color: Option<gpui::Hsla>,
}

impl Icon {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            size: px(20.0),
            color: None,
        }
    }

    pub fn size(mut self, size: gpui::Pixels) -> Self {
        self.size = size;
        self
    }

    pub fn color(mut self, color: gpui::Hsla) -> Self {
        self.color = Some(color);
        self
    }

    pub fn render(self, theme: &Theme) -> impl IntoElement {
        let color = self.color.unwrap_or(theme.foreground);
        let path = SharedString::from(format!("icons/{}.svg", self.name));

        svg()
            .path(path)
            .size(self.size)
            .text_color(color)
            .flex_shrink_0()
            .into_any_element()
    }
}
