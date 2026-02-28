//! Syntax Highlighted Code Viewer
//!
//! A high-performance code viewer with syntax highlighting powered by syntect,
//! supporting 100+ languages with theme integration.
//!
//! # Features
//!
//! - Syntax highlighting via syntect
//! - Line numbers (absolute and relative)
//! - Theme integration with DxTheme
//! - Cursor tracking and selection
//! - Search highlighting with regex support
//!
//! # Example
//!
//! ```rust,ignore
//! use dx_cli::ui::editor::{CodeViewer, LineNumberMode, ViewerConfig};
//!
//! let mut viewer = CodeViewer::new()
//!     .line_numbers(LineNumberMode::Relative)
//!     .theme("base16-ocean.dark");
//!
//! viewer.load_file("src/main.rs")?;
//! ```

mod viewer_cursor;
mod viewer_render;

use std::any::Any;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use syntect::highlighting::ThemeSet;
use syntect::parsing::SyntaxSet;

use crate::ui::components::traits::{
    Bounds, ComponentResult, DxComponent, KeyEvent, MouseEvent, RenderContext,
};
use crate::ui::editor::keybindings::{EditorKeybindings, KeyAction, VimMotion};
use crate::ui::editor::search::{SearchDirection, SearchEngine, SearchMode};

/// Line number display mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LineNumberMode {
    /// No line numbers
    None,
    /// Absolute line numbers (1, 2, 3, ...)
    #[default]
    Absolute,
    /// Relative line numbers from cursor
    Relative,
    /// Hybrid: absolute for current line, relative for others
    Hybrid,
}

/// Configuration for the code viewer
#[derive(Debug, Clone)]
pub struct ViewerConfig {
    /// Line number mode
    pub line_numbers: LineNumberMode,
    /// Tab width in spaces
    pub tab_width: u8,
    /// Show whitespace characters
    pub show_whitespace: bool,
    /// Wrap long lines
    pub word_wrap: bool,
    /// Highlight current line
    pub highlight_line: bool,
    /// Theme name (from syntect themes)
    pub theme_name: String,
    /// Scroll margin (lines to keep visible above/below cursor)
    pub scroll_margin: u16,
}

impl Default for ViewerConfig {
    fn default() -> Self {
        Self {
            line_numbers: LineNumberMode::Absolute,
            tab_width: 4,
            show_whitespace: false,
            word_wrap: false,
            highlight_line: true,
            theme_name: "base16-ocean.dark".to_string(),
            scroll_margin: 3,
        }
    }
}

/// A syntax-highlighted code viewer component
pub struct CodeViewer {
    /// Loaded file content (lines)
    lines: Vec<String>,
    /// Current file path
    file_path: Option<PathBuf>,
    /// Syntax set for highlighting
    syntax_set: SyntaxSet,
    /// Theme set
    theme_set: ThemeSet,
    /// Current syntax (index into syntax_set)
    current_syntax: Option<usize>,
    /// Cursor line (0-indexed)
    cursor_line: usize,
    /// Cursor column (0-indexed)
    cursor_col: usize,
    /// Scroll offset (first visible line)
    scroll_offset: usize,
    /// Horizontal scroll offset
    h_scroll_offset: usize,
    /// Selection start (line, col)
    selection_start: Option<(usize, usize)>,
    /// Selection end (line, col)
    selection_end: Option<(usize, usize)>,
    /// Search engine
    search: SearchEngine,
    /// Configuration
    config: ViewerConfig,
    /// Component bounds
    bounds: Bounds,
    /// Whether focused
    focused: bool,
    /// Visible height (lines)
    visible_height: usize,
    /// Visible width (columns)
    visible_width: usize,
    /// Keybindings manager
    keybindings: EditorKeybindings,
}

