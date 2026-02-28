//! Range slider for selecting min-max values

use super::interaction::{Event, PromptInteraction, State};
use super::{SYMBOLS, THEME};
use console::Term;
use owo_colors::OwoColorize;
use std::io;

pub struct RangeSlider {
    message: String,
    min_value: i64,
    max_value: i64,
    range_min: i64,
    range_max: i64,
    active_handle: Handle,
    state: State,
    last_render_lines: usize,
}

#[derive(PartialEq)]
enum Handle {
    Min,
    Max,
}

impl RangeSlider {
    pub fn new(message: impl Into<String>, min: i64, max: i64) -> Self {
        let mid = (min + max) / 2;
        Self {
            message: message.into(),
            min_value: min,
            max_value: mid,
            range_min: min,
            range_max: max,
            active_handle: Handle::Min,
            state: State::Active,
            last_render_lines: 0,
        }
    }

    pub fn initial_range(mut self, min: i64, max: i64) -> Self {
        self.min_value = min.clamp(self.range_min, self.range_max);
        self.max_value = max.clamp(self.range_min, self.range_max);
        self
    }

    fn render_slider(&self) -> String {
        let width = 40;
        let range = self.range_max - self.range_min;
        let min_pos = if range > 0 {
            ((self.min_value - self.range_min) as f64 / range as f64 * width as f64) as usize
        } else {
            0
        };
        let max_pos = if range > 0 {
            ((self.max_value - self.range_min) as f64 / range as f64 * width as f64) as usize
        } else {
            width
        };

        let mut slider = String::new();
        slider.push('[');
        for i in 0..width {
            if i == min_pos {
                slider.push('◀');
            } else if i == max_pos {
                slider.push('▶');
            } else if i > min_pos && i < max_pos {
                slider.push('━');
            } else {
                slider.push('─');
            }
        }
        slider.push(']');
        slider
    }
}

impl PromptInteraction for RangeSlider {
    type Output = (i64, i64);

    fn state(&self) -> State {
        self.state
    }

    fn on(&mut self, event: Event) {
        match event {
            Event::Key(key) => match key {
                console::Key::Enter => self.state = State::Submit,
                console::Key::Escape => self.state = State::Cancel,
                console::Key::Tab => {
                    self.active_handle = match self.active_handle {
                        Handle::Min => Handle::Max,
                        Handle::Max => Handle::Min,
                    };
                }
                console::Key::ArrowLeft | console::Key::Char('h') => match self.active_handle {
                    Handle::Min => {
                        self.min_value = (self.min_value - 1).max(self.range_min);
                    }
                    Handle::Max => {
                        self.max_value = (self.max_value - 1).max(self.min_value);
                    }
                },
                console::Key::ArrowRight | console::Key::Char('l') => match self.active_handle {
                    Handle::Min => {
                        self.min_value = (self.min_value + 1).min(self.max_value);
                    }
                    Handle::Max => {
                        self.max_value = (self.max_value + 1).min(self.range_max);
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

                let slider = self.render_slider();
                term.write_line(&format!("{}  {}", bar, theme.primary.apply_to(slider)))?;
                lines += 1;

                let min_marker = if self.active_handle == Handle::Min {
                    "▸"
                } else {
                    " "
                };
                let max_marker = if self.active_handle == Handle::Max {
                    "▸"
                } else {
                    " "
                };

                term.write_line(&format!(
                    "{}  {} Min: {}",
                    bar,
                    min_marker,
                    theme.primary.apply_to(self.min_value.to_string()).bold()
                ))?;
                lines += 1;

                term.write_line(&format!(
                    "{}  {} Max: {}",
                    bar,
                    max_marker,
                    theme.primary.apply_to(self.max_value.to_string()).bold()
                ))?;
                lines += 1;

                term.write_line(&format!(
                    "{}  {}",
                    bar,
                    theme.dim.apply_to("Tab: switch handle, ← →: adjust, Enter: confirm")
                ))?;
                lines += 1;
            }
            State::Submit => {
                let checkmark = theme.success.apply_to("✓");
                term.write_line(&format!(
                    "{} {}  {}",
                    checkmark,
                    self.message,
                    theme.dim.apply_to(format!("{} - {}", self.min_value, self.max_value))
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

    fn value(&self) -> (i64, i64) {
        (self.min_value, self.max_value)
    }
}

pub fn range_slider(message: impl Into<String>, min: i64, max: i64) -> RangeSlider {
    RangeSlider::new(message, min, max)
}
