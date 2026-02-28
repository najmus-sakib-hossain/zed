//! Time picker for selecting time

use super::interaction::{Event, PromptInteraction, State};
use super::{SYMBOLS, THEME};
use chrono::{Local, Timelike};
use console::Term;
use owo_colors::OwoColorize;
use std::io;

pub struct TimePicker {
    message: String,
    hour: u32,
    minute: u32,
    second: u32,
    use_24h: bool,
    active_field: TimeField,
    state: State,
    last_render_lines: usize,
}

#[derive(PartialEq)]
enum TimeField {
    Hour,
    Minute,
    Second,
}

impl TimePicker {
    pub fn new(message: impl Into<String>) -> Self {
        let now = chrono::Local::now();
        Self {
            message: message.into(),
            hour: now.hour(),
            minute: now.minute(),
            second: now.second(),
            use_24h: true,
            active_field: TimeField::Hour,
            state: State::Active,
            last_render_lines: 0,
        }
    }

    pub fn initial_time(mut self, hour: u32, minute: u32, second: u32) -> Self {
        self.hour = hour.clamp(0, 23);
        self.minute = minute.clamp(0, 59);
        self.second = second.clamp(0, 59);
        self
    }

    pub fn format_24h(mut self, use_24h: bool) -> Self {
        self.use_24h = use_24h;
        self
    }

    fn format_hour(&self) -> String {
        if self.use_24h {
            format!("{:02}", self.hour)
        } else {
            let h = if self.hour == 0 {
                12
            } else if self.hour > 12 {
                self.hour - 12
            } else {
                self.hour
            };
            format!("{:02}", h)
        }
    }

    fn am_pm(&self) -> &'static str {
        if self.hour < 12 { "AM" } else { "PM" }
    }
}

impl PromptInteraction for TimePicker {
    type Output = String;

    fn state(&self) -> State {
        self.state
    }

    fn on(&mut self, event: Event) {
        match event {
            Event::Key(key) => match key {
                console::Key::Enter => self.state = State::Submit,
                console::Key::Escape => self.state = State::Cancel,
                console::Key::Tab => {
                    self.active_field = match self.active_field {
                        TimeField::Hour => TimeField::Minute,
                        TimeField::Minute => TimeField::Second,
                        TimeField::Second => TimeField::Hour,
                    };
                }
                console::Key::ArrowUp => match self.active_field {
                    TimeField::Hour => {
                        self.hour = if self.hour == 23 { 0 } else { self.hour + 1 };
                    }
                    TimeField::Minute => {
                        self.minute = if self.minute == 59 {
                            0
                        } else {
                            self.minute + 1
                        };
                    }
                    TimeField::Second => {
                        self.second = if self.second == 59 {
                            0
                        } else {
                            self.second + 1
                        };
                    }
                },
                console::Key::ArrowDown => match self.active_field {
                    TimeField::Hour => {
                        self.hour = if self.hour == 0 { 23 } else { self.hour - 1 };
                    }
                    TimeField::Minute => {
                        self.minute = if self.minute == 0 {
                            59
                        } else {
                            self.minute - 1
                        };
                    }
                    TimeField::Second => {
                        self.second = if self.second == 0 {
                            59
                        } else {
                            self.second - 1
                        };
                    }
                },
                _ => {}
            },
            Event::Error => self.state = State::Error,
        }
    }

    fn render(&mut self, term: &Term) -> io::Result<()> {
        if self.last_render_lines > 0 {
            for _ in 0..self.last_render_lines {
                term.move_cursor_up(1)?;
                term.clear_line()?;
            }
        }

        let theme = THEME.read().unwrap();
        let symbols = &*SYMBOLS;
        let mut lines = 0;

        match self.state {
            State::Active => {
                let bar = theme.dim.apply_to(symbols.bar);
                term.write_line(&format!(
                    "{}{}",
                    theme.primary.apply_to(symbols.step_submit),
                    format!("  {}  ", self.message).bold()
                ))?;
                lines += 1;

                term.write_line(&format!("{}", bar))?;
                lines += 1;

                let hour_marker = if self.active_field == TimeField::Hour {
                    "▸"
                } else {
                    " "
                };
                let minute_marker = if self.active_field == TimeField::Minute {
                    "▸"
                } else {
                    " "
                };
                let second_marker = if self.active_field == TimeField::Second {
                    "▸"
                } else {
                    " "
                };

                let hour_display = if self.active_field == TimeField::Hour {
                    theme.primary.apply_to(self.format_hour()).bold().to_string()
                } else {
                    self.format_hour()
                };

                let minute_display = if self.active_field == TimeField::Minute {
                    theme.primary.apply_to(format!("{:02}", self.minute)).bold().to_string()
                } else {
                    format!("{:02}", self.minute)
                };

                let second_display = if self.active_field == TimeField::Second {
                    theme.primary.apply_to(format!("{:02}", self.second)).bold().to_string()
                } else {
                    format!("{:02}", self.second)
                };

                term.write_line(&format!("{}  {} Hour:   {}", bar, hour_marker, hour_display))?;
                lines += 1;

                term.write_line(&format!("{}  {} Minute: {}", bar, minute_marker, minute_display))?;
                lines += 1;

                term.write_line(&format!("{}  {} Second: {}", bar, second_marker, second_display))?;
                lines += 1;

                term.write_line(&format!("{}", bar))?;
                lines += 1;

                let time_display = if self.use_24h {
                    format!("🕐 {:02}:{:02}:{:02}", self.hour, self.minute, self.second)
                } else {
                    format!(
                        "🕐 {} {}",
                        format!("{}:{:02}:{:02}", self.format_hour(), self.minute, self.second),
                        self.am_pm()
                    )
                };

                term.write_line(&format!("{}  {}", bar, theme.primary.apply_to(time_display)))?;
                lines += 1;

                term.write_line(&format!(
                    "{}  {}",
                    bar,
                    theme.dim.apply_to("Tab: switch field, ↑↓: adjust, Enter: confirm")
                ))?;
                lines += 1;
            }
            State::Submit => {
                let checkmark = theme.success.apply_to("✓");
                let time_str = format!("{:02}:{:02}:{:02}", self.hour, self.minute, self.second);
                term.write_line(&format!(
                    "{} {}  {}",
                    checkmark,
                    self.message,
                    theme.dim.apply_to(time_str)
                ))?;
                lines += 1;
                term.write_line(&format!("{}", theme.dim.apply_to(symbols.bar)))?;
                lines += 1;
            }
            _ => {}
        }

        self.last_render_lines = lines;
        Ok(())
    }

    fn value(&self) -> String {
        format!("{:02}:{:02}:{:02}", self.hour, self.minute, self.second)
    }
}

pub fn time_picker(message: impl Into<String>) -> TimePicker {
    TimePicker::new(message)
}
