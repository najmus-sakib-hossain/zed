use crate::theme::{colors::Radius, Theme};
use gpui::{div, prelude::*, px, IntoElement};

// ─── Select ─────────────────────────────────────────────────────────────────
// A shadcn-ui style Select dropdown (display component).
//
// Usage:
//   Select::new("theme")
//       .placeholder("Select theme...")
//       .options(vec!["Light", "Dark", "System"])
//       .value(Some("Dark".into()))
//       .render(&theme)

pub struct Select {
    #[allow(dead_code)]
    id: String,
    placeholder: String,
    options: Vec<SelectOption>,
    selected_value: Option<String>,
    disabled: bool,
    open: bool,
}

impl Select {
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            placeholder: "Select an option...".to_string(),
            options: Vec::new(),
            selected_value: None,
            disabled: false,
            open: false,
        }
    }

    pub fn placeholder(mut self, text: impl Into<String>) -> Self {
        self.placeholder = text.into();
        self
    }

    pub fn options(mut self, options: Vec<impl Into<String>>) -> Self {
        self.options = options
            .into_iter()
            .map(|o| {
                let s = o.into();
                SelectOption::new(s.clone(), s)
            })
            .collect();
        self
    }

    pub fn select_options(mut self, options: Vec<SelectOption>) -> Self {
        self.options = options;
        self
    }

    pub fn value(mut self, value: Option<String>) -> Self {
        self.selected_value = value;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    #[allow(dead_code)]
    pub fn open(mut self, open: bool) -> Self {
        self.open = open;
        self
    }

    pub fn render(self, theme: &Theme) -> impl IntoElement {
        let display_text = self
            .selected_value
            .as_ref()
            .and_then(|v| self.options.iter().find(|o| &o.value == v).map(|o| o.label.clone()))
            .unwrap_or_else(|| self.placeholder.clone());

        let text_color = if self.selected_value.is_some() {
            theme.foreground
        } else {
            theme.muted_foreground
        };
        let bg = if self.disabled {
            theme.muted
        } else {
            theme.background
        };

        let mut container = div().flex().flex_col().gap(px(2.0));

        // Trigger
        let mut trigger = div()
            .flex()
            .items_center()
            .justify_between()
            .gap(px(8.0))
            .w_full()
            .min_h(px(36.0))
            .px(px(12.0))
            .py(px(8.0))
            .rounded(Radius::DEFAULT)
            .bg(bg)
            .border_1()
            .border_color(theme.border)
            .child(div().flex_1().text_size(px(14.0)).text_color(text_color).child(display_text))
            .child(div().text_size(px(10.0)).text_color(theme.muted_foreground).child("▼"));

        if !self.disabled {
            trigger = trigger.cursor_pointer().hover(move |style| style.border_color(theme.ring));
        } else {
            trigger = trigger.opacity(0.5);
        }

        container = container.child(trigger);

        // Dropdown (only when open)
        if self.open {
            let mut dropdown = div()
                .absolute()
                .mt(px(4.0))
                .w_full()
                .rounded(Radius::DEFAULT)
                .bg(theme.popover)
                .border_1()
                .border_color(theme.border)
                .py(px(4.0))
                .overflow_y_hidden()
                .max_h(px(200.0));

            for option in self.options {
                let selected =
                    self.selected_value.as_ref().map(|v| v == &option.value).unwrap_or(false);
                dropdown = dropdown.child(option.render(theme, selected));
            }

            container = container.child(dropdown);
        }

        container
    }
}

// ─── SelectOption ───────────────────────────────────────────────────────────

pub struct SelectOption {
    pub value: String,
    pub label: String,
    pub disabled: bool,
}

impl SelectOption {
    pub fn new(value: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            label: label.into(),
            disabled: false,
        }
    }

    #[allow(dead_code)]
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn render(self, theme: &Theme, selected: bool) -> impl IntoElement {
        let bg = if selected {
            theme.accent
        } else {
            gpui::transparent_black()
        };
        let text_color = if self.disabled {
            theme.muted_foreground
        } else {
            theme.popover_foreground
        };

        let mut el = div()
            .flex()
            .items_center()
            .gap(px(8.0))
            .px(px(8.0))
            .py(px(6.0))
            .mx(px(4.0))
            .rounded(Radius::SM)
            .bg(bg);

        if !self.disabled {
            el = el.cursor_pointer().hover(move |style| style.bg(theme.accent));
        } else {
            el = el.opacity(0.5);
        }

        // Check mark for selected
        if selected {
            el = el.child(
                div().text_size(px(12.0)).text_color(theme.foreground).w(px(16.0)).child("✓"),
            );
        } else {
            el = el.child(div().w(px(16.0)));
        }

        el = el.child(div().text_size(px(14.0)).text_color(text_color).child(self.label));

        el
    }
}
