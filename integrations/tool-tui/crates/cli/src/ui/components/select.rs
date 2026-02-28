//! Select component
//!
//! Dropdown select with keyboard navigation.
//!
//! # Example
//!
//! ```rust,ignore
//! use dx_cli::ui::components::select::Select;
//!
//! let select = Select::new(vec!["One".into(), "Two".into()])
//!     .placeholder("Choose...");
//! ```

use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem},
};

use super::traits::{Bounds, ComponentResult, DxComponent, KeyEvent, MouseEvent, RenderContext};

/// A dropdown select component for choosing from a list of options.
///
/// Select provides a compact way to choose a single value from many
/// options. When closed, it shows the selected value; when opened,
/// it displays a scrollable list of all options.
///
/// # Features
///
/// - Keyboard navigation (↑↓ to change selection)
/// - Enter/Space to toggle open/close
/// - Escape to close without changing
/// - Custom placeholder text
/// - Compact closed state
///
/// # Example
///
/// ```rust,ignore
/// use dx_cli::ui::components::select::Select;
///
/// let select = Select::new(vec![
///     "Option A".into(),
///     "Option B".into(),
///     "Option C".into(),
/// ])
/// .placeholder("Choose an option...")
/// .selected(0);
/// ```
pub struct Select {
    /// Available options to choose from
    options: Vec<String>,
    /// Index of currently selected option
    selected: usize,
    /// Whether the dropdown is open
    open: bool,
    /// Placeholder text when no selection
    placeholder: String,
    /// Whether the component has keyboard focus
    focused: bool,
    /// Component bounds for hit testing
    bounds: Bounds,
}

impl Select {
    /// Create a new select with options
    pub fn new(options: Vec<String>) -> Self {
        Self {
            options,
            selected: 0,
            open: false,
            placeholder: "Select an option...".to_string(),
            focused: false,
            bounds: Bounds::default(),
        }
    }

    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    pub fn selected(mut self, index: usize) -> Self {
        self.selected = index.min(self.options.len().saturating_sub(1));
        self
    }

    pub fn toggle(&mut self) {
        self.open = !self.open;
    }

    pub fn close(&mut self) {
        self.open = false;
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    pub fn select_next(&mut self) {
        if !self.options.is_empty() {
            self.selected = (self.selected + 1) % self.options.len();
        }
    }

    pub fn select_prev(&mut self) {
        if !self.options.is_empty() {
            self.selected = if self.selected == 0 {
                self.options.len() - 1
            } else {
                self.selected - 1
            };
        }
    }

    pub fn get_selected(&self) -> Option<&String> {
        self.options.get(self.selected)
    }

    pub fn render(&self, f: &mut Frame, area: Rect) {
        if self.open {
            self.render_open(f, area);
        } else {
            self.render_closed(f, area);
        }
    }

    fn render_closed(&self, f: &mut Frame, area: Rect) {
        let text = self.get_selected().map(|s| s.as_str()).unwrap_or(&self.placeholder);

        let style = if self.get_selected().is_some() {
            Style::default().fg(Color::White)
        } else {
            Style::default().fg(Color::Gray)
        };

        let line = Line::from(vec![
            Span::styled(text, style),
            Span::raw(" "),
            Span::styled("▼", Style::default().fg(Color::Cyan)),
        ]);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Gray));

        let paragraph = ratatui::widgets::Paragraph::new(vec![line]).block(block);
        f.render_widget(paragraph, area);
    }

    fn render_open(&self, f: &mut Frame, area: Rect) {
        let items: Vec<ListItem> = self
            .options
            .iter()
            .enumerate()
            .map(|(i, option)| {
                let is_selected = i == self.selected;
                let style = if is_selected {
                    Style::default().fg(Color::Black).bg(Color::Cyan).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                };

                let prefix = if is_selected { "▶ " } else { "  " };
                let content = format!("{}{}", prefix, option);

                ListItem::new(Line::from(Span::styled(content, style)))
            })
            .collect();

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan));

        let list = List::new(items).block(block);
        f.render_widget(list, area);
    }
}

impl DxComponent for Select {
    fn handle_key(&mut self, key: KeyEvent) -> ComponentResult {
        match key {
            KeyEvent::Enter => {
                if self.open {
                    self.close();
                    if let Some(option) = self.get_selected() {
                        return ComponentResult::Action(format!("select:{}", option));
                    }
                } else {
                    self.toggle();
                }
                ComponentResult::Consumed
            }
            KeyEvent::Up | KeyEvent::Char('k') => {
                if self.open {
                    self.select_prev();
                }
                ComponentResult::Consumed
            }
            KeyEvent::Down | KeyEvent::Char('j') => {
                if self.open {
                    self.select_next();
                }
                ComponentResult::Consumed
            }
            KeyEvent::Escape => {
                if self.open {
                    self.close();
                    ComponentResult::Consumed
                } else {
                    ComponentResult::Ignored
                }
            }
            KeyEvent::Tab => ComponentResult::FocusNext,
            KeyEvent::BackTab => ComponentResult::FocusPrev,
            _ => ComponentResult::Ignored,
        }
    }

    fn handle_mouse(&mut self, event: MouseEvent) -> ComponentResult {
        match event {
            MouseEvent::Click { x, y } => {
                if self.bounds.contains(x, y) {
                    if self.open {
                        let relative_y = y.saturating_sub(self.bounds.y + 1);
                        if (relative_y as usize) < self.options.len() {
                            self.selected = relative_y as usize;
                            self.close();
                            if let Some(option) = self.get_selected() {
                                return ComponentResult::Action(format!("select:{}", option));
                            }
                        }
                    } else {
                        self.toggle();
                    }
                }
                ComponentResult::Consumed
            }
            _ => ComponentResult::Ignored,
        }
    }

    fn render(&self, _ctx: &RenderContext<'_>) -> Vec<String> {
        if self.open {
            self.options
                .iter()
                .enumerate()
                .map(|(i, opt)| {
                    let prefix = if i == self.selected { "▶ " } else { "  " };
                    format!("{}{}", prefix, opt)
                })
                .collect()
        } else {
            let text = self.get_selected().map(|s| s.as_str()).unwrap_or(&self.placeholder);
            vec![format!("{} ▼", text)]
        }
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
    fn bounds(&self) -> Bounds {
        self.bounds
    }
    fn set_bounds(&mut self, bounds: Bounds) {
        self.bounds = bounds;
    }
    fn min_size(&self) -> (u16, u16) {
        (
            20,
            if self.open {
                (self.options.len() + 2) as u16
            } else {
                3
            },
        )
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
