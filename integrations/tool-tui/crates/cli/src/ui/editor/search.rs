//! Search Module for Code Editor
//!
//! Provides search functionality with regex support, forward/backward search,
//! and integration with Vim keybindings (/, ?, n, N).
//!
//! # Features
//!
//! - Literal and regex search modes
//! - Forward and backward search
//! - Case-sensitive and case-insensitive search
//! - Search highlighting
//! - Jump to next/previous match
//!
//! # Example
//!
//! ```rust,ignore
//! use dx_cli::ui::editor::search::{SearchEngine, SearchMode};
//!
//! let mut search = SearchEngine::new();
//! search.set_pattern("fn\\s+\\w+", SearchMode::Regex)?;
//! let matches = search.find_all(&lines);
//! ```

use anyhow::{Context, Result};
use regex::Regex;
use serde::{Deserialize, Serialize};

/// Search mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum SearchMode {
    /// Literal string search
    #[default]
    Literal,
    /// Regular expression search
    Regex,
}

/// Search direction
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SearchDirection {
    /// Search forward from cursor
    #[default]
    Forward,
    /// Search backward from cursor
    Backward,
}

/// A single search match
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchMatch {
    /// Line index (0-based)
    pub line: usize,
    /// Column start (0-based)
    pub col_start: usize,
    /// Column end (0-based, exclusive)
    pub col_end: usize,
    /// Matched text
    pub text: String,
}

impl SearchMatch {
    /// Create a new search match
    pub fn new(line: usize, col_start: usize, col_end: usize, text: String) -> Self {
        Self {
            line,
            col_start,
            col_end,
            text,
        }
    }

    /// Check if this match contains a position
    pub fn contains(&self, line: usize, col: usize) -> bool {
        self.line == line && col >= self.col_start && col < self.col_end
    }
}

/// Search engine for code editor
pub struct SearchEngine {
    /// Current search pattern
    pattern: Option<String>,
    /// Compiled regex (if in regex mode)
    regex: Option<Regex>,
    /// Search mode
    mode: SearchMode,
    /// Case sensitive
    case_sensitive: bool,
    /// Search direction
    direction: SearchDirection,
    /// All matches in the document
    matches: Vec<SearchMatch>,
    /// Current match index
    current_match: Option<usize>,
}

impl Default for SearchEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl SearchEngine {
    /// Create a new search engine
    pub fn new() -> Self {
        Self {
            pattern: None,
            regex: None,
            mode: SearchMode::Literal,
            case_sensitive: false,
            direction: SearchDirection::Forward,
            matches: Vec::new(),
            current_match: None,
        }
    }

    /// Set search pattern
    pub fn set_pattern(&mut self, pattern: &str, mode: SearchMode) -> Result<()> {
        self.pattern = Some(pattern.to_string());
        self.mode = mode;

        // Compile regex if in regex mode
        if mode == SearchMode::Regex {
            let regex_pattern = if self.case_sensitive {
                pattern.to_string()
            } else {
                format!("(?i){}", pattern)
            };

            self.regex = Some(
                Regex::new(&regex_pattern)
                    .with_context(|| format!("Invalid regex pattern: {}", pattern))?,
            );
        } else {
            self.regex = None;
        }

        Ok(())
    }

    /// Get current pattern
    pub fn pattern(&self) -> Option<&str> {
        self.pattern.as_deref()
    }

    /// Get search mode
    pub fn mode(&self) -> SearchMode {
        self.mode
    }

    /// Set case sensitivity
    pub fn set_case_sensitive(&mut self, sensitive: bool) {
        self.case_sensitive = sensitive;
    }

    /// Get case sensitivity
    pub fn is_case_sensitive(&self) -> bool {
        self.case_sensitive
    }

    /// Set search direction
    pub fn set_direction(&mut self, direction: SearchDirection) {
        self.direction = direction;
    }

    /// Get search direction
    pub fn direction(&self) -> SearchDirection {
        self.direction
    }

    /// Toggle search mode
    pub fn toggle_mode(&mut self) -> Result<()> {
        self.mode = match self.mode {
            SearchMode::Literal => SearchMode::Regex,
            SearchMode::Regex => SearchMode::Literal,
        };

        // Recompile pattern with new mode
        if let Some(pattern) = self.pattern.clone() {
            self.set_pattern(&pattern, self.mode)?;
        }

        Ok(())
    }

