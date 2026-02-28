//! Autocomplete prompt with fuzzy search

use super::interaction::{Event, PromptInteraction, State};
use super::{SYMBOLS, THEME};
use console::Term;
use owo_colors::OwoColorize;
use std::io;

/// An autocomplete prompt with fuzzy search.
pub struct Autocomplete<T: Clone> {
    message: String,
    items: Vec<AutocompleteItem<T>>,
    input: String,
    cursor: usize,
    filtered_indices: Vec<usize>,
    state: State,
    last_render_lines: usize,
}

#[derive(Clone)]
pub struct AutocompleteItem<T: Clone> {
    pub value: T,
    pub label: String,
    pub description: Option<String>,
}

impl<T: Clone> AutocompleteItem<T> {
    pub fn new(value: T, label: impl Into<String>) -> Self {
        Self {
            value,
            label: label.into(),
            description: None,
        }
    }

    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }
}

impl<T: Clone> Autocomplete<T> {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            items: Vec::new(),
            input: String::new(),
            cursor: 0,
            filtered_indices: Vec::new(),
            state: State::Active,
            last_render_lines: 0,
        }
    }

    pub fn item(mut self, value: T, label: impl Into<String>) -> Self {
        let item = AutocompleteItem::new(value, label);
        self.items.push(item);
        self.filtered_indices.push(self.items.len() - 1);
        self
    }

    pub fn item_with_description(
        mut self,
        value: T,
        label: impl Into<String>,
        description: impl Into<String>,
    ) -> Self {
        let item = AutocompleteItem::new(value, label).description(description);
        self.items.push(item);
        self.filtered_indices.push(self.items.len() - 1);
        self
    }

    fn update_filter(&mut self) {
        if self.input.is_empty() {
            self.filtered_indices = (0..self.items.len()).collect();
        } else {
            let input_lower = self.input.to_lowercase();
            self.filtered_indices = self
                .items
                .iter()
                .enumerate()
                .filter(|(_, item)| item.label.to_lowercase().contains(&input_lower))
                .map(|(i, _)| i)
                .collect();
        }
        if !self.filtered_indices.is_empty() && self.cursor >= self.filtered_indices.len() {
            self.cursor = self.filtered_indices.len() - 1;
        }
    }

    fn current_index(&self) -> Option<usize> {
        self.filtered_indices.get(self.cursor).copied()
    }
}

impl<T: Clone> PromptInteraction for Autocomplete<T> {
    type Output = T;

    fn state(&self) -> State {
        self.state
    }

    fn on(&mut self, event: Event) {
        match event {
            Event::Key(key) => match key {
                console::Key::Enter => {
                    if self.current_index().is_some() {
                        self.state = State::Submit;
                    }
                }
                console::Key::Escape => {
                    self.state = State::Cancel;
                }
                console::Key::ArrowUp | console::Key::Char('k') => {
                    if self.cursor > 0 {
                        self.cursor -= 1;
                    }
                }
                console::Key::ArrowDown | console::Key::Char('j') => {
                    if self.cursor + 1 < self.filtered_indices.len() {
                        self.cursor += 1;
                    }
                }
                console::Key::Backspace => {
                    self.input.pop();
                    self.update_filter();
                }
                console::Key::Char(c) if !c.is_control() => {
                    self.input.push(c);
                    self.update_filter();
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

                // Input line
                let display_input = if self.input.is_empty() {
                    format!("█{}", theme.dim.apply_to("Type to search..."))
                } else {
                    format!("{}█", self.input)
                };
                term.write_line(&format!("{}  {}", bar, display_input))?;
                lines += 1;

                // Filtered items
                let max_visible = 5;
                let visible_items = self.filtered_indices.iter().take(max_visible);

                for (display_idx, &item_idx) in visible_items.enumerate() {
                    let item = &self.items[item_idx];
                    let is_selected = display_idx == self.cursor;

                    let label = if is_selected {
                        theme.primary.apply_to(&item.label).to_string()
                    } else {
                        item.label.clone()
                    };

                    let desc = item
                        .description
                        .as_ref()
                        .map(|d| format!(" {}", theme.dim.apply_to(d)))
                        .unwrap_or_default();

                    let marker = if is_selected { ">" } else { " " };
                    term.write_line(&format!("{}  {} {}{}", bar, marker, label, desc))?;
                    lines += 1;
                }

                if self.filtered_indices.len() > max_visible {
                    term.write_line(&format!(
                        "{}  {}",
                        bar,
                        theme.dim.apply_to(format!(
                            "... {} more",
                            self.filtered_indices.len() - max_visible
                        ))
                    ))?;
                    lines += 1;
                }
            }
            State::Submit => {
                let checkmark = theme.success.apply_to("✓");
                let selected =
                    self.current_index().map(|i| self.items[i].label.clone()).unwrap_or_default();
                let display = theme.dim.apply_to(&selected);
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

    fn value(&self) -> T {
        self.current_index()
            .map(|i| self.items[i].value.clone())
            .expect("No item selected")
    }
}

pub fn autocomplete<T: Clone>(message: impl Into<String>) -> Autocomplete<T> {
    Autocomplete::new(message)
}
