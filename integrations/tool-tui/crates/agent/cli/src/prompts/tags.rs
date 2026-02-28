//! Tags input prompt for multiple text values

use super::interaction::{Event, PromptInteraction, State};
use super::{SYMBOLS, THEME};
use console::Term;
use owo_colors::OwoColorize;
use std::io;

/// A tags input prompt for entering multiple text values.
pub struct Tags {
    message: String,
    tags: Vec<String>,
    current_input: String,
    state: State,
    last_render_lines: usize,
    placeholder: Option<String>,
}

impl Tags {
    /// Creates a new tags input prompt.
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            tags: Vec::new(),
            current_input: String::new(),
            state: State::Active,
            last_render_lines: 0,
            placeholder: None,
        }
    }

    /// Sets a placeholder text.
    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = Some(placeholder.into());
        self
    }

    /// Sets initial tags.
    pub fn initial_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }
}

impl PromptInteraction for Tags {
    type Output = Vec<String>;

    fn state(&self) -> State {
        self.state
    }

    fn on(&mut self, event: Event) {
        match event {
            Event::Key(key) => match key {
                console::Key::Enter => {
                    if !self.current_input.trim().is_empty() {
                        self.tags.push(self.current_input.trim().to_string());
                        self.current_input.clear();
                    } else if !self.tags.is_empty() {
                        self.state = State::Submit;
                    }
                }
                console::Key::Escape => {
                    self.state = State::Cancel;
                }
                console::Key::Backspace => {
                    if self.current_input.is_empty() && !self.tags.is_empty() {
                        self.tags.pop();
                    } else {
                        self.current_input.pop();
                    }
                }
                console::Key::Char(',') => {
                    if !self.current_input.trim().is_empty() {
                        self.tags.push(self.current_input.trim().to_string());
                        self.current_input.clear();
                    }
                }
                console::Key::Char(c) if !c.is_control() => {
                    self.current_input.push(c);
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

                // Display existing tags
                if !self.tags.is_empty() {
                    let tags_display = self
                        .tags
                        .iter()
                        .map(|tag| format!("[{}]", theme.primary.apply_to(tag)))
                        .collect::<Vec<_>>()
                        .join(" ");
                    term.write_line(&format!("{}  {}", bar, tags_display))?;
                    lines += 1;
                }

                // Input line
                let display_input = if self.current_input.is_empty() {
                    format!(
                        "█{}",
                        self.placeholder
                            .as_ref()
                            .map(|p| theme.dim.apply_to(p).to_string())
                            .unwrap_or_else(|| theme
                                .dim
                                .apply_to("Type and press Enter or comma")
                                .to_string())
                    )
                } else {
                    format!("{}█", self.current_input)
                };
                term.write_line(&format!("{}  {}", bar, display_input))?;
                lines += 1;

                // Hint
                term.write_line(&format!(
                    "{}  {}",
                    bar,
                    theme.dim.apply_to(
                        "Enter/comma to add tag, Backspace to remove, Enter on empty to finish"
                    )
                ))?;
                lines += 1;
            }
            State::Submit => {
                let checkmark = theme.success.apply_to("✓");
                let display = if self.tags.is_empty() {
                    theme.dim.apply_to("none").to_string()
                } else {
                    theme.dim.apply_to(self.tags.join(", ")).to_string()
                };
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

    fn value(&self) -> Vec<String> {
        self.tags.clone()
    }
}

/// Creates a new tags input prompt.
pub fn tags(message: impl Into<String>) -> Tags {
    Tags::new(message)
}
