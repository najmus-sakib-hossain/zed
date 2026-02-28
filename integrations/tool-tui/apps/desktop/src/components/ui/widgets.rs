use gpui::{div, prelude::*, px, AnyElement, Hsla, IntoElement};

use crate::theme::{colors::Radius, Theme};

// ─── Stepper ────────────────────────────────────────────────────────────────
// Number stepper / spinner with increment/decrement buttons.
//
// Usage:
//   Stepper::new("quantity")
//       .value(5)
//       .min(0)
//       .max(100)
//       .label("Quantity")
//       .render(&theme)

pub struct Stepper {
    _id: String,
    value: i64,
    min: Option<i64>,
    max: Option<i64>,
    step: i64,
    label: Option<String>,
    disabled: bool,
}

#[allow(dead_code)]
impl Stepper {
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            _id: id.into(),
            value: 0,
            min: None,
            max: None,
            step: 1,
            label: None,
            disabled: false,
        }
    }

    pub fn value(mut self, v: i64) -> Self {
        self.value = v;
        self
    }

    pub fn min(mut self, v: i64) -> Self {
        self.min = Some(v);
        self
    }

    pub fn max(mut self, v: i64) -> Self {
        self.max = Some(v);
        self
    }

    pub fn step(mut self, s: i64) -> Self {
        self.step = s;
        self
    }

    pub fn label(mut self, l: impl Into<String>) -> Self {
        self.label = Some(l.into());
        self
    }

    pub fn disabled(mut self, v: bool) -> Self {
        self.disabled = v;
        self
    }

    pub fn render(self, theme: &Theme) -> impl IntoElement {
        let btn_hover = theme.muted;

        let mut container = div().flex().flex_col().gap(px(4.0));

        if let Some(label) = self.label {
            container = container.child(
                div()
                    .text_size(px(12.0))
                    .font_weight(gpui::FontWeight::MEDIUM)
                    .text_color(theme.foreground)
                    .child(label),
            );
        }

        let mut stepper = div()
            .flex()
            .items_center()
            .h(px(32.0))
            .border_1()
            .border_color(theme.border)
            .rounded(Radius::MD)
            .bg(theme.background);

        if self.disabled {
            stepper = stepper.opacity(0.5);
        }

        // Decrement
        let can_dec = self.min.is_none_or(|m| self.value > m);
        let mut dec_btn = div()
            .flex()
            .items_center()
            .justify_center()
            .w(px(32.0))
            .h_full()
            .border_r_1()
            .border_color(theme.border)
            .text_size(px(14.0))
            .text_color(if can_dec && !self.disabled {
                theme.foreground
            } else {
                theme.muted_foreground
            })
            .child("−");

        if can_dec && !self.disabled {
            dec_btn = dec_btn.cursor_pointer().hover(move |s| s.bg(btn_hover));
        }

        stepper = stepper.child(dec_btn);

        // Value display
        stepper = stepper.child(
            div()
                .flex()
                .items_center()
                .justify_center()
                .min_w(px(48.0))
                .h_full()
                .px(px(8.0))
                .text_size(px(13.0))
                .text_color(theme.foreground)
                .child(self.value.to_string()),
        );

        // Increment
        let can_inc = self.max.is_none_or(|m| self.value < m);
        let inc_hover = theme.muted;
        let mut inc_btn = div()
            .flex()
            .items_center()
            .justify_center()
            .w(px(32.0))
            .h_full()
            .border_l_1()
            .border_color(theme.border)
            .text_size(px(14.0))
            .text_color(if can_inc && !self.disabled {
                theme.foreground
            } else {
                theme.muted_foreground
            })
            .child("+");

        if can_inc && !self.disabled {
            inc_btn = inc_btn.cursor_pointer().hover(move |s| s.bg(inc_hover));
        }

        stepper = stepper.child(inc_btn);

        container = container.child(stepper);

        container
    }
}

// ─── ToggleGroup ────────────────────────────────────────────────────────────
// Segmented / toggle button group for selecting one of N options.

pub struct ToggleGroup {
    items: Vec<ToggleGroupItem>,
    compact: bool,
}

pub struct ToggleGroupItem {
    label: String,
    icon: Option<String>,
    active: bool,
    disabled: bool,
}

