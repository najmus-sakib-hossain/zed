use crate::theme::Theme;
use gpui::{div, prelude::*, px, IntoElement};

// ─── Label ──────────────────────────────────────────────────────────────────
// A shadcn-ui style Label for form fields.
//
// Usage:
//   Label::new("Email").render(&theme)
//   Label::new("Password").required(true).render(&theme)

pub struct Label {
    text: String,
    required: bool,
    disabled: bool,
    description: Option<String>,
}

impl Label {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            required: false,
            disabled: false,
            description: None,
        }
    }

    #[allow(dead_code)]
    pub fn required(mut self, required: bool) -> Self {
        self.required = required;
        self
    }

    #[allow(dead_code)]
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    #[allow(dead_code)]
    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    pub fn render(self, theme: &Theme) -> impl IntoElement {
        let text_color = if self.disabled {
            theme.muted_foreground
        } else {
            theme.foreground
        };

        let mut label = div().flex().flex_col().gap(px(4.0)).child(
            div()
                .flex()
                .items_center()
                .gap(px(2.0))
                .child(
                    div()
                        .text_size(px(14.0))
                        .font_weight(gpui::FontWeight::MEDIUM)
                        .text_color(text_color)
                        .line_height(px(14.0))
                        .child(self.text),
                )
                .when(self.required, |this| {
                    this.child(div().text_color(theme.destructive).text_size(px(14.0)).child("*"))
                }),
        );

        if let Some(desc) = self.description {
            label = label
                .child(div().text_size(px(12.0)).text_color(theme.muted_foreground).child(desc));
        }

        label
    }
}

// ─── Kbd (Keyboard Shortcut) ────────────────────────────────────────────────
// Renders a keyboard shortcut badge, like ⌘K or Ctrl+P.
//
// Usage:
//   Kbd::new("⌘K").render(&theme)

pub struct Kbd {
    keys: String,
}

impl Kbd {
    pub fn new(keys: impl Into<String>) -> Self {
        Self { keys: keys.into() }
    }

    pub fn render(self, theme: &Theme) -> impl IntoElement {
        div()
            .flex()
            .items_center()
            .px(px(6.0))
            .py(px(2.0))
            .rounded(px(4.0))
            .bg(theme.muted)
            .border_1()
            .border_color(theme.border)
            .text_size(px(11.0))
            .font_weight(gpui::FontWeight::MEDIUM)
            .text_color(theme.muted_foreground)
            .child(self.keys)
    }
}
