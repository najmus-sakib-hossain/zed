//! Confirmation prompt

use super::interaction::{Event, PromptInteraction, State};
use super::{SYMBOLS, THEME};
use console::Term;
use owo_colors::OwoColorize;
use std::io;

/// A yes/no confirmation prompt.
pub struct Confirm {
    message: String,
    default_value: bool,
    value: bool,
    state: State,
    last_render_lines: usize,
}

impl Confirm {
    /// Creates a new confirmation prompt.
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            default_value: true,
            value: true,
            state: State::Active,
            last_render_lines: 0,
        }
    }

    /// Sets the initial/default value.
    pub fn initial_value(mut self, value: bool) -> Self {
        self.default_value = value;
        self.value = value;
        self
    }
}

impl PromptInteraction for Confirm {
    type Output = bool;

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
                console::Key::ArrowLeft | console::Key::ArrowRight | console::Key::Tab => {
                    self.value = !self.value;
                }
                console::Key::Char('y') | console::Key::Char('Y') => {
                    self.value = true;
                    self.state = State::Submit;
                }
                console::Key::Char('n') | console::Key::Char('N') => {
                    self.value = false;
                    self.state = State::Submit;
                }
                _ => {}
            },
            Event::Error => {
                self.state = State::Error;
            }
        }
    }

    fn render(&mut self, term: &Term) -> io::Result<()> {
        // Clear previous render
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
                // Title line - diamond aligns with │ position
                let bar = theme.dim.apply_to(symbols.bar);
                let title_with_spaces = format!("  {}  ", self.message);
                term.write_line(&format!(
                    "{}{}",
                    theme.primary.apply_to(symbols.step_submit),
                    title_with_spaces.bold()
                ))?;
                lines += 1;

                // ONE blank line after title (with bar)
                term.write_line(&format!("{}", bar))?;
                lines += 1;

                // Options line (with bar) - show both Yes and No
                let yes = if self.value {
                    theme.primary.apply_to("Yes").to_string()
                } else {
                    theme.dim.apply_to("Yes").to_string()
                };
                let no = if !self.value {
                    theme.primary.apply_to("No").to_string()
                } else {
                    theme.dim.apply_to("No").to_string()
                };
                term.write_line(&format!("{}  {}  /  {}", bar, yes, no))?;
                lines += 1;
            }
            State::Submit => {
                let checkmark = theme.success.apply_to("✓");
                let answer = if self.value { "Yes" } else { "No" };
                let display = theme.dim.apply_to(answer);
                term.write_line(&format!("{} {}  {}", checkmark, self.message, display))?;
                lines += 1;
                // Add blank line with bar after completion
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

    fn value(&self) -> bool {
        self.value
    }
}

/// Creates a new confirmation prompt.
#[allow(unused)]
pub fn confirm(message: impl Into<String>) -> Confirm {
    Confirm::new(message)
}
