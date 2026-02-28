use gpui::{div, prelude::*, px, IntoElement};

use crate::theme::{colors::Radius, Theme};

// ─── Calendar ───────────────────────────────────────────────────────────────
// A simple month calendar grid.
//
// Usage:
//   Calendar::new(2025, 6)
//       .selected_day(15)
//       .today(12)
//       .render(&theme)

pub struct Calendar {
    year: u32,
    month: u32,
    selected_day: Option<u32>,
    today: Option<u32>,
    first_day_offset: u32, // 0 = Sunday start
}

#[allow(dead_code)]
impl Calendar {
    pub fn new(year: u32, month: u32) -> Self {
        Self {
            year,
            month,
            selected_day: None,
            today: None,
            first_day_offset: 0,
        }
    }

    pub fn selected_day(mut self, day: u32) -> Self {
        self.selected_day = Some(day);
        self
    }

    pub fn today(mut self, day: u32) -> Self {
        self.today = Some(day);
        self
    }

    /// 0 = Sunday, 1 = Monday, etc.
    pub fn first_day_offset(mut self, offset: u32) -> Self {
        self.first_day_offset = offset;
        self
    }

    fn days_in_month(year: u32, month: u32) -> u32 {
        match month {
            1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
            4 | 6 | 9 | 11 => 30,
            2 => {
                if (year.is_multiple_of(4) && !year.is_multiple_of(100)) || year.is_multiple_of(400)
                {
                    29
                } else {
                    28
                }
            }
            _ => 30,
        }
    }

    fn day_of_week(year: u32, month: u32, day: u32) -> u32 {
        // Zeller's congruence (simplified)
        let (y, m) = if month <= 2 {
            (year as i32 - 1, month as i32 + 12)
        } else {
            (year as i32, month as i32)
        };
        let q = day as i32;
        let k = y % 100;
        let j = y / 100;
        let h = (q + (13 * (m + 1)) / 5 + k + k / 4 + j / 4 - 2 * j) % 7;
        ((h + 6) % 7) as u32 // 0=Sunday
    }

