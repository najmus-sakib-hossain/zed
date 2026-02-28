//! Calendar view for date selection

use super::interaction::{Event, PromptInteraction, State};
use super::{SYMBOLS, THEME};
use chrono::Datelike;
use console::Term;
use owo_colors::OwoColorize;
use std::io;

pub struct CalendarView {
    message: String,
    year: i32,
    month: u32,
    selected_day: u32,
    state: State,
    last_render_lines: usize,
}

impl CalendarView {
    pub fn new(message: impl Into<String>) -> Self {
        let now = chrono::Local::now();
        Self {
            message: message.into(),
            year: now.year(),
            month: now.month(),
            selected_day: now.day(),
            state: State::Active,
            last_render_lines: 0,
        }
    }

    pub fn initial_date(mut self, year: i32, month: u32, day: u32) -> Self {
        self.year = year;
        self.month = month.clamp(1, 12);
        self.selected_day = day.clamp(1, self.days_in_month());
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

    fn first_day_of_month(&self) -> u32 {
        use chrono::{Datelike, NaiveDate};
        let date = NaiveDate::from_ymd_opt(self.year, self.month, 1).unwrap();
        date.weekday().num_days_from_sunday()
    }

    fn render_calendar(&self) -> Vec<String> {
        let theme = THEME.read().unwrap();
        let mut lines = Vec::new();

        lines.push("  Su Mo Tu We Th Fr Sa".to_string());

        let first_day = self.first_day_of_month();
        let days_in_month = self.days_in_month();

        let mut week = String::from("  ");
        for _ in 0..first_day {
            week.push_str("   ");
        }

        for day in 1..=days_in_month {
            let day_str = if day == self.selected_day {
                theme.primary.apply_to(format!("{:2}", day)).bold().to_string()
            } else {
                format!("{:2}", day)
            };

            week.push_str(&day_str);
            week.push(' ');

            if (first_day + day).is_multiple_of(7) {
                lines.push(week.clone());
                week = String::from("  ");
            }
        }

        if !week.trim().is_empty() {
            lines.push(week);
        }

        lines
    }
}

impl PromptInteraction for CalendarView {
    type Output = String;

    fn state(&self) -> State {
        self.state
    }

    fn on(&mut self, event: Event) {
        match event {
            Event::Key(key) => match key {
                console::Key::Enter => self.state = State::Submit,
                console::Key::Escape => self.state = State::Cancel,
                console::Key::ArrowUp | console::Key::Char('k') => {
                    if self.selected_day > 7 {
                        self.selected_day -= 7;
                    }
                }
                console::Key::ArrowDown | console::Key::Char('j') => {
                    if self.selected_day + 7 <= self.days_in_month() {
                        self.selected_day += 7;
                    }
                }
                console::Key::ArrowLeft | console::Key::Char('h') => {
                    if self.selected_day > 1 {
                        self.selected_day -= 1;
                    } else if self.month > 1 {
                        self.month -= 1;
                        self.selected_day = self.days_in_month();
                    }
                }
                console::Key::ArrowRight | console::Key::Char('l') => {
                    if self.selected_day < self.days_in_month() {
                        self.selected_day += 1;
                    } else if self.month < 12 {
                        self.month += 1;
                        self.selected_day = 1;
                    }
                }
                console::Key::Char('n') => {
                    if self.month < 12 {
                        self.month += 1;
                    } else {
                        self.month = 1;
                        self.year += 1;
                    }
                    self.selected_day = self.selected_day.min(self.days_in_month());
                }
                console::Key::Char('p') => {
                    if self.month > 1 {
                        self.month -= 1;
                    } else {
                        self.month = 12;
                        self.year -= 1;
                    }
                    self.selected_day = self.selected_day.min(self.days_in_month());
                }
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

                let month_year = format!("{} {}", self.month_name(), self.year);
                term.write_line(&format!(
                    "{}  📅 {}",
                    bar,
                    theme.primary.apply_to(month_year).bold()
                ))?;
                lines += 1;

                term.write_line(&format!("{}", bar))?;
                lines += 1;

                let calendar_lines = self.render_calendar();
                for line in calendar_lines {
                    term.write_line(&format!("{}  {}", bar, line))?;
                    lines += 1;
                }

                term.write_line(&format!("{}", bar))?;
                lines += 1;

                term.write_line(&format!(
                    "{}  {}",
                    bar,
                    theme.dim.apply_to("Arrow keys: navigate, n/p: next/prev month, Enter: select")
                ))?;
                lines += 1;
            }
            State::Submit => {
                let checkmark = theme.success.apply_to("✓");
                term.write_line(&format!(
                    "{} {}  {}",
                    checkmark,
                    self.message,
                    theme.dim.apply_to(format!(
                        "{}-{:02}-{:02}",
                        self.year, self.month, self.selected_day
                    ))
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
        format!("{}-{:02}-{:02}", self.year, self.month, self.selected_day)
    }
}

pub fn calendar(message: impl Into<String>) -> CalendarView {
    CalendarView::new(message)
}
