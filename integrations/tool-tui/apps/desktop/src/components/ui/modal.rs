use gpui::{div, prelude::*, px, AnyElement, IntoElement};

use crate::theme::{colors::Radius, Theme};

// ─── Modal ──────────────────────────────────────────────────────────────────
// Desktop modal dialog with overlay backdrop.
//
// Usage:
//   Modal::new("delete-confirm")
//       .title("Delete File?")
//       .description("This action cannot be undone.")
//       .footer(button_row)
//       .render(&theme)

pub struct Modal {
    _id: String,
    title: Option<String>,
    description: Option<String>,
    content: Option<AnyElement>,
    footer: Option<AnyElement>,
    width: f32,
    show_close: bool,
    overlay: bool,
}

#[allow(dead_code)]
impl Modal {
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            _id: id.into(),
            title: None,
            description: None,
            content: None,
            footer: None,
            width: 480.0,
            show_close: true,
            overlay: true,
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

    pub fn content(mut self, content: impl IntoElement) -> Self {
        self.content = Some(content.into_any_element());
        self
    }

    pub fn footer(mut self, footer: impl IntoElement) -> Self {
        self.footer = Some(footer.into_any_element());
        self
    }

    pub fn width(mut self, w: f32) -> Self {
        self.width = w;
        self
    }

    pub fn show_close(mut self, v: bool) -> Self {
        self.show_close = v;
        self
    }

    pub fn overlay(mut self, v: bool) -> Self {
        self.overlay = v;
        self
    }

    pub fn render(self, theme: &Theme) -> impl IntoElement {
        let overlay_color = theme.overlay;

        let mut backdrop = div().absolute().inset_0().flex().items_center().justify_center();

        if self.overlay {
            backdrop = backdrop.bg(overlay_color);
        }

        // Dialog card
        let mut dialog = div()
            .w(px(self.width))
            .max_h(gpui::relative(0.85))
            .bg(theme.background)
            .border_1()
            .border_color(theme.border)
            .rounded(Radius::LG)
            .shadow_lg()
            .flex()
            .flex_col()
            .overflow_hidden();

        // Header
        let has_header = self.title.is_some() || self.show_close;
        if has_header {
            let mut header = div()
                .flex()
                .items_start()
                .justify_between()
                .px(px(24.0))
                .pt(px(24.0))
                .pb(px(8.0));

            let mut title_area = div().flex().flex_col().gap(px(4.0)).flex_1();

            if let Some(title) = self.title {
                title_area = title_area.child(
                    div()
                        .text_size(px(16.0))
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .text_color(theme.foreground)
                        .child(title),
                );
            }

            if let Some(desc) = self.description {
                title_area = title_area.child(
                    div().text_size(px(13.0)).text_color(theme.muted_foreground).child(desc),
                );
            }

            header = header.child(title_area);

            if self.show_close {
                let close_hover = theme.muted;
                header = header.child(
                    div()
                        .flex()
                        .items_center()
                        .justify_center()
                        .w(px(24.0))
                        .h(px(24.0))
                        .rounded(Radius::SM)
                        .text_color(theme.muted_foreground)
                        .text_size(px(14.0))
                        .cursor_pointer()
                        .hover(move |s| s.bg(close_hover))
                        .child("×"),
                );
            }

            dialog = dialog.child(header);
        }

        // Content
        if let Some(content) = self.content {
            dialog = dialog
                .child(div().flex_1().overflow_y_hidden().px(px(24.0)).py(px(8.0)).child(content));
        }

        // Footer
        if let Some(footer) = self.footer {
            dialog = dialog.child(
                div()
                    .flex()
                    .items_center()
                    .justify_end()
                    .gap(px(8.0))
                    .px(px(24.0))
                    .py(px(16.0))
                    .border_t_1()
                    .border_color(theme.border)
                    .child(footer),
            );
        }

        backdrop.child(dialog)
    }
}

