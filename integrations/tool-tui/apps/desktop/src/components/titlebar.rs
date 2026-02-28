use crate::theme::Theme;
use gpui::{div, prelude::*, px, IntoElement};

// ─── TitleBar ───────────────────────────────────────────────────────────────
// A shadcn-ui style window TitleBar with window controls.
//
// Usage:
//   TitleBar::new("DX Desktop").render(&theme)

pub struct TitleBar {
    title: String,
    subtitle: Option<String>,
    show_controls: bool,
}

impl TitleBar {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            subtitle: None,
            show_controls: true,
        }
    }

    #[allow(dead_code)]
    pub fn subtitle(mut self, subtitle: impl Into<String>) -> Self {
        self.subtitle = Some(subtitle.into());
        self
    }

    #[allow(dead_code)]
    pub fn show_controls(mut self, show: bool) -> Self {
        self.show_controls = show;
        self
    }

    pub fn render(self, theme: &Theme) -> impl IntoElement {
        let mut bar = div()
            .flex()
            .items_center()
            .justify_between()
            .h(px(40.0))
            .px(px(16.0))
            .bg(theme.background)
            .border_b_1()
            .border_color(theme.border)
            .flex_shrink_0();

        // Left: title
        let mut title_section = div().flex().items_center().gap(px(8.0));

        title_section = title_section.child(
            div()
                .text_size(px(13.0))
                .font_weight(gpui::FontWeight::SEMIBOLD)
                .text_color(theme.foreground)
                .child(self.title),
        );

        if let Some(subtitle) = self.subtitle {
            title_section = title_section.child(
                div().text_size(px(12.0)).text_color(theme.muted_foreground).child(subtitle),
            );
        }

        bar = bar.child(title_section);

        // Right: window controls
        if self.show_controls {
            bar = bar.child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(0.0))
                    .child(WindowControlButton::minimize().render(theme))
                    .child(WindowControlButton::maximize().render(theme))
                    .child(WindowControlButton::close().render(theme)),
            );
        }

        bar
    }
}

// ─── WindowControlButton ────────────────────────────────────────────────────

#[derive(Clone, Copy)]
enum WindowControl {
    Minimize,
    Maximize,
    Close,
}

struct WindowControlButton {
    control: WindowControl,
}

impl WindowControlButton {
    fn minimize() -> Self {
        Self {
            control: WindowControl::Minimize,
        }
    }

    fn maximize() -> Self {
        Self {
            control: WindowControl::Maximize,
        }
    }

    fn close() -> Self {
        Self {
            control: WindowControl::Close,
        }
    }

    fn render(self, theme: &Theme) -> impl IntoElement {
        let (icon, hover_bg) = match self.control {
            WindowControl::Minimize => ("−", theme.secondary),
            WindowControl::Maximize => ("□", theme.secondary),
            WindowControl::Close => ("×", theme.destructive),
        };

        let hover_text = match self.control {
            WindowControl::Close => theme.destructive_foreground,
            _ => theme.foreground,
        };

        div()
            .flex()
            .items_center()
            .justify_center()
            .w(px(46.0))
            .h(px(32.0))
            .cursor_pointer()
            .hover(move |style| style.bg(hover_bg).text_color(hover_text))
            .child(div().text_size(px(14.0)).text_color(theme.foreground).child(icon))
    }
}
