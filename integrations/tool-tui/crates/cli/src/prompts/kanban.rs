//! Kanban board for task management

use super::interaction::{Event, PromptInteraction, State};
use super::{SYMBOLS, THEME};
use console::Term;
use owo_colors::OwoColorize;
use std::io;

#[derive(Clone)]
pub struct KanbanTask {
    pub id: String,
    pub title: String,
    pub description: String,
}

pub struct KanbanBoard {
    message: String,
    columns: Vec<(String, Vec<KanbanTask>)>,
    current_column: usize,
    cursor: usize,
    state: State,
    last_render_lines: usize,
}

impl KanbanBoard {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            columns: vec![
                ("To Do".to_string(), Vec::new()),
                ("In Progress".to_string(), Vec::new()),
                ("Done".to_string(), Vec::new()),
            ],
            current_column: 0,
            cursor: 0,
            state: State::Active,
            last_render_lines: 0,
        }
    }

    pub fn column(mut self, name: impl Into<String>) -> Self {
        self.columns.push((name.into(), Vec::new()));
        self
    }

    pub fn task(mut self, column_idx: usize, task: KanbanTask) -> Self {
        if let Some((_, tasks)) = self.columns.get_mut(column_idx) {
            tasks.push(task);
        }
        self
    }

    fn move_task_right(&mut self) {
        if self.current_column < self.columns.len() - 1 {
            let task = {
                if let Some((_, tasks)) = self.columns.get_mut(self.current_column) {
                    if self.cursor < tasks.len() {
                        Some(tasks.remove(self.cursor))
                    } else {
                        None
                    }
                } else {
                    None
                }
            };

            if let Some(task) = task {
                if let Some((_, next_tasks)) = self.columns.get_mut(self.current_column + 1) {
                    next_tasks.push(task);
                }

                if let Some((_, tasks)) = self.columns.get(self.current_column) {
                    if self.cursor >= tasks.len() && self.cursor > 0 {
                        self.cursor -= 1;
                    }
                }
            }
        }
    }

    fn move_task_left(&mut self) {
        if self.current_column > 0 {
            let task = {
                if let Some((_, tasks)) = self.columns.get_mut(self.current_column) {
                    if self.cursor < tasks.len() {
                        Some(tasks.remove(self.cursor))
                    } else {
                        None
                    }
                } else {
                    None
                }
            };

            if let Some(task) = task {
                if let Some((_, prev_tasks)) = self.columns.get_mut(self.current_column - 1) {
                    prev_tasks.push(task);
                }

                if let Some((_, tasks)) = self.columns.get(self.current_column) {
                    if self.cursor >= tasks.len() && self.cursor > 0 {
                        self.cursor -= 1;
                    }
                }
            }
        }
    }
}

impl PromptInteraction for KanbanBoard {
    type Output = Vec<(String, Vec<KanbanTask>)>;

    fn state(&self) -> State {
        self.state
    }

    fn on(&mut self, event: Event) {
        match event {
            Event::Key(key) => match key {
                console::Key::Enter => self.state = State::Submit,
                console::Key::Escape => self.state = State::Cancel,
                console::Key::Tab => {
                    self.current_column = (self.current_column + 1) % self.columns.len();
                    self.cursor = 0;
                }
                console::Key::ArrowUp | console::Key::Char('k') => {
                    if self.cursor > 0 {
                        self.cursor -= 1;
                    }
                }
                console::Key::ArrowDown | console::Key::Char('j') => {
                    if let Some((_, tasks)) = self.columns.get(self.current_column) {
                        if self.cursor < tasks.len().saturating_sub(1) {
                            self.cursor += 1;
                        }
                    }
                }
                console::Key::ArrowRight | console::Key::Char('l') => {
                    self.move_task_right();
                }
                console::Key::ArrowLeft | console::Key::Char('h') => {
                    self.move_task_left();
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

                let column_names: Vec<String> = self
                    .columns
                    .iter()
                    .enumerate()
                    .map(|(i, (name, tasks))| {
                        let display = format!("{} ({})", name, tasks.len());
                        if i == self.current_column {
                            theme.primary.apply_to(display).bold().to_string()
                        } else {
                            theme.dim.apply_to(display).to_string()
                        }
                    })
                    .collect();
                term.write_line(&format!("{}  {}", bar, column_names.join(" │ ")))?;
                lines += 1;

                term.write_line(&format!("{}", bar))?;
                lines += 1;

                if let Some((_, tasks)) = self.columns.get(self.current_column) {
                    if tasks.is_empty() {
                        term.write_line(&format!("{}  {}", bar, theme.dim.apply_to("No tasks")))?;
                        lines += 1;
                    } else {
                        for (i, task) in tasks.iter().take(6).enumerate() {
                            let marker = if i == self.cursor { "▸" } else { " " };
                            let title = if i == self.cursor {
                                theme.primary.apply_to(&task.title).bold().to_string()
                            } else {
                                task.title.clone()
                            };

                            term.write_line(&format!("{}  {} 📋 {}", bar, marker, title))?;
                            lines += 1;

                            if i == self.cursor && !task.description.is_empty() {
                                term.write_line(&format!(
                                    "{}     {}",
                                    bar,
                                    theme.dim.apply_to(&task.description)
                                ))?;
                                lines += 1;
                            }
                        }

                        if tasks.len() > 6 {
                            term.write_line(&format!(
                                "{}  {}",
                                bar,
                                theme.dim.apply_to(format!("... {} more", tasks.len() - 6))
                            ))?;
                            lines += 1;
                        }
                    }
                }

                term.write_line(&format!("{}", bar))?;
                lines += 1;

                term.write_line(&format!(
                    "{}  {}",
                    bar,
                    theme.dim.apply_to("Tab: column, ↑↓: task, ← →: move task, Enter: done")
                ))?;
                lines += 1;
            }
            State::Submit => {
                let checkmark = theme.success.apply_to("✓");
                let total_tasks: usize = self.columns.iter().map(|(_, tasks)| tasks.len()).sum();
                term.write_line(&format!(
                    "{} {}  {}",
                    checkmark,
                    self.message,
                    theme.dim.apply_to(format!(
                        "{} tasks across {} columns",
                        total_tasks,
                        self.columns.len()
                    ))
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

    fn value(&self) -> Vec<(String, Vec<KanbanTask>)> {
        self.columns.clone()
    }
}

pub fn kanban(message: impl Into<String>) -> KanbanBoard {
    KanbanBoard::new(message)
}
