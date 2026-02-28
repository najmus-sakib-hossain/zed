//! Search with filters for advanced filtering

use super::interaction::{Event, PromptInteraction, State};
use super::{SYMBOLS, THEME};
use console::Term;
use owo_colors::OwoColorize;
use std::io;

pub struct SearchFilter<T: Clone> {
    message: String,
    items: Vec<(T, String, Vec<String>)>,
    query: String,
    active_filters: Vec<String>,
    available_filters: Vec<String>,
    cursor: usize,
    mode: SearchMode,
    state: State,
    last_render_lines: usize,
}

#[derive(PartialEq)]
enum SearchMode {
    Search,
    Filter,
}

impl<T: Clone> SearchFilter<T> {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            items: Vec::new(),
            query: String::new(),
            active_filters: Vec::new(),
            available_filters: Vec::new(),
            cursor: 0,
            mode: SearchMode::Search,
            state: State::Active,
            last_render_lines: 0,
        }
    }

    pub fn item(mut self, value: T, label: impl Into<String>, tags: Vec<String>) -> Self {
        self.items.push((value, label.into(), tags));
        self
    }

    pub fn filter(mut self, filter: impl Into<String>) -> Self {
        self.available_filters.push(filter.into());
        self
    }

    fn matches_search(&self, label: &str, tags: &[String]) -> bool {
        if self.query.is_empty() {
            return true;
        }

        let query_lower = self.query.to_lowercase();
        let label_lower = label.to_lowercase();

        if label_lower.contains(&query_lower) {
            return true;
        }

        tags.iter().any(|tag| tag.to_lowercase().contains(&query_lower))
    }

    fn matches_filters(&self, tags: &[String]) -> bool {
        if self.active_filters.is_empty() {
            return true;
        }

        self.active_filters
            .iter()
            .all(|filter| tags.iter().any(|tag| tag.eq_ignore_ascii_case(filter)))
    }

    fn get_filtered_items(&self) -> Vec<(usize, &String)> {
        self.items
            .iter()
            .enumerate()
            .filter(|(_, (_, label, tags))| {
                self.matches_search(label, tags) && self.matches_filters(tags)
            })
            .map(|(i, (_, label, _))| (i, label))
            .collect()
    }

    fn toggle_filter(&mut self, filter: &str) {
        if let Some(pos) = self.active_filters.iter().position(|f| f == filter) {
            self.active_filters.remove(pos);
        } else {
            self.active_filters.push(filter.to_string());
        }
    }
}

impl<T: Clone> PromptInteraction for SearchFilter<T> {
    type Output = T;

    fn state(&self) -> State {
        self.state
    }

    fn on(&mut self, event: Event) {
        match event {
            Event::Key(key) => match key {
                console::Key::Enter => {
                    let filtered = self.get_filtered_items();
                    if !filtered.is_empty() && self.cursor < filtered.len() {
                        self.state = State::Submit;
                    }
                }
                console::Key::Escape => self.state = State::Cancel,
                console::Key::Tab => {
                    self.mode = match self.mode {
                        SearchMode::Search => SearchMode::Filter,
                        SearchMode::Filter => SearchMode::Search,
                    };
                }
                console::Key::Backspace if self.mode == SearchMode::Search => {
                    self.query.pop();
                    self.cursor = 0;
                }
                console::Key::Char(c) if self.mode == SearchMode::Search && !c.is_control() => {
                    self.query.push(c);
                    self.cursor = 0;
                }
                console::Key::ArrowUp | console::Key::Char('k')
                    if self.mode == SearchMode::Search =>
                {
                    if self.cursor > 0 {
                        self.cursor -= 1;
                    }
                }
                console::Key::ArrowDown | console::Key::Char('j')
                    if self.mode == SearchMode::Search =>
                {
                    let filtered = self.get_filtered_items();
                    if self.cursor < filtered.len().saturating_sub(1) {
                        self.cursor += 1;
                    }
                }
                console::Key::Char(' ') if self.mode == SearchMode::Filter => {
                    if self.cursor < self.available_filters.len() {
                        let filter = self.available_filters[self.cursor].clone();
                        self.toggle_filter(&filter);
                    }
                }
                console::Key::ArrowUp | console::Key::Char('k')
                    if self.mode == SearchMode::Filter =>
                {
                    if self.cursor > 0 {
                        self.cursor -= 1;
                    }
                }
                console::Key::ArrowDown | console::Key::Char('j')
                    if self.mode == SearchMode::Filter =>
                {
                    if self.cursor < self.available_filters.len().saturating_sub(1) {
                        self.cursor += 1;
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

                if self.mode == SearchMode::Search {
                    let display = if self.query.is_empty() {
                        format!("█{}", theme.dim.apply_to("Type to search..."))
                    } else {
                        format!("{}█", self.query)
                    };
                    term.write_line(&format!("{}  🔍 {}", bar, display))?;
                    lines += 1;

                    let filtered = self.get_filtered_items();
                    for (i, (_, label)) in filtered.iter().take(5).enumerate() {
                        let marker = if i == self.cursor { "▸" } else { " " };
                        let label_display = if i == self.cursor {
                            theme.primary.apply_to(label).bold().to_string()
                        } else {
                            label.to_string()
                        };
                        term.write_line(&format!("{}  {} {}", bar, marker, label_display))?;
                        lines += 1;
                    }

                    if filtered.len() > 5 {
                        term.write_line(&format!(
                            "{}  {}",
                            bar,
                            theme.dim.apply_to(format!("... {} more", filtered.len() - 5))
                        ))?;
                        lines += 1;
                    }
                } else {
                    term.write_line(&format!("{}  Filters:", bar))?;
                    lines += 1;

                    for (i, filter) in self.available_filters.iter().enumerate() {
                        let is_active = self.active_filters.contains(filter);
                        let marker = if i == self.cursor { "▸" } else { " " };
                        let checkbox = if is_active { "☑" } else { "☐" };
                        let label = if i == self.cursor {
                            theme.primary.apply_to(filter).bold().to_string()
                        } else {
                            filter.to_string()
                        };
                        term.write_line(&format!("{}  {} {} {}", bar, marker, checkbox, label))?;
                        lines += 1;
                    }
                }

                term.write_line(&format!("{}", bar))?;
                lines += 1;

                let help = if self.mode == SearchMode::Search {
                    "Tab: filters, ↑↓: navigate, Enter: select"
                } else {
                    "Tab: search, Space: toggle, ↑↓: navigate"
                };
                term.write_line(&format!("{}  {}", bar, theme.dim.apply_to(help)))?;
                lines += 1;
            }
            State::Submit => {
                let checkmark = theme.success.apply_to("✓");
                let filtered = self.get_filtered_items();
                let selected =
                    filtered.get(self.cursor).map(|(_, label)| label.as_str()).unwrap_or("");
                term.write_line(&format!(
                    "{} {}  {}",
                    checkmark,
                    self.message,
                    theme.dim.apply_to(selected)
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

    fn value(&self) -> T {
        let filtered = self.get_filtered_items();
        filtered
            .get(self.cursor)
            .and_then(|(idx, _)| self.items.get(*idx))
            .map(|(value, _, _)| value.clone())
            .expect("No item selected")
    }
}

pub fn search_filter<T: Clone>(message: impl Into<String>) -> SearchFilter<T> {
    SearchFilter::new(message)
}
