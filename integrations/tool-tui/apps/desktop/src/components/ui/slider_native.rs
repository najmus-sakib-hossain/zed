use gpui::{div, prelude::*, px, IntoElement};

use crate::theme::{colors::Radius, Theme};

// ─── Slider (Native) ────────────────────────────────────────────────────────
// Desktop-style slider / range input.  Renders a track + filled portion + thumb.
// Works as a static component – interactive state lives in the parent view.
//
// Usage:
//   SliderNative::new("volume")
//       .value(0.75)
//       .label("Volume")
//       .show_value(true)
//       .render(&theme)

pub struct SliderNative {
    id: String,
    value: f32,
    min: f32,
    max: f32,
    label: Option<String>,
    suffix: Option<String>,
    show_value: bool,
    disabled: bool,
    size: SliderSize,
    color: Option<gpui::Hsla>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SliderSize {
    Sm,
    Default,
    Lg,
}

#[allow(dead_code)]
impl SliderNative {
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            value: 0.0,
            min: 0.0,
            max: 1.0,
            label: None,
            suffix: None,
            show_value: false,
            disabled: false,
            size: SliderSize::Default,
            color: None,
        }
    }

    pub fn value(mut self, v: f32) -> Self {
        self.value = v;
        self
    }

    pub fn min(mut self, v: f32) -> Self {
        self.min = v;
        self
    }

    pub fn max(mut self, v: f32) -> Self {
        self.max = v;
        self
    }

    pub fn label(mut self, l: impl Into<String>) -> Self {
        self.label = Some(l.into());
        self
    }

    pub fn suffix(mut self, s: impl Into<String>) -> Self {
        self.suffix = Some(s.into());
        self
    }

    pub fn show_value(mut self, v: bool) -> Self {
        self.show_value = v;
        self
    }

    pub fn disabled(mut self, v: bool) -> Self {
        self.disabled = v;
        self
    }

    pub fn size(mut self, s: SliderSize) -> Self {
        self.size = s;
        self
    }

    pub fn color(mut self, c: gpui::Hsla) -> Self {
        self.color = Some(c);
        self
    }

    pub fn render(self, theme: &Theme) -> impl IntoElement {
        let range = (self.max - self.min).max(f32::EPSILON);
        let ratio = ((self.value - self.min) / range).clamp(0.0, 1.0);

        let track_h = match self.size {
            SliderSize::Sm => px(4.0),
            SliderSize::Default => px(6.0),
            SliderSize::Lg => px(8.0),
        };
        let thumb_size = match self.size {
            SliderSize::Sm => px(12.0),
            SliderSize::Default => px(16.0),
            SliderSize::Lg => px(20.0),
        };

        let fill_color = self.color.unwrap_or(theme.primary);
        let track_color = theme.muted;

        let mut wrapper = div().flex().flex_col().gap(px(4.0));

        // Label row
        if self.label.is_some() || self.show_value {
            let mut label_row = div().flex().items_center().justify_between();

            if let Some(label) = &self.label {
                label_row = label_row.child(
                    div()
                        .text_sm()
                        .font_weight(gpui::FontWeight::MEDIUM)
                        .text_color(theme.foreground)
                        .child(label.clone()),
                );
            }

            if self.show_value {
                let display = if let Some(ref sfx) = self.suffix {
                    format!("{:.0}{}", self.value, sfx)
                } else {
                    format!("{:.2}", self.value)
                };
                label_row = label_row
                    .child(div().text_sm().text_color(theme.muted_foreground).child(display));
            }

            wrapper = wrapper.child(label_row);
        }

        // Track
        let track = div()
            .flex()
            .items_center()
            .relative()
            .w_full()
            .h(track_h)
            .rounded(Radius::FULL)
            .bg(track_color)
            .overflow_hidden()
            // Filled portion
            .child(
                div()
                    .absolute()
                    .top_0()
                    .left_0()
                    .h_full()
                    .w(gpui::relative(ratio))
                    .bg(fill_color)
                    .rounded(Radius::FULL),
            );

        // Thumb (positioned at ratio)
        let thumb = div()
            .w(thumb_size)
            .h(thumb_size)
            .rounded(Radius::FULL)
            .bg(theme.background)
            .border_2()
            .border_color(fill_color)
            .shadow_sm()
            .cursor_pointer();

        let slider_row = div().flex().items_center().gap(px(4.0)).child(track).child(thumb);

        wrapper = wrapper.child(slider_row);

        if self.disabled {
            wrapper = wrapper.opacity(0.5).cursor_default();
        }

        wrapper
    }
}

