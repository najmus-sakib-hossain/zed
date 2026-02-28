//! Single selection prompt

use super::interaction::{Event, PromptInteraction, State};
use super::{SYMBOLS, THEME};
use console::Term;
use owo_colors::OwoColorize;
use std::io;

/// A single item in a select list.
#[derive(Clone)]
pub struct SelectItem<T: Clone> {
    /// The value returned when this item is selected.
    pub value: T,
    /// The label displayed to the user.
    pub label: String,
    /// An optional hint shown next to the label.
    pub hint: Option<String>,
}

impl<T: Clone> SelectItem<T> {
    /// Creates a new select item.
    pub fn new(value: T, label: impl Into<String>) -> Self {
        Self {
            value,
            label: label.into(),
            hint: None,
        }
    }

    /// Adds a hint to the item.
    pub fn hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = Some(hint.into());
        self
    }
}

/// A single-selection prompt with arrow key navigation.
pub struct Select<T: Clone> {
    message: String,
    items: Vec<SelectItem<T>>,
    cursor: usize,
    state: State,
    last_render_lines: usize,
    filter: String,
    filtered_indices: Vec<usize>,
}

impl<T: Clone> Select<T> {
    /// Creates a new select prompt.
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            items: Vec::new(),
            cursor: 0,
            state: State::Active,
            last_render_lines: 0,
            filter: String::new(),
            filtered_indices: Vec::new(),
        }
    }

    /// Adds an item to the selection list.
    pub fn item(mut self, value: T, label: impl Into<String>, hint: impl Into<String>) -> Self {
        let label = label.into();
        let hint_str = hint.into();
        let item = if hint_str.is_empty() {
            SelectItem::new(value, label)
        } else {
            SelectItem::new(value, label).hint(hint_str)
        };
        self.items.push(item);
        self.filtered_indices.push(self.items.len() - 1);
        self
    }

    /// Sets all items at once.
    #[allow(unused)]
    pub fn items(mut self, items: Vec<SelectItem<T>>) -> Self {
        let count = items.len();
        self.items = items;
        self.filtered_indices = (0..count).collect();
        self
    }

    /// Sets the initial cursor position.
    #[allow(unused)]
    pub fn initial_value(mut self, index: usize) -> Self {
        self.cursor = index.min(self.items.len().saturating_sub(1));
        self
    }

    /// Updates the filter and filtered indices.
    fn update_filter(&mut self) {
        if self.filter.is_empty() {
            self.filtered_indices = (0..self.items.len()).collect();
        } else {
            let filter_lower = self.filter.to_lowercase();
            self.filtered_indices = self
                .items
                .iter()
                .enumerate()
                .filter(|(_, item)| item.label.to_lowercase().contains(&filter_lower))
                .map(|(i, _)| i)
                .collect();
        }
        // Ensure cursor is valid
        if !self.filtered_indices.is_empty() && self.cursor >= self.filtered_indices.len() {
            self.cursor = self.filtered_indices.len() - 1;
        }
    }

    /// Gets the currently selected item index in the original items list.
    fn selected_index(&self) -> Option<usize> {
        self.filtered_indices.get(self.cursor).copied()
    }
}

impl<T: Clone> PromptInteraction for Select<T> {
    type Output = T;

    fn state(&self) -> State {
        self.state
    }

    fn on(&mut self, event: Event) {
        match event {
            Event::Key(key) => match key {
                console::Key::Enter => {
                    if self.selected_index().is_some() {
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
                console::Key::Home => {
                    self.cursor = 0;
                }
                console::Key::End => {
                    self.cursor = self.filtered_indices.len().saturating_sub(1);
                }
                console::Key::Backspace => {
                    self.filter.pop();
                    self.update_filter();
                }
                console::Key::Char(c) if !c.is_control() => {
                    self.filter.push(c);
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
                // NO │ prefix on prompt line
                term.write_line(&format!("♦ {}", self.message))?;
                lines += 1;

                // Items (NO │ prefix)
                let max_visible = 8;
                let start = if self.cursor >= max_visible {
                    self.cursor - max_visible + 1
                } else {
                    0
                };
                let end = (start + max_visible).min(self.filtered_indices.len());

                for display_idx in start..end {
                    let item_idx = self.filtered_indices[display_idx];
                    let item = &self.items[item_idx];
                    let is_selected = display_idx == self.cursor;

                    let radio = if is_selected {
                        theme.primary.apply_to(symbols.radio_active).to_string()
                    } else {
                        theme.dim.apply_to(symbols.radio_inactive).to_string()
                    };

                    let label = if is_selected {
                        theme.primary.apply_to(&item.label).to_string()
                    } else {
                        item.label.clone()
                    };

                    let hint = item
                        .hint
                        .as_ref()
                        .map(|h| format!(" {}", theme.dim.apply_to(h)))
                        .unwrap_or_default();

                    term.write_line(&format!("  {}  {}{}", radio, label, hint))?;
                    lines += 1;
                }

                // Show scroll indicator if needed
                if self.filtered_indices.len() > max_visible {
                    let remaining = self.filtered_indices.len() - end;
                    if remaining > 0 {
                        term.write_line(&format!(
                            "  {}",
                            theme.dim.apply_to(format!("... {} more", remaining))
                        ))?;
                        lines += 1;
                    }
                }
                // Blank line with │ after prompt
                term.write_line(&format!("{}", theme.dim.apply_to(symbols.bar)))?;
                lines += 1;
            }
            State::Submit => {
                let checkmark = theme.success.apply_to("✓");
                let selected =
                    self.selected_index().map(|i| self.items[i].label.clone()).unwrap_or_default();
                let display = theme.dim.apply_to(&selected);
                term.write_line(&format!("{} {}  {}", checkmark, self.message, display))?;
                lines += 1;
                // Add blank line with bar after completion
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
        self.selected_index()
            .map(|i| self.items[i].value.clone())
            .expect("No item selected")
    }
}

/// Creates a new select prompt.
#[allow(unused)]
pub fn select<T: Clone>(message: impl Into<String>) -> Select<T> {
    Select::new(message)
}
