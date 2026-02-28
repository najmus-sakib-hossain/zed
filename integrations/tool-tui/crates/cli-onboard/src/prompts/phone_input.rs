//! Phone number input with formatting

use super::interaction::{Event, PromptInteraction, State};
use super::{SYMBOLS, THEME};
use console::Term;
use std::io;

pub struct PhoneInput {
    message: String,
    value: String,
    country_code: String,
    state: State,
    last_render_lines: usize,
    error_message: Option<String>,
}

impl PhoneInput {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            value: String::new(),
            country_code: "+1".to_string(),
            state: State::Active,
            last_render_lines: 0,
            error_message: None,
        }
    }

    pub fn country_code(mut self, code: impl Into<String>) -> Self {
        self.country_code = code.into();
        self
    }

    pub fn initial_value(mut self, value: impl Into<String>) -> Self {
        self.value = value.into();
        self
    }

    fn format_phone(&self) -> String {
        let digits: String = self.value.chars().filter(|c| c.is_ascii_digit()).collect();

        if digits.len() <= 3 {
            digits
        } else if digits.len() <= 6 {
            format!("({}) {}", &digits[..3], &digits[3..])
        } else if digits.len() <= 10 {
            format!("({}) {}-{}", &digits[..3], &digits[3..6], &digits[6..])
        } else {
            format!("({}) {}-{}", &digits[..3], &digits[3..6], &digits[6..10])
        }
    }

    fn validate_phone(&self) -> Result<(), String> {
        let digits: String = self.value.chars().filter(|c| c.is_ascii_digit()).collect();

        if digits.is_empty() {
            return Err("Phone number cannot be empty".to_string());
        }

        if digits.len() < 10 {
            return Err("Phone number must be at least 10 digits".to_string());
        }

        if digits.len() > 15 {
            return Err("Phone number is too long".to_string());
        }

        Ok(())
    }
}

impl PromptInteraction for PhoneInput {
    type Output = String;

    fn state(&self) -> State {
        self.state
    }

    fn on(&mut self, event: Event) {
        match event {
            Event::Key(key) => match key {
                console::Key::Enter => match self.validate_phone() {
                    Ok(_) => self.state = State::Submit,
                    Err(msg) => self.error_message = Some(msg),
                },
                console::Key::Escape => self.state = State::Cancel,
                console::Key::Backspace => {
                    self.value.pop();
                    self.error_message = None;
                }
                console::Key::Char(c)
                    if c.is_ascii_digit()
                        || c == '+'
                        || c == '-'
                        || c == ' '
                        || c == '('
                        || c == ')' =>
                {
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
                let formatted = self.format_phone();
                let display = if formatted.is_empty() {
                    format!("█{}", theme.dim.apply_to("(555) 123-4567"))
                } else {
                    format!("{}█", formatted)
                };

                term.write_line(&format!("♦ {}", self.message))?;
                lines += 1;

                term.write_line(&format!(
                    "{} {} {}",
                    bar,
                    theme.primary.apply_to(&self.country_code),
                    display
                ))?;
                lines += 1;

                if let Some(error) = &self.error_message {
                    term.write_line(&format!("{} {}", bar, theme.error.apply_to(error)))?;
                    lines += 1;
                }
            }
            State::Submit => {
                let checkmark = theme.success.apply_to("✓");
                let full_number = format!("{} {}", self.country_code, self.format_phone());
                term.write_line(&format!(
                    "{} {}  {}",
                    checkmark,
                    self.message,
                    theme.dim.apply_to(full_number)
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
        format!("{} {}", self.country_code, self.format_phone())
    }
}

pub fn phone_input(message: impl Into<String>) -> PhoneInput {
    PhoneInput::new(message)
}
