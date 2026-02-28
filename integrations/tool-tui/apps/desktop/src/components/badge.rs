use crate::theme::{colors::Radius, Theme};
use gpui::{div, prelude::*, px, Hsla, IntoElement};

// ─── Badge ──────────────────────────────────────────────────────────────────
// A shadcn-ui style Badge for displaying status, labels, or counts.
//
// Usage:
//   Badge::new("New").variant(BadgeVariant::Default).render(&theme)
//   Badge::secondary("Beta").render(&theme)
//   Badge::destructive("Error").render(&theme)

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BadgeVariant {
    Default,
    Secondary,
    Destructive,
    Outline,
    Success,
    Warning,
}

pub struct Badge {
    label: String,
    variant: BadgeVariant,
}

impl Badge {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            variant: BadgeVariant::Default,
        }
    }

    pub fn secondary(label: impl Into<String>) -> Self {
        Self::new(label).variant(BadgeVariant::Secondary)
    }

    pub fn destructive(label: impl Into<String>) -> Self {
        Self::new(label).variant(BadgeVariant::Destructive)
    }

    pub fn outline(label: impl Into<String>) -> Self {
        Self::new(label).variant(BadgeVariant::Outline)
    }

    pub fn success(label: impl Into<String>) -> Self {
        Self::new(label).variant(BadgeVariant::Success)
    }

    pub fn warning(label: impl Into<String>) -> Self {
        Self::new(label).variant(BadgeVariant::Warning)
    }

    pub fn variant(mut self, variant: BadgeVariant) -> Self {
        self.variant = variant;
        self
    }

    pub fn render(self, theme: &Theme) -> impl IntoElement {
        let (bg, fg, border) = self.variant_colors(theme);
        let has_border = self.variant == BadgeVariant::Outline;

        let mut el = div()
            .flex()
            .items_center()
            .px(px(10.0))
            .py(px(2.0))
            .rounded(Radius::FULL)
            .bg(bg)
            .text_color(fg)
            .text_size(px(12.0))
            .font_weight(gpui::FontWeight::MEDIUM)
            .line_height(px(16.0));

        if has_border {
            el = el.border_1().border_color(border);
        }

        el.child(self.label)
    }

    fn variant_colors(&self, theme: &Theme) -> (Hsla, Hsla, Hsla) {
        match self.variant {
            BadgeVariant::Default => (theme.primary, theme.primary_foreground, theme.primary),
            BadgeVariant::Secondary => {
                (theme.secondary, theme.secondary_foreground, theme.secondary)
            }
            BadgeVariant::Destructive => {
                (theme.destructive, theme.destructive_foreground, theme.destructive)
            }
            BadgeVariant::Outline => (gpui::transparent_black(), theme.foreground, theme.border),
            BadgeVariant::Success => (theme.success, theme.success_foreground, theme.success),
            BadgeVariant::Warning => (theme.warning, theme.warning_foreground, theme.warning),
        }
    }
}