impl CodeViewer {
    /// Create a new code viewer with default settings
    pub fn new() -> Self {
        Self {
            lines: Vec::new(),
            file_path: None,
            syntax_set: SyntaxSet::load_defaults_newlines(),
            theme_set: ThemeSet::load_defaults(),
            current_syntax: None,
            cursor_line: 0,
            cursor_col: 0,
            scroll_offset: 0,
            h_scroll_offset: 0,
            selection_start: None,
            selection_end: None,
            search: SearchEngine::new(),
            config: ViewerConfig::default(),
            bounds: Bounds::default(),
            focused: false,
            visible_height: 24,
            visible_width: 80,
            keybindings: EditorKeybindings::new(),
        }
    }

    /// Set line number mode
    pub fn line_numbers(mut self, mode: LineNumberMode) -> Self {
        self.config.line_numbers = mode;
        self
    }

    /// Set theme name
    pub fn theme(mut self, name: impl Into<String>) -> Self {
        self.config.theme_name = name.into();
        self
    }

    /// Set configuration
    pub fn config(mut self, config: ViewerConfig) -> Self {
        self.config = config;
        self
    }

    /// Load keybindings from config file
    pub fn load_keybindings<P: AsRef<Path>>(&mut self, path: P) -> Result<()> {
        self.keybindings = EditorKeybindings::from_config(path)?;
        Ok(())
    }

    /// Set keybindings directly
    pub fn keybindings(mut self, keybindings: EditorKeybindings) -> Self {
        self.keybindings = keybindings;
        self
    }

    /// Load content from a file
    pub fn load_file<P: AsRef<Path>>(&mut self, path: P) -> Result<()> {
        let path = path.as_ref();
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read file: {}", path.display()))?;

        self.lines = content.lines().map(String::from).collect();
        self.file_path = Some(path.to_path_buf());

        // Detect syntax from extension
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("txt");
        self.current_syntax = self
            .syntax_set
            .find_syntax_by_extension(ext)
            .map(|s| self.syntax_set.syntaxes().iter().position(|x| x.name == s.name))
            .flatten();

        // Reset cursor
        self.cursor_line = 0;
        self.cursor_col = 0;
        self.scroll_offset = 0;
        self.h_scroll_offset = 0;
        self.selection_start = None;
        self.selection_end = None;

        Ok(())
    }

    /// Load content from string
    pub fn load_content(&mut self, content: &str, language: Option<&str>) {
        self.lines = content.lines().map(String::from).collect();
        self.file_path = None;

        if let Some(lang) = language {
            self.current_syntax = self
                .syntax_set
                .find_syntax_by_extension(lang)
                .or_else(|| self.syntax_set.find_syntax_by_name(lang))
                .map(|s| self.syntax_set.syntaxes().iter().position(|x| x.name == s.name))
                .flatten();
        }

        self.cursor_line = 0;
        self.cursor_col = 0;
        self.scroll_offset = 0;
    }

    /// Get the current theme
    fn get_theme(&self) -> &syntect::highlighting::Theme {
        self.theme_set
            .themes
            .get(&self.config.theme_name)
            .unwrap_or_else(|| &self.theme_set.themes["base16-ocean.dark"])
    }

    /// Get the current syntax reference
    fn get_syntax(&self) -> &syntect::parsing::SyntaxReference {
        self.current_syntax
            .and_then(|i| self.syntax_set.syntaxes().get(i))
            .unwrap_or_else(|| self.syntax_set.find_syntax_plain_text())
    }

    /// Get line number width (digits needed)
    fn line_number_width(&self) -> usize {
        if self.config.line_numbers == LineNumberMode::None {
            0
        } else {
            let max_line = self.lines.len().max(1);
            ((max_line as f64).log10().floor() as usize) + 1 + 2 // +2 for padding
        }
    }

    /// Format line number based on mode
    fn format_line_number(&self, line_idx: usize) -> String {
        let width = self.line_number_width().saturating_sub(2);
        match self.config.line_numbers {
            LineNumberMode::None => String::new(),
            LineNumberMode::Absolute => format!("{:>width$} │ ", line_idx + 1),
            LineNumberMode::Relative => {
                let rel = if line_idx == self.cursor_line {
                    0
                } else {
                    (line_idx as isize - self.cursor_line as isize).unsigned_abs()
                };
                format!("{:>width$} │ ", rel)
            }
            LineNumberMode::Hybrid => {
                if line_idx == self.cursor_line {
                    format!("{:>width$} │ ", line_idx + 1)
                } else {
                    let rel = (line_idx as isize - self.cursor_line as isize).unsigned_abs();
                    format!("{:>width$} │ ", rel)
                }
            }
        }
    }

