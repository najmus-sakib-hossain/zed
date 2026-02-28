//! List editor prompt for managing a list of items

use super::interaction::{Event, PromptInteraction, State};
use super::{SYMBOLS, THEME};
use console::Term;
use owo_colors::OwoColorize;
use std::io;

/// A list editor prompt for managing items.
pub struct ListEditor {
    message: String,
    items: Vec<String>,
    current_input: String,
    cursor: usize,
    mode: EditMode,
    state: State,
    last_render_lines: usize,
}

#[derive(PartialEq)]
enum EditMode {
    View,
    Add,
}

impl ListEditor {
    /// Creates a new list editor prompt.
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            items: Vec::new(),
            current_input: String::new(),
            cursor: 0,
            mode: EditMode::View,
            state: State::Active,
            last_render_lines: 0,
        }
    }

    /// Sets initial items.
    pub fn initial_items(mut self, items: Vec<String>) -> Self {
        self.items = items;
        self
    }
}

impl PromptInteraction for ListEditor {
    type Output = Vec<String>;

    fn state(&self) -> State {
        self.state
    }

    fn on(&mut self, event: Event) {
        match event {
            Event::Key(key) => match key {
                console::Key::Enter => {
                    if self.mode == EditMode::Add {
                        if !self.current_input.trim().is_empty() {
                            self.items.push(self.current_input.trim().to_string());
                            self.current_input.clear();
                        }
                        self.mode = EditMode::View;
                    } else {
                        self.state = State::Submit;
                    }
                }
                console::Key::Escape => {
                    if self.mode == EditMode::Add {
                        self.current_input.clear();
                        self.mode = EditMode::View;
                    } else {
                        self.state = State::Cancel;
                    }
                }
                console::Key::Char('a') if self.mode == EditMode::View => {
                    self.mode = EditMode::Add;
                }
                console::Key::Char('d')
                    if self.mode == EditMode::View && !self.items.is_empty() =>
                {
                    self.items.remove(self.cursor);
                    if self.cursor >= self.items.len() && self.cursor > 0 {
                        self.cursor -= 1;
                    }
                }
                console::Key::ArrowUp | console::Key::Char('k')
                    if self.mode == EditMode::View && self.cursor > 0 =>
                {
                    self.cursor -= 1;
                }
                console::Key::ArrowDown | console::Key::Char('j')
                    if self.mode == EditMode::View && self.cursor + 1 < self.items.len() =>
                {
                    self.cursor += 1;
                }
                console::Key::Backspace if self.mode == EditMode::Add => {
                    self.current_input.pop();
                }
                console::Key::Char(c) if self.mode == EditMode::Add && !c.is_control() => {
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

                if self.mode == EditMode::Add {
                    // Add mode
                    let display = if self.current_input.is_empty() {
                        format!("█{}", theme.dim.apply_to("Type item name..."))
                    } else {
                        format!("{}█", self.current_input)
                    };
                    term.write_line(&format!(
                        "{}  {} {}",
                        bar,
                        theme.primary.apply_to("Adding:"),
                        display
                    ))?;
                    lines += 1;
                } else {
                    // View mode - show items
                    if self.items.is_empty() {
                        term.write_line(&format!(
                            "{}  {}",
                            bar,
                            theme.dim.apply_to("No items yet. Press 'a' to add.")
                        ))?;
                        lines += 1;
                    } else {
                        for (i, item) in self.items.iter().enumerate() {
                            let marker = if i == self.cursor { ">" } else { " " };
                            let item_display = if i == self.cursor {
                                theme.primary.apply_to(item).to_string()
                            } else {
                                item.clone()
                            };
                            term.write_line(&format!("{}  {} {}", bar, marker, item_display))?;
                            lines += 1;
                        }
                    }
                }

                // Controls hint
                let hint = if self.mode == EditMode::Add {
                    "Enter to add, Esc to cancel"
                } else {
                    "a: add, d: delete, ↑↓: navigate, Enter: done"
                };
                term.write_line(&format!("{}  {}", bar, theme.dim.apply_to(hint)))?;
                lines += 1;
            }
            State::Submit => {
                let checkmark = theme.success.apply_to("✓");
                let display = if self.items.is_empty() {
                    theme.dim.apply_to("empty list").to_string()
                } else {
                    theme
                        .dim
                        .apply_to(format!("{} items", self.items.len()))
                        .to_string()
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
        self.items.clone()
    }
}

/// Creates a new list editor prompt.
pub fn list_editor(message: impl Into<String>) -> ListEditor {
    ListEditor::new(message)
}
