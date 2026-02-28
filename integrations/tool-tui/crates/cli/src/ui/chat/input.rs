use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

#[derive(Debug, Clone)]
pub struct InputState {
    pub content: String,
    pub cursor_position: usize,
    pub scroll_offset: usize,
    pub selection_start: Option<usize>,
    pub selection_end: Option<usize>,
}

impl InputState {
    pub fn new() -> Self {
        Self {
            content: String::new(),
            cursor_position: 0,
            scroll_offset: 0,
            selection_start: None,
            selection_end: None,
        }
    }

    pub fn has_selection(&self) -> bool {
        self.selection_start.is_some() && self.selection_end.is_some()
    }

    pub fn clear_selection(&mut self) {
        self.selection_start = None;
        self.selection_end = None;
    }

    pub fn select_all(&mut self) {
        if !self.content.is_empty() {
            self.selection_start = Some(0);
            self.selection_end = Some(self.content.len());
            self.cursor_position = self.content.len();
        }
    }

    pub fn get_selected_text(&self) -> Option<String> {
        if let (Some(start), Some(end)) = (self.selection_start, self.selection_end) {
            let (start, end) = if start < end {
                (start, end)
            } else {
                (end, start)
            };
            Some(self.content[start..end].to_string())
        } else {
            None
        }
    }

