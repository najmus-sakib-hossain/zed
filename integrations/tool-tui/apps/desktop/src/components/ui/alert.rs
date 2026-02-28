use crate::theme::{colors::Radius, Theme};
use gpui::{div, prelude::*, px, AnyElement, Hsla, IntoElement};

// ─── Alert ──────────────────────────────────────────────────────────────────
// A shadcn-ui style Alert component for displaying important messages.
//
// Usage:
//   Alert::new("Heads up!")
//       .description("You can add components to your app using the CLI.")
//       .variant(AlertVariant::Default)
//       .render(&theme)

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AlertVariant {
    Default,
    Destructive,
    Success,
    Warning,
    Info,
}

pub struct Alert {
    title: String,
    description: Option<String>,
    variant: AlertVariant,
    icon: Option<String>,
}

impl Alert {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            description: None,
            variant: AlertVariant::Default,
            icon: None,
        }
    }

    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    pub fn variant(mut self, variant: AlertVariant) -> Self {
        self.variant = variant;
        self
    }

    pub fn icon(mut self, icon: impl Into<String>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    pub fn render(self, theme: &Theme) -> impl IntoElement {
        let (bg, border, icon_color, title_color) = self.variant_colors(theme);
        let default_icon = match self.variant {
            AlertVariant::Default => "ℹ",
            AlertVariant::Destructive => "⚠",
            AlertVariant::Success => "✓",
            AlertVariant::Warning => "⚡",
            AlertVariant::Info => "ℹ",
        };
        let icon = self.icon.unwrap_or_else(|| default_icon.to_string());

        let mut alert = div()
            .flex()
            .gap(px(12.0))
            .w_full()
            .p(px(16.0))
            .rounded(Radius::LG)
            .bg(bg)
            .border_1()
            .border_color(border);

        // Icon
        alert = alert.child(
            div()
                .text_color(icon_color)
                .text_size(px(16.0))
                .mt(px(1.0))
                .flex_shrink_0()
                .child(icon),
        );

        // Content
        let mut content = div().flex().flex_col().gap(px(4.0)).flex_1();
        content = content.child(
            div()
                .text_size(px(14.0))
                .font_weight(gpui::FontWeight::MEDIUM)
                .text_color(title_color)
                .line_height(px(20.0))
                .child(self.title),
        );

        if let Some(desc) = self.description {
            content = content
                .child(div().text_size(px(14.0)).text_color(theme.muted_foreground).child(desc));
        }

        alert = alert.child(content);
        alert
    }

    fn variant_colors(&self, theme: &Theme) -> (Hsla, Hsla, Hsla, Hsla) {
        match self.variant {
            AlertVariant::Default => {
                (theme.background, theme.border, theme.foreground, theme.foreground)
            }
            AlertVariant::Destructive => {
                let bg = Hsla {
                    h: theme.destructive.h,
                    s: theme.destructive.s,
                    l: theme.destructive.l,
                    a: 0.1,
                };
                (bg, theme.destructive, theme.destructive, theme.destructive)
            }
            AlertVariant::Success => {
                let bg = Hsla {
                    h: theme.success.h,
                    s: theme.success.s,
                    l: theme.success.l,
                    a: 0.1,
                };
                (bg, theme.success, theme.success, theme.success)
            }
            AlertVariant::Warning => {
                let bg = Hsla {
                    h: theme.warning.h,
                    s: theme.warning.s,
                    l: theme.warning.l,
                    a: 0.1,
                };
                (bg, theme.warning, theme.warning, theme.warning)
            }
            AlertVariant::Info => {
                let bg = Hsla {
                    h: theme.info.h,
                    s: theme.info.s,
                    l: theme.info.l,
                    a: 0.1,
                };
                (bg, theme.info, theme.info, theme.info)
            }
        }
    }
}

// ─── Toast ──────────────────────────────────────────────────────────────────
// A shadcn-ui style Toast notification.
//
// Usage:
//   Toast::new("Event has been created")
//       .description("Sunday, December 03, 2023 at 9:00 AM")
//       .variant(ToastVariant::Default)
//       .render(&theme)

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ToastVariant {
    Default,
    Destructive,
    Success,
}

pub struct Toast {
    title: String,
    description: Option<String>,
    variant: ToastVariant,
    action: Option<String>,
}

pub struct Toaster {
    items: Vec<Toast>,
    max_width: gpui::Pixels,
}

