use gpui::{div, prelude::*, px, AnyElement, IntoElement};

use crate::theme::{colors::Radius, Theme};

// ─── Toolbar ────────────────────────────────────────────────────────────────
// Desktop-native toolbar component for top-level actions.
// Typically placed below the titlebar in desktop apps.
//
// Usage:
//   Toolbar::new()
//       .left(div().child("File"))
//       .center(div().child("Search"))
//       .right(div().child("Settings"))
//       .render(&theme)

pub struct Toolbar {
    left: Vec<AnyElement>,
    center: Vec<AnyElement>,
    right: Vec<AnyElement>,
    bordered: bool,
    height: f32,
}

impl Toolbar {
    pub fn new() -> Self {
        Self {
            left: Vec::new(),
            center: Vec::new(),
            right: Vec::new(),
            bordered: true,
            height: 40.0,
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

    pub fn bordered(mut self, bordered: bool) -> Self {
        self.bordered = bordered;
        self
    }

    pub fn height(mut self, height: f32) -> Self {
        self.height = height;
        self
    }

    pub fn render(self, theme: &Theme) -> impl IntoElement {
        let mut bar = div()
            .flex()
            .items_center()
            .justify_between()
            .w_full()
            .h(px(self.height))
            .px(px(12.0))
            .bg(theme.background)
            .flex_shrink_0();

        if self.bordered {
            bar = bar.border_b_1().border_color(theme.border);
        }

        // Left section
        let mut left = div().flex().items_center().gap(px(4.0)).flex_shrink_0();
        for el in self.left {
            left = left.child(el);
        }

        // Center section (takes remaining space)
        let mut center =
            div().flex().items_center().justify_center().gap(px(4.0)).flex_1().mx(px(8.0));
        for el in self.center {
            center = center.child(el);
        }

        // Right section
        let mut right = div().flex().items_center().gap(px(4.0)).flex_shrink_0();
        for el in self.right {
            right = right.child(el);
        }

        bar.child(left).child(center).child(right)
    }
}

// ─── ToolbarButton ──────────────────────────────────────────────────────────
// Compact button designed for toolbar usage.

pub struct ToolbarButton {
    label: String,
    icon: Option<String>,
    active: bool,
    disabled: bool,
}

#[allow(dead_code)]
impl ToolbarButton {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            icon: None,
            active: false,
            disabled: false,
        }
    }

    pub fn icon_only(icon: impl Into<String>) -> Self {
        Self {
            label: String::new(),
            icon: Some(icon.into()),
            active: false,
            disabled: false,
        }
    }

    pub fn with_icon(mut self, icon: impl Into<String>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    pub fn active(mut self, active: bool) -> Self {
        self.active = active;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn render(self, theme: &Theme) -> impl IntoElement {
        let bg = if self.active {
            theme.accent
        } else {
            gpui::transparent_black()
        };
        let fg = if self.active {
            theme.accent_foreground
        } else {
            theme.muted_foreground
        };
        let hover_bg = theme.accent;

        let mut el = div()
            .flex()
            .items_center()
            .justify_center()
            .gap(px(4.0))
            .h(px(28.0))
            .px(px(8.0))
            .rounded(Radius::SM)
            .bg(bg)
            .text_color(fg)
            .text_size(px(12.0));

        if self.disabled {
            el = el.opacity(0.5);
        } else {
            el = el.cursor_pointer().hover(move |style| style.bg(hover_bg));
        }

        if let Some(icon) = self.icon {
            el = el.child(div().child(icon));
        }
        if !self.label.is_empty() {
            el = el.child(self.label);
        }

        el
    }
}

// ─── ToolbarSeparator ───────────────────────────────────────────────────────

pub struct ToolbarSeparator;

impl ToolbarSeparator {
    pub fn render(theme: &Theme) -> impl IntoElement {
        div().w(px(1.0)).h(px(20.0)).mx(px(4.0)).bg(theme.border).flex_shrink_0()
    }
}

// ─── ToolbarGroup ───────────────────────────────────────────────────────────
// Groups toolbar buttons with a subtle background.

pub struct ToolbarGroup {
    children: Vec<AnyElement>,
}

#[allow(dead_code)]
impl ToolbarGroup {
    pub fn new() -> Self {
        Self {
            children: Vec::new(),
        }
    }

    pub fn child(mut self, element: impl IntoElement) -> Self {
        self.children.push(element.into_any_element());
        self
    }

    pub fn render(self, theme: &Theme) -> impl IntoElement {
        let mut group = div()
            .flex()
            .items_center()
            .gap(px(1.0))
            .rounded(Radius::DEFAULT)
            .bg(theme.muted)
            .p(px(2.0));

        for child in self.children {
            group = group.child(child);
        }

        group
    }
}
