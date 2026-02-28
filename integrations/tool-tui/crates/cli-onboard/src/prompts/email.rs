//! Email input with validation

use super::interaction::{Event, PromptInteraction, State};
use super::{SYMBOLS, THEME};
use console::Term;
use std::io;

pub struct EmailInput {
    message: String,
    value: String,
    state: State,
    last_render_lines: usize,
    error_message: Option<String>,
}

impl EmailInput {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            value: String::new(),
            state: State::Active,
            last_render_lines: 0,
            error_message: None,
        }
    }

    pub fn initial_value(mut self, value: impl Into<String>) -> Self {
        self.value = value.into();
        self
    }

    fn validate_email(&self) -> Result<(), String> {
        if self.value.is_empty() {
            return Err("Email cannot be empty".to_string());
        }

        if !self.value.contains('@') {
            return Err("Email must contain @".to_string());
        }

        let parts: Vec<&str> = self.value.split('@').collect();
        if parts.len() != 2 {
            return Err("Invalid email format".to_string());
        }

        if parts[0].is_empty() {
            return Err("Email username cannot be empty".to_string());
        }

        if parts[1].is_empty() || !parts[1].contains('.') {
            return Err("Invalid email domain".to_string());
        }

        Ok(())
    }
}

impl PromptInteraction for EmailInput {
    type Output = String;

    fn state(&self) -> State {
        self.state
    }

    fn on(&mut self, event: Event) {
        match event {
            Event::Key(key) => match key {
                console::Key::Enter => match self.validate_email() {
                    Ok(_) => self.state = State::Submit,
                    Err(msg) => self.error_message = Some(msg),
                },
                console::Key::Escape => self.state = State::Cancel,
                console::Key::Backspace => {
                    self.value.pop();
                    self.error_message = None;
                }
                console::Key::Char(c) if !c.is_control() && !c.is_whitespace() => {
                    self.value.push(c);
                    self.error_message = None;
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
                let display = if self.value.is_empty() {
                    format!("█{}", theme.dim.apply_to("user@example.com"))
                } else {
                    format!("{}█", self.value)
                };
                term.write_line(&format!("♦ {}", self.message))?;
                lines += 1;

                term.write_line(&format!("{} {}", bar, display))?;
                lines += 1;

                if let Some(error) = &self.error_message {
                    term.write_line(&format!("{} {}", bar, theme.error.apply_to(error)))?;
                    lines += 1;
                }
            }
            State::Submit => {
                let checkmark = theme.success.apply_to("✓");
                term.write_line(&format!(
                    "{} {}  {}",
                    checkmark,
                    self.message,
                    theme.dim.apply_to(&self.value)
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
        self.value.clone()
    }
}

pub fn email(message: impl Into<String>) -> EmailInput {
    EmailInput::new(message)
}