    /// Search for pattern
    pub fn search(&mut self, pattern: &str, mode: SearchMode) -> Result<()> {
        self.search.set_pattern(pattern, mode)?;
        self.search.find_all(&self.lines);
        Ok(())
    }

    /// Search forward (Vim /)
    pub fn search_forward(&mut self, pattern: &str) -> Result<()> {
        self.search.set_direction(SearchDirection::Forward);
        self.search(pattern, SearchMode::Literal)
    }

    /// Search backward (Vim ?)
    pub fn search_backward(&mut self, pattern: &str) -> Result<()> {
        self.search.set_direction(SearchDirection::Backward);
        self.search(pattern, SearchMode::Literal)
    }

    /// Toggle search mode (literal/regex)
    pub fn toggle_search_mode(&mut self) -> Result<()> {
        self.search.toggle_mode()?;
        if self.search.pattern().is_some() {
            self.search.find_all(&self.lines);
        }
        Ok(())
    }

    /// Get search mode
    pub fn search_mode(&self) -> SearchMode {
        self.search.mode()
    }

    /// Get search pattern
    pub fn search_pattern(&self) -> Option<&str> {
        self.search.pattern()
    }

    /// Get search match count
    pub fn search_match_count(&self) -> usize {
        self.search.match_count()
    }

    /// Go to next search match
    pub fn next_match(&mut self) {
        if let Some(m) = self.search.next_match(self.cursor_line, self.cursor_col) {
            self.cursor_line = m.line;
            self.cursor_col = m.col_start;
            self.ensure_cursor_visible();
        }
    }

    /// Go to previous search match
    pub fn prev_match(&mut self) {
        if let Some(m) = self.search.prev_match(self.cursor_line, self.cursor_col) {
            self.cursor_line = m.line;
            self.cursor_col = m.col_start;
            self.ensure_cursor_visible();
        }
    }

    /// Clear search
    pub fn clear_search(&mut self) {
        self.search.clear();
    }

    /// Get current line content
    pub fn current_line(&self) -> Option<&str> {
        self.lines.get(self.cursor_line).map(|s| s.as_str())
    }

    /// Get current file path
    pub fn file_path(&self) -> Option<&Path> {
        self.file_path.as_deref()
    }

    /// Get total line count
    pub fn line_count(&self) -> usize {
        self.lines.len()
    }

    /// Get viewport information for minimap
    pub fn viewport_info(&self) -> (usize, usize) {
        let end = (self.scroll_offset + self.visible_height).min(self.lines.len());
        (self.scroll_offset, end)
    }

    /// Get all lines (for minimap)
    pub fn lines(&self) -> &[String] {
        &self.lines
    }

    /// Get current syntax index (for minimap)
    pub fn syntax_index(&self) -> Option<usize> {
        self.current_syntax
    }

    /// Jump to line (from minimap click)
    pub fn jump_to_line(&mut self, line: usize) {
        self.cursor_line = line.min(self.lines.len().saturating_sub(1));
        self.ensure_cursor_visible();
    }
}

impl Default for CodeViewer {
    fn default() -> Self {
        Self::new()
    }
}

