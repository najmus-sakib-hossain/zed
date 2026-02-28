//! Sidebar component for file trees and navigation
//!
//! A collapsible sidebar with nested items, icons, badges, and keyboard navigation.
//!
//! # Example
//!
//! ```rust,ignore
//! use dx_cli::ui::components::sidebar::{Sidebar, SidebarItem};
//!
//! let sidebar = Sidebar::new()
//!     .item(SidebarItem::new("src").children(vec![
//!         SidebarItem::new("main.rs").icon("󰈙"),
//!         SidebarItem::new("lib.rs").icon("󰈙"),
//!     ]))
//!     .item(SidebarItem::new("Cargo.toml").icon(""));
//! ```

use super::traits::{Bounds, ComponentResult, DxComponent, KeyEvent, MouseEvent, RenderContext};
use crate::ui::theme::animation::{BorderStyle, GradientAnimation, RainbowAnimation};
use crate::ui::theme::tokens::{Color, SolidColor};
use std::any::Any;
use std::time::Duration;

/// A single item in the sidebar
#[derive(Debug, Clone)]
pub struct SidebarItem {
    /// Item label
    pub label: String,
    /// Icon (Nerd Font recommended)
    pub icon: Option<String>,
    /// Badge text (e.g., file count)
    pub badge: Option<String>,
    /// Child items
    pub children: Vec<SidebarItem>,
    /// Whether this item is expanded
    pub expanded: bool,
    /// Whether this item is selectable
    pub selectable: bool,
    /// Custom data attached to the item
    pub data: Option<String>,
    /// Indentation level
    pub level: usize,
}

impl SidebarItem {
    /// Create a new sidebar item
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            icon: None,
            badge: None,
            children: Vec::new(),
            expanded: false,
            selectable: true,
            data: None,
            level: 0,
        }
    }

    /// Set the icon
    pub fn icon(mut self, icon: impl Into<String>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    /// Set the badge
    pub fn badge(mut self, badge: impl Into<String>) -> Self {
        self.badge = Some(badge.into());
        self
    }

    /// Set child items
    pub fn children(mut self, children: Vec<SidebarItem>) -> Self {
        self.children = children;
        self
    }

    /// Add a child item
    pub fn child(mut self, child: SidebarItem) -> Self {
        self.children.push(child);
        self
    }

    /// Set expanded state
    pub fn expanded(mut self, expanded: bool) -> Self {
        self.expanded = expanded;
        self
    }

    /// Set selectable state
    pub fn selectable(mut self, selectable: bool) -> Self {
        self.selectable = selectable;
        self
    }

    /// Attach custom data
    pub fn data(mut self, data: impl Into<String>) -> Self {
        self.data = Some(data.into());
        self
    }

    /// Check if this item has children
    pub fn has_children(&self) -> bool {
        !self.children.is_empty()
    }

    /// Get flattened visible items (for rendering)
    fn flatten(&self, level: usize) -> Vec<(SidebarItem, usize)> {
        let mut result = Vec::new();
        let mut item = self.clone();
        item.level = level;
        result.push((item.clone(), level));

        if self.expanded {
            for child in &self.children {
                result.extend(child.flatten(level + 1));
            }
        }

        result
    }
}

/// Sidebar display mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SidebarMode {
    /// Full sidebar with labels
    Full,
    /// Collapsed sidebar with icons only
    Collapsed,
    /// Hidden sidebar
    Hidden,
}

/// Sidebar border rendering style
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SidebarBorderStyle {
    /// Standard single-color border
    Default,
    /// Animated gradient border
    Gradient,
    /// Animated rainbow border
    Rainbow,
}

/// Sidebar component
pub struct Sidebar {
    /// Root items
    items: Vec<SidebarItem>,
    /// Currently selected index
    selected: usize,
    /// Scroll offset
    scroll_offset: usize,
    /// Display mode
    mode: SidebarMode,
    /// Border rendering style
    border_style: SidebarBorderStyle,
    /// Component bounds
    bounds: Bounds,
    /// Component ID
    id: Option<String>,
    /// Whether focused
    focused: bool,
    /// Visible height (for scrolling)
    visible_height: usize,
    /// Title
    title: Option<String>,
}

