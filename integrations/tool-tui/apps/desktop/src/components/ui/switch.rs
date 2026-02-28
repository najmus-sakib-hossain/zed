use crate::theme::{colors::Radius, Theme};
use gpui::{div, prelude::*, px, IntoElement};

// ─── Switch ─────────────────────────────────────────────────────────────────
// A shadcn-ui style toggle Switch.
//
// Usage:
//   Switch::new("notifications").checked(true).render(&theme)

pub struct Switch {
    #[allow(dead_code)]
    id: String,
    checked: bool,
    disabled: bool,
    label: Option<String>,
}

impl Switch {
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            checked: false,
            disabled: false,
            label: None,
        }
    }

    pub fn checked(mut self, checked: bool) -> Self {
        self.checked = checked;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    #[allow(dead_code)]
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn render(self, theme: &Theme) -> impl IntoElement {
        let track_bg = if self.checked {
            theme.primary
        } else {
            theme.input
        };

        let thumb_offset = if self.checked { px(18.0) } else { px(2.0) };

        let mut container = div().flex().items_center().gap(px(8.0));

        // Track
        let mut track = div()
            .relative()
            .w(px(40.0))
            .h(px(22.0))
            .rounded(Radius::FULL)
            .bg(track_bg)
            .flex_shrink_0();

        // Thumb
        let thumb = div()
            .absolute()
            .top(px(2.0))
            .left(thumb_offset)
            .size(px(18.0))
            .rounded(Radius::FULL)
            .bg(theme.background);

        track = track.child(thumb);

        if !self.disabled {
            track = track.cursor_pointer();
        } else {
            track = track.opacity(0.5);
        }

        container = container.child(track);

        if let Some(label) = self.label {
            container = container.child(
                div()
                    .text_size(px(14.0))
                    .text_color(if self.disabled {
                        theme.muted_foreground
                    } else {
                        theme.foreground
                    })
                    .child(label),
            );
        }

        container
    }
}

// ─── Checkbox ───────────────────────────────────────────────────────────────
// A shadcn-ui style Checkbox.
//
// Usage:
//   Checkbox::new("terms").checked(true).label("Accept terms").render(&theme)

pub struct Checkbox {
    #[allow(dead_code)]
    id: String,
    checked: bool,
    disabled: bool,
    label: Option<String>,
    indeterminate: bool,
}

impl Checkbox {
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            checked: false,
            disabled: false,
            label: None,
            indeterminate: false,
        }
    }

    pub fn checked(mut self, checked: bool) -> Self {
        self.checked = checked;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    #[allow(dead_code)]
    pub fn indeterminate(mut self, value: bool) -> Self {
        self.indeterminate = value;
        self
    }

    pub fn render(self, theme: &Theme) -> impl IntoElement {
        let bg = if self.checked || self.indeterminate {
            theme.primary
        } else {
            gpui::transparent_black()
        };
        let border = if self.checked || self.indeterminate {
            theme.primary
        } else {
            theme.border
        };
        let check_color = theme.primary_foreground;

        let icon = if self.indeterminate {
            "−"
        } else if self.checked {
            "✓"
        } else {
            ""
        };

        let mut container = div().flex().items_center().gap(px(8.0));

        // Box
        let mut checkbox = div()
            .flex()
            .items_center()
            .justify_center()
            .size(px(16.0))
            .rounded(Radius::SM)
            .bg(bg)
            .border_1()
            .border_color(border)
            .flex_shrink_0();

        if !icon.is_empty() {
            checkbox = checkbox.child(
                div()
                    .text_size(px(10.0))
                    .font_weight(gpui::FontWeight::BOLD)
                    .text_color(check_color)
                    .child(icon),
            );
        }

        if !self.disabled {
            checkbox = checkbox.cursor_pointer();
        } else {
            checkbox = checkbox.opacity(0.5);
        }

        container = container.child(checkbox);

        if let Some(label) = self.label {
            container = container.child(
                div()
                    .text_size(px(14.0))
                    .text_color(if self.disabled {
                        theme.muted_foreground
                    } else {
                        theme.foreground
                    })
                    .when(!self.disabled, |this| this.cursor_pointer())
                    .child(label),
            );
        }

        container
    }
}

// ─── RadioGroup ─────────────────────────────────────────────────────────────
// A shadcn-ui style RadioGroup.
//
// Usage:
//   RadioGroup::new("size")
//       .option("sm", "Small")
//       .option("md", "Medium")
//       .option("lg", "Large")
//       .value("md")
//       .render(&theme)

pub struct RadioGroup {
    #[allow(dead_code)]
    id: String,
    options: Vec<(String, String)>,
    value: Option<String>,
    disabled: bool,
    orientation: RadioOrientation,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RadioOrientation {
    Vertical,
    Horizontal,
}

#[allow(dead_code)]
impl RadioGroup {
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            options: Vec::new(),
            value: None,
            disabled: false,
            orientation: RadioOrientation::Vertical,
        }
    }

    pub fn option(mut self, value: impl Into<String>, label: impl Into<String>) -> Self {
        self.options.push((value.into(), label.into()));
        self
    }

    pub fn value(mut self, value: impl Into<String>) -> Self {
        self.value = Some(value.into());
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn horizontal(mut self) -> Self {
        self.orientation = RadioOrientation::Horizontal;
        self
    }

    pub fn render(self, theme: &Theme) -> impl IntoElement {
        let is_horizontal = self.orientation == RadioOrientation::Horizontal;
        let mut group = div().flex().gap(px(8.0));

        if !is_horizontal {
            group = group.flex_col();
        }

        for (val, label) in self.options {
            let selected = self.value.as_ref().map(|v| v == &val).unwrap_or(false);
            group = group.child(
                RadioItem {
                    value: val,
                    label,
                    selected,
                    disabled: self.disabled,
                }
                .render(theme),
            );
        }

        group
    }
}

struct RadioItem {
    #[allow(dead_code)]
    value: String,
    label: String,
    selected: bool,
    disabled: bool,
}

impl RadioItem {
    fn render(self, theme: &Theme) -> impl IntoElement {
        let border = if self.selected {
            theme.primary
        } else {
            theme.border
        };

        let mut container = div().flex().items_center().gap(px(8.0));

        // Circle
        let mut circle = div()
            .flex()
            .items_center()
            .justify_center()
            .size(px(16.0))
            .rounded(Radius::FULL)
            .border_1()
            .border_color(border)
            .flex_shrink_0();

        if self.selected {
            circle = circle.child(div().size(px(8.0)).rounded(Radius::FULL).bg(theme.primary));
        }

        if !self.disabled {
            circle = circle.cursor_pointer();
        } else {
            circle = circle.opacity(0.5);
        }

        container = container.child(circle);
        container = container.child(
            div()
                .text_size(px(14.0))
                .text_color(if self.disabled {
                    theme.muted_foreground
                } else {
                    theme.foreground
                })
                .child(self.label),
        );

        container
    }
}