impl DxComponent for CodeViewer {
    fn handle_key(&mut self, key: KeyEvent) -> ComponentResult {
        // Map key through keybindings system
        let action = match key {
            KeyEvent::Char(c) => self.keybindings.map_key(c),
            KeyEvent::Down => KeyAction::Motion(VimMotion::Down),
            KeyEvent::Up => KeyAction::Motion(VimMotion::Up),
            KeyEvent::Left => KeyAction::Motion(VimMotion::Left),
            KeyEvent::Right => KeyAction::Motion(VimMotion::Right),
            KeyEvent::Home => KeyAction::Motion(VimMotion::LineStart),
            KeyEvent::End => KeyAction::Motion(VimMotion::LineEnd),
            KeyEvent::PageUp => KeyAction::Motion(VimMotion::PageUp),
            KeyEvent::PageDown => KeyAction::Motion(VimMotion::PageDown),
            KeyEvent::Escape => {
                self.selection_start = None;
                self.selection_end = None;
                return ComponentResult::Consumed;
            }
            KeyEvent::Tab => return ComponentResult::FocusNext,
            KeyEvent::BackTab => return ComponentResult::FocusPrev,
            KeyEvent::Ctrl('b') => KeyAction::Motion(VimMotion::PageUp),
            KeyEvent::Ctrl('f') => KeyAction::Motion(VimMotion::PageDown),
            _ => KeyAction::None,
        };

        // Execute action
        match action {
            KeyAction::Motion(motion) => {
                let count = self.keybindings.count().unwrap_or(1);
                match motion {
                    VimMotion::Up => self.cursor_up(count),
                    VimMotion::Down => self.cursor_down(count),
                    VimMotion::Left => self.cursor_left(count),
                    VimMotion::Right => self.cursor_right(count),
                    VimMotion::WordForward => {
                        for _ in 0..count {
                            self.word_forward();
                        }
                    }
                    VimMotion::WordBackward => {
                        for _ in 0..count {
                            self.word_backward();
                        }
                    }
                    VimMotion::WordEnd => {
                        for _ in 0..count {
                            self.word_end();
                        }
                    }
                    VimMotion::LineStart => self.cursor_home(),
                    VimMotion::LineEnd => self.cursor_end(),
                    VimMotion::FileStart => self.cursor_top(),
                    VimMotion::FileEnd => self.cursor_bottom(),
                    VimMotion::PageUp => self.page_up(),
                    VimMotion::PageDown => self.page_down(),
                }
                ComponentResult::Consumed
            }
            KeyAction::NextMatch => {
                self.next_match();
                ComponentResult::Consumed
            }
            KeyAction::PrevMatch => {
                self.prev_match();
                ComponentResult::Consumed
            }
            KeyAction::SearchForward => {
                // TODO: Open search input
                ComponentResult::Consumed
            }
            KeyAction::None => ComponentResult::Ignored,
            _ => ComponentResult::Ignored,
        }
    }$') | KeyEvent::End => {
                self.cursor_end();
                ComponentResult::Consumed
            }
            KeyEvent::Char('g') => {
                self.cursor_top();
                ComponentResult::Consumed
            }
            KeyEvent::Char('G') => {
                self.cursor_bottom();
                ComponentResult::Consumed
            }
            KeyEvent::PageUp | KeyEvent::Ctrl('b') => {
                self.page_up();
                ComponentResult::Consumed
            }
            KeyEvent::PageDown | KeyEvent::Ctrl('f') => {
                self.page_down();
                ComponentResult::Consumed
            }
            KeyEvent::Char('n') => {
                self.next_match();
                ComponentResult::Consumed
            }
            KeyEvent::Char('N') => {
                self.prev_match();
                ComponentResult::Consumed
            }
            KeyEvent::Escape => {
                self.selection_start = None;
                self.selection_end = None;
                ComponentResult::Consumed
            }
            KeyEvent::Tab => ComponentResult::FocusNext,
            KeyEvent::BackTab => ComponentResult::FocusPrev,
            _ => ComponentResult::Ignored,
        }
    }

    fn handle_mouse(&mut self, event: MouseEvent) -> ComponentResult {
        match event {
            MouseEvent::Click { x, y } => {
                let line_num_width = self.line_number_width() as u16;
                let relative_y = y.saturating_sub(self.bounds.y) as usize;
                let relative_x = x.saturating_sub(self.bounds.x + line_num_width) as usize;

                self.cursor_line = self.scroll_offset + relative_y;
                self.cursor_col = self.h_scroll_offset + relative_x;
                self.clamp_cursor_col();

                self.cursor_line = self.cursor_line.min(self.lines.len().saturating_sub(1));
                ComponentResult::Consumed
            }
            MouseEvent::ScrollUp { .. } => {
                self.scroll_offset = self.scroll_offset.saturating_sub(3);
                ComponentResult::Consumed
            }
            MouseEvent::ScrollDown { .. } => {
                let max_scroll = self.lines.len().saturating_sub(self.visible_height);
                self.scroll_offset = (self.scroll_offset + 3).min(max_scroll);
                ComponentResult::Consumed
            }
            _ => ComponentResult::Ignored,
        }
    }

    fn render(&self, ctx: &RenderContext<'_>) -> Vec<String> {
        self.render_internal(ctx)
    }

    fn is_focusable(&self) -> bool {
        true
    }

    fn set_focused(&mut self, focused: bool) {
        self.focused = focused;
    }

    fn is_focused(&self) -> bool {
        self.focused
    }

    fn id(&self) -> Option<&str> {
        Some("code_viewer")
    }

    fn bounds(&self) -> Bounds {
        self.bounds
    }

    fn set_bounds(&mut self, bounds: Bounds) {
        self.bounds = bounds;
        self.visible_height = bounds.height as usize;
        self.visible_width = bounds.width as usize;
    }

    fn min_size(&self) -> (u16, u16) {
        (40, 10)
    }

    fn preferred_size(&self) -> (u16, u16) {
        (80, 24)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_viewer_creation() {
        let viewer = CodeViewer::new();
        assert_eq!(viewer.cursor_line, 0);
        assert_eq!(viewer.cursor_col, 0);
    }

    #[test]
    fn test_load_content() {
        let mut viewer = CodeViewer::new();
        viewer.load_content("line 1\nline 2\nline 3", Some("txt"));
        assert_eq!(viewer.line_count(), 3);
    }

    #[test]
    fn test_cursor_movement() {
        let mut viewer = CodeViewer::new();
        viewer.load_content("line 1\nline 2\nline 3", None);

        viewer.cursor_down(1);
        assert_eq!(viewer.cursor_line, 1);

        viewer.cursor_up(1);
        assert_eq!(viewer.cursor_line, 0);

        viewer.cursor_bottom();
        assert_eq!(viewer.cursor_line, 2);

        viewer.cursor_top();
        assert_eq!(viewer.cursor_line, 0);
    }

    #[test]
    fn test_line_number_modes() {
        let mut viewer = CodeViewer::new();
        viewer.load_content("a\nb\nc\nd\ne", None);
        viewer.cursor_line = 2;

        viewer.config.line_numbers = LineNumberMode::Absolute;
        let num = viewer.format_line_number(2);
        assert!(num.contains("3"));

        viewer.config.line_numbers = LineNumberMode::Relative;
        let num = viewer.format_line_number(0);
        assert!(num.contains("2"));

        let num_current = viewer.format_line_number(2);
        assert!(num_current.contains("0"));
    }

    #[test]
    fn test_search() {
        let mut viewer = CodeViewer::new();
        viewer.load_content("hello world\nfoo bar\nhello again", None);

        viewer
            .search("hello", SearchMode::Literal)
            .unwrap();
        assert_eq!(viewer.search_match_count(), 2);
    }

    #[test]
    fn test_regex_search() {
        let mut viewer = CodeViewer::new();
        viewer.load_content("fn main() {\n    let x = 42;\n}", None);

        viewer
            .search(r"let\s+\w+", SearchMode::Regex)
            .unwrap();
        assert_eq!(viewer.search_match_count(), 1);
    }

    #[test]
    fn test_goto_line() {
        let mut viewer = CodeViewer::new();
        viewer.load_content("a\nb\nc\nd\ne", None);

        viewer.goto_line(3);
        assert_eq!(viewer.cursor_line, 2); // 0-indexed
    }
}
