use crate::theme::Theme;
use gpui::{div, prelude::*, px, IntoElement};

// ─── Separator ──────────────────────────────────────────────────────────────
// A shadcn-ui style Separator (divider line).
//
// Usage:
//   Separator::horizontal().render(&theme)
//   Separator::vertical().render(&theme)

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SeparatorOrientation {
    Horizontal,
    Vertical,
}

pub struct Separator {
    orientation: SeparatorOrientation,
    label: Option<String>,
}

impl Separator {
    pub fn horizontal() -> Self {
        Self {
            orientation: SeparatorOrientation::Horizontal,
            label: None,
        }
    }

    pub fn vertical() -> Self {
        Self {
            orientation: SeparatorOrientation::Vertical,
            label: None,
        }
    }

    #[allow(dead_code)]
    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn render(self, theme: &Theme) -> impl IntoElement {
        if let Some(label) = self.label {
            // Separator with label (like "OR" divider)
            div()
                .flex()
                .items_center()
                .gap(px(16.0))
                .w_full()
                .child(div().flex_1().h(px(1.0)).bg(theme.border))
                .child(div().text_color(theme.muted_foreground).text_size(px(12.0)).child(label))
                .child(div().flex_1().h(px(1.0)).bg(theme.border))
        } else {
            match self.orientation {
                SeparatorOrientation::Horizontal => div()
                    .flex()
                    .items_center()
                    .w_full()
                    .child(div().w_full().h(px(1.0)).bg(theme.border).flex_shrink_0()),
                SeparatorOrientation::Vertical => div()
                    .flex()
                    .items_center()
                    .h_full()
                    .child(div().h_full().w(px(1.0)).bg(theme.border).flex_shrink_0()),
            }
        }
    }
}
