use crate::theme::{colors::Radius, Theme};
use gpui::{div, prelude::*, px, IntoElement};

// ─── Progress ───────────────────────────────────────────────────────────────
// A shadcn-ui style Progress bar.
//
// Usage:
//   Progress::new(75.0).render(&theme)
//   Progress::new(40.0).size(ProgressSize::Lg).color(theme.success).render(&theme)

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ProgressSize {
    Sm,
    Default,
    Lg,
}

pub struct Progress {
    value: f32, // 0-100
    size: ProgressSize,
    color: Option<gpui::Hsla>,
    show_label: bool,
}

impl Progress {
    pub fn new(value: f32) -> Self {
        Self {
            value: value.clamp(0.0, 100.0),
            size: ProgressSize::Default,
            color: None,
            show_label: false,
        }
    }

    #[allow(dead_code)]
    pub fn size(mut self, size: ProgressSize) -> Self {
        self.size = size;
        self
    }

    #[allow(dead_code)]
    pub fn color(mut self, color: gpui::Hsla) -> Self {
        self.color = Some(color);
        self
    }

    #[allow(dead_code)]
    pub fn show_label(mut self, show: bool) -> Self {
        self.show_label = show;
        self
    }

    pub fn render(self, theme: &Theme) -> impl IntoElement {
        let height = match self.size {
            ProgressSize::Sm => px(4.0),
            ProgressSize::Default => px(8.0),
            ProgressSize::Lg => px(12.0),
        };
        let fill_color = self.color.unwrap_or(theme.primary);
        let width_pct = self.value / 100.0;

        let mut container = div().flex().items_center().gap(px(8.0)).w_full();

        // Track
        let track = div()
            .flex_1()
            .h(height)
            .rounded(Radius::FULL)
            .bg(theme.secondary)
            .overflow_hidden()
            .child(
                div().h_full().w(gpui::relative(width_pct)).rounded(Radius::FULL).bg(fill_color),
            );

        container = container.child(track);

        if self.show_label {
            container = container.child(
                div()
                    .text_size(px(12.0))
                    .text_color(theme.muted_foreground)
                    .min_w(px(36.0))
                    .text_right()
                    .child(format!("{}%", self.value as u32)),
            );
        }

        container
    }
}

// ─── Skeleton ───────────────────────────────────────────────────────────────
// A shadcn-ui style Skeleton loading placeholder.
//
// Usage:
//   Skeleton::new().w(px(200.0)).h(px(20.0)).render(&theme)
//   Skeleton::circle(40.0).render(&theme)
//   Skeleton::text_lines(3).render(&theme)

pub struct Skeleton {
    width: Option<gpui::Pixels>,
    height: Option<gpui::Pixels>,
    variant: SkeletonVariant,
}

#[derive(Debug, Clone, Copy)]
enum SkeletonVariant {
    Rectangle,
    Circle(f32),
    TextLines(u32),
}

impl Skeleton {
    pub fn new() -> Self {
        Self {
            width: None,
            height: None,
            variant: SkeletonVariant::Rectangle,
        }
    }

    pub fn circle(size: f32) -> Self {
        Self {
            width: Some(px(size)),
            height: Some(px(size)),
            variant: SkeletonVariant::Circle(size),
        }
    }

    pub fn text_lines(count: u32) -> Self {
        Self {
            width: None,
            height: None,
            variant: SkeletonVariant::TextLines(count),
        }
    }

    pub fn w(mut self, width: gpui::Pixels) -> Self {
        self.width = Some(width);
        self
    }

    pub fn h(mut self, height: gpui::Pixels) -> Self {
        self.height = Some(height);
        self
    }

    pub fn render(self, theme: &Theme) -> impl IntoElement {
        match self.variant {
            SkeletonVariant::Rectangle => {
                let mut el = div().bg(theme.muted).rounded(Radius::DEFAULT);
                if let Some(w) = self.width {
                    el = el.w(w);
                } else {
                    el = el.w_full();
                }
                if let Some(h) = self.height {
                    el = el.h(h);
                } else {
                    el = el.h(px(20.0));
                }
                el.into_any_element()
            }
            SkeletonVariant::Circle(size) => div()
                .size(px(size))
                .rounded(Radius::FULL)
                .bg(theme.muted)
                .flex_shrink_0()
                .into_any_element(),
            SkeletonVariant::TextLines(count) => {
                let mut container = div().flex().flex_col().gap(px(8.0)).w_full();
                for i in 0..count {
                    let width = if i == count - 1 {
                        gpui::relative(0.6)
                    } else {
                        gpui::relative(1.0)
                    };
                    container = container
                        .child(div().h(px(16.0)).w(width).rounded(Radius::DEFAULT).bg(theme.muted));
                }
                container.into_any_element()
            }
        }
    }
}
