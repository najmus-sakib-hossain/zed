//! Text input prompt

use super::cursor::StringCursor;
use super::interaction::{Event, PromptInteraction, State, Validate};
use super::{SYMBOLS, THEME};
use console::Term;
use owo_colors::OwoColorize;
use std::io;

/// A text input prompt with placeholder and validation support.
#[allow(unused)]
pub struct Input<V>
where
    V: Fn(&str) -> Validate<String>,
{
    message: String,
    placeholder: String,
    default_value: Option<String>,
    cursor: StringCursor,
    validate: Option<V>,
    state: State,
    error: Option<String>,
    last_render_lines: usize,
    multiline: bool,
}

#[allow(unused)]
impl<V> Input<V>
where
    V: Fn(&str) -> Validate<String>,
{
    /// Creates a new input prompt with the given message.
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            placeholder: String::new(),
            default_value: None,
            cursor: StringCursor::new(),
            validate: None,
            state: State::Active,
            error: None,
            last_render_lines: 0,
            multiline: false,
        }
    }

    /// Sets the placeholder text shown when input is empty.
    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    /// Sets a default value.
    pub fn default_value(mut self, value: impl Into<String>) -> Self {
        self.default_value = Some(value.into());
        self
    }

    /// Sets the initial value.
    pub fn initial_value(mut self, value: impl Into<String>) -> Self {
        self.cursor = StringCursor::from(value.into());
        self
    }

    /// Sets a validation function.
    pub fn validate(mut self, f: V) -> Self {
        self.validate = Some(f);
        self
    }

    /// Enables multiline input mode.
    pub fn multiline(mut self, enable: bool) -> Self {
        self.multiline = enable;
        self
    }

    /// Validates the current value.
    fn do_validate(&mut self) -> bool {
        if let Some(ref validate) = self.validate {
            let value = self.get_value();
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

    /// Gets the current or default value.
    fn get_value(&self) -> String {
        let value = self.cursor.value();
        if value.is_empty() {
            self.default_value.clone().unwrap_or_default()
        } else {
            value
        }
    }
}

impl<V> PromptInteraction for Input<V>
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
                    if self.multiline {
                        self.cursor.insert('\n');
                    } else if self.do_validate() {
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
                console::Key::ArrowUp if self.multiline => {
                    self.cursor.move_up();
                }
                console::Key::ArrowDown if self.multiline => {
                    self.cursor.move_down();
                }
                console::Key::Home => {
                    self.cursor.move_home();
                }
                console::Key::End => {
                    self.cursor.move_end();
                }
                console::Key::Tab => {
                    // Use default value if input is empty
                    if self.cursor.is_empty()
                        && let Some(ref default) = self.default_value
                    {
                        self.cursor = StringCursor::from(default.clone());
                    }
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
                let bar = theme.dim.apply_to(symbols.bar);
                let value = self.cursor.value();
                let display = if value.is_empty() {
                    format!("█{}", theme.dim.apply_to(&self.placeholder))
                } else {
                    format!("{}█", value)
                };
                term.write_line(&format!("♦ {}", self.message))?;
                lines += 1;

                term.write_line(&format!("{} {}", bar, display))?;
                lines += 1;

                // Error line if present
                if let Some(ref error) = self.error {
                    term.write_line(&format!("{} {}", bar, theme.error.apply_to(error)))?;
                    lines += 1;
                }
            }
            State::Submit => {
                let checkmark = theme.success.apply_to("✓");
                let value = self.get_value();
                let display = theme.dim.apply_to(&value);
                term.write_line(&format!("{} {}  {}", checkmark, self.message, display))?;
                lines += 1;
                // Blank line with │ after prompt
                term.write_line(&format!("{}", theme.dim.apply_to(symbols.bar)))?;
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

    fn value(&self) -> String {
        self.get_value()
    }
}

/// Creates a new text input prompt.
#[allow(unused)]
pub fn input(message: impl Into<String>) -> Input<fn(&str) -> Validate<String>> {
    Input::new(message)
}
