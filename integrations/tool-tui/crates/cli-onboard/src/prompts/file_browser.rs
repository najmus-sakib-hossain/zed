//! File browser for selecting files and directories

use super::interaction::{Event, PromptInteraction, State};
use super::{SYMBOLS, THEME};
use console::Term;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

#[derive(Clone)]
struct FileEntry {
    path: PathBuf,
    name: String,
    is_dir: bool,
    expanded: bool,
}

pub struct FileBrowser {
    message: String,
    current_dir: PathBuf,
    entries: Vec<FileEntry>,
    cursor: usize,
    state: State,
    last_render_lines: usize,
    allow_dirs: bool,
}

impl FileBrowser {
    pub fn new(message: impl Into<String>) -> Self {
        let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let mut browser = Self {
            message: message.into(),
            current_dir: current_dir.clone(),
            entries: Vec::new(),
            cursor: 0,
            state: State::Active,
            last_render_lines: 0,
            allow_dirs: false,
        };
        browser.load_entries();
        browser
    }

    pub fn start_dir(mut self, path: impl AsRef<Path>) -> Self {
        self.current_dir = path.as_ref().to_path_buf();
        self.load_entries();
        self
    }

    pub fn allow_directories(mut self, allow: bool) -> Self {
        self.allow_dirs = allow;
        self
    }

    fn load_entries(&mut self) {
        self.entries.clear();
        self.cursor = 0;

        // Add parent directory entry
        if self.current_dir.parent().is_some() {
            self.entries.push(FileEntry {
                path: self.current_dir.parent().unwrap().to_path_buf(),
                name: "..".to_string(),
                is_dir: true,
                expanded: false,
            });
        }

        // Read directory entries
        if let Ok(read_dir) = fs::read_dir(&self.current_dir) {
            let mut entries: Vec<_> = read_dir
                .filter_map(|e| e.ok())
                .map(|e| {
                    let path = e.path();
                    let name = e.file_name().to_string_lossy().to_string();
                    let is_dir = path.is_dir();
                    FileEntry {
                        path,
                        name,
                        is_dir,
                        expanded: false,
                    }
                })
                .collect();

            // Sort: directories first, then files, alphabetically
            entries.sort_by(|a, b| match (a.is_dir, b.is_dir) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
            });

            self.entries.extend(entries);
        }
    }
}

impl PromptInteraction for FileBrowser {
    type Output = PathBuf;

    fn state(&self) -> State {
        self.state
    }

    fn on(&mut self, event: Event) {
        match event {
            Event::Key(key) => match key {
                console::Key::Enter => {
                    if let Some(entry) = self.entries.get(self.cursor) {
                        if entry.is_dir {
                            self.current_dir = entry.path.clone();
                            self.load_entries();
                        } else {
                            self.state = State::Submit;
                        }
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
                    if self.cursor < self.entries.len().saturating_sub(1) {
                        self.cursor += 1;
                    }
                }
                console::Key::Char(' ') => {
                    if let Some(entry) = self.entries.get(self.cursor)
                        && entry.is_dir
                        && self.allow_dirs
                    {
                        self.state = State::Submit;
                    }
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
                // NO │ prefix on prompt line
                term.write_line(&format!("♦ {}", self.message))?;
                lines += 1;

                // Show current directory
                let dir_display = self.current_dir.display().to_string();
                term.write_line(&format!(
                    "{} {}",
                    bar,
                    theme.dim.apply_to(format!("📁 {}", dir_display))
                ))?;
                lines += 1;

                // Show entries (limit to 10 for visibility)
                let visible_entries = self.entries.iter().take(10).enumerate();
                for (i, entry) in visible_entries {
                    let is_selected = i == self.cursor;
                    let icon = if entry.name == ".." {
                        "⬆️ "
                    } else if entry.is_dir {
                        "📁"
                    } else {
                        "📄"
                    };

                    let marker = if is_selected { "▸" } else { " " };
                    let name = if is_selected {
                        theme.primary.apply_to(&entry.name).to_string()
                    } else {
                        entry.name.clone()
                    };

                    term.write_line(&format!("{} {} {} {}", bar, marker, icon, name))?;
                    lines += 1;
                }

                if self.entries.len() > 10 {
                    term.write_line(&format!(
                        "{} {}",
                        bar,
                        theme.dim.apply_to(format!("... and {} more", self.entries.len() - 10))
                    ))?;
                    lines += 1;
                }

                let help = if self.allow_dirs {
                    "↑↓: navigate, Enter: open/select, Space: select dir, Esc: cancel"
                } else {
                    "↑↓: navigate, Enter: open/select, Esc: cancel"
                };
                term.write_line(&format!("{} {}", bar, theme.dim.apply_to(help)))?;
                lines += 1;
                // Blank line with │ after prompt
                term.write_line(&format!("{}", bar))?;
                lines += 1;
            }
            State::Submit => {
                let checkmark = theme.success.apply_to("✓");
                let selected_path = self
                    .entries
                    .get(self.cursor)
                    .map(|e| e.path.display().to_string())
                    .unwrap_or_default();
                term.write_line(&format!(
                    "{} {}  {}",
                    checkmark,
                    self.message,
                    theme.dim.apply_to(selected_path)
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

    fn value(&self) -> PathBuf {
        self.entries
            .get(self.cursor)
            .map(|e| e.path.clone())
            .unwrap_or_else(|| self.current_dir.clone())
    }
}

pub fn file_browser(message: impl Into<String>) -> FileBrowser {
    FileBrowser::new(message)
}
