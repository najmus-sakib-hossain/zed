//! Emoji picker with categories

use super::interaction::{Event, PromptInteraction, State};
use super::{SYMBOLS, THEME};
use console::Term;
use std::io;

pub struct EmojiPicker {
    message: String,
    categories: Vec<(String, Vec<(String, String)>)>,
    current_category: usize,
    cursor: usize,
    query: String,
    state: State,
    last_render_lines: usize,
}

impl EmojiPicker {
    pub fn new(message: impl Into<String>) -> Self {
        let mut picker = Self {
            message: message.into(),
            categories: Vec::new(),
            current_category: 0,
            cursor: 0,
            query: String::new(),
            state: State::Active,
            last_render_lines: 0,
        };
        picker.init_default_emojis();
        picker
    }

    fn init_default_emojis(&mut self) {
        self.categories = vec![
            (
                "Smileys".to_string(),
                vec![
                    ("😀".to_string(), "grinning".to_string()),
                    ("😃".to_string(), "smile".to_string()),
                    ("😄".to_string(), "happy".to_string()),
                    ("😁".to_string(), "grin".to_string()),
                    ("😅".to_string(), "sweat".to_string()),
                    ("😂".to_string(), "joy".to_string()),
                    ("🤣".to_string(), "rofl".to_string()),
                    ("😊".to_string(), "blush".to_string()),
                ],
            ),
            (
                "Gestures".to_string(),
                vec![
                    ("👍".to_string(), "thumbsup".to_string()),
                    ("👎".to_string(), "thumbsdown".to_string()),
                    ("👏".to_string(), "clap".to_string()),
                    ("🙌".to_string(), "raised_hands".to_string()),
                    ("👋".to_string(), "wave".to_string()),
                    ("🤝".to_string(), "handshake".to_string()),
                    ("🙏".to_string(), "pray".to_string()),
                    ("✌️".to_string(), "peace".to_string()),
                ],
            ),
            (
                "Objects".to_string(),
                vec![
                    ("💻".to_string(), "laptop".to_string()),
                    ("📱".to_string(), "phone".to_string()),
                    ("⌨️".to_string(), "keyboard".to_string()),
                    ("🖱️".to_string(), "mouse".to_string()),
                    ("🖥️".to_string(), "desktop".to_string()),
                    ("📧".to_string(), "email".to_string()),
                    ("📁".to_string(), "folder".to_string()),
                    ("📊".to_string(), "chart".to_string()),
                ],
            ),
            (
                "Symbols".to_string(),
                vec![
                    ("✅".to_string(), "check".to_string()),
                    ("❌".to_string(), "cross".to_string()),
                    ("⚠️".to_string(), "warning".to_string()),
                    ("🔥".to_string(), "fire".to_string()),
                    ("⭐".to_string(), "star".to_string()),
                    ("💡".to_string(), "bulb".to_string()),
                    ("🚀".to_string(), "rocket".to_string()),
                    ("🎯".to_string(), "target".to_string()),
                ],
            ),
        ];
    }

    fn get_filtered_emojis(&self) -> Vec<(String, String)> {
        if self.query.is_empty() {
            if let Some((_, emojis)) = self.categories.get(self.current_category) {
                return emojis.clone();
            }
            return Vec::new();
        }

        let query_lower = self.query.to_lowercase();
        let mut results = Vec::new();

        for (_, emojis) in &self.categories {
            for (emoji, name) in emojis {
                if name.to_lowercase().contains(&query_lower) {
                    results.push((emoji.clone(), name.clone()));
                }
            }
        }

        results
    }
}

impl PromptInteraction for EmojiPicker {
    type Output = String;

    fn state(&self) -> State {
        self.state
    }

    fn on(&mut self, event: Event) {
        match event {
            Event::Key(key) => match key {
                console::Key::Enter => {
                    let emojis = self.get_filtered_emojis();
                    if !emojis.is_empty() && self.cursor < emojis.len() {
                        self.state = State::Submit;
                    }
                }
                console::Key::Escape => self.state = State::Cancel,
                console::Key::Tab => {
                    if self.query.is_empty() {
                        self.current_category = (self.current_category + 1) % self.categories.len();
                        self.cursor = 0;
                    }
                }
                console::Key::Backspace => {
                    self.query.pop();
                    self.cursor = 0;
                }
                console::Key::Char(c) if !c.is_control() => {
                    self.query.push(c);
                    self.cursor = 0;
                }
                console::Key::ArrowUp | console::Key::Char('k') => {
                    if self.cursor > 0 {
                        self.cursor -= 1;
                    }
                }
                console::Key::ArrowDown | console::Key::Char('j') => {
                    let emojis = self.get_filtered_emojis();
                    if self.cursor < emojis.len().saturating_sub(1) {
                        self.cursor += 1;
                    }
                }
                console::Key::ArrowLeft | console::Key::Char('h') if self.query.is_empty() => {
                    if self.current_category > 0 {
                        self.current_category -= 1;
                        self.cursor = 0;
                    }
                }
                console::Key::ArrowRight | console::Key::Char('l') if self.query.is_empty() => {
                    if self.current_category < self.categories.len() - 1 {
                        self.current_category += 1;
                        self.cursor = 0;
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
                // NO │ prefix on prompt line
                term.write_line(&format!("♦ {}", self.message))?;
                lines += 1;

                if self.query.is_empty() {
                    let category_names: Vec<String> = self
                        .categories
                        .iter()
                        .enumerate()
                        .map(|(i, (name, _))| {
                            if i == self.current_category {
                                theme.primary.apply_to(name).bold().to_string()
                            } else {
                                theme.dim.apply_to(name).to_string()
                            }
                        })
                        .collect();
                    term.write_line(&format!("  {}", category_names.join(" | ")))?;
                    lines += 1;
                } else {
                    let display = format!("{}█", self.query);
                    term.write_line(&format!("  🔍 {}", display))?;
                    lines += 1;
                }

                let emojis = self.get_filtered_emojis();
                for (i, (emoji, name)) in emojis.iter().take(8).enumerate() {
                    let marker = if i == self.cursor { "▸" } else { " " };
                    let display = if i == self.cursor {
                        format!("{} {}", emoji, theme.primary.apply_to(name).bold())
                    } else {
                        format!("{} {}", emoji, name)
                    };
                    term.write_line(&format!("  {} {}", marker, display))?;
                    lines += 1;
                }

                if emojis.len() > 8 {
                    term.write_line(&format!(
                        "  {}",
                        theme.dim.apply_to(format!("... {} more", emojis.len() - 8))
                    ))?;
                    lines += 1;
                }

                let help = if self.query.is_empty() {
                    "← →: category, ↑↓: select, Type: search, Enter: confirm"
                } else {
                    "Type: search, ↑↓: select, Backspace: clear, Enter: confirm"
                };
                term.write_line(&format!("  {}", theme.dim.apply_to(help)))?;
                lines += 1;
                // Blank line with │ after prompt
                term.write_line(&format!("{}", theme.dim.apply_to(symbols.bar)))?;
                lines += 1;
            }
            State::Submit => {
                let checkmark = theme.success.apply_to("✓");
                let emojis = self.get_filtered_emojis();
                let selected = emojis
                    .get(self.cursor)
                    .map(|(e, n)| format!("{} {}", e, n))
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

    fn value(&self) -> String {
        let emojis = self.get_filtered_emojis();
        emojis.get(self.cursor).map(|(emoji, _)| emoji.clone()).unwrap_or_default()
    }
}

pub fn emoji_picker(message: impl Into<String>) -> EmojiPicker {
    EmojiPicker::new(message)
}