// ─── RangeSlider ────────────────────────────────────────────────────────────
// Two-thumb range slider for min/max selection.
//
// Usage:
//   RangeSlider::new("price")
//       .low(20.0)
//       .high(80.0)
//       .min(0.0).max(100.0)
//       .render(&theme)

pub struct RangeSlider {
    id: String,
    low: f32,
    high: f32,
    min: f32,
    max: f32,
    label: Option<String>,
    show_values: bool,
    disabled: bool,
    color: Option<gpui::Hsla>,
}

#[allow(dead_code)]
impl RangeSlider {
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            low: 0.0,
            high: 1.0,
            min: 0.0,
            max: 1.0,
            label: None,
            show_values: false,
            disabled: false,
            color: None,
        }
    }

    pub fn low(mut self, v: f32) -> Self {
        self.low = v;
        self
    }

    pub fn high(mut self, v: f32) -> Self {
        self.high = v;
        self
    }

    pub fn min(mut self, v: f32) -> Self {
        self.min = v;
        self
    }

    pub fn max(mut self, v: f32) -> Self {
        self.max = v;
        self
    }

    pub fn label(mut self, l: impl Into<String>) -> Self {
        self.label = Some(l.into());
        self
    }

    pub fn show_values(mut self, v: bool) -> Self {
        self.show_values = v;
        self
    }

    pub fn disabled(mut self, v: bool) -> Self {
        self.disabled = v;
        self
    }

    pub fn color(mut self, c: gpui::Hsla) -> Self {
        self.color = Some(c);
        self
    }

    pub fn render(self, theme: &Theme) -> impl IntoElement {
        let range = (self.max - self.min).max(f32::EPSILON);
        let low_ratio = ((self.low - self.min) / range).clamp(0.0, 1.0);
        let high_ratio = ((self.high - self.min) / range).clamp(0.0, 1.0);
        let fill_color = self.color.unwrap_or(theme.primary);

        let mut wrapper = div().flex().flex_col().gap(px(4.0));

        if let Some(label) = &self.label {
            let mut label_row = div().flex().items_center().justify_between().child(
                div()
                    .text_sm()
                    .font_weight(gpui::FontWeight::MEDIUM)
                    .text_color(theme.foreground)
                    .child(label.clone()),
            );

            if self.show_values {
                label_row = label_row.child(
                    div()
                        .text_sm()
                        .text_color(theme.muted_foreground)
                        .child(format!("{:.0} – {:.0}", self.low, self.high)),
                );
            }

            wrapper = wrapper.child(label_row);
        }

        // Track with range highlight
        let track = div()
            .relative()
            .w_full()
            .h(px(6.0))
            .rounded(Radius::FULL)
            .bg(theme.muted)
            .child(
                div()
                    .absolute()
                    .top_0()
                    .h_full()
                    .left(gpui::relative(low_ratio))
                    .w(gpui::relative(high_ratio - low_ratio))
                    .bg(fill_color)
                    .rounded(Radius::FULL),
            );

        let thumb = || {
            div()
                .w(px(16.0))
                .h(px(16.0))
                .rounded(Radius::FULL)
                .bg(theme.background)
                .border_2()
                .border_color(fill_color)
                .shadow_sm()
                .cursor_pointer()
        };

        let slider_row = div()
            .flex()
            .items_center()
            .gap(px(4.0))
            .child(thumb())
            .child(track)
            .child(thumb());

        wrapper = wrapper.child(slider_row);

        if self.disabled {
            wrapper = wrapper.opacity(0.5).cursor_default();
        }

        wrapper
    }
}
