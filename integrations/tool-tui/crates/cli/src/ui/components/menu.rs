//! Menu component
//!
//! Renders a selectable menu with shortcuts and separators.
//!
//! # Example
//!
//! ```rust,ignore
//! use dx_cli::ui::components::menu::{Menu, MenuItem};
//!
//! let menu = Menu::new(vec![
//!     MenuItem::new("Open"),
//!     MenuItem::new("Save").shortcut("Ctrl+S"),
//!     MenuItem::separator(),
//!     MenuItem::new("Exit"),
//! ]);
//! ```

use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem},
};

use super::traits::{Bounds, ComponentResult, DxComponent, KeyEvent, MouseEvent, RenderContext};

/// A single item within a [`Menu`].
///
/// Menu items can be:
/// - Regular items with a label and optional keyboard shortcut
/// - Disabled items that cannot be selected
/// - Separators to visually group items
///
/// # Example
///
/// ```rust,ignore
/// let item = MenuItem::new("Save")
///     .shortcut("Ctrl+S")
///     .disabled(false);
/// ```
#[derive(Debug, Clone)]
pub struct MenuItem {
    /// The display text for this menu item
    pub label: String,
    /// Optional keyboard shortcut displayed on the right
    pub shortcut: Option<String>,
    /// Whether this item is disabled (grayed out)
    pub disabled: bool,
    /// Whether this item is a separator line
    pub separator: bool,
}

impl MenuItem {
    /// Create a new menu item
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            shortcut: None,
            disabled: false,
            separator: false,
        }
    }

    /// Set shortcut label
    pub fn shortcut(mut self, shortcut: impl Into<String>) -> Self {
        self.shortcut = Some(shortcut.into());
        self
    }

    /// Set disabled state
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Create a separator item
    pub fn separator() -> Self {
        Self {
            label: String::new(),
            shortcut: None,
            disabled: false,
            separator: true,
        }
    }
}

/// A dropdown/context menu component with keyboard navigation.
///
/// Menus display a list of selectable items with optional shortcuts
/// and visual separators. Supports keyboard navigation with Up/Down
/// arrows and Enter to select.
///
/// # Features
///
/// - Keyboard navigation (↑↓, Enter, Escape)
/// - Keyboard shortcut hints
/// - Disabled items
/// - Visual separators between groups
/// - Optional title header
///
/// # Example
///
/// ```rust,ignore
/// use dx_cli::ui::components::menu::{Menu, MenuItem};
///
/// let menu = Menu::new(vec![
///     MenuItem::new("New File").shortcut("Ctrl+N"),
///     MenuItem::new("Open...").shortcut("Ctrl+O"),
///     MenuItem::separator(),
///     MenuItem::new("Save").shortcut("Ctrl+S"),
///     MenuItem::new("Save As...").shortcut("Ctrl+Shift+S"),
///     MenuItem::separator(),
///     MenuItem::new("Exit").shortcut("Alt+F4"),
/// ]).title("File");
/// ```
pub struct Menu {
    /// The list of menu items
    items: Vec<MenuItem>,
    /// Currently selected/highlighted index
    selected: usize,
    /// Optional title displayed at the top
    title: Option<String>,
    /// Whether the menu has keyboard focus
    focused: bool,
    /// Component bounds for hit testing
    bounds: Bounds,
}

impl Menu {
    /// Create a new menu
    pub fn new(items: Vec<MenuItem>) -> Self {
        Self {
            items,
            selected: 0,
            title: None,
            focused: false,
            bounds: Bounds::default(),
        }
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn selected(mut self, index: usize) -> Self {
        self.selected = index.min(self.items.len().saturating_sub(1));
        self
    }

    pub fn select_next(&mut self) {
        self.selected = (self.selected + 1) % self.items.len();
    }

    pub fn select_prev(&mut self) {
        self.selected = if self.selected == 0 {
            self.items.len().saturating_sub(1)
        } else {
            self.selected - 1
        };
    }

    pub fn get_selected(&self) -> Option<&MenuItem> {
        self.items.get(self.selected)
    }

    pub fn render(&self, f: &mut Frame, area: Rect) {
        let items: Vec<ListItem> = self
            .items
            .iter()
            .enumerate()
            .map(|(i, item)| {
                if item.separator {
                    return ListItem::new(Line::from("─".repeat(area.width as usize - 4)));
                }

                let is_selected = i == self.selected;
                let style = if item.disabled {
                    Style::default().fg(Color::DarkGray)
                } else if is_selected {
                    Style::default().fg(Color::Black).bg(Color::Cyan).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                };

                let mut spans = vec![Span::raw("  ")];

                if is_selected && !item.disabled {
                    spans.push(Span::styled("▶ ", Style::default().fg(Color::Cyan)));
                } else {
                    spans.push(Span::raw("  "));
                }

                spans.push(Span::styled(&item.label, style));

                if let Some(shortcut) = &item.shortcut {
                    let padding = area
                        .width
                        .saturating_sub(item.label.len() as u16 + shortcut.len() as u16 + 8);
                    spans.push(Span::raw(" ".repeat(padding as usize)));
                    spans.push(Span::styled(shortcut, Style::default().fg(Color::Gray)));
                }

                ListItem::new(Line::from(spans))
            })
            .collect();

        let mut block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan));