// ─── ConfirmDialog ──────────────────────────────────────────────────────────
// Specialized confirm/cancel dialog.

pub struct ConfirmDialog {
    title: String,
    message: String,
    confirm_label: String,
    cancel_label: String,
    destructive: bool,
}

#[allow(dead_code)]
impl ConfirmDialog {
    pub fn new(title: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            message: message.into(),
            confirm_label: "Confirm".to_string(),
            cancel_label: "Cancel".to_string(),
            destructive: false,
        }
    }

    pub fn confirm_label(mut self, label: impl Into<String>) -> Self {
        self.confirm_label = label.into();
        self
    }

    pub fn cancel_label(mut self, label: impl Into<String>) -> Self {
        self.cancel_label = label.into();
        self
    }

    pub fn destructive(mut self, v: bool) -> Self {
        self.destructive = v;
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

        let confirm_hover =
            gpui::hsla(confirm_bg.h, confirm_bg.s, (confirm_bg.l - 0.05).max(0.0), confirm_bg.a);
        let cancel_hover = theme.muted;

        // Build footer buttons
        let footer = div()
            .flex()
            .items_center()
            .gap(px(8.0))
            .child(
                div()
                    .px(px(16.0))
                    .py(px(8.0))
                    .rounded(Radius::MD)
                    .border_1()
                    .border_color(theme.border)
                    .text_color(theme.foreground)
                    .text_size(px(13.0))
                    .cursor_pointer()
                    .hover(move |s| s.bg(cancel_hover))
                    .child(self.cancel_label.clone()),
            )
            .child(
                div()
                    .px(px(16.0))
                    .py(px(8.0))
                    .rounded(Radius::MD)
                    .bg(confirm_bg)
                    .text_color(confirm_fg)
                    .text_size(px(13.0))
                    .font_weight(gpui::FontWeight::MEDIUM)
                    .cursor_pointer()
                    .hover(move |s| s.bg(confirm_hover))
                    .child(self.confirm_label.clone()),
            );

        Modal::new("confirm-dialog")
            .title(self.title)
            .description(self.message)
            .footer(footer)
            .width(420.0)
            .render(theme)
    }
}

// ─── Drawer ─────────────────────────────────────────────────────────────────
// Side drawer that slides in from left/right edge.

#[derive(Clone, Copy)]
pub enum DrawerSide {
    Left,
    Right,
}

pub struct Drawer {
    side: DrawerSide,
    width: f32,
    title: Option<String>,
    content: Option<AnyElement>,
    footer: Option<AnyElement>,
    overlay: bool,
}

#[allow(dead_code)]
impl Drawer {
    pub fn new(side: DrawerSide) -> Self {
        Self {
            side,
            width: 360.0,
            title: None,
            content: None,
            footer: None,
            overlay: true,
        }
    }

