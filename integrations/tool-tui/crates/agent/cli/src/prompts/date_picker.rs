//! Date picker for selecting dates

use super::interaction::{Event, PromptInteraction, State};
use super::{SYMBOLS, THEME};
use chrono::{Datelike, Local};
use console::Term;
use owo_colors::OwoColorize;
use std::io;

pub struct DatePicker {
    message: String,
    year: i32,
    month: u32,
    day: u32,
    active_field: DateField,
    state: State,
    last_render_lines: usize,
}

#[derive(PartialEq)]
enum DateField {
    Year,
    Month,
    Day,
}

impl DatePicker {
    pub fn new(message: impl Into<String>) -> Self {
        let now = chrono::Local::now();
        Self {
            message: message.into(),
            year: now.year(),
            month: now.month(),
            day: now.day(),
            active_field: DateField::Year,
            state: State::Active,
            last_render_lines: 0,
        }
    }

    pub fn initial_date(mut self, year: i32, month: u32, day: u32) -> Self {
        self.year = year;
        self.month = month.clamp(1, 12);
        self.day = day.clamp(1, self.days_in_month());
        self
    }

    fn days_in_month(&self) -> u32 {
        match self.month {
            1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
            4 | 6 | 9 | 11 => 30,
            2 => {
                if self.is_leap_year() {
                    29
                } else {
                    28
                }
            }
            _ => 31,
        }
    }

    fn is_leap_year(&self) -> bool {
        (self.year % 4 == 0 && self.year % 100 != 0) || (self.year % 400 == 0)
    }

    fn month_name(&self) -> &'static str {
        match self.month {
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
}

impl PromptInteraction for DatePicker {
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
                        DateField::Year => DateField::Month,
                        DateField::Month => DateField::Day,
                        DateField::Day => DateField::Year,
                    };
                }
                console::Key::ArrowUp => match self.active_field {
                    DateField::Year => self.year += 1,
                    DateField::Month => {
                        self.month = if self.month == 12 { 1 } else { self.month + 1 };
                        self.day = self.day.min(self.days_in_month());
                    }
                    DateField::Day => {
                        self.day = if self.day >= self.days_in_month() {
                            1
                        } else {
                            self.day + 1
                        };
                    }
                },
                console::Key::ArrowDown => match self.active_field {
                    DateField::Year => self.year -= 1,
                    DateField::Month => {
                        self.month = if self.month == 1 { 12 } else { self.month - 1 };
                        self.day = self.day.min(self.days_in_month());
                    }
                    DateField::Day => {
                        self.day = if self.day == 1 {
                            self.days_in_month()
                        } else {
                            self.day - 1
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

                let year_marker = if self.active_field == DateField::Year {
                    "▸"
                } else {
                    " "
                };
                let month_marker = if self.active_field == DateField::Month {
                    "▸"
                } else {
                    " "
                };
                let day_marker = if self.active_field == DateField::Day {
                    "▸"
                } else {
                    " "
                };

                let year_display = if self.active_field == DateField::Year {
                    theme
                        .primary
                        .apply_to(self.year.to_string())
                        .bold()
                        .to_string()
                } else {
                    self.year.to_string()
                };

                let month_display = if self.active_field == DateField::Month {
                    theme
                        .primary
                        .apply_to(format!("{:02}", self.month))
                        .bold()
                        .to_string()
                } else {
                    format!("{:02}", self.month)
                };

                let day_display = if self.active_field == DateField::Day {
                    theme
                        .primary
                        .apply_to(format!("{:02}", self.day))
                        .bold()
                        .to_string()
                } else {
                    format!("{:02}", self.day)
                };

                term.write_line(&format!("{}  {} Year:  {}", bar, year_marker, year_display))?;
                lines += 1;

                term.write_line(&format!(
                    "{}  {} Month: {} ({})",
                    bar,
                    month_marker,
                    month_display,
                    self.month_name()
                ))?;
                lines += 1;

                term.write_line(&format!("{}  {} Day:   {}", bar, day_marker, day_display))?;
                lines += 1;

                term.write_line(&format!("{}", bar))?;
                lines += 1;

                term.write_line(&format!(
                    "{}  📅 {}-{:02}-{:02}",
                    bar,
                    theme.primary.apply_to(self.year),
                    theme.primary.apply_to(self.month),
                    theme.primary.apply_to(self.day)
                ))?;
                lines += 1;

                term.write_line(&format!(
                    "{}  {}",
                    bar,
                    theme
                        .dim
                        .apply_to("Tab: switch field, ↑↓: adjust, Enter: confirm")
                ))?;
                lines += 1;
            }
            State::Submit => {
                let checkmark = theme.success.apply_to("✓");
                term.write_line(&format!(
                    "{} {}  {}",
                    checkmark,
                    self.message,
                    theme
                        .dim
                        .apply_to(format!("{}-{:02}-{:02}", self.year, self.month, self.day))
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
        format!("{}-{:02}-{:02}", self.year, self.month, self.day)
    }
}

pub fn date_picker(message: impl Into<String>) -> DatePicker {
    DatePicker::new(message)
}