        if let Some(title) = &self.title {
            block = block.title(title.as_str());
        }

        let list = List::new(items).block(block);
        f.render_widget(list, area);
    }
}

impl DxComponent for Menu {
    fn handle_key(&mut self, key: KeyEvent) -> ComponentResult {
        match key {
            KeyEvent::Up | KeyEvent::Char('k') => {
                self.select_prev();
                ComponentResult::Consumed
            }
            KeyEvent::Down | KeyEvent::Char('j') => {
                self.select_next();
                ComponentResult::Consumed
            }
            KeyEvent::Enter => {
                if let Some(item) = self.get_selected() {
                    if !item.disabled && !item.separator {
                        return ComponentResult::Action(format!("menu:select:{}", item.label));
                    }
                }
                ComponentResult::Consumed
            }
            KeyEvent::Escape => ComponentResult::Close,
            _ => ComponentResult::Ignored,
        }
    }

    fn handle_mouse(&mut self, event: MouseEvent) -> ComponentResult {
        match event {
            MouseEvent::Click { x, y } => {
                if self.bounds.contains(x, y) {
                    let relative_y = y.saturating_sub(self.bounds.y + 1);
                    if (relative_y as usize) < self.items.len() {
                        self.selected = relative_y as usize;
                        if let Some(item) = self.get_selected() {
                            if !item.disabled && !item.separator {
                                return ComponentResult::Action(format!(
                                    "menu:select:{}",
                                    item.label
                                ));
                            }
                        }
                    }
                }
                ComponentResult::Consumed
            }
            _ => ComponentResult::Ignored,
        }
    }

    fn render(&self, _ctx: &RenderContext<'_>) -> Vec<String> {
        self.items
            .iter()
            .enumerate()
            .map(|(i, item)| {
                if item.separator {
                    "─".repeat(20)
                } else {
                    let prefix = if i == self.selected { "▶ " } else { "  " };
                    format!("{}{}", prefix, item.label)
                }
            })
            .collect()
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
        (20, (self.items.len() + 2) as u16)
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

pub struct ContextMenu {
    menu: Menu,
    position: (u16, u16),
    visible: bool,
}

impl ContextMenu {
    pub fn new(items: Vec<MenuItem>) -> Self {
        Self {
            menu: Menu::new(items),
            position: (0, 0),
            visible: false,
        }
    }

    pub fn show_at(&mut self, x: u16, y: u16) {
        self.position = (x, y);
        self.visible = true;
    }

    pub fn hide(&mut self) {
        self.visible = false;
    }

    pub fn is_visible(&self) -> bool {
        self.visible
    }

    pub fn select_next(&mut self) {
        self.menu.select_next();
    }

    pub fn select_prev(&mut self) {
        self.menu.select_prev();
    }

    pub fn get_selected(&self) -> Option<&MenuItem> {
        self.menu.get_selected()
    }

    pub fn render(&self, f: &mut Frame, screen_area: Rect) {
        if !self.visible {
            return;
        }

        let width = 30;
        let height = (self.menu.items.len() + 2).min(20) as u16;

        let x = self.position.0.min(screen_area.width.saturating_sub(width));
        let y = self.position.1.min(screen_area.height.saturating_sub(height));

        let area = Rect {
            x,
            y,
            width,
            height,
        };

        self.menu.render(f, area);
    }
}
