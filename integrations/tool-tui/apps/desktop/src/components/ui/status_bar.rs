use gpui::{div, prelude::*, px, AnyElement, IntoElement};

use crate::theme::{colors::Radius, Theme};

use super::helpers::with_alpha;

// ─── StatusBar ──────────────────────────────────────────────────────────────
// Desktop-native bottom status bar component.
// Common in IDEs, editors, and productivity desktop apps.
//
// Usage:
//   StatusBar::new()
//       .left(StatusBarItem::text("Ready"))
//       .right(StatusBarItem::text("UTF-8"))
//       .render(&theme)

pub struct StatusBar {
    left: Vec<AnyElement>,
    center: Vec<AnyElement>,
    right: Vec<AnyElement>,
    height: f32,
}

impl StatusBar {
    pub fn new() -> Self {
        Self {
            left: Vec::new(),
            center: Vec::new(),
            right: Vec::new(),
            height: 24.0,
        }
    }

    pub fn left(mut self, element: impl IntoElement) -> Self {
        self.left.push(element.into_any_element());
        self
    }

    pub fn center(mut self, element: impl IntoElement) -> Self {
        self.center.push(element.into_any_element());
        self
    }

    pub fn right(mut self, element: impl IntoElement) -> Self {
        self.right.push(element.into_any_element());
        self
    }

    pub fn height(mut self, height: f32) -> Self {
        self.height = height;
        self
    }

    pub fn render(self, theme: &Theme) -> impl IntoElement {
        let bar = div()
            .flex()
            .items_center()
            .justify_between()
            .w_full()
            .h(px(self.height))
            .px(px(8.0))
            .bg(theme.primary)
            .flex_shrink_0()
            .text_size(px(11.0))
            .text_color(theme.primary_foreground);

        // Left section
        let mut left = div().flex().items_center().gap(px(2.0)).flex_shrink_0();
        for el in self.left {
            left = left.child(el);
        }

        // Center section
        let mut center = div().flex().items_center().justify_center().gap(px(2.0)).flex_1();
        for el in self.center {
            center = center.child(el);
        }

        // Right section
        let mut right = div().flex().items_center().gap(px(2.0)).flex_shrink_0();
        for el in self.right {
            right = right.child(el);
        }

        bar.child(left).child(center).child(right)
    }
}

// ─── StatusBarItem ──────────────────────────────────────────────────────────
// Individual item within a status bar.

pub struct StatusBarItem {
    content: StatusBarContent,
    icon: Option<String>,
    clickable: bool,
}

enum StatusBarContent {
    Text(String),
    Custom(AnyElement),
}

#[allow(dead_code)]
impl StatusBarItem {
    pub fn text(text: impl Into<String>) -> Self {
        Self {
            content: StatusBarContent::Text(text.into()),
            icon: None,
            clickable: false,
        }
    }

    pub fn custom(element: impl IntoElement) -> Self {
        Self {
            content: StatusBarContent::Custom(element.into_any_element()),
            icon: None,
            clickable: false,
        }
    }

    pub fn icon(mut self, icon: impl Into<String>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    pub fn clickable(mut self, clickable: bool) -> Self {
        self.clickable = clickable;
        self
    }

    pub fn render(self, theme: &Theme) -> impl IntoElement {
        let hover_bg = with_alpha(theme.primary_foreground, 0.15);

        let mut el = div()
            .flex()
            .items_center()
            .gap(px(4.0))
            .h_full()
            .px(px(6.0))
            .rounded(Radius::SM);

        if self.clickable {
            el = el.cursor_pointer().hover(move |style| style.bg(hover_bg));
        }

        if let Some(icon) = self.icon {
            el = el.child(div().text_size(px(12.0)).child(icon));
        }

        match self.content {
            StatusBarContent::Text(text) => {
                el = el.child(text);
            }
            StatusBarContent::Custom(element) => {
                el = el.child(element);
            }
        }

        el
    }
}