    /// Find all matches in the given lines
    pub fn find_all(&mut self, lines: &[String]) -> &[SearchMatch] {
        self.matches.clear();
        self.current_match = None;

        let pattern = match &self.pattern {
            Some(p) => p,
            None => return &self.matches,
        };

        match self.mode {
            SearchMode::Literal => self.find_literal(lines, pattern),
            SearchMode::Regex => self.find_regex(lines),
        }

        &self.matches
    }

    /// Find literal matches
    fn find_literal(&mut self, lines: &[String], pattern: &str) {
        let search_pattern = if self.case_sensitive {
            pattern.to_string()
        } else {
            pattern.to_lowercase()
        };

        for (line_idx, line) in lines.iter().enumerate() {
            let search_line = if self.case_sensitive {
                line.clone()
            } else {
                line.to_lowercase()
            };

            let mut start = 0;
            while let Some(pos) = search_line[start..].find(&search_pattern) {
                let col_start = start + pos;
                let col_end = col_start + pattern.len();
                let text = line[col_start..col_end].to_string();

                self.matches
                    .push(SearchMatch::new(line_idx, col_start, col_end, text));

                start = col_start + 1;
            }
        }
    }

    /// Find regex matches
    fn find_regex(&mut self, lines: &[String]) {
        let regex = match &self.regex {
            Some(r) => r,
            None => return,
        };

        for (line_idx, line) in lines.iter().enumerate() {
            for mat in regex.find_iter(line) {
                let col_start = mat.start();
                let col_end = mat.end();
                let text = mat.as_str().to_string();

                self.matches
                    .push(SearchMatch::new(line_idx, col_start, col_end, text));
            }
        }
    }

    /// Get total match count
    pub fn match_count(&self) -> usize {
        self.matches.len()
    }

    /// Get all matches
    pub fn matches(&self) -> &[SearchMatch] {
        &self.matches
    }

    /// Get current match index
    pub fn current_match_index(&self) -> Option<usize> {
        self.current_match
    }

    /// Find next match from cursor position
    pub fn next_match(&mut self, cursor_line: usize, cursor_col: usize) -> Option<&SearchMatch> {
        if self.matches.is_empty() {
            return None;
        }

        // Find first match after cursor
        let next_idx = self
            .matches
            .iter()
            .position(|m| {
                m.line > cursor_line || (m.line == cursor_line && m.col_start > cursor_col)
            })
            .unwrap_or(0); // Wrap to first match

        self.current_match = Some(next_idx);
        self.matches.get(next_idx)
    }

    /// Find previous match from cursor position
    pub fn prev_match(&mut self, cursor_line: usize, cursor_col: usize) -> Option<&SearchMatch> {
        if self.matches.is_empty() {
            return None;
        }

        // Find last match before cursor
        let prev_idx = self
            .matches
            .iter()
            .rposition(|m| {
                m.line < cursor_line || (m.line == cursor_line && m.col_start < cursor_col)
            })
            .unwrap_or(self.matches.len() - 1); // Wrap to last match

        self.current_match = Some(prev_idx);
        self.matches.get(prev_idx)
    }

    /// Get match at specific index
    pub fn get_match(&self, index: usize) -> Option<&SearchMatch> {
        self.matches.get(index)
    }

    /// Clear search
    pub fn clear(&mut self) {
        self.pattern = None;
        self.regex = None;
        self.matches.clear();
        self.current_match = None;
    }

    /// Check if a position is within any match
    pub fn is_match_at(&self, line: usize, col: usize) -> bool {
        self.matches.iter().any(|m| m.contains(line, col))
    }