    pub fn delete_selection(&mut self) {
        if let (Some(start), Some(end)) = (self.selection_start, self.selection_end) {
            let (start, end) = if start < end {
                (start, end)
            } else {
                (end, start)
            };
            self.content.drain(start..end);
            self.cursor_position = start;
            self.clear_selection();
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> InputAction {
        match (key.code, key.modifiers) {
            (KeyCode::Char('c'), KeyModifiers::CONTROL) => InputAction::Exit,
            (KeyCode::Char('d'), KeyModifiers::CONTROL) if self.content.is_empty() => {
                InputAction::Exit
            }
            (KeyCode::Char('a'), KeyModifiers::CONTROL) => {
                self.select_all();
                InputAction::None
            }
            (KeyCode::Char('v'), KeyModifiers::CONTROL) => {
                // Paste from clipboard
                if let Ok(clipboard_content) = cli_clipboard::get_contents() {
                    if self.has_selection() {
                        self.delete_selection();
                    }
                    for c in clipboard_content.chars() {
                        self.insert_char(c);
                    }
                }
                InputAction::None
            }
            (KeyCode::Backspace, KeyModifiers::CONTROL) => {
                // Clear all text
                self.content.clear();
                self.cursor_position = 0;
                self.scroll_offset = 0;
                self.clear_selection();
                InputAction::None
            }
            (KeyCode::Enter, KeyModifiers::NONE) if !self.content.trim().is_empty() => {
                let msg = self.content.clone();
                self.content.clear();
                self.cursor_position = 0;
                self.scroll_offset = 0;
                self.clear_selection();
                InputAction::Submit(msg)
            }
            (KeyCode::Enter, KeyModifiers::SHIFT) => {
                if self.has_selection() {
                    self.delete_selection();
                }
                self.insert_char('\n');
                InputAction::None
            }
            (KeyCode::Backspace, _) => {
                if self.has_selection() {
                    self.delete_selection();
                } else {
                    self.delete_char();
                }
                InputAction::None
            }
            (KeyCode::Delete, _) => {
                if self.has_selection() {
                    self.delete_selection();
                } else {
                    self.delete_char_forward();
                }
                InputAction::None
            }
            (KeyCode::Left, KeyModifiers::SHIFT) => {
                if self.selection_start.is_none() {
                    self.selection_start = Some(self.cursor_position);
                }
                self.move_cursor_left();
                self.selection_end = Some(self.cursor_position);
                InputAction::None
            }
            (KeyCode::Right, KeyModifiers::SHIFT) => {
                if self.selection_start.is_none() {
                    self.selection_start = Some(self.cursor_position);
                }
                self.move_cursor_right();
                self.selection_end = Some(self.cursor_position);
                InputAction::None
            }
            (KeyCode::Left, _) => {
                self.clear_selection();
                self.move_cursor_left();
                InputAction::None
            }
            (KeyCode::Right, _) => {
                self.clear_selection();
                self.move_cursor_right();
                InputAction::None
            }
            (KeyCode::Up, _) => InputAction::PreviousHistory,
            (KeyCode::Down, _) => InputAction::NextHistory,
            (KeyCode::Home, _) => {
                self.clear_selection();
                self.cursor_position = 0;
                InputAction::None
            }
            (KeyCode::End, _) => {
                self.clear_selection();
                self.cursor_position = self.content.len();
                InputAction::None
            }
            (KeyCode::Char('e'), KeyModifiers::CONTROL) => {
                self.clear_selection();
                self.cursor_position = self.content.len();
                InputAction::None
            }
            (KeyCode::Char('u'), KeyModifiers::CONTROL) => {
                self.content.drain(..self.cursor_position);
                self.cursor_position = 0;
                self.clear_selection();
                InputAction::None
            }
            (KeyCode::Char('k'), KeyModifiers::CONTROL) => {
                self.content.truncate(self.cursor_position);
                self.clear_selection();
                InputAction::None
            }
            (KeyCode::Char('w'), KeyModifiers::CONTROL) => {
                self.delete_word();
                self.clear_selection();
                InputAction::None
            }
            (KeyCode::Char(c), KeyModifiers::NONE) | (KeyCode::Char(c), KeyModifiers::SHIFT) => {
                if self.has_selection() {
                    self.delete_selection();
                }
                self.insert_char(c);
                InputAction::None
            }
            _ => InputAction::None,
        }
    }

    fn insert_char(&mut self, c: char) {
        self.content.insert(self.cursor_position, c);
        self.cursor_position += c.len_utf8();
    }

    fn delete_char(&mut self) {
        if self.cursor_position > 0 {
            let prev_pos = self.prev_char_boundary();
            self.content.drain(prev_pos..self.cursor_position);
            self.cursor_position = prev_pos;
        }
    }

    fn delete_char_forward(&mut self) {
        if self.cursor_position < self.content.len() {
            let next_pos = self.next_char_boundary();
            self.content.drain(self.cursor_position..next_pos);
        }
    }

    fn delete_word(&mut self) {
        if self.cursor_position == 0 {
            return;
        }

        let mut pos = self.cursor_position;
        let chars: Vec<char> = self.content.chars().collect();

        // Skip trailing whitespace
        while pos > 0 && chars[pos - 1].is_whitespace() {
            pos -= 1;
        }

        // Delete word characters
        while pos > 0 && !chars[pos - 1].is_whitespace() {
            pos -= 1;
        }

        self.content.drain(pos..self.cursor_position);
        self.cursor_position = pos;
    }

    fn move_cursor_left(&mut self) {
        if self.cursor_position > 0 {
            self.cursor_position = self.prev_char_boundary();
        }
    }

    fn move_cursor_right(&mut self) {
        if self.cursor_position < self.content.len() {
            self.cursor_position = self.next_char_boundary();
        }
    }

    fn prev_char_boundary(&self) -> usize {
        let mut pos = self.cursor_position.saturating_sub(1);
        while pos > 0 && !self.content.is_char_boundary(pos) {
            pos -= 1;
        }
        pos
    }

    fn next_char_boundary(&self) -> usize {
        let mut pos = self.cursor_position + 1;
        while pos < self.content.len() && !self.content.is_char_boundary(pos) {
            pos += 1;
        }
        pos.min(self.content.len())
    }

    pub fn visible_content(&self, width: usize) -> &str {
        let end = (self.scroll_offset + width).min(self.content.len());
        &self.content[self.scroll_offset..end]
    }

    pub fn update_scroll(&mut self, width: usize) {
        if self.cursor_position < self.scroll_offset {
            self.scroll_offset = self.cursor_position;
        } else if self.cursor_position >= self.scroll_offset + width {
            self.scroll_offset = self.cursor_position.saturating_sub(width - 1);
        }
    }
}

impl Default for InputState {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputAction {
    None,
    Submit(String),
    Exit,
    PreviousHistory,
    NextHistory,
}