#[allow(dead_code)]
impl ToggleGroup {
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            compact: false,
        }
    }

    pub fn item(mut self, item: ToggleGroupItem) -> Self {
        self.items.push(item);
        self
    }

    pub fn items(mut self, items: Vec<ToggleGroupItem>) -> Self {
        self.items = items;
        self
    }

    pub fn compact(mut self, v: bool) -> Self {
        self.compact = v;
        self
    }

    pub fn render(self, theme: &Theme) -> impl IntoElement {
        let h = if self.compact { 28.0 } else { 32.0 };
        let px_val = if self.compact { 8.0 } else { 12.0 };

        let mut group = div().flex().items_center().bg(theme.muted).rounded(Radius::MD).p(px(2.0));

        for item in self.items {
            let active_bg = if item.active {
                theme.background
            } else {
                gpui::transparent_black()
            };
            let fg = if item.disabled {
                theme.muted_foreground
            } else if item.active {
                theme.foreground
            } else {
                theme.muted_foreground
            };
            let hover_bg = theme.background;

            let mut btn = div()
                .flex()
                .items_center()
                .justify_center()
                .gap(px(4.0))
                .h(px(h))
                .px(px(px_val))
                .rounded(Radius::SM)
                .bg(active_bg)
                .text_color(fg)
                .text_size(px(12.0));

            if item.active {
                btn = btn.shadow_sm();
            }

            if !item.disabled {
                btn = btn.cursor_pointer().hover(move |s| s.bg(hover_bg));
            }

            if let Some(icon) = item.icon {
                btn = btn.child(div().text_size(px(14.0)).child(icon));
            }

            btn = btn.child(item.label);
            group = group.child(btn);
        }

        group
    }
}

#[allow(dead_code)]
impl ToggleGroupItem {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            icon: None,
            active: false,
            disabled: false,
        }
    }

    pub fn icon(mut self, icon: impl Into<String>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    pub fn active(mut self, v: bool) -> Self {
        self.active = v;
        self
    }

    pub fn disabled(mut self, v: bool) -> Self {
        self.disabled = v;
        self
    }
}

// ─── Timeline ───────────────────────────────────────────────────────────────
// Vertical timeline / activity log.

pub struct Timeline {
    items: Vec<TimelineItem>,
}

pub struct TimelineItem {
    title: String,
    description: Option<String>,
    timestamp: Option<String>,
    icon: Option<String>,
    color: Option<Hsla>,
    content: Option<AnyElement>,
}

#[allow(dead_code)]
impl Timeline {
    pub fn new() -> Self {
        Self { items: Vec::new() }
    }

    pub fn item(mut self, item: TimelineItem) -> Self {
        self.items.push(item);
        self
    }

    pub fn items(mut self, items: Vec<TimelineItem>) -> Self {
        self.items = items;
        self
    }

    pub fn render(self, theme: &Theme) -> impl IntoElement {
        let mut container = div().flex().flex_col().gap(px(0.0));

        let total = self.items.len();
        for (i, item) in self.items.into_iter().enumerate() {
            let is_last = i == total - 1;
            container = container.child(item.render(theme, is_last));
        }

        container
    }
}