impl Default for Sidebar {
    fn default() -> Self {
        Self::new()
    }
}

impl Sidebar {
    /// Create a new empty sidebar
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            selected: 0,
            scroll_offset: 0,
            mode: SidebarMode::Full,
            border_style: SidebarBorderStyle::Default,
            bounds: Bounds::default(),
            id: None,
            focused: false,
            visible_height: 20,
            title: None,
        }
    }

    /// Add an item
    pub fn item(mut self, item: SidebarItem) -> Self {
        self.items.push(item);
        self
    }

    /// Set all items
    pub fn items(mut self, items: Vec<SidebarItem>) -> Self {
        self.items = items;
        self
    }

    /// Set the display mode
    pub fn mode(mut self, mode: SidebarMode) -> Self {
        self.mode = mode;
        self
    }

    /// Set the border style
    pub fn border_style(mut self, border_style: SidebarBorderStyle) -> Self {
        self.border_style = border_style;
        self
    }

    /// Set the component ID
    pub fn with_id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// Set the title
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Get flattened visible items
    fn visible_items(&self) -> Vec<(SidebarItem, usize)> {
        let mut result = Vec::new();
        for item in &self.items {
            result.extend(item.flatten(0));
        }
        result
    }

    /// Map a flattened index to a path in the tree
    fn path_for_index(&self, index: usize) -> Option<Vec<usize>> {
        fn walk(
            items: &[SidebarItem],
            index: usize,
            current: &mut usize,
            path: &mut Vec<usize>,
        ) -> Option<Vec<usize>> {
            for (i, item) in items.iter().enumerate() {
                if *current == index {
                    let mut found = path.clone();
                    found.push(i);
                    return Some(found);
                }
                *current += 1;

                if item.expanded && !item.children.is_empty() {
                    path.push(i);
                    if let Some(found) = walk(&item.children, index, current, path) {
                        return Some(found);
                    }
                    path.pop();
                }
            }
            None
        }

        let mut current = 0;
        walk(&self.items, index, &mut current, &mut Vec::new())
    }

    /// Find an item by path (immutable)
    fn item_by_path<'a>(items: &'a [SidebarItem], path: &[usize]) -> Option<&'a SidebarItem> {
        let (head, tail) = path.split_first()?;
        let item = items.get(*head)?;
        if tail.is_empty() {
            Some(item)
        } else {
            Self::item_by_path(&item.children, tail)
        }
    }

    /// Find an item by path (mutable)
    fn item_mut_by_path<'a>(
        items: &'a mut [SidebarItem],
        path: &[usize],
    ) -> Option<&'a mut SidebarItem> {
        let (head, tail) = path.split_first()?;
        let item = items.get_mut(*head)?;
        if tail.is_empty() {
            Some(item)
        } else {
            Self::item_mut_by_path(&mut item.children, tail)
        }
    }

    /// Get the currently selected item
    pub fn selected_item(&self) -> Option<&SidebarItem> {
        let items = self.visible_items();
        items
            .get(self.selected)
            .map(|(_item, _)| {
                // Find the actual item in the tree
                self.find_item_at_index(self.selected)
            })
            .flatten()
    }

    /// Find item at a flattened index
    fn find_item_at_index(&self, index: usize) -> Option<&SidebarItem> {
        let path = self.path_for_index(index)?;
        Self::item_by_path(&self.items, &path)
    }

    /// Move selection up
    fn select_prev(&mut self) {
        let items = self.visible_items();
        if items.is_empty() {
            return;
        }

        // Find previous selectable item
        let mut new_index = self.selected;
        loop {
            if new_index == 0 {
                new_index = items.len() - 1;
            } else {
                new_index -= 1;
            }

            if items[new_index].0.selectable || new_index == self.selected {
                break;
            }
        }
        self.selected = new_index;
        self.ensure_visible();
    }

    /// Move selection down
    fn select_next(&mut self) {
        let items = self.visible_items();
        if items.is_empty() {
            return;
        }

        // Find next selectable item
        let mut new_index = self.selected;
        loop {
            new_index = (new_index + 1) % items.len();

            if items[new_index].0.selectable || new_index == self.selected {
                break;
            }
        }
        self.selected = new_index;
        self.ensure_visible();
    }

    /// Toggle expand/collapse on current item
    fn toggle_expand(&mut self) {
        if let Some(path) = self.path_for_index(self.selected) {
            if let Some(item) = Self::item_mut_by_path(&mut self.items, &path) {
                if item.has_children() {
                    item.expanded = !item.expanded;
                }
            }

            let items = self.visible_items();
            if self.selected >= items.len() {
                self.selected = items.len().saturating_sub(1);
                self.ensure_visible();
            }
        }
    }

    /// Ensure the selected item is visible
    fn ensure_visible(&mut self) {
        if self.selected < self.scroll_offset {
            self.scroll_offset = self.selected;
        } else if self.selected >= self.scroll_offset + self.visible_height {
            self.scroll_offset = self.selected - self.visible_height + 1;
        }
    }

    /// Render item line
    fn render_item(
        &self,
        item: &SidebarItem,
        level: usize,
        is_selected: bool,
        row: usize,
        ctx: &RenderContext<'_>,
    ) -> String {
        let indent = "  ".repeat(level);
        let expand_icon = if item.has_children() {
            if item.expanded { "▼ " } else { "▶ " }
        } else {
            "  "
        };

        let icon = item.icon.as_deref().unwrap_or("");
        let icon_space = if icon.is_empty() { "" } else { " " };

        let _badge = item.badge.as_ref().map(|b| format!(" ({})", b)).unwrap_or_default();

        let border = self.vertical_border(row, ctx);
        let selection = if is_selected && ctx.focused {
            format!("{}>", border)
        } else {
            format!("{} ", border)
        };

        match self.mode {
            SidebarMode::Full => {
                format!(
                    "{}{}{}{}{}{}",
                    selection, indent, expand_icon, icon, icon_space, item.label
                )
            }
            SidebarMode::Collapsed => {
                format!("{} {}", selection, icon)
            }
            SidebarMode::Hidden => String::new(),
        }
    }

    fn border_width(&self) -> usize {
        let width = if self.bounds.width == 0 {
            20
        } else {
            self.bounds.width as usize
        };
        width.max(4)
    }

    fn gradient_color_from_token(color: &Color) -> SolidColor {
        match color {
            Color::Solid(solid) => *solid,
            Color::Gradient(gradient) => gradient.start,
            Color::Rainbow(rainbow) => rainbow.at(0.0),
        }
    }

    fn vertical_border(&self, row: usize, ctx: &RenderContext<'_>) -> String {
        match self.border_style {
            SidebarBorderStyle::Default => "│".to_string(),
            SidebarBorderStyle::Gradient => {
                let start = Self::gradient_color_from_token(&ctx.theme.tokens.colors.primary);
                let end = Self::gradient_color_from_token(&ctx.theme.tokens.colors.accent);
                let gradient = GradientAnimation::new(start, end, Duration::from_secs(2));
                gradient.apply("│")
            }
            SidebarBorderStyle::Rainbow => {
                let rainbow = RainbowAnimation::new();
                rainbow.vertical_border(row + ctx.frame as usize)
            }
        }
    }

    fn top_border(&self, ctx: &RenderContext<'_>) -> String {
        let width = self.border_width();
        match self.border_style {
            SidebarBorderStyle::Default => {
                "┌".to_string() + &"─".repeat(width.saturating_sub(2)) + "┐"
            }
            SidebarBorderStyle::Gradient => {
                let start = Self::gradient_color_from_token(&ctx.theme.tokens.colors.primary);
                let end = Self::gradient_color_from_token(&ctx.theme.tokens.colors.accent);
                let gradient = GradientAnimation::new(start, end, Duration::from_secs(2));
                gradient.apply(&("┌".to_string() + &"─".repeat(width.saturating_sub(2)) + "┐"))
            }
            SidebarBorderStyle::Rainbow => {
                let rainbow = RainbowAnimation::new();
                rainbow.border(width, BorderStyle::TopSquare)
            }
        }
    }

    fn bottom_border(&self, ctx: &RenderContext<'_>) -> String {
        let width = self.border_width();
        match self.border_style {
            SidebarBorderStyle::Default => {
                "└".to_string() + &"─".repeat(width.saturating_sub(2)) + "┘"
            }
            SidebarBorderStyle::Gradient => {
                let start = Self::gradient_color_from_token(&ctx.theme.tokens.colors.primary);
                let end = Self::gradient_color_from_token(&ctx.theme.tokens.colors.accent);
                let gradient = GradientAnimation::new(start, end, Duration::from_secs(2));
                gradient.apply(&("└".to_string() + &"─".repeat(width.saturating_sub(2)) + "┘"))
            }
            SidebarBorderStyle::Rainbow => {
                let rainbow = RainbowAnimation::new();
                rainbow.border(width, BorderStyle::BottomSquare)
            }
        }
    }
}

