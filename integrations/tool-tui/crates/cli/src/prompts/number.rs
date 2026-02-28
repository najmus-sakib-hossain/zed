//! Number input prompt with validation

use super::interaction::{Event, PromptInteraction, State};
use super::{SYMBOLS, THEME};
use console::Term;
use owo_colors::OwoColorize;
use std::io;

/// A number input prompt with optional min/max validation.
pub struct Number {
    message: String,
    value: String,
    min: Option<i64>,
    max: Option<i64>,
    state: State,
    last_render_lines: usize,
    error_message: Option<String>,
}

impl Number {
    /// Creates a new number input prompt.
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            value: String::new(),
            min: None,
            max: None,
            state: State::Active,
            last_render_lines: 0,
            error_message: None,
        }
    }

    /// Sets the minimum allowed value.
    pub fn min(mut self, min: i64) -> Self {
        self.min = Some(min);
        self
    }

    /// Sets the maximum allowed value.
    pub fn max(mut self, max: i64) -> Self {
        self.max = Some(max);
        self
    }

    /// Sets an initial value.
    pub fn initial_value(mut self, value: i64) -> Self {
        self.value = value.to_string();
        self
    }

    fn validate(&self) -> Result<i64, String> {
        let num = self
            .value
            .parse::<i64>()
            .map_err(|_| "Please enter a valid number".to_string())?;

        if let Some(min) = self.min {
            if num < min {
                return Err(format!("Value must be at least {}", min));
            }
        }

        if let Some(max) = self.max {
            if num > max {
                return Err(format!("Value must be at most {}", max));
            }
        }

        Ok(num)
    }
}

impl PromptInteraction for Number {
    type Output = i64;

    fn state(&self) -> State {
        self.state
    }

    fn on(&mut self, event: Event) {
        match event {
            Event::Key(key) => match key {
                console::Key::Enter => {
                    if !self.value.is_empty() {
                        match self.validate() {
                            Ok(_) => {
                                self.state = State::Submit;
                            }
                            Err(msg) => {
                                self.error_message = Some(msg);
                            }
                        }
                    }
                }
                console::Key::Escape => {
                    self.state = State::Cancel;
                }
                console::Key::Backspace => {
                    self.value.pop();
                    self.error_message = None;
                }
                console::Key::Char(c) if c.is_ascii_digit() || c == '-' => {
                    self.value.push(c);
                    self.error_message = None;
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

                let display_value = if self.value.is_empty() {
                    format!("█{}", theme.dim.apply_to("Enter a number..."))
                } else {
                    format!("{}█", self.value)
                };
                term.write_line(&format!("{}  {}", bar, display_value))?;
                lines += 1;

                if let Some(error) = &self.error_message {
                    term.write_line(&format!("{}  {}", bar, theme.error.apply_to(error)))?;
                    lines += 1;
                }
            }
            State::Submit => {
                let checkmark = theme.success.apply_to("✓");
                let display = theme.dim.apply_to(&self.value);
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
        self.validate().unwrap_or(0)
    }
}

/// Creates a new number input prompt.
pub fn number(message: impl Into<String>) -> Number {
    Number::new(message)
}
