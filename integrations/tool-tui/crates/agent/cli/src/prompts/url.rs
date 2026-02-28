//! URL input with protocol validation

use super::interaction::{Event, PromptInteraction, State};
use super::{SYMBOLS, THEME};
use console::Term;
use owo_colors::OwoColorize;
use std::io;

pub struct UrlInput {
    message: String,
    value: String,
    require_https: bool,
    state: State,
    last_render_lines: usize,
    error_message: Option<String>,
}

impl UrlInput {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            value: String::new(),
            require_https: false,
            state: State::Active,
            last_render_lines: 0,
            error_message: None,
        }
    }

    pub fn require_https(mut self, require: bool) -> Self {
        self.require_https = require;
        self
    }

    pub fn initial_value(mut self, value: impl Into<String>) -> Self {
        self.value = value.into();
        self
    }

    fn validate_url(&self) -> Result<(), String> {
        if self.value.is_empty() {
            return Err("URL cannot be empty".to_string());
        }

        let lower = self.value.to_lowercase();

        if !lower.starts_with("http://") && !lower.starts_with("https://") {
            return Err("URL must start with http:// or https://".to_string());
        }

        if self.require_https && !lower.starts_with("https://") {
            return Err("URL must use HTTPS protocol".to_string());
        }

        if self.value.len() < 12 {
            return Err("URL is too short".to_string());
        }

        Ok(())
    }
}

impl PromptInteraction for UrlInput {
    type Output = String;

    fn state(&self) -> State {
        self.state
    }

    fn on(&mut self, event: Event) {
        match event {
            Event::Key(key) => match key {
                console::Key::Enter => match self.validate_url() {
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
                term.write_line(&format!(
                    "{}{}",
                    theme.primary.apply_to(symbols.step_submit),
                    format!("  {}  ", self.message).bold()
                ))?;
                lines += 1;

                term.write_line(&format!("{}", bar))?;
                lines += 1;

                let placeholder = if self.require_https {
                    "https://example.com"
                } else {
                    "https://example.com or http://example.com"
                };

                let display = if self.value.is_empty() {
                    format!("█{}", theme.dim.apply_to(placeholder))
                } else {
                    format!("{}█", self.value)
                };
                term.write_line(&format!("{}  {}", bar, display))?;
                lines += 1;

                if let Some(error) = &self.error_message {
                    term.write_line(&format!("{}  {}", bar, theme.error.apply_to(error)))?;
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

pub fn url(message: impl Into<String>) -> UrlInput {
    UrlInput::new(message)
}
