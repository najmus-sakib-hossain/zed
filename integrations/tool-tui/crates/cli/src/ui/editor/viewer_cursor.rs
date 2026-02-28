//! Cursor movement logic for CodeViewer

use crate::ui::editor::viewer::CodeViewer;

impl CodeViewer {
    /// Ensure cursor is visible (scroll if needed)
    pub(crate) fn ensure_cursor_visible(&mut self) {
        let margin = self.config.scroll_margin as usize;

        // Vertical scrolling
        if self.cursor_line < self.scroll_offset + margin {
            self.scroll_offset = self.cursor_line.saturating_sub(margin);
        } else if self.cursor_line >= self.scroll_offset + self.visible_height - margin {
            self.scroll_offset = self.cursor_line + margin + 1 - self.visible_height;
        }

        // Horizontal scrolling
        let line_num_width = self.line_number_width();
        let content_width = self.visible_width.saturating_sub(line_num_width);

        if self.cursor_col < self.h_scroll_offset {
            self.h_scroll_offset = self.cursor_col;
        } else if self.cursor_col >= self.h_scroll_offset + content_width {
            self.h_scroll_offset = self.cursor_col + 1 - content_width;
        }
    }

    /// Clamp cursor column to line length
    pub(crate) fn clamp_cursor_col(&mut self) {
        if let Some(line) = self.lines.get(self.cursor_line) {
            self.cursor_col = self.cursor_col.min(line.len());
        }
    }

    /// Move cursor up
    pub fn cursor_up(&mut self, count: usize) {
        self.cursor_line = self.cursor_line.saturating_sub(count);
        self.clamp_cursor_col();
        self.ensure_cursor_visible();
    }

    /// Move cursor down
    pub fn cursor_down(&mut self, count: usize) {
        self.cursor_line = (self.cursor_line + count).min(self.lines.len().saturating_sub(1));
        self.clamp_cursor_col();
        self.ensure_cursor_visible();
    }

    /// Move cursor left
    pub fn cursor_left(&mut self, count: usize) {
        self.cursor_col = self.cursor_col.saturating_sub(count);
        self.ensure_cursor_visible();
    }

    /// Move cursor right
    pub fn cursor_right(&mut self, count: usize) {
        self.cursor_col += count;
        self.clamp_cursor_col();
        self.ensure_cursor_visible();
    }

    /// Move to start of line
    pub fn cursor_home(&mut self) {
        self.cursor_col = 0;
        self.ensure_cursor_visible();
    }

    /// Move to end of line
    pub fn cursor_end(&mut self) {
        if let Some(line) = self.lines.get(self.cursor_line) {
            self.cursor_col = line.len();
        }
        self.ensure_cursor_visible();
    }

    /// Move to first line
    pub fn cursor_top(&mut self) {
        self.cursor_line = 0;
        self.cursor_col = 0;
        self.ensure_cursor_visible();
    }

    /// Move to last line
    pub fn cursor_bottom(&mut self) {
        self.cursor_line = self.lines.len().saturating_sub(1);
        self.cursor_col = 0;
        self.ensure_cursor_visible();
    }

    /// Move forward one word
    pub fn word_forward(&mut self) {
        if let Some(line) = self.lines.get(self.cursor_line) {
            let chars: Vec<char> = line.chars().collect();
            let mut pos = self.cursor_col;

            // Skip current word
            while pos < chars.len() && !chars[pos].is_whitespace() {
                pos += 1;
            }
            // Skip whitespace
            while pos < chars.len() && chars[pos].is_whitespace() {
                pos += 1;
            }

            self.cursor_col = pos;
            self.ensure_cursor_visible();
        }
    }

    /// Move backward one word
    pub fn word_backward(&mut self) {
        if let Some(line) = self.lines.get(self.cursor_line) {
            let chars: Vec<char> = line.chars().collect();
            let mut pos = self.cursor_col;

            if pos > 0 {
                pos -= 1;
                // Skip whitespace
                while pos > 0 && chars[pos].is_whitespace() {
                    pos -= 1;
                }
                // Skip word
                while pos > 0 && !chars[pos].is_whitespace() {
                    pos -= 1;
                }
                if pos > 0 || chars[0].is_whitespace() {
                    pos += 1;
                }
            }

            self.cursor_col = pos;
            self.ensure_cursor_visible();
        }
    }

    /// Move to end of word
    pub fn word_end(&mut self) {
        if let Some(line) = self.lines.get(self.cursor_line) {
            let chars: Vec<char> = line.chars().collect();
            let mut pos = self.cursor_col;

            if pos < chars.len() {
                pos += 1;
                // Skip whitespace
                while pos < chars.len() && chars[pos].is_whitespace() {
                    pos += 1;
                }
                // Move to end of word
                while pos < chars.len() && !chars[pos].is_whitespace() {
                    pos += 1;
                }
                if pos > 0 {
                    pos -= 1;
                }
            }

            self.cursor_col = pos;
            self.ensure_cursor_visible();
        }
    }

    /// Page up
    pub fn page_up(&mut self) {
        let page_size = self.visible_height.saturating_sub(2);
        self.cursor_line = self.cursor_line.saturating_sub(page_size);
        self.scroll_offset = self.scroll_offset.saturating_sub(page_size);
        self.clamp_cursor_col();
    }

    /// Page down
    pub fn page_down(&mut self) {
        let page_size = self.visible_height.saturating_sub(2);
        self.cursor_line = (self.cursor_line + page_size).min(self.lines.len().saturating_sub(1));
        self.scroll_offset = (self.scroll_offset + page_size)
            .min(self.lines.len().saturating_sub(self.visible_height));
        self.clamp_cursor_col();
    }

    /// Go to specific line number
    pub fn goto_line(&mut self, line: usize) {
        self.cursor_line = (line.saturating_sub(1)).min(self.lines.len().saturating_sub(1));
        self.cursor_col = 0;
        self.ensure_cursor_visible();
    }
}
