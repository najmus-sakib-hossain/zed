//! Password input prompt

use super::cursor::StringCursor;
use super::interaction::{Event, PromptInteraction, State, Validate};
use super::{SYMBOLS, THEME};
use console::Term;
use owo_colors::OwoColorize;
use std::io;
use zeroize::Zeroizing;

/// A masked password input prompt.
#[allow(unused)]
pub struct Password<V>
where
    V: Fn(&str) -> Validate<String>,
{
    message: String,
    mask: char,
    cursor: StringCursor,
    validate: Option<V>,
    state: State,
    error: Option<String>,
    last_render_lines: usize,
}

#[allow(unused)]
impl<V> Password<V>
where
    V: Fn(&str) -> Validate<String>,
{
    /// Creates a new password prompt.
    pub fn new(message: impl Into<String>) -> Self {
        let symbols = &*SYMBOLS;
        Self {
            message: message.into(),
            mask: symbols.password_mask,
            cursor: StringCursor::new(),
            validate: None,
            state: State::Active,
            error: None,
            last_render_lines: 0,
        }
    }

    /// Sets the mask character.
    pub fn mask(mut self, mask: char) -> Self {
        self.mask = mask;
        self
    }

    /// Sets a validation function.
    pub fn validate(mut self, f: V) -> Self {
        self.validate = Some(f);
        self
    }

    /// Validates the current value.
    fn do_validate(&mut self) -> bool {
        if let Some(ref validate) = self.validate {
            let value = self.cursor.value();
            match validate(&value) {
                Validate::Valid => {
                    self.error = None;
                    true
                }
                Validate::Invalid(msg) => {
                    self.error = Some(msg);
                    false
                }
            }
        } else {
            self.error = None;
            true
        }
    }
}

impl<V> PromptInteraction for Password<V>
where
    V: Fn(&str) -> Validate<String>,
{
    type Output = Zeroizing<String>;

    fn state(&self) -> State {
        self.state
    }

    fn on(&mut self, event: Event) {
        match event {
            Event::Key(key) => match key {
                console::Key::Enter => {
                    if self.do_validate() {
                        self.state = State::Submit;
                    }
                }
                console::Key::Escape => {
                    self.state = State::Cancel;
                }
                console::Key::Backspace => {
                    self.cursor.delete_left();
                    self.error = None;
                }
                console::Key::Del => {
                    self.cursor.delete_right();
                    self.error = None;
                }
                console::Key::ArrowLeft => {
                    self.cursor.move_left();
                }
                console::Key::ArrowRight => {
                    self.cursor.move_right();
                }
                console::Key::Home => {
                    self.cursor.move_home();
                }
                console::Key::End => {
                    self.cursor.move_end();
                }
                console::Key::Char(c) => {
                    self.cursor.insert(c);
                    self.error = None;
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
                // Title line
                let symbol = theme.primary.apply_to(symbols.step_active);
                term.write_line(&format!("{} {}", symbol, self.message.bold()))?;
                lines += 1;

                // Input line (masked)
                let bar = theme.dim.apply_to(symbols.bar);
                let len = self.cursor.len();
                let pos = self.cursor.position();

                let display = if len == 0 {
                    // Show cursor placeholder
                    " ".on_white().to_string()
                } else {
                    // Show masked characters with cursor
                    let left = self.mask.to_string().repeat(pos);
                    let cursor_char = if pos < len {
                        self.mask.to_string().on_white().black().to_string()
                    } else {
                        " ".on_white().to_string()
                    };
                    let right = if pos < len {
                        self.mask.to_string().repeat(len - pos - 1)
                    } else {
                        String::new()
                    };
                    format!("{}{}{}", left, cursor_char, right)
                };

                term.write_line(&format!("{}  {}", bar, display))?;
                lines += 1;

                // Error line
                if let Some(ref error) = self.error {
                    let error_msg = theme.error.apply_to(format!("  {}", error));
                    term.write_line(&format!("{}{}", bar, error_msg))?;
                    lines += 1;
                }

                // Bottom bar
                let bar_end = theme.dim.apply_to(symbols.bar_end);
                term.write_line(&format!("{}", bar_end))?;
                lines += 1;
            }
            State::Submit => {
                let symbol = theme.success.apply_to(symbols.step_submit);
                let len = self.cursor.len();
                let masked = self.mask.to_string().repeat(len);
                term.write_line(&format!(
                    "{} {}  {}",
                    symbol,
                    self.message.bold(),
                    theme.dim.apply_to(masked)
                ))?;
                lines += 1;
            }
            State::Cancel => {
                let symbol = theme.error.apply_to(symbols.step_submit);
                term.write_line(&format!(
                    "{} {}  {}",
                    symbol,
                    self.message.strikethrough(),
                    theme.dim.apply_to("cancelled")
                ))?;
                lines += 1;
            }
            State::Error => {
                let symbol = theme.error.apply_to(symbols.step_submit);
                term.write_line(&format!(
                    "{} {}  {}",
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

    fn value(&self) -> Zeroizing<String> {
        Zeroizing::new(self.cursor.value())
    }
}

/// Creates a new password prompt.
#[allow(unused)]
pub fn password(message: impl Into<String>) -> Password<fn(&str) -> Validate<String>> {
    Password::new(message)
}
