use gpui::{div, prelude::*, px, AnyElement, Hsla, IntoElement};

use crate::theme::{colors::Radius, Theme};

// ─── Notification ───────────────────────────────────────────────────────────
// Desktop-native notification / toast component with auto-dismiss support.
// Designed for system-tray style notifications in desktop apps.
//
// Usage:
//   Notification::success("Build complete", "Project compiled in 2.3s")
//       .dismissible(true)
//       .render(&theme)

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NotificationVariant {
    Info,
    Success,
    Warning,
    Error,
}

pub struct Notification {
    variant: NotificationVariant,
    title: String,
    message: Option<String>,
    dismissible: bool,
    action_label: Option<String>,
    icon: Option<String>,
}

#[allow(dead_code)]
impl Notification {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            variant: NotificationVariant::Info,
            title: title.into(),
            message: None,
            dismissible: true,
            action_label: None,
            icon: None,
        }
    }

    pub fn info(title: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(title).variant(NotificationVariant::Info).message(message)
    }

    pub fn success(title: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(title).variant(NotificationVariant::Success).message(message)
    }

    pub fn warning(title: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(title).variant(NotificationVariant::Warning).message(message)
    }

    pub fn error(title: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new(title).variant(NotificationVariant::Error).message(message)
    }

    pub fn variant(mut self, variant: NotificationVariant) -> Self {
        self.variant = variant;
        self
    }

    pub fn message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(message.into());
        self
    }

    pub fn dismissible(mut self, dismissible: bool) -> Self {
        self.dismissible = dismissible;
        self
    }

    pub fn action_label(mut self, label: impl Into<String>) -> Self {
        self.action_label = Some(label.into());
        self
    }

    pub fn icon(mut self, icon: impl Into<String>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    fn default_icon(&self) -> &str {
        match self.variant {
            NotificationVariant::Info => "ℹ",
            NotificationVariant::Success => "✓",
            NotificationVariant::Warning => "⚠",
            NotificationVariant::Error => "✕",
        }
    }

    fn variant_color(&self, theme: &Theme) -> Hsla {
        match self.variant {
            NotificationVariant::Info => theme.info,
            NotificationVariant::Success => theme.success,
            NotificationVariant::Warning => theme.warning,
            NotificationVariant::Error => theme.destructive,
        }
    }

    pub fn render(self, theme: &Theme) -> impl IntoElement {
        let accent = self.variant_color(theme);
        let icon_text = self.icon.clone().unwrap_or_else(|| self.default_icon().to_string());

        let mut container = div()
            .flex()
            .items_start()
            .gap(px(12.0))
            .w(px(360.0))
            .p(px(16.0))
            .rounded(Radius::LG)
            .bg(theme.card)
            .border_1()
            .border_color(theme.border)
            .border_l_4()
            .overflow_hidden();

        // Set left border color to variant accent
        container = container.border_color(accent);

        // Icon
        container = container.child(
            div()
                .flex_shrink_0()
                .size(px(20.0))
                .flex()
                .items_center()
                .justify_center()
                .rounded(Radius::FULL)
                .text_color(accent)
                .text_size(px(14.0))
                .font_weight(gpui::FontWeight::BOLD)
                .child(icon_text),
        );

        // Content
        let mut content = div().flex().flex_col().gap(px(4.0)).flex_1();

        content = content.child(
            div()
                .text_size(px(13.0))
                .font_weight(gpui::FontWeight::SEMIBOLD)
                .text_color(theme.foreground)
                .child(self.title),
        );

        if let Some(message) = self.message {
            content = content
                .child(div().text_size(px(12.0)).text_color(theme.muted_foreground).child(message));
        }

        // Action button
        if let Some(action) = self.action_label {
            content = content.child(
                div()
                    .mt(px(8.0))
                    .text_size(px(12.0))
                    .font_weight(gpui::FontWeight::MEDIUM)
                    .text_color(accent)
                    .cursor_pointer()
                    .child(action),
            );
        }

        container = container.child(content);

        // Dismiss button
        if self.dismissible {
            container = container.child(
                div()
                    .flex_shrink_0()
                    .size(px(20.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(Radius::SM)
                    .text_color(theme.muted_foreground)
                    .text_size(px(12.0))
                    .cursor_pointer()
                    .hover(move |style| style.bg(theme.accent))
                    .child("✕"),
            );
        }

        container
    }
}

// ─── NotificationStack ──────────────────────────────────────────────────────
// Stacks multiple notifications with proper spacing.

pub struct NotificationStack {
    notifications: Vec<AnyElement>,
    position: NotificationPosition,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NotificationPosition {
    TopRight,
    TopLeft,
    BottomRight,
    BottomLeft,
}

#[allow(dead_code)]
impl NotificationStack {
    pub fn new() -> Self {
        Self {
            notifications: Vec::new(),
            position: NotificationPosition::TopRight,
        }
    }

    pub fn position(mut self, position: NotificationPosition) -> Self {
        self.position = position;
        self
    }

    pub fn notification(mut self, notification: impl IntoElement) -> Self {
        self.notifications.push(notification.into_any_element());
        self
    }

    pub fn render(self) -> impl IntoElement {
        let mut stack = div().absolute().flex().flex_col().gap(px(8.0));

        match self.position {
            NotificationPosition::TopRight => {
                stack = stack.top(px(8.0)).right(px(8.0));
            }
            NotificationPosition::TopLeft => {
                stack = stack.top(px(8.0)).left(px(8.0));
            }
            NotificationPosition::BottomRight => {
                stack = stack.bottom(px(8.0)).right(px(8.0));
            }
            NotificationPosition::BottomLeft => {
                stack = stack.bottom(px(8.0)).left(px(8.0));
            }
        }

        for notification in self.notifications {
            stack = stack.child(notification);
        }

        stack
    }
}
