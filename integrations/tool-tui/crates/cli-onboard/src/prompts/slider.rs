//! Slider prompt for selecting numeric values

use super::interaction::{Event, PromptInteraction, State};
use super::{SYMBOLS, THEME};
use console::Term;
use owo_colors::OwoColorize;
use std::io;

/// A slider prompt for selecting numeric values.
pub struct Slider {
    message: String,
    value: i64,
    min: i64,
    max: i64,
    step: i64,
    state: State,
    last_render_lines: usize,
}

impl Slider {
    /// Creates a new slider prompt.
    pub fn new(message: impl Into<String>, min: i64, max: i64) -> Self {
        let mid = (min + max) / 2;
        Self {
            message: message.into(),
            value: mid,
            min,
            max,
            step: 1,
            state: State::Active,
            last_render_lines: 0,
        }
    }

    /// Sets the step size for the slider.
    pub fn step(mut self, step: i64) -> Self {
        self.step = step;
        self
    }

    /// Sets the initial value.
    pub fn initial_value(mut self, value: i64) -> Self {
        self.value = value.clamp(self.min, self.max);
        self
    }

    fn render_slider(&self) -> String {
        let width = 40;
        let range = self.max - self.min;
        let position = if range > 0 {
            ((self.value - self.min) as f64 / range as f64 * width as f64) as usize
        } else {
            0
        };

        let mut slider = String::new();
        slider.push('[');
        for i in 0..width {
            if i == position {
                slider.push('●');
            } else if i < position {
                slider.push('━');
            } else {
                slider.push('─');
            }
        }
        slider.push(']');
        slider
    }
}

impl PromptInteraction for Slider {
    type Output = i64;

    fn state(&self) -> State {
        self.state
    }

    fn on(&mut self, event: Event) {
        match event {
            Event::Key(key) => match key {
                console::Key::Enter => {
                    self.state = State::Submit;
                }
                console::Key::Escape => {
                    self.state = State::Cancel;
                }
                console::Key::ArrowLeft | console::Key::Char('h') => {
                    self.value = (self.value - self.step).max(self.min);
                }
                console::Key::ArrowRight | console::Key::Char('l') => {
                    self.value = (self.value + self.step).min(self.max);
                }
                console::Key::Home => {
                    self.value = self.min;
                }
                console::Key::End => {
                    self.value = self.max;
                }
                _ => {}
            },
            Event::Error => {
                self.state = State::Error;
            }
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
                let title_with_spaces = format!("  {}  ", self.message);
                term.write_line(&format!(
                    "{}{}",
                    theme.primary.apply_to(symbols.step_submit),
                    title_with_spaces.bold()
                ))?;
                lines += 1;

                term.write_line(&format!("{}", bar))?;
                lines += 1;

                // Slider visualization
                let slider = self.render_slider();
                term.write_line(&format!("{}  {}", bar, theme.primary.apply_to(slider)))?;
                lines += 1;

                // Current value
                term.write_line(&format!(
                    "{}  Value: {} (min: {}, max: {})",
                    bar,
                    theme.primary.apply_to(self.value.to_string()).bold(),
                    self.min,
                    self.max
                ))?;
                lines += 1;

                // Hint
                term.write_line(&format!(
                    "{}  {}",
                    bar,
                    theme.dim.apply_to("Use ← → arrows to adjust, Enter to confirm")
                ))?;
                lines += 1;
            }
            State::Submit => {
                let checkmark = theme.success.apply_to("✓");
                let display = theme.dim.apply_to(self.value.to_string());
                term.write_line(&format!("{} {}  {}", checkmark, self.message, display))?;
                lines += 1;
                term.write_line(&format!("{}", theme.dim.apply_to(symbols.bar)))?;
                lines += 1;
            }
            State::Cancel => {
                let bar = theme.dim.apply_to(symbols.bar);
                let symbol = theme.error.apply_to(symbols.step_submit);
                term.write_line(&format!(
                    "{}{} {}  {}",
                    bar,
                    symbol,
                    self.message.strikethrough(),
                    theme.dim.apply_to("cancelled")
                ))?;
                lines += 1;
            }
            State::Error => {
                let bar = theme.dim.apply_to(symbols.bar);
                let symbol = theme.error.apply_to(symbols.step_submit);
                term.write_line(&format!(
                    "{}{} {}  {}",
                    bar,
                    symbol,
                    self.message.bold(),
                    theme.error.apply_to("error")
                ))?;
                lines += 1;
            }
        }

        self.last_render_lines = lines;
        Ok(())
    }

    fn value(&self) -> i64 {
        self.value
    }
}

/// Creates a new slider prompt.
pub fn slider(message: impl Into<String>, min: i64, max: i64) -> Slider {
    Slider::new(message, min, max)
}
