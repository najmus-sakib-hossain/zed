//! Text input prompt with validation

use super::interaction::{Event, PromptInteraction, State, Validate};
use super::{SYMBOLS, THEME};
use console::Term;
use owo_colors::OwoColorize;
use std::io;

/// A text input prompt with optional validation.
pub struct Text<V>
where
    V: Fn(&str) -> Validate<String>,
{
    message: String,
    value: String,
    placeholder: Option<String>,
    validator: Option<V>,
    state: State,
    last_render_lines: usize,
    error_message: Option<String>,
}

impl<V> Text<V>
where
    V: Fn(&str) -> Validate<String>,
{
    /// Creates a new text input prompt.
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            value: String::new(),
            placeholder: None,
            validator: None,
            state: State::Active,
            last_render_lines: 0,
            error_message: None,
        }
    }

    /// Sets a placeholder text.
    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = Some(placeholder.into());
        self
    }

    /// Sets a validator function.
    pub fn validate(mut self, validator: V) -> Self {
        self.validator = Some(validator);
        self
    }

    /// Sets an initial value.
    pub fn initial_value(mut self, value: impl Into<String>) -> Self {
        self.value = value.into();
        self
    }
}

impl<V> PromptInteraction for Text<V>
where
    V: Fn(&str) -> Validate<String>,
{
    type Output = String;

    fn state(&self) -> State {
        self.state
    }

    fn on(&mut self, event: Event) {
        match event {
            Event::Key(key) => match key {
                console::Key::Enter => {
                    if let Some(validator) = &self.validator {
                        match validator(&self.value) {
                            Validate::Valid => {
                                self.state = State::Submit;
                            }
                            Validate::Invalid(msg) => {
                                self.error_message = Some(msg);
                            }
                        }
                    } else {
                        self.state = State::Submit;
                    }
                }
                console::Key::Escape => {
                    self.state = State::Cancel;
                }
                console::Key::Backspace => {
                    self.value.pop();
                    self.error_message = None;
                }
                console::Key::Char(c) if !c.is_control() => {
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
                let display_value = if self.value.is_empty() {
                    format!(
                        "█{}",
                        self.placeholder
                            .as_ref()
                            .map(|p| theme.dim.apply_to(p).to_string())
                            .unwrap_or_default()
                    )
                } else {
                    format!("{}█", self.value)
                };
                // NO │ prefix on prompt line
                term.write_line(&format!("♦ {}  {}", self.message, display_value))?;
                lines += 1;
            }
            State::Submit => {
                let checkmark = theme.success.apply_to("✓");
                let display = theme.dim.apply_to(&self.value);
                // NO │ prefix on prompt line
                term.write_line(&format!("{} {}  {}", checkmark, self.message, display))?;
                lines += 1;
                // Blank line with │ after prompt
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

    fn value(&self) -> String {
        self.value.clone()
    }
}

/// Creates a new text input prompt.
pub fn text(message: impl Into<String>) -> Text<fn(&str) -> Validate<String>> {
    Text::new(message)
}
