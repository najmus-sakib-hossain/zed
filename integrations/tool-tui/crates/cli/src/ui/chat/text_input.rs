use crossterm::event::{KeyCode, KeyModifiers};

#[derive(Debug, Clone)]
pub struct TextInput {
    pub content: String,
    pub cursor_position: usize,
    pub selection_start: Option<usize>,
    pub selection_end: Option<usize>,
}

impl TextInput {
    pub fn new() -> Self {
        Self {
            content: String::new(),
            cursor_position: 0,
            selection_start: None,
            selection_end: None,
        }
    }

    pub fn with_content(content: String) -> Self {
        let cursor_position = content.len();
        Self {
            content,
            cursor_position,
            selection_start: None,
            selection_end: None,
        }
    }

    pub fn handle_key(&mut self, key: KeyCode, modifiers: KeyModifiers) -> TextInputAction {
        match (key, modifiers) {
            // Ctrl+A - Select all
            (KeyCode::Char('a'), KeyModifiers::CONTROL) => {
                if !self.content.is_empty() {
                    self.selection_start = Some(0);
                    self.selection_end = Some(self.content.len());
                    self.cursor_position = self.content.len();
                }
                TextInputAction::None
            }
            // Ctrl+C - Copy
            (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
                if let Some(text) = self.get_selected_text() {
                    TextInputAction::Copy(text)
                } else {
                    TextInputAction::None
                }
            }
            // Ctrl+X - Cut
            (KeyCode::Char('x'), KeyModifiers::CONTROL) => {
                if let Some(text) = self.get_selected_text() {
                    self.delete_selection();
                    TextInputAction::Cut(text)
                } else {
                    TextInputAction::None
                }
            }
            // Ctrl+V - Paste (handled externally)
            (KeyCode::Char('v'), KeyModifiers::CONTROL) => TextInputAction::RequestPaste,
            // Ctrl+Backspace - Delete all
            (KeyCode::Backspace, KeyModifiers::CONTROL) => {
                self.content.clear();
                self.cursor_position = 0;
                self.clear_selection();
                TextInputAction::Changed
            }
            // Backspace - Delete character or selection
            (KeyCode::Backspace, KeyModifiers::NONE) => {
                if self.has_selection() {
                    self.delete_selection();
                } else if self.cursor_position > 0 {
                    self.content.remove(self.cursor_position - 1);
                    self.cursor_position -= 1;
                }
                self.clear_selection();
                TextInputAction::Changed
            }
            // Delete - Delete character forward
            (KeyCode::Delete, KeyModifiers::NONE) => {
                if self.has_selection() {
                    self.delete_selection();
                } else if self.cursor_position < self.content.len() {
                    self.content.remove(self.cursor_position);
                }
                self.clear_selection();
                TextInputAction::Changed
            }
            // Left arrow
            (KeyCode::Left, KeyModifiers::NONE) => {
                if self.cursor_position > 0 {
                    self.cursor_position -= 1;
                }
                self.clear_selection();
                TextInputAction::None
            }
            // Shift+Left - Select left
            (KeyCode::Left, KeyModifiers::SHIFT) => {
                if self.cursor_position > 0 {
                    if !self.has_selection() {
                        self.selection_start = Some(self.cursor_position);
                    }
                    self.cursor_position -= 1;
                    self.selection_end = Some(self.cursor_position);
                }
                TextInputAction::None
            }
            // Right arrow
            (KeyCode::Right, KeyModifiers::NONE) => {
                if self.cursor_position < self.content.len() {
                    self.cursor_position += 1;
                }
                self.clear_selection();
                TextInputAction::None
            }
            // Shift+Right - Select right
            (KeyCode::Right, KeyModifiers::SHIFT) => {
                if self.cursor_position < self.content.len() {
                    if !self.has_selection() {
                        self.selection_start = Some(self.cursor_position);
                    }
                    self.cursor_position += 1;
                    self.selection_end = Some(self.cursor_position);
                }
                TextInputAction::None
            }
            // Home - Go to start
            (KeyCode::Home, KeyModifiers::NONE) => {
                self.cursor_position = 0;
                self.clear_selection();
                TextInputAction::None
            }
            // End - Go to end
            (KeyCode::End, KeyModifiers::NONE) => {
                self.cursor_position = self.content.len();
                self.clear_selection();
                TextInputAction::None
            }
            // Character input
            (KeyCode::Char(c), KeyModifiers::NONE | KeyModifiers::SHIFT) => {
                // Check if it's a number key (0-9)
                if c.is_ascii_digit() {
                    // Return NumberKey action first, let the caller decide whether to close modal
                    return TextInputAction::NumberKey(c);
                }

                if self.has_selection() {
                    self.delete_selection();
                }
                self.content.insert(self.cursor_position, c);
                self.cursor_position += 1;
                self.clear_selection();
                TextInputAction::Changed
            }
            _ => TextInputAction::None,
        }
    }

    pub fn insert_text(&mut self, text: &str) {
        if self.has_selection() {
            self.delete_selection();
        }
        self.content.insert_str(self.cursor_position, text);
        self.cursor_position += text.len();
        self.clear_selection();
    }

    pub fn insert_char(&mut self, c: char) {
        if self.has_selection() {
            self.delete_selection();
        }
        self.content.insert(self.cursor_position, c);
        self.cursor_position += 1;
        self.clear_selection();
    }

    pub fn has_selection(&self) -> bool {
        self.selection_start.is_some() && self.selection_end.is_some()
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
            self.content.replace_range(start..end, "");
            self.cursor_position = start;
            self.clear_selection();
        }
    }

    pub fn clear_selection(&mut self) {
        self.selection_start = None;
        self.selection_end = None;
    }

    pub fn clear(&mut self) {
        self.content.clear();
        self.cursor_position = 0;
        self.clear_selection();
    }
}

impl Default for TextInput {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub enum TextInputAction {
    None,
    Changed,
    Copy(String),
    Cut(String),
    RequestPaste,
    NumberKey(char),
}