    /// Get match at position
    pub fn match_at(&self, line: usize, col: usize) -> Option<&SearchMatch> {
        self.matches.iter().find(|m| m.contains(line, col))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_lines() -> Vec<String> {
        vec![
            "fn main() {".to_string(),
            "    let x = 42;".to_string(),
            "    println!(\"Hello, world!\");".to_string(),
            "    let y = x * 2;".to_string(),
            "}".to_string(),
        ]
    }

    #[test]
    fn test_literal_search() {
        let mut search = SearchEngine::new();
        let lines = sample_lines();

        search.set_pattern("let", SearchMode::Literal).unwrap();
        let matches = search.find_all(&lines);

        assert_eq!(matches.len(), 2);
        assert_eq!(matches[0].line, 1);
        assert_eq!(matches[1].line, 3);
    }

    #[test]
    fn test_case_insensitive_search() {
        let mut search = SearchEngine::new();
        let lines = sample_lines();

        search.set_case_sensitive(false);
        search.set_pattern("HELLO", SearchMode::Literal).unwrap();
        let matches = search.find_all(&lines);

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].line, 2);
    }

    #[test]
    fn test_regex_search() {
        let mut search = SearchEngine::new();
        let lines = sample_lines();

        search.set_pattern(r"let\s+\w+", SearchMode::Regex).unwrap();
        let matches = search.find_all(&lines);

        assert_eq!(matches.len(), 2);
        assert_eq!(matches[0].text, "let x");
        assert_eq!(matches[1].text, "let y");
    }

    #[test]
    fn test_regex_function_names() {
        let mut search = SearchEngine::new();
        let lines = sample_lines();

        search
            .set_pattern(r"fn\s+(\w+)", SearchMode::Regex)
            .unwrap();
        let matches = search.find_all(&lines);

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].line, 0);
        assert!(matches[0].text.contains("main"));
    }

    #[test]
    fn test_next_match() {
        let mut search = SearchEngine::new();
        let lines = sample_lines();

        search.set_pattern("let", SearchMode::Literal).unwrap();
        search.find_all(&lines);

        // From start, should find first match
        let next = search.next_match(0, 0);
        assert!(next.is_some());
        assert_eq!(next.unwrap().line, 1);

        // From after first match, should find second
        let next = search.next_match(1, 10);
        assert!(next.is_some());
        assert_eq!(next.unwrap().line, 3);

        // From after last match, should wrap to first
        let next = search.next_match(4, 0);
        assert!(next.is_some());
        assert_eq!(next.unwrap().line, 1);
    }

    #[test]
    fn test_prev_match() {
        let mut search = SearchEngine::new();
        let lines = sample_lines();

        search.set_pattern("let", SearchMode::Literal).unwrap();
        search.find_all(&lines);

        // From end, should find last match
        let prev = search.prev_match(4, 0);
        assert!(prev.is_some());
        assert_eq!(prev.unwrap().line, 3);

        // From after second match, should find first
        let prev = search.prev_match(3, 0);
        assert!(prev.is_some());
        assert_eq!(prev.unwrap().line, 1);

        // From before first match, should wrap to last
        let prev = search.prev_match(0, 0);
        assert!(prev.is_some());
        assert_eq!(prev.unwrap().line, 3);
    }

    #[test]
    fn test_toggle_mode() {
        let mut search = SearchEngine::new();

        assert_eq!(search.mode(), SearchMode::Literal);

        search.toggle_mode().unwrap();
        assert_eq!(search.mode(), SearchMode::Regex);

        search.toggle_mode().unwrap();
        assert_eq!(search.mode(), SearchMode::Literal);
    }

    #[test]
    fn test_clear() {
        let mut search = SearchEngine::new();
        let lines = sample_lines();

        search.set_pattern("let", SearchMode::Literal).unwrap();
        search.find_all(&lines);

        assert_eq!(search.match_count(), 2);

        search.clear();
        assert_eq!(search.match_count(), 0);
        assert!(search.pattern().is_none());
    }

    #[test]
    fn test_invalid_regex() {
        let mut search = SearchEngine::new();

        let result = search.set_pattern("[invalid", SearchMode::Regex);
        assert!(result.is_err());
    }

    #[test]
    fn test_match_at_position() {
        let mut search = SearchEngine::new();
        let lines = sample_lines();

        search.set_pattern("let", SearchMode::Literal).unwrap();
        search.find_all(&lines);

        // Position within first match
        assert!(search.is_match_at(1, 4));
        assert!(search.is_match_at(1, 5));
        assert!(search.is_match_at(1, 6));

        // Position outside matches
        assert!(!search.is_match_at(1, 3));
        assert!(!search.is_match_at(1, 7));
        assert!(!search.is_match_at(0, 0));
    }
}