impl DxComponent for Sidebar {
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
            KeyEvent::Enter | KeyEvent::Right | KeyEvent::Char('l') => {
                let items = self.visible_items();
                if let Some((item, _)) = items.get(self.selected) {
                    if item.has_children() {
                        self.toggle_expand();
                        ComponentResult::Consumed
                    } else if let Some(ref data) = item.data {
                        ComponentResult::ActionWithData("select".into(), data.clone())
                    } else {
                        ComponentResult::Action(format!("select:{}", item.label))
                    }
                } else {
                    ComponentResult::Ignored
                }
            }
            KeyEvent::Left | KeyEvent::Char('h') => {
                self.toggle_expand();
                ComponentResult::Consumed
            }
            KeyEvent::Tab => ComponentResult::FocusNext,
            KeyEvent::BackTab => ComponentResult::FocusPrev,
            KeyEvent::Home => {
                self.selected = 0;
                self.scroll_offset = 0;
                ComponentResult::Consumed
            }
            KeyEvent::End => {
                let items = self.visible_items();
                if !items.is_empty() {
                    self.selected = items.len() - 1;
                    self.ensure_visible();
                }
                ComponentResult::Consumed
            }
            KeyEvent::PageUp => {
                let items = self.visible_items();
                if !items.is_empty() {
                    self.selected = self.selected.saturating_sub(self.visible_height);
                    self.ensure_visible();
                }
                ComponentResult::Consumed
            }
            KeyEvent::PageDown => {
                let items = self.visible_items();
                if !items.is_empty() {
                    self.selected = (self.selected + self.visible_height).min(items.len() - 1);
                    self.ensure_visible();
                }
                ComponentResult::Consumed
            }
            _ => ComponentResult::Ignored,
        }
    }

    fn handle_mouse(&mut self, event: MouseEvent) -> ComponentResult {
        match event {
            MouseEvent::Click { x, y } => {
                if self.bounds.contains(x, y) {
                    let relative_y = (y - self.bounds.y) as usize;
                    let clicked_index = self.scroll_offset + relative_y;
                    let items = self.visible_items();

                    if clicked_index < items.len() {
                        self.selected = clicked_index;
                        return ComponentResult::Consumed;
                    }
                }
                ComponentResult::Ignored
            }
            MouseEvent::DoubleClick { x, y } => {
                if self.bounds.contains(x, y) {
                    let relative_y = (y - self.bounds.y) as usize;
                    let clicked_index = self.scroll_offset + relative_y;
                    let items = self.visible_items();

                    if let Some((item, _)) = items.get(clicked_index) {
                        if let Some(ref data) = item.data {
                            return ComponentResult::ActionWithData("open".into(), data.clone());
                        } else {
                            return ComponentResult::Action(format!("open:{}", item.label));
                        }
                    }
                }
                ComponentResult::Ignored
            }
            MouseEvent::ScrollUp { .. } => {
                self.scroll_offset = self.scroll_offset.saturating_sub(3);
                ComponentResult::Consumed
            }
            MouseEvent::ScrollDown { .. } => {
                let items = self.visible_items();
                let max_offset = items.len().saturating_sub(self.visible_height);
                self.scroll_offset = (self.scroll_offset + 3).min(max_offset);
                ComponentResult::Consumed
            }
            _ => ComponentResult::Ignored,
        }
    }

    fn render(&self, ctx: &RenderContext<'_>) -> Vec<String> {
        if self.mode == SidebarMode::Hidden {
            return Vec::new();
        }

        let mut lines = Vec::new();

        // Title
        if let Some(ref title) = self.title {
            let title_line = if ctx.focused {
                format!("┌─ {} ─┐", title)
            } else {
                format!("┌─ {} ─┐", title)
            };
            lines.push(title_line);
        } else {
            lines.push(self.top_border(ctx));
        }

        // Items
        let items = self.visible_items();
        let start = self.scroll_offset;
        let end = (start + self.visible_height).min(items.len());

        for (i, (item, level)) in items.iter().enumerate().skip(start).take(end - start) {
            let is_selected = i == self.selected;
            let row = i.saturating_sub(start) + 1;
            lines.push(self.render_item(item, *level, is_selected, row, ctx));
        }

        // Padding
        for row in lines.len()..(self.visible_height + 2) {
            let border = self.vertical_border(row, ctx);
            let inner = " ".repeat(self.border_width().saturating_sub(2));
            lines.push(format!("{}{}{}", border, inner, border));
        }

        // Bottom border
        lines.push(self.bottom_border(ctx));

        lines
    }

    fn is_focusable(&self) -> bool {
        self.mode != SidebarMode::Hidden
    }

    fn set_focused(&mut self, focused: bool) {
        self.focused = focused;
    }

    fn is_focused(&self) -> bool {
        self.focused
    }

    fn id(&self) -> Option<&str> {
        self.id.as_deref()
    }

    fn bounds(&self) -> Bounds {
        self.bounds
    }

    fn set_bounds(&mut self, bounds: Bounds) {
        self.bounds = bounds;
        self.visible_height = bounds.height.saturating_sub(2) as usize; // Account for borders
    }

    fn min_size(&self) -> (u16, u16) {
        match self.mode {
            SidebarMode::Full => (20, 10),
            SidebarMode::Collapsed => (4, 10),
            SidebarMode::Hidden => (0, 0),
        }
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
    use crate::ui::theme::DxTheme;

    #[test]
    fn test_sidebar_item_creation() {
        let item = SidebarItem::new("test").icon("󰈙").badge("5").expanded(true);

        assert_eq!(item.label, "test");
        assert_eq!(item.icon, Some("󰈙".to_string()));
        assert_eq!(item.badge, Some("5".to_string()));
        assert!(item.expanded);
    }

    #[test]
    fn test_sidebar_navigation() {
        let mut sidebar = Sidebar::new()
            .item(SidebarItem::new("Item 1"))
            .item(SidebarItem::new("Item 2"))
            .item(SidebarItem::new("Item 3"));

        assert_eq!(sidebar.selected, 0);

        sidebar.handle_key(KeyEvent::Down);
        assert_eq!(sidebar.selected, 1);

        sidebar.handle_key(KeyEvent::Down);
        assert_eq!(sidebar.selected, 2);

        sidebar.handle_key(KeyEvent::Up);
        assert_eq!(sidebar.selected, 1);
    }

    #[test]
    fn test_sidebar_nested_items() {
        let sidebar = Sidebar::new().item(
            SidebarItem::new("src")
                .expanded(true)
                .children(vec![SidebarItem::new("main.rs"), SidebarItem::new("lib.rs")]),
        );

        let items = sidebar.visible_items();
        assert_eq!(items.len(), 3); // src + 2 children
        assert_eq!(items[0].0.label, "src");
        assert_eq!(items[1].0.label, "main.rs");
        assert_eq!(items[2].0.label, "lib.rs");
    }

    #[test]
    fn test_sidebar_collapsed_mode() {
        let sidebar = Sidebar::new()
            .mode(SidebarMode::Collapsed)
            .item(SidebarItem::new("test").icon("󰈙"));

        assert!(sidebar.is_focusable());
        assert_eq!(sidebar.min_size(), (4, 10));
    }

    #[test]
    fn test_sidebar_toggle_expand() {
        let mut sidebar = Sidebar::new().items(vec![
            SidebarItem::new("src")
                .expanded(false)
                .children(vec![SidebarItem::new("main.rs")]),
        ]);

        assert_eq!(sidebar.visible_items().len(), 1);
        sidebar.handle_key(KeyEvent::Enter);
        assert_eq!(sidebar.visible_items().len(), 2);
        sidebar.handle_key(KeyEvent::Enter);
        assert_eq!(sidebar.visible_items().len(), 1);
    }
}
