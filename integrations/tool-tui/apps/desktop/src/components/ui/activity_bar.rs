use gpui::{div, prelude::*, px, IntoElement};

use crate::theme::{colors::Radius, Theme};

// ─── ActivityBar ────────────────────────────────────────────────────────────
// Vertical icon strip (like VS Code's Activity Bar) for top-level navigation.
//
// Usage:
//   ActivityBar::new()
//       .item(ActivityBarItem::new("files", "📁").active(true))
//       .item(ActivityBarItem::new("search", "🔍"))
//       .item(ActivityBarItem::new("git", "🔀"))
//       .bottom_item(ActivityBarItem::new("settings", "⚙"))
//       .render(&theme)

pub struct ActivityBar {
    items: Vec<ActivityBarItem>,
    bottom_items: Vec<ActivityBarItem>,
    width: f32,
}

pub struct ActivityBarItem {
    id: String,
    icon: String,
    tooltip: Option<String>,
    active: bool,
    badge: Option<String>,
}

#[allow(dead_code)]
impl ActivityBarItem {
    pub fn new(id: impl Into<String>, icon: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            icon: icon.into(),
            tooltip: None,
            active: false,
            badge: None,
        }
    }

    pub fn tooltip(mut self, tooltip: impl Into<String>) -> Self {
        self.tooltip = Some(tooltip.into());
        self
    }

    pub fn active(mut self, active: bool) -> Self {
        self.active = active;
        self
    }

    pub fn badge(mut self, badge: impl Into<String>) -> Self {
        self.badge = Some(badge.into());
        self
    }
}

#[allow(dead_code)]
impl ActivityBar {
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            bottom_items: Vec::new(),
            width: 48.0,
        }
    }

    pub fn item(mut self, item: ActivityBarItem) -> Self {
        self.items.push(item);
        self
    }

    pub fn bottom_item(mut self, item: ActivityBarItem) -> Self {
        self.bottom_items.push(item);
        self
    }

    pub fn width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }

    pub fn render(self, theme: &Theme) -> impl IntoElement {
        let bar = div()
            .flex()
            .flex_col()
            .justify_between()
            .w(px(self.width))
            .h_full()
            .bg(theme.background)
            .border_r_1()
            .border_color(theme.border)
            .flex_shrink_0();

        // Top items
        let mut top = div().flex().flex_col().items_center().pt(px(4.0));
        for item in self.items {
            top = top.child(Self::render_item(&item, self.width, theme));
        }

        // Bottom items
        let mut bottom = div().flex().flex_col().items_center().pb(px(4.0));
        for item in self.bottom_items {
            bottom = bottom.child(Self::render_item(&item, self.width, theme));
        }

        bar.child(top).child(bottom)
    }

    fn render_item(item: &ActivityBarItem, bar_width: f32, theme: &Theme) -> impl IntoElement {
        let icon_size = bar_width - 12.0;
        let is_active = item.active;
        let active_border = theme.primary;
        let hover_bg = theme.accent;

        let fg = if is_active {
            theme.foreground
        } else {
            theme.muted_foreground
        };

        let mut el = div()
            .relative()
            .flex()
            .items_center()
            .justify_center()
            .w(px(bar_width))
            .h(px(icon_size))
            .text_color(fg)
            .text_size(px(20.0))
            .cursor_pointer()
            .hover(move |style| style.text_color(theme.foreground));

        // Active indicator (left border)
        if is_active {
            el = el.child(
                div()
                    .absolute()
                    .left_0()
                    .top(px(6.0))
                    .bottom(px(6.0))
                    .w(px(2.0))
                    .bg(active_border)
                    .rounded_r(Radius::SM),
            );
        }

        // Icon
        el = el.child(
            div()
                .flex()
                .items_center()
                .justify_center()
                .size(px(icon_size - 8.0))
                .rounded(Radius::MD)
                .hover(move |style| style.bg(hover_bg))
                .child(item.icon.clone()),
        );

        // Badge
        if let Some(ref badge) = item.badge {
            el = el.child(
                div()
                    .absolute()
                    .top(px(4.0))
                    .right(px(6.0))
                    .min_w(px(16.0))
                    .h(px(16.0))
                    .px(px(4.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(Radius::FULL)
                    .bg(theme.primary)
                    .text_color(theme.primary_foreground)
                    .text_size(px(9.0))
                    .font_weight(gpui::FontWeight::BOLD)
                    .child(badge.clone()),
            );
        }

        el
    }
}
