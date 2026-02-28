//! Table editor for editing tabular data

use super::interaction::{Event, PromptInteraction, State};
use super::{SYMBOLS, THEME};
use console::Term;
use owo_colors::OwoColorize;
use std::io;

pub struct TableEditor {
    message: String,
    headers: Vec<String>,
    rows: Vec<Vec<String>>,
    cursor_row: usize,
    cursor_col: usize,
    editing: bool,
    edit_buffer: String,
    state: State,
    last_render_lines: usize,
}

impl TableEditor {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            headers: Vec::new(),
            rows: Vec::new(),
            cursor_row: 0,
            cursor_col: 0,
            editing: false,
            edit_buffer: String::new(),
            state: State::Active,
            last_render_lines: 0,
        }
    }

    pub fn headers(mut self, headers: Vec<String>) -> Self {
        self.headers = headers;
        self
    }

    pub fn rows(mut self, rows: Vec<Vec<String>>) -> Self {
        self.rows = rows;
        self
    }

    pub fn add_row(mut self, row: Vec<String>) -> Self {
        self.rows.push(row);
        self
    }

    fn render_table(&self) -> Vec<String> {
        let theme = THEME.read().unwrap();
        let mut lines = Vec::new();

        if self.headers.is_empty() {
            return lines;
        }

        // Calculate column widths
        let mut widths: Vec<usize> = self.headers.iter().map(|h| h.len()).collect();
        for row in &self.rows {
            for (i, cell) in row.iter().enumerate() {
                if i < widths.len() {
                    widths[i] = widths[i].max(cell.len());
                }
            }
        }

        // Header row
        let mut header_line = String::from("│ ");
        for (i, header) in self.headers.iter().enumerate() {
            let width = widths.get(i).copied().unwrap_or(10);
            header_line.push_str(&format!("{:width$} │ ", header, width = width));
        }
        lines.push(theme.primary.apply_to(header_line).bold().to_string());

        // Separator
        let mut sep = String::from("├");
        for width in &widths {
            sep.push_str(&"─".repeat(width + 2));
            sep.push('┼');
        }
        sep.pop();
        sep.push('┤');
        lines.push(theme.dim.apply_to(sep).to_string());

        // Data rows
        for (row_idx, row) in self.rows.iter().enumerate() {
            let mut row_line = String::from("│ ");
            for (col_idx, cell) in row.iter().enumerate() {
                let width = widths.get(col_idx).copied().unwrap_or(10);
                let is_active = row_idx == self.cursor_row && col_idx == self.cursor_col;

                let cell_display = if is_active && self.editing {
                    format!("{}█", self.edit_buffer)
                } else if is_active {
                    theme
                        .primary
                        .apply_to(format!("{:width$}", cell, width = width))
                        .bold()
                        .to_string()
                } else {
                    format!("{:width$}", cell, width = width)
                };

                row_line.push_str(&format!("{} │ ", cell_display));
            }
            lines.push(row_line);
        }

        lines
    }
}

impl PromptInteraction for TableEditor {
    type Output = Vec<Vec<String>>;

    fn state(&self) -> State {
        self.state
    }

    fn on(&mut self, event: Event) {
        if self.editing {
            match event {
                Event::Key(key) => match key {
                    console::Key::Enter => {
                        if self.cursor_row < self.rows.len()
                            && self.cursor_col < self.rows[self.cursor_row].len()
                        {
                            self.rows[self.cursor_row][self.cursor_col] = self.edit_buffer.clone();
                        }
                        self.editing = false;
                        self.edit_buffer.clear();
                    }
                    console::Key::Escape => {
                        self.editing = false;
                        self.edit_buffer.clear();
                    }
                    console::Key::Backspace => {
                        self.edit_buffer.pop();
                    }
                    console::Key::Char(c) if !c.is_control() => {
                        self.edit_buffer.push(c);
                    }
                    _ => {}
                },
                Event::Error => self.state = State::Error,
            }
        } else {
            match event {
                Event::Key(key) => match key {
                    console::Key::Enter => self.state = State::Submit,
                    console::Key::Escape => self.state = State::Cancel,
                    console::Key::Char('e') | console::Key::Char(' ') => {
                        if self.cursor_row < self.rows.len()
                            && self.cursor_col < self.rows[self.cursor_row].len()
                        {
                            self.editing = true;
                            self.edit_buffer = self.rows[self.cursor_row][self.cursor_col].clone();
                        }
                    }
                    console::Key::ArrowUp | console::Key::Char('k') => {
                        if self.cursor_row > 0 {
                            self.cursor_row -= 1;
                        }
                    }
                    console::Key::ArrowDown | console::Key::Char('j') => {
                        if self.cursor_row < self.rows.len().saturating_sub(1) {
                            self.cursor_row += 1;
                        }
                    }
                    console::Key::ArrowLeft | console::Key::Char('h') => {
                        if self.cursor_col > 0 {
                            self.cursor_col -= 1;
                        }
                    }
                    console::Key::ArrowRight | console::Key::Char('l') => {
                        if self.cursor_col < self.headers.len().saturating_sub(1) {
                            self.cursor_col += 1;
                        }
                    }
                    _ => {}
                },
                Event::Error => self.state = State::Error,
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
                term.write_line(&format!(
                    "{}{}",
                    theme.primary.apply_to(symbols.step_submit),
                    format!("  {}  ", self.message).bold()
                ))?;
                lines += 1;

                term.write_line(&format!("{}", bar))?;
                lines += 1;

                let table_lines = self.render_table();
                for line in table_lines {
                    term.write_line(&format!("{}  {}", bar, line))?;
                    lines += 1;
                }

                term.write_line(&format!("{}", bar))?;
                lines += 1;

                let help = if self.editing {
                    "Type to edit, Enter: save, Esc: cancel"
                } else {
                    "Arrow keys: navigate, e/Space: edit, Enter: done"
                };
                term.write_line(&format!("{}  {}", bar, theme.dim.apply_to(help)))?;
                lines += 1;
            }
            State::Submit => {
                let checkmark = theme.success.apply_to("✓");
                term.write_line(&format!(
                    "{} {}  {}",
                    checkmark,
                    self.message,
                    theme.dim.apply_to(format!("{} rows edited", self.rows.len()))
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

    fn value(&self) -> Vec<Vec<String>> {
        self.rows.clone()
    }
}

pub fn table_editor(message: impl Into<String>) -> TableEditor {
    TableEditor::new(message)
}
