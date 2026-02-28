use gpui::{div, prelude::*, px, IntoElement};

use crate::theme::{colors::Radius, Theme};

// ─── Toggle ─────────────────────────────────────────────────────────────────
// A two-state toggle button (pressed / not-pressed), common in desktop
// toolbars for bold, italic, sidebar visibility, etc.
//
// Usage:
//   Toggle::new("bold", "B")
//       .pressed(true)
//       .render(&theme)

pub struct Toggle {
    id: String,
    label: String,
    icon: Option<String>,
    pressed: bool,
    disabled: bool,
    size: ToggleSize,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ToggleSize {
    Sm,
    Default,
    Lg,
}

#[allow(dead_code)]
impl Toggle {
    pub fn new(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            icon: None,
            pressed: false,
            disabled: false,
            size: ToggleSize::Default,
        }
    }

    pub fn icon(mut self, icon: impl Into<String>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    pub fn pressed(mut self, pressed: bool) -> Self {
        self.pressed = pressed;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn size(mut self, size: ToggleSize) -> Self {
        self.size = size;
        self
    }

    pub fn render(self, theme: &Theme) -> impl IntoElement {
        let (h, pad_x) = match self.size {
            ToggleSize::Sm => (px(28.0), px(8.0)),
            ToggleSize::Default => (px(34.0), px(10.0)),
            ToggleSize::Lg => (px(40.0), px(14.0)),
        };

        let (bg, fg) = if self.pressed {
            (theme.accent, theme.accent_foreground)
        } else {
            (gpui::transparent_black(), theme.muted_foreground)
        };

        let hover_bg = if self.pressed {
            theme.accent
        } else {
            theme.ghost_hover
        };

        let mut el = div()
            .id(gpui::SharedString::from(self.id))
            .flex()
            .items_center()
            .justify_center()
            .h(h)
            .px(pad_x)
            .rounded(Radius::MD)
            .bg(bg)
            .text_color(fg)
            .text_sm()
            .font_weight(gpui::FontWeight::MEDIUM)
            .cursor_pointer()
            .hover(move |s| s.bg(hover_bg));

        if let Some(icon) = &self.icon {
            el = el.child(
                div()
                    .mr(if self.label.is_empty() {
                        px(0.0)
                    } else {
                        px(6.0)
                    })
                    .child(icon.clone()),
            );
        }

        if !self.label.is_empty() {
            el = el.child(self.label);
        }

        if self.disabled {
            el = el.opacity(0.5).cursor_default();
        }

        el
    }
}

// ─── ToggleRow ──────────────────────────────────────────────────────────────
// A row of toggles in a connected group (like a segmented control).
//
// Usage:
//   ToggleRow::new()
//       .toggle(Toggle::new("bold", "B").pressed(true))
//       .toggle(Toggle::new("italic", "I"))
//       .render(&theme)

pub struct ToggleRow {
    children: Vec<Toggle>,
}

#[allow(dead_code)]
impl ToggleRow {
    pub fn new() -> Self {
        Self {
            children: Vec::new(),
        }
    }

    pub fn toggle(mut self, toggle: Toggle) -> Self {
        self.children.push(toggle);
        self
    }

    pub fn render(self, theme: &Theme) -> impl IntoElement {
        let mut row = div()
            .flex()
            .items_center()
            .rounded(Radius::MD)
            .border_1()
            .border_color(theme.border)
            .overflow_hidden();

        for t in self.children {
            row = row.child(t.render(theme));
        }

        row
    }
}