    fn month_name(month: u32) -> &'static str {
        match month {
            1 => "January",
            2 => "February",
            3 => "March",
            4 => "April",
            5 => "May",
            6 => "June",
            7 => "July",
            8 => "August",
            9 => "September",
            10 => "October",
            11 => "November",
            12 => "December",
            _ => "Unknown",
        }
    }

    pub fn render(self, theme: &Theme) -> impl IntoElement {
        let cell_size = 32.0;
        let total_days = Self::days_in_month(self.year, self.month);
        let first_dow = Self::day_of_week(self.year, self.month, 1);
        let start_offset = (first_dow + 7 - self.first_day_offset) % 7;

        let mut cal = div()
            .flex()
            .flex_col()
            .gap(px(4.0))
            .p(px(12.0))
            .bg(theme.card)
            .border_1()
            .border_color(theme.border)
            .rounded(Radius::MD);

        // Header: month/year with nav
        let nav_hover = theme.muted;
        cal = cal.child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .pb(px(8.0))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_center()
                        .w(px(24.0))
                        .h(px(24.0))
                        .rounded(Radius::SM)
                        .text_color(theme.muted_foreground)
                        .text_size(px(12.0))
                        .cursor_pointer()
                        .hover(move |s| s.bg(nav_hover))
                        .child("‹"),
                )
                .child(
                    div()
                        .text_size(px(13.0))
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .text_color(theme.foreground)
                        .child(format!("{} {}", Self::month_name(self.month), self.year)),
                )
                .child({
                    let nav_hover2 = theme.muted;
                    div()
                        .flex()
                        .items_center()
                        .justify_center()
                        .w(px(24.0))
                        .h(px(24.0))
                        .rounded(Radius::SM)
                        .text_color(theme.muted_foreground)
                        .text_size(px(12.0))
                        .cursor_pointer()
                        .hover(move |s| s.bg(nav_hover2))
                        .child("›")
                }),
        );

        // Day headers
        let day_labels = if self.first_day_offset == 1 {
            ["Mo", "Tu", "We", "Th", "Fr", "Sa", "Su"]
        } else {
            ["Su", "Mo", "Tu", "We", "Th", "Fr", "Sa"]
        };

        let mut header_row = div().flex().gap(px(2.0));
        for label in &day_labels {
            header_row = header_row.child(
                div()
                    .flex()
                    .items_center()
                    .justify_center()
                    .w(px(cell_size))
                    .h(px(cell_size))
                    .text_size(px(10.0))
                    .font_weight(gpui::FontWeight::MEDIUM)
                    .text_color(theme.muted_foreground)
                    .child(*label),
            );
        }
        cal = cal.child(header_row);

        // Day grid
        let mut current_day: u32 = 1;
        let weeks = ((start_offset + total_days) as f32 / 7.0).ceil() as u32;

        for week in 0..weeks {
            let mut week_row = div().flex().gap(px(2.0));

            for dow in 0..7u32 {
                let cell_index = week * 7 + dow;

                if cell_index < start_offset || current_day > total_days {
                    // Empty cell
                    week_row = week_row.child(div().w(px(cell_size)).h(px(cell_size)));
                } else {
                    let day = current_day;
                    let is_selected = self.selected_day == Some(day);
                    let is_today = self.today == Some(day);

                    let bg = if is_selected {
                        theme.primary
                    } else {
                        gpui::transparent_black()
                    };
                    let fg = if is_selected {
                        theme.primary_foreground
                    } else {
                        theme.foreground
                    };
                    let hover_bg = theme.accent;

                    let mut cell = div()
                        .flex()
                        .items_center()
                        .justify_center()
                        .w(px(cell_size))
                        .h(px(cell_size))
                        .rounded(Radius::SM)
                        .bg(bg)
                        .text_color(fg)
                        .text_size(px(12.0))
                        .cursor_pointer()
                        .hover(move |s| s.bg(hover_bg));

                    if is_today && !is_selected {
                        cell = cell.border_1().border_color(theme.primary);
                    }

                    cell = cell.child(day.to_string());

                    week_row = week_row.child(cell);
                    current_day += 1;
                }
            }

            cal = cal.child(week_row);
        }

        cal
    }
}

// ─── DatePicker ─────────────────────────────────────────────────────────────
// Date picker field with calendar popup.

pub struct DatePicker {
    _id: String,
    label: Option<String>,
    value: Option<String>,
    placeholder: String,
    disabled: bool,
    calendar: Option<Calendar>,
    open: bool,
}

#[allow(dead_code)]
impl DatePicker {
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            _id: id.into(),
            label: None,
            value: None,
            placeholder: "Pick a date".to_string(),
            disabled: false,
            calendar: None,
            open: false,
        }
    }

    pub fn label(mut self, l: impl Into<String>) -> Self {
        self.label = Some(l.into());
        self
    }

    pub fn value(mut self, v: impl Into<String>) -> Self {
        self.value = Some(v.into());
        self
    }

    pub fn placeholder(mut self, p: impl Into<String>) -> Self {
        self.placeholder = p.into();
        self
    }

    pub fn disabled(mut self, v: bool) -> Self {
        self.disabled = v;
        self
    }

    pub fn calendar(mut self, cal: Calendar) -> Self {
        self.calendar = Some(cal);
        self
    }

    pub fn open(mut self, v: bool) -> Self {
        self.open = v;
        self
    }

    pub fn render(self, theme: &Theme) -> impl IntoElement {
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

        // Trigger button
        let display_text = self.value.unwrap_or(self.placeholder);
        let has_value = !display_text.starts_with("Pick");
        let text_color = if has_value {
            theme.foreground
        } else {
            theme.muted_foreground
        };

        let hover_border = theme.ring;
        let mut trigger = div()
            .flex()
            .items_center()
            .justify_between()
            .h(px(36.0))
            .px(px(12.0))
            .border_1()
            .border_color(theme.border)
            .rounded(Radius::MD)
            .bg(theme.background)
            .text_color(text_color)
            .text_size(px(13.0))
            .cursor_pointer()
            .hover(move |s| s.border_color(hover_border));

        if self.disabled {
            trigger = trigger.opacity(0.5).cursor_default();
        }

        trigger = trigger.child(display_text);
        trigger =
            trigger.child(div().text_size(px(12.0)).text_color(theme.muted_foreground).child("📅"));

        let mut wrapper = div().relative();
        wrapper = wrapper.child(trigger);

        // Calendar dropdown
        if self.open {
            if let Some(cal) = self.calendar {
                wrapper = wrapper.child(
                    div().absolute().top(px(40.0)).left_0().shadow_lg().child(cal.render(theme)),
                );
            }
        }

        container = container.child(wrapper);

        container
    }
}

