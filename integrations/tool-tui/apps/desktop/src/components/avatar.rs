use crate::theme::{colors::Radius, Theme};
use gpui::{div, prelude::*, px, IntoElement};

// ─── Avatar ─────────────────────────────────────────────────────────────────
// A shadcn-ui style Avatar component with fallback support.
//
// Usage:
//   Avatar::new().fallback("JD").render(&theme)
//   Avatar::new().fallback("A").size(AvatarSize::Lg).render(&theme)

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AvatarSize {
    Sm,
    Default,
    Lg,
    Xl,
}

pub struct Avatar {
    fallback_text: String,
    size: AvatarSize,
}

impl Avatar {
    pub fn new() -> Self {
        Self {
            fallback_text: String::new(),
            size: AvatarSize::Default,
        }
    }

    pub fn fallback(mut self, text: impl Into<String>) -> Self {
        self.fallback_text = text.into();
        self
    }

    pub fn size(mut self, size: AvatarSize) -> Self {
        self.size = size;
        self
    }

    pub fn render(self, theme: &Theme) -> impl IntoElement {
        let (dimension, font) = match self.size {
            AvatarSize::Sm => (px(24.0), px(10.0)),
            AvatarSize::Default => (px(40.0), px(14.0)),
            AvatarSize::Lg => (px(48.0), px(18.0)),
            AvatarSize::Xl => (px(64.0), px(24.0)),
        };

        div()
            .flex()
            .items_center()
            .justify_center()
            .size(dimension)
            .rounded(Radius::FULL)
            .bg(theme.muted)
            .overflow_hidden()
            .flex_shrink_0()
            .child(
                div()
                    .text_size(font)
                    .font_weight(gpui::FontWeight::MEDIUM)
                    .text_color(theme.muted_foreground)
                    .child(self.fallback_text),
            )
    }
}

// ─── AvatarGroup ────────────────────────────────────────────────────────────

pub struct AvatarGroup {
    avatars: Vec<String>,
    max_count: usize,
    size: AvatarSize,
}

#[allow(dead_code)]
impl AvatarGroup {
    pub fn new(avatars: Vec<impl Into<String>>) -> Self {
        Self {
            avatars: avatars.into_iter().map(|a| a.into()).collect(),
            max_count: 5,
            size: AvatarSize::Default,
        }
    }

    pub fn max_count(mut self, count: usize) -> Self {
        self.max_count = count;
        self
    }

    pub fn size(mut self, size: AvatarSize) -> Self {
        self.size = size;
        self
    }

    pub fn render(self, theme: &Theme) -> impl IntoElement {
        let visible = self.avatars.len().min(self.max_count);
        let remaining = self.avatars.len().saturating_sub(self.max_count);

        let mut group = div().flex().items_center();

        for text in self.avatars.iter().take(visible) {
            group = group.child(
                div()
                    .ml(px(-8.0))
                    .border_2()
                    .border_color(theme.background)
                    .rounded(Radius::FULL)
                    .child(Avatar::new().fallback(text.clone()).size(self.size).render(theme)),
            );
        }

        if remaining > 0 {
            group = group.child(
                div()
                    .ml(px(-8.0))
                    .border_2()
                    .border_color(theme.background)
                    .rounded(Radius::FULL)
                    .child(
                        Avatar::new()
                            .fallback(format!("+{}", remaining))
                            .size(self.size)
                            .render(theme),
                    ),
            );
        }

        group
    }
}
