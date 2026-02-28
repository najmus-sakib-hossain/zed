use crate::components::ui::misc::EmptyState;
use crate::theme::{colors::Radius, Theme};
use gpui::{div, prelude::*, px, svg, IntoElement, MouseButton, SharedString};

// ─── IconGrid ───────────────────────────────────────────────────────────────
// A responsive grid of icon cards with selection, hover, and click support.

/// A single icon to render in the grid
#[derive(Clone)]
#[allow(dead_code)]
pub struct IconGridItem {
    pub index: usize,
    pub name: String,
    pub pack: String,
    pub svg_body: String,
    pub width: f32,
    pub height: f32,
    pub selected: bool,
}

/// Icon grid component - renders a responsive flex-wrap grid of icon cards
pub struct IconGrid {
    items: Vec<IconGridItem>,
    cell_size: gpui::Pixels,
    icon_display_size: gpui::Pixels,
}

impl IconGrid {
    pub fn new(items: Vec<IconGridItem>) -> Self {
        Self {
            items,
            cell_size: px(120.0),
            icon_display_size: px(48.0),
        }
    }

    #[allow(dead_code)]
    pub fn cell_size(mut self, size: gpui::Pixels) -> Self {
        self.cell_size = size;
        self
    }

    #[allow(dead_code)]
    pub fn icon_size(mut self, size: gpui::Pixels) -> Self {
        self.icon_display_size = size;
        self
    }

    pub fn render(self, theme: &Theme) -> impl IntoElement {
        if self.items.is_empty() {
            return EmptyState::new("No icons found")
                .icon("🔍")
                .description("Try a different search term or filter")
                .render(theme)
                .into_any_element();
        }

        let mut grid = div()
            .flex()
            .flex_wrap()
            .gap(px(8.0))
            .p(px(16.0))
            .on_mouse_move(|_event, _window, _cx| {});

        for item in self.items {
            grid = grid
                .child(IconCell::new(item, self.cell_size, self.icon_display_size).render(theme));
        }

        grid.into_any_element()
    }
}

// ─── IconCell ───────────────────────────────────────────────────────────────

struct IconCell {
    item: IconGridItem,
    cell_size: gpui::Pixels,
    icon_size: gpui::Pixels,
}

impl IconCell {
    fn new(item: IconGridItem, cell_size: gpui::Pixels, icon_size: gpui::Pixels) -> Self {
        Self {
            item,
            cell_size,
            icon_size,
        }
    }

    fn render(self, theme: &Theme) -> impl IntoElement {
        let is_selected = self.item.selected;
        let border_col = if is_selected {
            theme.ring
        } else {
            theme.border
        };
        let bg = if is_selected {
            theme.accent
        } else {
            theme.card
        };
        let hover_bg = theme.accent;
        let icon_name = self.item.name.clone();
        let icon_pack = self.item.pack.clone();
        let cell_height = self.cell_size + px(20.0); // extra space for label

        div()
            .id("icon-cell")
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap(px(8.0))
            .w(self.cell_size)
            .h(cell_height)
            .rounded(Radius::MD)
            .bg(bg)
            .border_1()
            .border_color(border_col)
            .cursor_pointer()
            .hover(move |style| style.bg(hover_bg).border_color(theme.ring))
            .active(move |style| style.opacity(0.8))
            .on_mouse_down(MouseButton::Left, move |_event, _window, _cx| {
                println!("Icon clicked: {} ({})", icon_name, icon_pack);
            })
            // Icon
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_center()
                    .size(self.icon_size + px(16.0))
                    .child(self.render_icon_preview(theme)),
            )
            // Label
            .child(
                div()
                    .text_size(px(11.0))
                    .text_color(theme.muted_foreground)
                    .overflow_x_hidden()
                    .max_w(self.cell_size - px(8.0))
                    .text_center()
                    .child(truncate_name(&self.item.name, 14)),
            )
    }

    fn render_icon_preview(&self, theme: &Theme) -> impl IntoElement {
        let asset_path =
            SharedString::from(format!("icons/{}/{}.svg", self.item.pack, self.item.name));
        svg()
            .path(asset_path)
            .text_color(theme.foreground)
            .size(self.icon_size)
            .into_any_element()
    }
}

fn truncate_name(name: &str, max_len: usize) -> String {
    if name.len() <= max_len {
        name.to_string()
    } else {
        format!("{}…", &name[..max_len - 1])
    }
}
