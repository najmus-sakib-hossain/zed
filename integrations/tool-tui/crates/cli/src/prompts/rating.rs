//! Rating prompt with star selection

use super::interaction::{Event, PromptInteraction, State};
use super::{SYMBOLS, THEME};
use console::Term;
use owo_colors::OwoColorize;
use std::io;

/// A rating prompt with star selection.
pub struct Rating {
    message: String,
    value: usize,
    max: usize,
    state: State,
    last_render_lines: usize,
}

impl Rating {
    /// Creates a new rating prompt.
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            value: 0,
            max: 5,
            state: State::Active,
            last_render_lines: 0,
        }
    }

    /// Sets the maximum rating value.
    pub fn max(mut self, max: usize) -> Self {
        self.max = max;
        self
    }

    /// Sets the initial value.
    pub fn initial_value(mut self, value: usize) -> Self {
        self.value = value.min(self.max);
        self
    }

    fn render_stars(&self, theme: &super::DxTheme) -> String {
        let mut stars = String::new();
        for i in 1..=self.max {
            if i <= self.value {
                stars.push_str(&theme.primary.apply_to("★").to_string());
            } else {
                stars.push_str(&theme.dim.apply_to("☆").to_string());
            }
            if i < self.max {
                stars.push(' ');
            }
        }
        stars
    }
}

impl PromptInteraction for Rating {
    type Output = usize;

    fn state(&self) -> State {
        self.state
    }

    fn on(&mut self, event: Event) {
        match event {
            Event::Key(key) => match key {
                console::Key::Enter => {
                    if self.value > 0 {
                        self.state = State::Submit;
                    }
                }
                console::Key::Escape => {
                    self.state = State::Cancel;
                }
                console::Key::ArrowLeft | console::Key::Char('h') => {
                    if self.value > 0 {
                        self.value -= 1;
                    }
                }
                console::Key::ArrowRight | console::Key::Char('l') => {
                    if self.value < self.max {
                        self.value += 1;
                    }
                }
                console::Key::Char(c) if c.is_ascii_digit() => {
                    let num = c.to_digit(10).unwrap() as usize;
                    if num <= self.max {
                        self.value = num;
                    }
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
                // NO │ prefix on prompt line
                term.write_line(&format!("♦ {}", self.message))?;
                lines += 1;

                // Stars
                let stars = self.render_stars(&theme);
                term.write_line(&format!("  {}", stars))?;
                lines += 1;

                // Rating value
                let rating_text = if self.value == 0 {
                    theme.dim.apply_to("No rating selected").to_string()
                } else {
                    format!(
                        "{} / {}",
                        theme.primary.apply_to(self.value.to_string()).bold(),
                        self.max
                    )
                };
                term.write_line(&format!("  {}", rating_text))?;
                lines += 1;

                // Hint
                term.write_line(&format!(
                    "  {}",
                    theme.dim.apply_to("Use ← → arrows or numbers, Enter to confirm")
                ))?;
                lines += 1;
                // Blank line with │ after prompt
                term.write_line(&format!("{}", theme.dim.apply_to(symbols.bar)))?;
                lines += 1;
            }
            State::Submit => {
                let checkmark = theme.success.apply_to("✓");
                let stars = "★".repeat(self.value);
                let display =
                    theme.dim.apply_to(format!("{} ({}/{})", stars, self.value, self.max));
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

    fn value(&self) -> usize {
        self.value
    }
}

/// Creates a new rating prompt.
pub fn rating(message: impl Into<String>) -> Rating {
    Rating::new(message)
}