#[allow(dead_code)]
impl TimelineItem {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            description: None,
            timestamp: None,
            icon: None,
            color: None,
            content: None,
        }
    }

    pub fn description(mut self, d: impl Into<String>) -> Self {
        self.description = Some(d.into());
        self
    }

    pub fn timestamp(mut self, t: impl Into<String>) -> Self {
        self.timestamp = Some(t.into());
        self
    }

    pub fn icon(mut self, icon: impl Into<String>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    pub fn color(mut self, color: Hsla) -> Self {
        self.color = Some(color);
        self
    }

    pub fn content(mut self, content: impl IntoElement) -> Self {
        self.content = Some(content.into_any_element());
        self
    }

    fn render(self, theme: &Theme, is_last: bool) -> impl IntoElement {
        let dot_color = self.color.unwrap_or(theme.primary);

        let mut row = div().flex().gap(px(12.0)).w_full();

        // Left: dot + line
        let mut left = div().flex().flex_col().items_center().w(px(20.0)).flex_shrink_0();

        // Dot
        let mut dot = div()
            .w(px(10.0))
            .h(px(10.0))
            .rounded(Radius::FULL)
            .bg(dot_color)
            .flex_shrink_0();

        if let Some(icon) = self.icon {
            dot = div()
                .w(px(20.0))
                .h(px(20.0))
                .rounded(Radius::FULL)
                .bg(dot_color)
                .flex()
                .items_center()
                .justify_center()
                .text_color(theme.primary_foreground)
                .text_size(px(10.0))
                .flex_shrink_0()
                .child(icon);
        }

        left = left.child(dot);

        // Connecting line
        if !is_last {
            left = left.child(div().w(px(2.0)).flex_1().min_h(px(24.0)).bg(theme.border));
        }

        row = row.child(left);

        // Right: content
        let mut right = div().flex().flex_col().gap(px(2.0)).pb(px(16.0)).flex_1();

        // Title row with timestamp
        let mut title_row = div().flex().items_center().justify_between().w_full();

        title_row = title_row.child(
            div()
                .text_size(px(13.0))
                .font_weight(gpui::FontWeight::MEDIUM)
                .text_color(theme.foreground)
                .child(self.title),
        );

        if let Some(ts) = self.timestamp {
            title_row = title_row
                .child(div().text_size(px(11.0)).text_color(theme.muted_foreground).child(ts));
        }

        right = right.child(title_row);

        if let Some(desc) = self.description {
            right = right
                .child(div().text_size(px(12.0)).text_color(theme.muted_foreground).child(desc));
        }

        if let Some(content) = self.content {
            right = right.child(div().mt(px(4.0)).child(content));
        }

        row = row.child(right);

        row
    }
}

// ─── Steps ──────────────────────────────────────────────────────────────────
// Horizontal steps/wizard indicator.

#[derive(Clone, Copy, PartialEq)]
pub enum StepStatus {
    Completed,
    Current,
    Upcoming,
}

pub struct Steps {
    items: Vec<StepItem>,
}

pub struct StepItem {
    label: String,
    status: StepStatus,
}

#[allow(dead_code)]
impl Steps {
    pub fn new() -> Self {
        Self { items: Vec::new() }
    }

    pub fn step(mut self, item: StepItem) -> Self {
        self.items.push(item);
        self
    }

    pub fn render(self, theme: &Theme) -> impl IntoElement {
        let mut row = div().flex().items_center().gap(px(4.0)).w_full();

        let total = self.items.len();
        for (i, step) in self.items.into_iter().enumerate() {
            // Step circle + label
            let (circle_bg, circle_fg, label_color) = match step.status {
                StepStatus::Completed => {
                    (theme.primary, theme.primary_foreground, theme.foreground)
                }
                StepStatus::Current => (theme.primary, theme.primary_foreground, theme.foreground),
                StepStatus::Upcoming => {
                    (theme.muted, theme.muted_foreground, theme.muted_foreground)
                }
            };

            let number = if matches!(step.status, StepStatus::Completed) {
                "✓".to_string()
            } else {
                (i + 1).to_string()
            };

            let step_el = div()
                .flex()
                .items_center()
                .gap(px(6.0))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_center()
                        .w(px(24.0))
                        .h(px(24.0))
                        .rounded(Radius::FULL)
                        .bg(circle_bg)
                        .text_color(circle_fg)
                        .text_size(px(11.0))
                        .font_weight(gpui::FontWeight::BOLD)
                        .child(number),
                )
                .child(
                    div()
                        .text_size(px(12.0))
                        .font_weight(if matches!(step.status, StepStatus::Current) {
                            gpui::FontWeight::SEMIBOLD
                        } else {
                            gpui::FontWeight::NORMAL
                        })
                        .text_color(label_color)
                        .child(step.label),
                );

            row = row.child(step_el);

            // Connector line
            if i < total - 1 {
                row = row.child(
                    div()
                        .flex_1()
                        .h(px(2.0))
                        .mx(px(4.0))
                        .bg(if matches!(step.status, StepStatus::Completed) {
                            theme.primary
                        } else {
                            theme.border
                        })
                        .rounded(Radius::FULL),
                );
            }
        }

        row
    }
}

#[allow(dead_code)]
impl StepItem {
    pub fn new(label: impl Into<String>, status: StepStatus) -> Self {
        Self {
            label: label.into(),
            status,
        }
    }
}