impl Toaster {
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            max_width: px(380.0),
        }
    }

    pub fn toast(mut self, toast: Toast) -> Self {
        self.items.push(toast);
        self
    }

    #[allow(dead_code)]
    pub fn max_width(mut self, width: gpui::Pixels) -> Self {
        self.max_width = width;
        self
    }

    pub fn render(self, theme: &Theme) -> impl IntoElement {
        let mut stack = div().flex().flex_col().gap(px(8.0)).w(self.max_width);

        for toast in self.items {
            stack = stack.child(toast.render(theme));
        }

        stack
    }
}

impl Toast {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            description: None,
            variant: ToastVariant::Default,
            action: None,
        }
    }

    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    pub fn variant(mut self, variant: ToastVariant) -> Self {
        self.variant = variant;
        self
    }

    #[allow(dead_code)]
    pub fn action(mut self, label: impl Into<String>) -> Self {
        self.action = Some(label.into());
        self
    }

    pub fn render(self, theme: &Theme) -> impl IntoElement {
        let (bg, border, title_color) = match self.variant {
            ToastVariant::Default => (theme.background, theme.border, theme.foreground),
            ToastVariant::Destructive => {
                (theme.destructive, theme.destructive, theme.destructive_foreground)
            }
            ToastVariant::Success => (theme.background, theme.success, theme.foreground),
        };

        let mut toast = div()
            .flex()
            .items_center()
            .justify_between()
            .gap(px(16.0))
            .w(px(360.0))
            .p(px(16.0))
            .rounded(Radius::LG)
            .bg(bg)
            .border_1()
            .border_color(border);

        // Content
        let mut content = div().flex().flex_col().gap(px(2.0)).flex_1();
        content = content.child(
            div()
                .text_size(px(14.0))
                .font_weight(gpui::FontWeight::MEDIUM)
                .text_color(title_color)
                .child(self.title),
        );

        if let Some(desc) = self.description {
            content = content
                .child(div().text_size(px(13.0)).text_color(theme.muted_foreground).child(desc));
        }

        toast = toast.child(content);

        // Action button
        if let Some(action_label) = self.action {
            toast = toast.child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .child(
                        div()
                            .px(px(12.0))
                            .py(px(4.0))
                            .rounded(Radius::DEFAULT)
                            .border_1()
                            .border_color(theme.border)
                            .text_size(px(12.0))
                            .text_color(theme.foreground)
                            .cursor_pointer()
                            .hover(move |style| style.bg(theme.accent))
                            .child(action_label),
                    )
                    .child(
                        div()
                            .text_size(px(14.0))
                            .text_color(theme.muted_foreground)
                            .cursor_pointer()
                            .child("×"),
                    ),
            );
        } else {
            toast = toast.child(
                div()
                    .text_size(px(14.0))
                    .text_color(theme.muted_foreground)
                    .cursor_pointer()
                    .child("×"),
            );
        }

        toast
    }
}

// ─── AlertDialog ────────────────────────────────────────────────────────────
// A shadcn-ui style AlertDialog (modal confirmation).
//
// Usage:
//   AlertDialog::new("Are you sure?")
//       .description("This action cannot be undone.")
//       .confirm_label("Continue")
//       .cancel_label("Cancel")
//       .render(&theme)

pub struct AlertDialog {
    title: String,
    description: Option<String>,
    confirm_label: String,
    cancel_label: String,
    destructive: bool,
}