    pub fn width(mut self, w: f32) -> Self {
        self.width = w;
        self
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn content(mut self, content: impl IntoElement) -> Self {
        self.content = Some(content.into_any_element());
        self
    }

    pub fn footer(mut self, footer: impl IntoElement) -> Self {
        self.footer = Some(footer.into_any_element());
        self
    }

    pub fn overlay(mut self, v: bool) -> Self {
        self.overlay = v;
        self
    }

    pub fn render(self, theme: &Theme) -> impl IntoElement {
        let mut backdrop = div().absolute().inset_0().flex();

        if self.overlay {
            backdrop = backdrop.bg(theme.overlay);
        }

        // Position
        match self.side {
            DrawerSide::Left => {
                backdrop = backdrop.justify_start();
            }
            DrawerSide::Right => {
                backdrop = backdrop.justify_end();
            }
        }

        // Drawer panel
        let mut panel = div()
            .w(px(self.width))
            .h_full()
            .bg(theme.background)
            .border_1()
            .border_color(theme.border)
            .shadow_lg()
            .flex()
            .flex_col();

        // Header
        if let Some(title) = self.title {
            let close_hover = theme.muted;
            panel = panel.child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .h(px(48.0))
                    .px(px(16.0))
                    .border_b_1()
                    .border_color(theme.border)
                    .child(
                        div()
                            .text_size(px(14.0))
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .text_color(theme.foreground)
                            .child(title),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_center()
                            .w(px(24.0))
                            .h(px(24.0))
                            .rounded(Radius::SM)
                            .text_color(theme.muted_foreground)
                            .text_size(px(14.0))
                            .cursor_pointer()
                            .hover(move |s| s.bg(close_hover))
                            .child("×"),
                    ),
            );
        }

        // Content
        if let Some(content) = self.content {
            panel = panel.child(div().flex_1().overflow_y_hidden().p(px(16.0)).child(content));
        }

        // Footer
        if let Some(footer) = self.footer {
            panel = panel.child(
                div()
                    .flex()
                    .items_center()
                    .justify_end()
                    .gap(px(8.0))
                    .px(px(16.0))
                    .py(px(12.0))
                    .border_t_1()
                    .border_color(theme.border)
                    .child(footer),
            );
        }

        backdrop.child(panel)
    }
}

// ─── Popconfirm ─────────────────────────────────────────────────────────────
// Lightweight inline confirm popover attached near the trigger.

pub struct Popconfirm {
    title: String,
    description: Option<String>,
    confirm_label: String,
    cancel_label: String,
    destructive: bool,
}

#[allow(dead_code)]
impl Popconfirm {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            description: None,
            confirm_label: "Yes".to_string(),
            cancel_label: "No".to_string(),
            destructive: false,
        }
    }

    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    pub fn confirm_label(mut self, l: impl Into<String>) -> Self {
        self.confirm_label = l.into();
        self
    }

    pub fn cancel_label(mut self, l: impl Into<String>) -> Self {
        self.cancel_label = l.into();
        self
    }

    pub fn destructive(mut self, v: bool) -> Self {
        self.destructive = v;
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
        let confirm_hover =
            gpui::hsla(confirm_bg.h, confirm_bg.s, (confirm_bg.l - 0.05).max(0.0), confirm_bg.a);
        let cancel_hover = theme.muted;

        let mut card = div()
            .w(px(260.0))
            .bg(theme.popover)
            .border_1()
            .border_color(theme.border)
            .rounded(Radius::MD)
            .shadow_md()
            .p(px(12.0))
            .flex()
            .flex_col()
            .gap(px(8.0));

        card = card.child(
            div()
                .text_size(px(13.0))
                .font_weight(gpui::FontWeight::MEDIUM)
                .text_color(theme.popover_foreground)
                .child(self.title),
        );

        if let Some(desc) = self.description {
            card = card
                .child(div().text_size(px(12.0)).text_color(theme.muted_foreground).child(desc));
        }

        // Buttons
        card = card.child(
            div()
                .flex()
                .items_center()
                .justify_end()
                .gap(px(6.0))
                .mt(px(4.0))
                .child(
                    div()
                        .px(px(10.0))
                        .py(px(4.0))
                        .rounded(Radius::SM)
                        .border_1()
                        .border_color(theme.border)
                        .text_color(theme.foreground)
                        .text_size(px(11.0))
                        .cursor_pointer()
                        .hover(move |s| s.bg(cancel_hover))
                        .child(self.cancel_label),
                )
                .child(
                    div()
                        .px(px(10.0))
                        .py(px(4.0))
                        .rounded(Radius::SM)
                        .bg(confirm_bg)
                        .text_color(confirm_fg)
                        .text_size(px(11.0))
                        .font_weight(gpui::FontWeight::MEDIUM)
                        .cursor_pointer()
                        .hover(move |s| s.bg(confirm_hover))
                        .child(self.confirm_label),
                ),
        );

        card
    }
}
