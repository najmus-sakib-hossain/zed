//! String cursor for text editing

use std::fmt::{Display, Formatter, Result};
use zeroize::{Zeroize, ZeroizeOnDrop};

/// A cursor for editing text strings with full navigation support.
#[derive(Default, Clone)]
#[allow(unused)]
pub struct StringCursor {
    value: Vec<char>,
    cursor: usize,
}

impl Zeroize for StringCursor {
    fn zeroize(&mut self) {
        self.value.zeroize();
        self.cursor = 0;
    }
}

/// Returns the indices of word boundaries in the given string.
#[allow(unused)]
fn word_jump_indices(value: &[char]) -> Vec<usize> {
    let mut indices = vec![0];
    let mut in_word = false;

    for (i, ch) in value.iter().enumerate() {
        if ch.is_whitespace() {
            in_word = false;
        } else if !in_word {
            indices.push(i);
            in_word = true;
        }
    }

    indices.push(value.len());
    indices
}

/// Returns the indices of line starts in the given string.
#[allow(unused)]
fn line_jump_indices(value: &[char]) -> Vec<usize> {
    value.split(|c| *c == '\n').fold(vec![0], |mut acc, line| {
        acc.push(acc.last().unwrap() + line.len() + 1);
        acc
    })
}

#[allow(unused)]
impl StringCursor {
    /// Creates a new empty cursor.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns `true` if the cursor contains no characters.
    pub fn is_empty(&self) -> bool {
        self.value.is_empty()
    }

    /// Returns the length of the string.
    pub fn len(&self) -> usize {
        self.value.len()
    }

    /// Returns the current cursor position.
    pub fn position(&self) -> usize {
        self.cursor
    }

    /// Returns a character at the current cursor position.
    pub fn current(&self) -> Option<char> {
        self.value.get(self.cursor).copied()
    }

    /// Extends the cursor with the given string.
    pub fn extend(&mut self, s: &str) {
        for c in s.chars() {
            self.value.push(c);
        }
        self.cursor = self.value.len();
    }

    /// Inserts a character at the current cursor position.
    pub fn insert(&mut self, chr: char) {
        self.value.insert(self.cursor, chr);
        self.cursor += 1;
    }

    /// Deletes the character to the left of the cursor.
    pub fn delete_left(&mut self) {
        if self.cursor > 0 {
            self.value.remove(self.cursor - 1);
            self.cursor -= 1;
        }
    }

    /// Deletes the character to the right of the cursor.
    pub fn delete_right(&mut self) {
        if self.cursor < self.value.len() {
            self.value.remove(self.cursor);
        }
    }

    /// Deletes the word to the left of the cursor.
    pub fn delete_word_to_the_left(&mut self) {
        let jumps = word_jump_indices(&self.value);
        let ix = jumps.binary_search(&self.cursor).unwrap_or_else(|i| i);
        let new_pos = jumps[ix.saturating_sub(1)];
        for _ in new_pos..self.cursor {
            self.value.remove(new_pos);
        }
        self.cursor = new_pos;
    }

    /// Moves the cursor one position left.
    pub fn move_left(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
        }
    }

    /// Moves the cursor one position right.
    pub fn move_right(&mut self) {
        if self.cursor < self.value.len() {
            self.cursor += 1;
        }
    }

    /// Moves the cursor up one line.
    pub fn move_up(&mut self) {
        let jumps = line_jump_indices(&self.value);
        self.cursor = match jumps.binary_search(&self.cursor) {
            Ok(ix) if ix + 1 < jumps.len() => {
                let target_line = ix.saturating_sub(1);
                jumps[target_line]
            }
            Ok(ix) | Err(ix) => {
                let ix = ix.saturating_sub(1);
                let target_line = ix.saturating_sub(1);
                let offset = std::cmp::min(
                    self.cursor - jumps[ix],
                    (jumps[ix] - jumps[target_line]).saturating_sub(1),
                );
                jumps[target_line] + offset
            }
        }
    }

    /// Moves the cursor down one line.
    pub fn move_down(&mut self) {
        let jumps = line_jump_indices(&self.value);
        self.cursor = match jumps.binary_search(&self.cursor) {
            Ok(ix) if ix + 1 < jumps.len() => {
                let target_line = std::cmp::min(ix + 1, jumps.len().saturating_sub(2));
                jumps[target_line]
            }
            Ok(ix) => jumps[ix].saturating_sub(1),
            Err(ix) => {
                let ix = ix.saturating_sub(1);
                let target_line = std::cmp::min(ix + 1, jumps.len().saturating_sub(2));
                let target_next = std::cmp::min(target_line + 1, jumps.len().saturating_sub(1));
                let offset = std::cmp::min(
                    self.cursor - jumps[ix],
                    (jumps[target_next] - jumps[target_line]).saturating_sub(1),
                );
                jumps[target_line] + offset
            }
        }
    }

    /// Moves the cursor left by a word.
    pub fn move_left_by_word(&mut self) {
        let jumps = word_jump_indices(&self.value);
        let ix = jumps.binary_search(&self.cursor).unwrap_or_else(|i| i);
        self.cursor = jumps[ix.saturating_sub(1)];
    }

    /// Moves the cursor right by a word.
    pub fn move_right_by_word(&mut self) {
        let jumps = word_jump_indices(&self.value);
        let ix = jumps
            .binary_search(&self.cursor)
            .map_or_else(|i| i, |i| i + 1);
        self.cursor = jumps[std::cmp::min(ix, jumps.len().saturating_sub(1))];
    }

    /// Moves the cursor to the start of the line.
    pub fn move_home(&mut self) {
        let jumps = line_jump_indices(&self.value);
        self.cursor = match jumps.binary_search(&self.cursor) {
            Ok(_) => self.cursor,
            Err(ix) => jumps[ix.saturating_sub(1)],
        }
    }

    /// Moves the cursor to the end of the line.
    pub fn move_end(&mut self) {
        let jumps = line_jump_indices(&self.value);
        self.cursor = match jumps.binary_search(&self.cursor) {
            Ok(ix) | Err(ix) => jumps[ix].saturating_sub(1).max(0),
        }
    }

    /// Splits the cursor into left, cursor, and right parts.
    pub fn split(&self) -> (&str, String, &str) {
        let left: String = self.value[..self.cursor].iter().collect();
        let cursor_char = self.current().unwrap_or(' ');
        let right_start = if self.cursor < self.value.len() {
            self.cursor + 1
        } else {
            self.cursor
        };
        let right: String = self.value[right_start..].iter().collect();

        // We need to return &str but we have String, so we'll use a different approach
        // Return owned strings wrapped in a tuple
        (
            Box::leak(left.into_boxed_str()),
            cursor_char.to_string(),
            Box::leak(right.into_boxed_str()),
        )
    }

    /// Returns the string value.
    pub fn value(&self) -> String {
        self.value.iter().collect()
    }

    /// Clears all content.
    pub fn clear(&mut self) {
        self.value.clear();
        self.cursor = 0;
    }
}

impl Display for StringCursor {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "{}", self.value())
    }
}

impl From<&str> for StringCursor {
    fn from(s: &str) -> Self {
        let mut cursor = Self::new();
        cursor.extend(s);
        cursor
    }
}

impl From<String> for StringCursor {
    fn from(s: String) -> Self {
        Self::from(s.as_str())
    }
}

impl ZeroizeOnDrop for StringCursor {}