// ─── TimePicker ─────────────────────────────────────────────────────────────
// Simple time picker display.

pub struct TimePicker {
    _id: String,
    hour: u32,
    minute: u32,
    label: Option<String>,
    use_24h: bool,
    disabled: bool,
}

#[allow(dead_code)]
impl TimePicker {
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            _id: id.into(),
            hour: 12,
            minute: 0,
            label: None,
            use_24h: true,
            disabled: false,
        }
    }

    pub fn hour(mut self, h: u32) -> Self {
        self.hour = h.min(23);
        self
    }

    pub fn minute(mut self, m: u32) -> Self {
        self.minute = m.min(59);
        self
    }

    pub fn label(mut self, l: impl Into<String>) -> Self {
        self.label = Some(l.into());
        self
    }

    pub fn use_24h(mut self, v: bool) -> Self {
        self.use_24h = v;
        self
    }

    pub fn disabled(mut self, v: bool) -> Self {
        self.disabled = v;
        self
    }

    pub fn render(self, theme: &Theme) -> impl IntoElement {
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

        let time_text = if self.use_24h {
            format!("{:02}:{:02}", self.hour, self.minute)
        } else {
            let period = if self.hour < 12 { "AM" } else { "PM" };
            let h12 = if self.hour == 0 {
                12
            } else if self.hour > 12 {
                self.hour - 12
            } else {
                self.hour
            };
            format!("{:02}:{:02} {}", h12, self.minute, period)
        };

        let hover_border = theme.ring;
        let btn_hover = theme.muted;
        let mut picker = div()
            .flex()
            .items_center()
            .gap(px(4.0))
            .h(px(36.0))
            .px(px(12.0))
            .border_1()
            .border_color(theme.border)
            .rounded(Radius::MD)
            .bg(theme.background)
            .hover(move |s| s.border_color(hover_border));

        if self.disabled {
            picker = picker.opacity(0.5);
        }

        // Up button
        picker = picker.child(
            div()
                .flex()
                .items_center()
                .justify_center()
                .w(px(20.0))
                .h(px(20.0))
                .rounded(Radius::SM)
                .text_color(theme.muted_foreground)
                .text_size(px(10.0))
                .cursor_pointer()
                .hover(move |s| s.bg(btn_hover))
                .child("▲"),
        );

        // Time display
        picker = picker.child(
            div()
                .text_size(px(13.0))
                .font_weight(gpui::FontWeight::MEDIUM)
                .text_color(theme.foreground)
                .child(time_text),
        );

        // Down button
        let btn_hover2 = theme.muted;
        picker = picker.child(
            div()
                .flex()
                .items_center()
                .justify_center()
                .w(px(20.0))
                .h(px(20.0))
                .rounded(Radius::SM)
                .text_color(theme.muted_foreground)
                .text_size(px(10.0))
                .cursor_pointer()
                .hover(move |s| s.bg(btn_hover2))
                .child("▼"),
        );

        // Clock icon
        picker = picker.child(
            div()
                .text_size(px(12.0))
                .text_color(theme.muted_foreground)
                .ml(px(4.0))
                .child("🕐"),
        );

        container = container.child(picker);

        container
    }
}
