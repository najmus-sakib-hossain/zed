use crate::theme::{colors::Radius, Theme};
use gpui::{div, prelude::*, px, IntoElement};

// ─── Input ──────────────────────────────────────────────────────────────────
// A shadcn-ui style Input field (display-only, since GPUI text editing
// requires FocusHandle + KeyDown handling at the view level).
//
// Usage:
//   Input::new("email").placeholder("Enter your email").render(&theme)
//   Input::new("search").value("hello").icon_left("🔍").render(&theme)

pub struct Input {
    #[allow(dead_code)]
    id: String,
    value: String,
    placeholder: String,
    disabled: bool,
    icon_left: Option<String>,
    icon_right: Option<String>,
    is_focused: bool,
    has_error: bool,
}

impl Input {
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            value: String::new(),
            placeholder: String::new(),
            disabled: false,
            icon_left: None,
            icon_right: None,
            is_focused: false,
            has_error: false,
        }
    }

    pub fn value(mut self, value: impl Into<String>) -> Self {
        self.value = value.into();
        self
    }

    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn icon_left(mut self, icon: impl Into<String>) -> Self {
        self.icon_left = Some(icon.into());
        self
    }

    pub fn icon_right(mut self, icon: impl Into<String>) -> Self {
        self.icon_right = Some(icon.into());
        self
    }

    pub fn focused(mut self, focused: bool) -> Self {
        self.is_focused = focused;
        self
    }

    pub fn error(mut self, has_error: bool) -> Self {
        self.has_error = has_error;
        self
    }

    pub fn render(self, theme: &Theme) -> impl IntoElement {
        let border_color = if self.has_error {
            theme.destructive
        } else if self.is_focused {
            theme.ring
        } else {
            theme.border
        };

        let bg = if self.disabled {
            theme.muted
        } else {
            theme.background
        };
        let text_color = if self.value.is_empty() {
            theme.muted_foreground
        } else {
            theme.foreground
        };

        let display_text = if self.value.is_empty() {
            self.placeholder.clone()
        } else {
            self.value.clone()
        };

        let mut input = div()
            .flex()
            .items_center()
            .gap(px(8.0))
            .w_full()
            .min_h(px(36.0))
            .px(px(12.0))
            .py(px(8.0))
            .rounded(Radius::DEFAULT)
            .bg(bg)
            .border_1()
            .border_color(border_color)
            .text_size(px(14.0));

        if self.disabled {
            input = input.opacity(0.5);
        } else {
            input = input.cursor_text();
        }

        // Focus ring effect
        if self.is_focused {
            input = input.border_2().border_color(theme.ring);
        }

        // Left icon
        if let Some(icon) = self.icon_left {
            input =
                input.child(div().text_color(theme.muted_foreground).flex_shrink_0().child(icon));
        }

        // Text value
        input = input.child(div().flex_1().text_color(text_color).child(display_text));

        // Right icon
        if let Some(icon) = self.icon_right {
            input =
                input.child(div().text_color(theme.muted_foreground).flex_shrink_0().child(icon));
        }

        // Blinking cursor when focused
        if self.is_focused {
            input = input.child(div().w(px(2.0)).h(px(16.0)).bg(theme.foreground).flex_shrink_0());
        }

        input
    }
}

// ─── Textarea ───────────────────────────────────────────────────────────────
// A multi-line text area component.

pub struct Textarea {
    #[allow(dead_code)]
    id: String,
    value: String,
    placeholder: String,
    disabled: bool,
    rows: u32,
}

#[allow(dead_code)]
impl Textarea {
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            value: String::new(),
            placeholder: "Type here...".to_string(),
            disabled: false,
            rows: 3,
        }
    }

    pub fn value(mut self, value: impl Into<String>) -> Self {
        self.value = value.into();
        self
    }

    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn rows(mut self, rows: u32) -> Self {
        self.rows = rows;
        self
    }

    pub fn render(self, theme: &Theme) -> impl IntoElement {
        let bg = if self.disabled {
            theme.muted
        } else {
            theme.background
        };
        let text_color = if self.value.is_empty() {
            theme.muted_foreground
        } else {
            theme.foreground
        };
        let display_text = if self.value.is_empty() {
            self.placeholder.clone()
        } else {
            self.value.clone()
        };
        let min_height = px(self.rows as f32 * 22.0 + 16.0);

        let mut textarea = div()
            .w_full()
            .min_h(min_height)
            .px(px(12.0))
            .py(px(8.0))
            .rounded(Radius::DEFAULT)
            .bg(bg)
            .border_1()
            .border_color(theme.border)
            .text_size(px(14.0))
            .text_color(text_color);

        if self.disabled {
            textarea = textarea.opacity(0.5);
        } else {
            textarea = textarea.cursor_text().hover(move |style| style.border_color(theme.ring));
        }

        textarea.child(display_text)
    }
}

// ─── InputArea (backwards-compatible chat input) ────────────────────────────

pub struct InputArea {
    theme: Theme,
}

impl InputArea {
    pub fn new(theme: Theme) -> Self {
        Self { theme }
    }

    pub fn render(self) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .px(px(24.0))
            .py(px(16.0))
            .gap(px(8.0))
            .child(self.render_input_field())
            .child(self.render_bottom_bar())
    }

    fn render_input_field(&self) -> impl IntoElement {
        div()
            .flex()
            .items_center()
            .gap(px(8.0))
            .px(px(16.0))
            .py(px(12.0))
            .rounded(Radius::MD)
            .bg(self.theme.card)
            .border_1()
            .border_color(self.theme.border)
            .hover(move |style| style.border_color(self.theme.ring))
            .child(
                div()
                    .flex_1()
                    .text_size(px(14.0))
                    .text_color(self.theme.muted_foreground)
                    .child("Ask Codex anything, @ to add files, / for commands"),
            )
            .child(
                div()
                    .size(px(32.0))
                    .rounded(Radius::FULL)
                    .bg(self.theme.primary)
                    .flex()
                    .items_center()
                    .justify_center()
                    .cursor_pointer()
                    .hover(move |style| style.opacity(0.9))
                    .child(div().text_color(self.theme.primary_foreground).child("▶")),
            )
    }

    fn render_bottom_bar(&self) -> impl IntoElement {
        div()
            .flex()
            .items_center()
            .justify_between()
            .px(px(8.0))
            .child(
                div()
                    .flex()
                    .gap(px(8.0))
                    .text_size(px(11.0))
                    .text_color(self.theme.muted_foreground)
                    .child("GPT-5.2-Codex")
                    .child("High"),
            )
            .child(
                div()
                    .flex()
                    .gap(px(8.0))
                    .text_size(px(11.0))
                    .text_color(self.theme.muted_foreground)
                    .child("Local")
                    .child("Worktree")
                    .child("Cloud"),
            )
            .child(
                div()
                    .text_size(px(11.0))
                    .text_color(self.theme.muted_foreground)
                    .child("🔧 (neo)actual-redesign"),
            )
    }
}
