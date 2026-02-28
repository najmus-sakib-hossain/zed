//! Tabs component
//!
//! Renders horizontal tabs with selection.
//!
//! # Example
//!
//! ```rust,ignore
//! use dx_cli::ui::components::tabs::Tabs;
//!
//! let tabs = Tabs::new(vec!["Home".into(), "Settings".into()]);
//! ```

use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

use super::traits::{Bounds, ComponentResult, DxComponent, KeyEvent, MouseEvent, RenderContext};

/// A horizontal tab bar component for navigation between views.
///
/// Tabs provide a way to switch between different content panels.
/// The selected tab is visually highlighted with underline styling.
///
/// # Features
///
/// - Horizontal tab layout
/// - Keyboard navigation (←→ to switch tabs)
/// - Visual indicator for selected tab
/// - Separator between tabs
///
/// # Example
///
/// ```rust,ignore
/// use dx_cli::ui::components::tabs::Tabs;
///
/// let tabs = Tabs::new(vec![
///     "Overview".into(),
///     "Details".into(),
///     "Settings".into(),
/// ]).selected(0);
///
/// // Get current selection
/// if let Some(tab) = tabs.get_selected() {
///     println!("Selected: {}", tab);
/// }
/// ```
pub struct Tabs {
    /// Tab labels
    tabs: Vec<String>,
    /// Index of currently selected tab
    selected: usize,
    /// Whether the component has keyboard focus
    focused: bool,
    /// Component bounds for hit testing
    bounds: Bounds,
}

impl Tabs {
    /// Create a new tabs component
    pub fn new(tabs: Vec<String>) -> Self {
        Self {
            tabs,
            selected: 0,
            focused: false,
            bounds: Bounds::default(),
        }
    }

    pub fn selected(mut self, index: usize) -> Self {
        self.selected = index.min(self.tabs.len().saturating_sub(1));
        self
    }

    pub fn select_next(&mut self) {
        if !self.tabs.is_empty() {
            self.selected = (self.selected + 1) % self.tabs.len();
        }
    }

    pub fn select_prev(&mut self) {
        if !self.tabs.is_empty() {
            self.selected = if self.selected == 0 {
                self.tabs.len() - 1
            } else {
                self.selected - 1
            };
        }
    }

    pub fn get_selected(&self) -> Option<&String> {
        self.tabs.get(self.selected)
    }

    pub fn render(&self, f: &mut Frame, area: Rect) {
        let mut spans = Vec::new();

        for (i, tab) in self.tabs.iter().enumerate() {
            let is_selected = i == self.selected;

            if i > 0 {
                spans.push(Span::styled(" │ ", Style::default().fg(Color::DarkGray)));
            }

            let style = if is_selected {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
            } else {
                Style::default().fg(Color::Gray)
            };

            spans.push(Span::styled(format!(" {} ", tab), style));
        }

        let line = Line::from(spans);
        let block = Block::default()
            .borders(Borders::BOTTOM)
            .border_style(Style::default().fg(Color::DarkGray));

        let paragraph = Paragraph::new(vec![line]).block(block);
        f.render_widget(paragraph, area);
    }
}

impl DxComponent for Tabs {
    fn handle_key(&mut self, key: KeyEvent) -> ComponentResult {
        match key {
            KeyEvent::Left | KeyEvent::Char('h') => {
                self.select_prev();
                if let Some(tab) = self.get_selected() {
                    return ComponentResult::Action(format!("tab:changed:{}", tab));
                }
                ComponentResult::Consumed
            }
            KeyEvent::Right | KeyEvent::Char('l') => {
                self.select_next();
                if let Some(tab) = self.get_selected() {
                    return ComponentResult::Action(format!("tab:changed:{}", tab));
                }
                ComponentResult::Consumed
            }
            KeyEvent::Char(c) if c.is_ascii_digit() => {
                let index = c.to_digit(10).unwrap_or(0) as usize;
                if index > 0 && index <= self.tabs.len() {
                    self.selected = index - 1;
                    if let Some(tab) = self.get_selected() {
                        return ComponentResult::Action(format!("tab:changed:{}", tab));
                    }
                }
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
                if self.bounds.contains(x, y) {
                    let mut offset = self.bounds.x + 1;
                    for (i, tab) in self.tabs.iter().enumerate() {
                        let tab_width = tab.len() as u16 + 4;
                        if x >= offset && x < offset + tab_width {
                            self.selected = i;
                            if let Some(selected_tab) = self.get_selected() {
                                return ComponentResult::Action(format!(
                                    "tab:changed:{}",
                                    selected_tab
                                ));
                            }
                            break;
                        }
                        offset += tab_width + 3;
                    }
                }
                ComponentResult::Consumed
            }
            _ => ComponentResult::Ignored,
        }
    }

    fn render(&self, _ctx: &RenderContext<'_>) -> Vec<String> {
        let tabs_str: String = self
            .tabs
            .iter()
            .enumerate()
            .map(|(i, t)| {
                if i == self.selected {
                    format!("[{}]", t)
                } else {
                    format!(" {} ", t)
                }
            })
            .collect::<Vec<_>>()
            .join(" │ ");
        vec![tabs_str]
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
        let width: usize = self.tabs.iter().map(|t| t.len() + 4).sum::<usize>()
            + (self.tabs.len().saturating_sub(1) * 3);
        (width.min(100) as u16, 2)
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
