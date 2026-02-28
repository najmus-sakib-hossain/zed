use gpui::{div, prelude::*, px, AnyElement, IntoElement};

use crate::theme::Theme;

// ─── FormField ──────────────────────────────────────────────────────────────
// Form field wrapper with label, description, and error message.
//
// Usage:
//   FormField::new("email")
//       .label("Email Address")
//       .description("We'll never share your email.")
//       .required(true)
//       .child(input)
//       .error("Invalid email format")
//       .render(&theme)

pub struct FormField {
    id: String,
    label: Option<String>,
    description: Option<String>,
    error: Option<String>,
    required: bool,
    content: Option<AnyElement>,
}

#[allow(dead_code)]
impl FormField {
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: None,
            description: None,
            error: None,
            required: false,
            content: None,
        }
    }

    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    pub fn error(mut self, error: impl Into<String>) -> Self {
        self.error = Some(error.into());
        self
    }

    pub fn required(mut self, required: bool) -> Self {
        self.required = required;
        self
    }

    pub fn child(mut self, element: impl IntoElement) -> Self {
        self.content = Some(element.into_any_element());
        self
    }

    pub fn render(self, theme: &Theme) -> impl IntoElement {
        let mut field = div().flex().flex_col().gap(px(6.0)).w_full();

        // Label
        if let Some(label) = self.label {
            let mut label_el = div()
                .flex()
                .items_center()
                .gap(px(2.0))
                .text_size(px(14.0))
                .font_weight(gpui::FontWeight::MEDIUM)
                .text_color(theme.foreground)
                .child(label);

            if self.required {
                label_el = label_el
                    .child(div().text_color(theme.destructive).text_size(px(14.0)).child("*"));
            }

            field = field.child(label_el);
        }

        // Description
        if let Some(desc) = self.description {
            field = field
                .child(div().text_size(px(12.0)).text_color(theme.muted_foreground).child(desc));
        }

        // Content
        if let Some(content) = self.content {
            field = field.child(content);
        }

        // Error message
        if let Some(error) = self.error {
            field = field.child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(4.0))
                    .text_size(px(12.0))
                    .text_color(theme.destructive)
                    .child("⚠")
                    .child(error),
            );
        }

        field
    }
}

// ─── FormGroup ──────────────────────────────────────────────────────────────
// Groups form fields with consistent spacing and optional title.

pub struct FormGroup {
    title: Option<String>,
    description: Option<String>,
    fields: Vec<AnyElement>,
    horizontal: bool,
}

#[allow(dead_code)]
impl FormGroup {
    pub fn new() -> Self {
        Self {
            title: None,
            description: None,
            fields: Vec::new(),
            horizontal: false,
        }
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    pub fn field(mut self, field: impl IntoElement) -> Self {
        self.fields.push(field.into_any_element());
        self
    }

    pub fn horizontal(mut self, horizontal: bool) -> Self {
        self.horizontal = horizontal;
        self
    }

    pub fn render(self, theme: &Theme) -> impl IntoElement {
        let mut group = div().flex().flex_col().gap(px(16.0));

        // Group header
        if self.title.is_some() || self.description.is_some() {
            let mut header = div().flex().flex_col().gap(px(4.0));

            if let Some(title) = self.title {
                header = header.child(
                    div()
                        .text_size(px(16.0))
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .text_color(theme.foreground)
                        .child(title),
                );
            }

            if let Some(desc) = self.description {
                header = header.child(
                    div().text_size(px(13.0)).text_color(theme.muted_foreground).child(desc),
                );
            }

            group = group.child(header);
        }

        // Fields
        let mut fields_container = div().gap(px(12.0));

        if self.horizontal {
            fields_container = fields_container.flex().flex_row().flex_wrap().items_end();
        } else {
            fields_container = fields_container.flex().flex_col();
        }

        for field in self.fields {
            if self.horizontal {
                fields_container =
                    fields_container.child(div().flex_1().min_w(px(200.0)).child(field));
            } else {
                fields_container = fields_container.child(field);
            }
        }

        group.child(fields_container)
    }
}

// ─── SettingsRow ────────────────────────────────────────────────────────────
// A settings row with label, description, and control area.
// Common in desktop app preferences/settings pages.
//
// Usage:
//   SettingsRow::new("Auto Save")
//       .description("Automatically save files after editing.")
//       .control(Switch::new("auto-save").checked(true).render(&theme))
//       .render(&theme)

pub struct SettingsRow {
    label: String,
    description: Option<String>,
    control: Option<AnyElement>,
    bordered: bool,
}

#[allow(dead_code)]
impl SettingsRow {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            description: None,
            control: None,
            bordered: true,
        }
    }

    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    pub fn control(mut self, element: impl IntoElement) -> Self {
        self.control = Some(element.into_any_element());
        self
    }

    pub fn bordered(mut self, bordered: bool) -> Self {
        self.bordered = bordered;
        self
    }

    pub fn render(self, theme: &Theme) -> impl IntoElement {
        let mut row = div().flex().items_center().justify_between().py(px(12.0)).gap(px(16.0));

        if self.bordered {
            row = row.border_b_1().border_color(theme.border);
        }

        // Label area
        let mut label_area = div().flex().flex_col().gap(px(2.0)).flex_1();

        label_area = label_area.child(
            div()
                .text_size(px(14.0))
                .font_weight(gpui::FontWeight::MEDIUM)
                .text_color(theme.foreground)
                .child(self.label),
        );

        if let Some(desc) = self.description {
            label_area = label_area
                .child(div().text_size(px(12.0)).text_color(theme.muted_foreground).child(desc));
        }

        row = row.child(label_area);

        // Control
        if let Some(control) = self.control {
            row = row.child(div().flex_shrink_0().child(control));
        }

        row
    }
}