impl AlertDialog {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            description: None,
            confirm_label: "Continue".to_string(),
            cancel_label: "Cancel".to_string(),
            destructive: false,
        }
    }

    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    pub fn confirm_label(mut self, label: impl Into<String>) -> Self {
        self.confirm_label = label.into();
        self
    }

    pub fn cancel_label(mut self, label: impl Into<String>) -> Self {
        self.cancel_label = label.into();
        self
    }

    #[allow(dead_code)]
    pub fn destructive(mut self, destructive: bool) -> Self {
        self.destructive = destructive;
        self
    }

    pub fn render(self, theme: &Theme) -> impl IntoElement {
        let confirm_bg = if self.destructive {
            theme.destructive
        } else {
            theme.primary
        };
        let confirm_fg = if self.destructive {
            theme.destructive_foreground
        } else {
            theme.primary_foreground
        };

        // Overlay + Dialog
        div()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .bg(theme.overlay)
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(16.0))
                    .w(px(420.0))
                    .p(px(24.0))
                    .rounded(Radius::LG)
                    .bg(theme.background)
                    .border_1()
                    .border_color(theme.border)
                    // Title
                    .child(
                        div()
                            .text_size(px(18.0))
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .text_color(theme.foreground)
                            .child(self.title),
                    )
                    // Description
                    .when_some(self.description, |this, desc| {
                        this.child(
                            div()
                                .text_size(px(14.0))
                                .text_color(theme.muted_foreground)
                                .child(desc),
                        )
                    })
                    // Actions
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_end()
                            .gap(px(8.0))
                            .pt(px(8.0))
                            // Cancel
                            .child(
                                div()
                                    .px(px(16.0))
                                    .py(px(8.0))
                                    .rounded(Radius::DEFAULT)
                                    .bg(theme.secondary)
                                    .text_size(px(14.0))
                                    .font_weight(gpui::FontWeight::MEDIUM)
                                    .text_color(theme.secondary_foreground)
                                    .cursor_pointer()
                                    .hover(move |style| style.bg(theme.accent))
                                    .child(self.cancel_label),
                            )
                            // Confirm
                            .child(
                                div()
                                    .px(px(16.0))
                                    .py(px(8.0))
                                    .rounded(Radius::DEFAULT)
                                    .bg(confirm_bg)
                                    .text_size(px(14.0))
                                    .font_weight(gpui::FontWeight::MEDIUM)
                                    .text_color(confirm_fg)
                                    .cursor_pointer()
                                    .hover(move |style| style.opacity(0.9))
                                    .child(self.confirm_label),
                            ),
                    ),
            )
    }
}

// ─── Dialog ─────────────────────────────────────────────────────────────────
// A generic Dialog (modal) container.
//
// Usage:
//   Dialog::new("Edit Profile")
//       .description("Make changes to your profile here.")
//       .child(div().child("form fields..."))
//       .footer(div().child("buttons"))
//       .render(&theme)

pub struct Dialog {
    title: String,
    description: Option<String>,
    children: Vec<AnyElement>,
    footer: Option<AnyElement>,
    width: gpui::Pixels,
}

impl Dialog {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            description: None,
            children: Vec::new(),
            footer: None,
            width: px(480.0),
        }
    }

    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    pub fn child(mut self, child: impl IntoElement) -> Self {
        self.children.push(child.into_any_element());
        self
    }

    pub fn footer(mut self, footer: impl IntoElement) -> Self {
        self.footer = Some(footer.into_any_element());
        self
    }

    #[allow(dead_code)]
    pub fn width(mut self, width: gpui::Pixels) -> Self {
        self.width = width;
        self
    }

    pub fn render(self, theme: &Theme) -> impl IntoElement {
        // Overlay
        div()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .bg(theme.overlay)
            .child({
                let mut dialog = div()
                    .flex()
                    .flex_col()
                    .w(self.width)
                    .max_h(gpui::relative(0.85))
                    .rounded(Radius::LG)
                    .bg(theme.background)
                    .border_1()
                    .border_color(theme.border)
                    .overflow_hidden();

                // Header
                let mut header = div().flex().flex_col().gap(px(6.0)).p(px(24.0)).pb(px(0.0));

                // Close button
                header = header.child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .child(
                            div()
                                .text_size(px(18.0))
                                .font_weight(gpui::FontWeight::SEMIBOLD)
                                .text_color(theme.foreground)
                                .child(self.title),
                        )
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .justify_center()
                                .size(px(24.0))
                                .rounded(Radius::SM)
                                .cursor_pointer()
                                .hover(move |style| style.bg(theme.accent))
                                .child(
                                    div()
                                        .text_size(px(14.0))
                                        .text_color(theme.muted_foreground)
                                        .child("×"),
                                ),
                        ),
                );

                if let Some(desc) = self.description {
                    header = header.child(
                        div().text_size(px(14.0)).text_color(theme.muted_foreground).child(desc),
                    );
                }

                dialog = dialog.child(header);

                // Content
                if !self.children.is_empty() {
                    let mut content = div().p(px(24.0)).overflow_y_hidden();
                    for child in self.children {
                        content = content.child(child);
                    }
                    dialog = dialog.child(content);
                }

                // Footer
                if let Some(footer) = self.footer {
                    dialog = dialog.child(
                        div()
                            .flex()
                            .items_center()
                            .justify_end()
                            .gap(px(8.0))
                            .p(px(24.0))
                            .pt(px(0.0))
                            .border_t_1()
                            .border_color(theme.border)
                            .child(footer),
                    );
                }

                dialog
            })
    }
}
