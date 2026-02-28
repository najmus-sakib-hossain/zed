//! JSON editor with syntax validation

use super::interaction::{Event, PromptInteraction, State};
use super::{SYMBOLS, THEME};
use console::Term;
use owo_colors::OwoColorize;
use std::io;

pub struct JsonEditor {
    message: String,
    content: String,
    cursor_pos: usize,
    state: State,
    last_render_lines: usize,
    error_message: Option<String>,
}

impl JsonEditor {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            content: String::from("{}"),
            cursor_pos: 1,
            state: State::Active,
            last_render_lines: 0,
            error_message: None,
        }
    }

    pub fn initial_json(mut self, json: impl Into<String>) -> Self {
        self.content = json.into();
        self.cursor_pos = self.content.len();
        self
    }

    fn validate_json(&self) -> Result<(), String> {
        serde_json::from_str::<serde_json::Value>(&self.content)
            .map(|_| ())
            .map_err(|e| format!("Invalid JSON: {}", e))
    }

    fn format_json(&self) -> Result<String, String> {
        let value: serde_json::Value =
            serde_json::from_str(&self.content).map_err(|e| format!("Parse error: {}", e))?;
        serde_json::to_string_pretty(&value).map_err(|e| format!("Format error: {}", e))
    }

    fn render_content(&self) -> Vec<String> {
        self.content.lines().take(8).map(|line| line.to_string()).collect()
    }
}

impl PromptInteraction for JsonEditor {
    type Output = String;

    fn state(&self) -> State {
        self.state
    }

    fn on(&mut self, event: Event) {
        match event {
            Event::Key(key) => match key {
                console::Key::Enter if self.content.contains('\n') => match self.validate_json() {
                    Ok(_) => self.state = State::Submit,
                    Err(msg) => self.error_message = Some(msg),
                },
                console::Key::Enter => {
                    self.content.insert(self.cursor_pos, '\n');
                    self.cursor_pos += 1;
                    self.error_message = None;
                }
                console::Key::Escape => self.state = State::Cancel,
                console::Key::Backspace => {
                    if self.cursor_pos > 0 {
                        self.content.remove(self.cursor_pos - 1);
                        self.cursor_pos -= 1;
                        self.error_message = None;
                    }
                }
                console::Key::Char(c) if !c.is_control() => {
                    self.content.insert(self.cursor_pos, c);
                    self.cursor_pos += 1;
                    self.error_message = None;
                }
                console::Key::ArrowLeft => {
                    if self.cursor_pos > 0 {
                        self.cursor_pos -= 1;
                    }
                }
                console::Key::ArrowRight => {
                    if self.cursor_pos < self.content.len() {
                        self.cursor_pos += 1;
                    }
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

                let content_lines = self.render_content();
                for line in content_lines {
                    term.write_line(&format!("{}  {}", bar, theme.primary.apply_to(line)))?;
                    lines += 1;
                }

                if self.content.lines().count() > 8 {
                    term.write_line(&format!("{}  {}", bar, theme.dim.apply_to("...")))?;
                    lines += 1;
                }

                term.write_line(&format!("{}", bar))?;
                lines += 1;

                if let Some(error) = &self.error_message {
                    term.write_line(&format!("{}  {}", bar, theme.error.apply_to(error)))?;
                    lines += 1;
                } else if self.validate_json().is_ok() {
                    term.write_line(&format!(
                        "{}  {}",
                        bar,
                        theme.success.apply_to("✓ Valid JSON")
                    ))?;
                    lines += 1;
                }

                term.write_line(&format!(
                    "{}  {}",
                    bar,
                    theme.dim.apply_to("Type to edit, Enter: submit (multi-line), Esc: cancel")
                ))?;
                lines += 1;
            }
            State::Submit => {
                let checkmark = theme.success.apply_to("✓");
                term.write_line(&format!(
                    "{} {}  {}",
                    checkmark,
                    self.message,
                    theme.dim.apply_to("JSON saved")
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
        self.format_json().unwrap_or_else(|_| self.content.clone())
    }
}

pub fn json_editor(message: impl Into<String>) -> JsonEditor {
    JsonEditor::new(message)
}
