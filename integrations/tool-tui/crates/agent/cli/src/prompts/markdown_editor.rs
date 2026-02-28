//! Markdown editor with preview

use super::interaction::{Event, PromptInteraction, State};
use super::{SYMBOLS, THEME};
use console::Term;
use owo_colors::OwoColorize;
use std::io;

pub struct MarkdownEditor {
    message: String,
    content: String,
    cursor_line: usize,
    show_preview: bool,
    state: State,
    last_render_lines: usize,
}

impl MarkdownEditor {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            content: String::new(),
            cursor_line: 0,
            show_preview: false,
            state: State::Active,
            last_render_lines: 0,
        }
    }

    pub fn initial_content(mut self, content: impl Into<String>) -> Self {
        self.content = content.into();
        self.cursor_line = self.content.lines().count().saturating_sub(1);
        self
    }

    fn get_lines(&self) -> Vec<String> {
        self.content.lines().map(|s| s.to_string()).collect()
    }

    fn render_preview(&self) -> Vec<String> {
        let lines = self.get_lines();
        let mut preview = Vec::new();

        for line in lines {
            if line.starts_with("# ") {
                preview.push(format!("━━ {} ━━", &line[2..]));
            } else if line.starts_with("## ") {
                preview.push(format!("── {} ──", &line[3..]));
            } else if line.starts_with("### ") {
                preview.push(format!("• {}", &line[4..]));
            } else if line.starts_with("- ") || line.starts_with("* ") {
                preview.push(format!("  • {}", &line[2..]));
            } else if line.starts_with("**") && line.ends_with("**") && line.len() > 4 {
                preview.push(line[2..line.len() - 2].to_string());
            } else if line.starts_with("*") && line.ends_with("*") && line.len() > 2 {
                preview.push(line[1..line.len() - 1].to_string());
            } else if line.starts_with("`") && line.ends_with("`") && line.len() > 2 {
                preview.push(format!("「{}」", &line[1..line.len() - 1]));
            } else {
                preview.push(line);
            }
        }

        preview
    }
}

impl PromptInteraction for MarkdownEditor {
    type Output = String;

    fn state(&self) -> State {
        self.state
    }

    fn on(&mut self, event: Event) {
        match event {
            Event::Key(key) => match key {
                console::Key::Enter if self.content.contains('\n') => {
                    self.state = State::Submit;
                }
                console::Key::Enter => {
                    self.content.push('\n');
                    self.cursor_line += 1;
                }
                console::Key::Escape => self.state = State::Cancel,
                console::Key::Tab => {
                    self.show_preview = !self.show_preview;
                }
                console::Key::Backspace => {
                    if self.content.ends_with('\n') {
                        self.content.pop();
                        if self.cursor_line > 0 {
                            self.cursor_line -= 1;
                        }
                    } else {
                        self.content.pop();
                    }
                }
                console::Key::Char(c) if !c.is_control() => {
                    self.content.push(c);
                }
                console::Key::ArrowUp => {
                    if self.cursor_line > 0 {
                        self.cursor_line -= 1;
                    }
                }
                console::Key::ArrowDown => {
                    let lines = self.get_lines();
                    if self.cursor_line < lines.len().saturating_sub(1) {
                        self.cursor_line += 1;
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
                let mode = if self.show_preview { "Preview" } else { "Edit" };
                term.write_line(&format!(
                    "{}{}  [{}]",
                    theme.primary.apply_to(symbols.step_submit),
                    format!("  {}  ", self.message).bold(),
                    theme.primary.apply_to(mode)
                ))?;
                lines += 1;

                term.write_line(&format!("{}", bar))?;
                lines += 1;

                if self.show_preview {
                    let preview_lines = self.render_preview();
                    for line in preview_lines.iter().take(8) {
                        term.write_line(&format!("{}  {}", bar, line))?;
                        lines += 1;
                    }
                } else {
                    let content_lines = self.get_lines();
                    for (i, line) in content_lines.iter().take(8).enumerate() {
                        let marker = if i == self.cursor_line { "▸" } else { " " };
                        term.write_line(&format!("{}  {} {}", bar, marker, line))?;
                        lines += 1;
                    }

                    if content_lines.len() > 8 {
                        term.write_line(&format!("{}  {}", bar, theme.dim.apply_to("...")))?;
                        lines += 1;
                    }
                }

                term.write_line(&format!("{}", bar))?;
                lines += 1;

                term.write_line(&format!(
                    "{}  {}",
                    bar,
                    theme
                        .dim
                        .apply_to("Tab: toggle preview, Enter: new line/submit, Esc: cancel")
                ))?;
                lines += 1;
            }
            State::Submit => {
                let checkmark = theme.success.apply_to("✓");
                let line_count = self.get_lines().len();
                term.write_line(&format!(
                    "{} {}  {}",
                    checkmark,
                    self.message,
                    theme.dim.apply_to(format!("{} lines", line_count))
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

    fn value(&self) -> String {
        self.content.clone()
    }
}

pub fn markdown_editor(message: impl Into<String>) -> MarkdownEditor {
    MarkdownEditor::new(message)
}
