//! Code snippet selector with syntax highlighting hints

use super::interaction::{Event, PromptInteraction, State};
use super::{SYMBOLS, THEME};
use console::Term;
use owo_colors::OwoColorize;
use std::io;

#[derive(Clone)]
pub struct CodeSnippet {
    pub name: String,
    pub language: String,
    pub code: String,
    pub description: String,
}

pub struct CodeSnippetPicker {
    message: String,
    snippets: Vec<CodeSnippet>,
    cursor: usize,
    show_preview: bool,
    state: State,
    last_render_lines: usize,
}

impl CodeSnippetPicker {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            snippets: Vec::new(),
            cursor: 0,
            show_preview: false,
            state: State::Active,
            last_render_lines: 0,
        }
    }

    pub fn snippet(mut self, snippet: CodeSnippet) -> Self {
        self.snippets.push(snippet);
        self
    }

    fn get_language_icon(&self, lang: &str) -> &'static str {
        match lang.to_lowercase().as_str() {
            "rust" | "rs" => "🦀",
            "javascript" | "js" => "📜",
            "typescript" | "ts" => "📘",
            "python" | "py" => "🐍",
            "go" => "🐹",
            "java" => "☕",
            "c" | "cpp" | "c++" => "⚙️",
            "html" => "🌐",
            "css" => "🎨",
            "json" => "📋",
            "yaml" | "yml" => "📄",
            "bash" | "sh" => "🐚",
            _ => "📝",
        }
    }

    fn render_code_preview(&self, code: &str) -> Vec<String> {
        code.lines()
            .take(5)
            .map(|line| {
                if line.trim().is_empty() {
                    String::new()
                } else {
                    format!("  {}", line)
                }
            })
            .collect()
    }
}

impl PromptInteraction for CodeSnippetPicker {
    type Output = CodeSnippet;

    fn state(&self) -> State {
        self.state
    }

    fn on(&mut self, event: Event) {
        match event {
            Event::Key(key) => match key {
                console::Key::Enter => {
                    if !self.snippets.is_empty() && self.cursor < self.snippets.len() {
                        self.state = State::Submit;
                    }
                }
                console::Key::Escape => self.state = State::Cancel,
                console::Key::Tab | console::Key::Char(' ') => {
                    self.show_preview = !self.show_preview;
                }
                console::Key::ArrowUp | console::Key::Char('k') => {
                    if self.cursor > 0 {
                        self.cursor -= 1;
                    }
                }
                console::Key::ArrowDown | console::Key::Char('j') => {
                    if self.cursor < self.snippets.len().saturating_sub(1) {
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

                for (i, snippet) in self.snippets.iter().enumerate() {
                    let marker = if i == self.cursor { "▸" } else { " " };
                    let icon = self.get_language_icon(&snippet.language);
                    let name_display = if i == self.cursor {
                        theme.primary.apply_to(&snippet.name).bold().to_string()
                    } else {
                        snippet.name.clone()
                    };

                    term.write_line(&format!(
                        "{}  {} {} {} {}",
                        bar,
                        marker,
                        icon,
                        name_display,
                        theme.dim.apply_to(format!("({})", snippet.language))
                    ))?;
                    lines += 1;

                    if i == self.cursor {
                        term.write_line(&format!(
                            "{}    {}",
                            bar,
                            theme.dim.apply_to(&snippet.description)
                        ))?;
                        lines += 1;
                    }
                }

                if self.show_preview
                    && let Some(snippet) = self.snippets.get(self.cursor)
                {
                    term.write_line(&format!("{}", bar))?;
                    lines += 1;

                    term.write_line(&format!(
                        "{}  {}",
                        bar,
                        theme.primary.apply_to("Preview:").bold()
                    ))?;
                    lines += 1;

                    let preview_lines = self.render_code_preview(&snippet.code);
                    for line in preview_lines {
                        term.write_line(&format!("{}  {}", bar, theme.dim.apply_to(line)))?;
                        lines += 1;
                    }

                    if snippet.code.lines().count() > 5 {
                        term.write_line(&format!("{}  {}", bar, theme.dim.apply_to("...")))?;
                        lines += 1;
                    }
                }

                term.write_line(&format!("{}", bar))?;
                lines += 1;

                term.write_line(&format!(
                    "{}  {}",
                    bar,
                    theme.dim.apply_to("↑↓: navigate, Space/Tab: preview, Enter: select")
                ))?;
                lines += 1;
            }
            State::Submit => {
                let checkmark = theme.success.apply_to("✓");
                let selected = self
                    .snippets
                    .get(self.cursor)
                    .map(|s| format!("{} ({})", s.name, s.language))
                    .unwrap_or_default();
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

    fn value(&self) -> CodeSnippet {
        self.snippets.get(self.cursor).cloned().unwrap_or_else(|| CodeSnippet {
            name: String::new(),
            language: String::new(),
            code: String::new(),
            description: String::new(),
        })
    }
}

pub fn code_snippet(message: impl Into<String>) -> CodeSnippetPicker {
    CodeSnippetPicker::new(message)
}
