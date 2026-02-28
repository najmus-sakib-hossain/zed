//! Matrix select for 2D grid selection

use super::interaction::{Event, PromptInteraction, State};
use super::{SYMBOLS, THEME};
use console::Term;
use owo_colors::OwoColorize;
use std::io;

pub struct MatrixSelect<T: Clone> {
    message: String,
    items: Vec<Vec<(T, String)>>,
    cursor_row: usize,
    cursor_col: usize,
    selected: Vec<Vec<bool>>,
    state: State,
    last_render_lines: usize,
}

impl<T: Clone> MatrixSelect<T> {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            items: Vec::new(),
            cursor_row: 0,
            cursor_col: 0,
            selected: Vec::new(),
            state: State::Active,
            last_render_lines: 0,
        }
    }

    pub fn row(mut self, items: Vec<(T, String)>) -> Self {
        let row_selected = vec![false; items.len()];
        self.items.push(items);
        self.selected.push(row_selected);
        self
    }

    fn toggle_current(&mut self) {
        if self.cursor_row < self.selected.len()
            && self.cursor_col < self.selected[self.cursor_row].len()
        {
            self.selected[self.cursor_row][self.cursor_col] =
                !self.selected[self.cursor_row][self.cursor_col];
        }
    }

    fn get_selected_values(&self) -> Vec<T> {
        let mut result = Vec::new();
        for (row_idx, row) in self.items.iter().enumerate() {
            for (col_idx, (value, _)) in row.iter().enumerate() {
                if self
                    .selected
                    .get(row_idx)
                    .and_then(|r| r.get(col_idx))
                    .copied()
                    .unwrap_or(false)
                {
                    result.push(value.clone());
                }
            }
        }
        result
    }
}

impl<T: Clone> PromptInteraction for MatrixSelect<T> {
    type Output = Vec<T>;

    fn state(&self) -> State {
        self.state
    }

    fn on(&mut self, event: Event) {
        match event {
            Event::Key(key) => match key {
                console::Key::Enter => self.state = State::Submit,
                console::Key::Escape => self.state = State::Cancel,
                console::Key::Char(' ') => self.toggle_current(),
                console::Key::ArrowUp | console::Key::Char('k') => {
                    if self.cursor_row > 0 {
                        self.cursor_row -= 1;
                    }
                }
                console::Key::ArrowDown | console::Key::Char('j') => {
                    if self.cursor_row < self.items.len().saturating_sub(1) {
                        self.cursor_row += 1;
                    }
                }
                console::Key::ArrowLeft | console::Key::Char('h') => {
                    if self.cursor_col > 0 {
                        self.cursor_col -= 1;
                    }
                }
                console::Key::ArrowRight | console::Key::Char('l') => {
                    if let Some(row) = self.items.get(self.cursor_row)
                        && self.cursor_col < row.len().saturating_sub(1)
                    {
                        self.cursor_col += 1;
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

                for (row_idx, row) in self.items.iter().enumerate() {
                    let mut row_line = String::from("  ");

                    for (col_idx, (_, label)) in row.iter().enumerate() {
                        let is_cursor = row_idx == self.cursor_row && col_idx == self.cursor_col;
                        let is_selected = self
                            .selected
                            .get(row_idx)
                            .and_then(|r| r.get(col_idx))
                            .copied()
                            .unwrap_or(false);

                        let checkbox = if is_selected { "☑" } else { "☐" };
                        let display = if is_cursor {
                            format!("▸{} {}", checkbox, theme.primary.apply_to(label).bold())
                        } else {
                            format!(" {} {}", checkbox, label)
                        };

                        row_line.push_str(&display);
                        row_line.push_str("  ");
                    }

                    term.write_line(&format!("{}  {}", bar, row_line))?;
                    lines += 1;
                }

                term.write_line(&format!("{}", bar))?;
                lines += 1;

                let selected_count = self.get_selected_values().len();
                term.write_line(&format!(
                    "{}  {} selected",
                    bar,
                    theme.primary.apply_to(selected_count)
                ))?;
                lines += 1;

                term.write_line(&format!(
                    "{}  {}",
                    bar,
                    theme.dim.apply_to("Arrow keys: navigate, Space: toggle, Enter: confirm")
                ))?;
                lines += 1;
            }
            State::Submit => {
                let checkmark = theme.success.apply_to("✓");
                let count = self.get_selected_values().len();
                term.write_line(&format!(
                    "{} {}  {}",
                    checkmark,
                    self.message,
                    theme.dim.apply_to(format!("{} items selected", count))
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

    fn value(&self) -> Vec<T> {
        self.get_selected_values()
    }
}

pub fn matrix_select<T: Clone>(message: impl Into<String>) -> MatrixSelect<T> {
    MatrixSelect::new(message)
}
