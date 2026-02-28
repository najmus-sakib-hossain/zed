//! DX Component Trait System
//!
//! This module defines the core `DxComponent` trait that all interactive
//! UI components implement, providing a unified interface for rendering,
//! event handling, and focus management.
//!
//! # Architecture
//!
//! Components follow a reactive architecture:
//! 1. **State**: Internal component state
//! 2. **Render**: Convert state to visual output
//! 3. **Handle**: Process input events and update state
//! 4. **Focus**: Manage keyboard focus chain
//!
//! # Example
//!
//! ```rust,ignore
//! use dx_cli::ui::components::trait::{DxComponent, ComponentResult, KeyEvent};
//!
//! struct MyButton {
//!     label: String,
//!     focused: bool,
//! }
//!
//! impl DxComponent for MyButton {
//!     fn handle_key(&mut self, key: KeyEvent) -> ComponentResult {
//!         match key {
//!             KeyEvent::Enter => ComponentResult::Action("clicked".into()),
//!             KeyEvent::Tab => ComponentResult::FocusNext,
//!             _ => ComponentResult::Consumed,
//!         }
//!     }
//!
//!     fn is_focusable(&self) -> bool { true }
//!     fn set_focused(&mut self, focused: bool) { self.focused = focused; }
//!     fn is_focused(&self) -> bool { self.focused }
//! }
//! ```

use std::any::Any;
use std::fmt::Debug;

/// Key event for component handling
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyEvent {
    /// Character input
    Char(char),
    /// Enter/Return key
    Enter,
    /// Escape key
    Escape,
    /// Tab key
    Tab,
    /// Shift+Tab
    BackTab,
    /// Backspace key
    Backspace,
    /// Delete key
    Delete,
    /// Up arrow
    Up,
    /// Down arrow
    Down,
    /// Left arrow
    Left,
    /// Right arrow
    Right,
    /// Home key
    Home,
    /// End key
    End,
    /// Page Up
    PageUp,
    /// Page Down
    PageDown,
    /// Ctrl+key combination
    Ctrl(char),
    /// Alt+key combination
    Alt(char),
    /// Function key (F1-F12)
    F(u8),
    /// Unknown/other key
    Unknown,
}

impl KeyEvent {
    /// Check if this is a navigation key
    pub fn is_navigation(&self) -> bool {
        matches!(
            self,
            KeyEvent::Up
                | KeyEvent::Down
                | KeyEvent::Left
                | KeyEvent::Right
                | KeyEvent::Home
                | KeyEvent::End
                | KeyEvent::PageUp
                | KeyEvent::PageDown
                | KeyEvent::Tab
                | KeyEvent::BackTab
        )
    }

    /// Check if this is a modifier combination
    pub fn is_modified(&self) -> bool {
        matches!(self, KeyEvent::Ctrl(_) | KeyEvent::Alt(_))
    }
}

/// Mouse event for component handling
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MouseEvent {
    /// Left click at position
    Click { x: u16, y: u16 },
    /// Right click at position
    RightClick { x: u16, y: u16 },
    /// Double click at position
    DoubleClick { x: u16, y: u16 },
    /// Mouse moved to position
    Move { x: u16, y: u16 },
    /// Scroll up
    ScrollUp { x: u16, y: u16 },
    /// Scroll down
    ScrollDown { x: u16, y: u16 },
    /// Drag operation
    Drag { x: u16, y: u16 },
    /// Mouse button released
    Release { x: u16, y: u16 },
}

impl MouseEvent {
    /// Get the position of this mouse event
    pub fn position(&self) -> (u16, u16) {
        match self {
            MouseEvent::Click { x, y }
            | MouseEvent::RightClick { x, y }
            | MouseEvent::DoubleClick { x, y }
            | MouseEvent::Move { x, y }
            | MouseEvent::ScrollUp { x, y }
            | MouseEvent::ScrollDown { x, y }
            | MouseEvent::Drag { x, y }
            | MouseEvent::Release { x, y } => (*x, *y),
        }
    }
}

/// Result of component event handling
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ComponentResult {
    /// Event was consumed, no further action needed
    Consumed,
    /// Event was not handled, pass to parent
    Ignored,
    /// Request focus on next component
    FocusNext,
    /// Request focus on previous component
    FocusPrev,
    /// Request specific component focus by ID
    FocusId(String),
    /// Trigger an action with name
    Action(String),
    /// Trigger an action with name and data
    ActionWithData(String, String),
    /// Request component close/dismiss
    Close,
    /// Request application exit
    Exit,
    /// Submit current value
    Submit(String),
    /// Cancel current operation
    Cancel,
    /// Request redraw
    Redraw,
}

/// Component bounds (position and size)
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Bounds {
    /// X position (column)
    pub x: u16,
    /// Y position (row)
    pub y: u16,
    /// Width in columns
    pub width: u16,
    /// Height in rows
    pub height: u16,
}

impl Bounds {
    /// Create new bounds
    pub const fn new(x: u16, y: u16, width: u16, height: u16) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    /// Check if a point is within these bounds
    pub fn contains(&self, x: u16, y: u16) -> bool {
        x >= self.x && x < self.x + self.width && y >= self.y && y < self.y + self.height
    }

    /// Get the inner bounds with padding
    pub fn inner(&self, padding: u16) -> Self {
        Self {
            x: self.x + padding,
            y: self.y + padding,
            width: self.width.saturating_sub(padding * 2),
            height: self.height.saturating_sub(padding * 2),
        }
    }

    /// Split horizontally at a position
    pub fn split_horizontal(&self, at: u16) -> (Self, Self) {
        let at = at.min(self.height);
        (
            Self {
                x: self.x,
                y: self.y,
                width: self.width,
                height: at,
            },
            Self {
                x: self.x,
                y: self.y + at,
                width: self.width,
                height: self.height.saturating_sub(at),
            },
        )
    }

    /// Split vertically at a position
    pub fn split_vertical(&self, at: u16) -> (Self, Self) {
        let at = at.min(self.width);
        (
            Self {
                x: self.x,
                y: self.y,
                width: at,
                height: self.height,
            },
            Self {
                x: self.x + at,
                y: self.y,
                width: self.width.saturating_sub(at),
                height: self.height,
            },
        )
    }
}

/// Render context passed to components
pub struct RenderContext<'a> {
    /// Available bounds for rendering
    pub bounds: Bounds,
    /// Whether the component is focused
    pub focused: bool,
    /// Theme tokens for styling
    pub theme: &'a crate::ui::theme::DxTheme,
    /// Current frame/tick for animations
    pub frame: u64,
}

/// The core component trait for interactive UI elements
pub trait DxComponent: Send + Sync {
    /// Handle a keyboard event
    fn handle_key(&mut self, key: KeyEvent) -> ComponentResult {
        let _ = key;
        ComponentResult::Ignored
    }

    /// Handle a mouse event
    fn handle_mouse(&mut self, event: MouseEvent) -> ComponentResult {
        let _ = event;
        ComponentResult::Ignored
    }

    /// Render the component to a string buffer
    fn render(&self, ctx: &RenderContext<'_>) -> Vec<String>;

    /// Check if this component can receive focus
    fn is_focusable(&self) -> bool {
        false
    }

    /// Set the focused state
    fn set_focused(&mut self, focused: bool) {
        let _ = focused;
    }

    /// Get the focused state
    fn is_focused(&self) -> bool {
        false
    }

    /// Get the component's unique ID
    fn id(&self) -> Option<&str> {
        None
    }

    /// Get the component's bounds
    fn bounds(&self) -> Bounds {
        Bounds::default()
    }

    /// Set the component's bounds
    fn set_bounds(&mut self, bounds: Bounds) {
        let _ = bounds;
    }

    /// Get the minimum size required by this component
    fn min_size(&self) -> (u16, u16) {
        (1, 1)
    }

    /// Get the preferred size for this component
    fn preferred_size(&self) -> (u16, u16) {
        self.min_size()
    }

    /// Tick/update the component (for animations)
    fn tick(&mut self) {}

    /// Get child components (for containers)
    fn children(&self) -> Vec<&dyn DxComponent> {
        Vec::new()
    }

    /// Get mutable child components
    fn children_mut(&mut self) -> Vec<&mut dyn DxComponent> {
        Vec::new()
    }

    /// Convert to Any for downcasting
    fn as_any(&self) -> &dyn Any;

    /// Convert to mutable Any for downcasting
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

/// Focus direction for navigation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusDirection {
    /// Move to next focusable component
    Next,
    /// Move to previous focusable component
    Prev,
    /// Move up
    Up,
    /// Move down
    Down,
    /// Move left
    Left,
    /// Move right
    Right,
}

/// Focus manager for component trees
pub struct FocusManager {
    /// Currently focused component ID
    focused_id: Option<String>,
    /// Focus order (list of component IDs)
    focus_order: Vec<String>,
    /// Current focus index
    focus_index: usize,
}

impl Default for FocusManager {
    fn default() -> Self {
        Self::new()
    }
}

impl FocusManager {
    /// Create a new focus manager
    pub fn new() -> Self {
        Self {
            focused_id: None,
            focus_order: Vec::new(),
            focus_index: 0,
        }
    }

    /// Register a component ID in focus order
    pub fn register(&mut self, id: impl Into<String>) {
        let id = id.into();
        if !self.focus_order.contains(&id) {
            self.focus_order.push(id);
        }
    }

    /// Unregister a component ID
    pub fn unregister(&mut self, id: &str) {
        self.focus_order.retain(|i| i != id);
        if self.focused_id.as_deref() == Some(id) {
            self.focused_id = None;
        }
    }

    /// Get the currently focused ID
    pub fn focused(&self) -> Option<&str> {
        self.focused_id.as_deref()
    }

    /// Set focus to a specific ID
    pub fn set_focus(&mut self, id: impl Into<String>) {
        let id = id.into();
        if let Some(idx) = self.focus_order.iter().position(|i| i == &id) {
            self.focus_index = idx;
            self.focused_id = Some(id);
        }
    }

    /// Move focus in a direction
    pub fn move_focus(&mut self, direction: FocusDirection) -> Option<&str> {
        if self.focus_order.is_empty() {
            return None;
        }

        match direction {
            FocusDirection::Next | FocusDirection::Down | FocusDirection::Right => {
                self.focus_index = (self.focus_index + 1) % self.focus_order.len();
            }
            FocusDirection::Prev | FocusDirection::Up | FocusDirection::Left => {
                self.focus_index = if self.focus_index == 0 {
                    self.focus_order.len() - 1
                } else {
                    self.focus_index - 1
                };
            }
        }

        self.focused_id = self.focus_order.get(self.focus_index).cloned();
        self.focused_id.as_deref()
    }

    /// Clear all focus
    pub fn clear(&mut self) {
        self.focused_id = None;
        self.focus_order.clear();
        self.focus_index = 0;
    }
}

// Note: impl_component_any! macro is defined in lib.rs and re-exported from prelude

#[cfg(test)]
mod tests {
    use super::*;

    struct TestButton {
        label: String,
        focused: bool,
        bounds: Bounds,
    }

    impl DxComponent for TestButton {
        fn handle_key(&mut self, key: KeyEvent) -> ComponentResult {
            match key {
                KeyEvent::Enter => ComponentResult::Action("clicked".into()),
                KeyEvent::Tab => ComponentResult::FocusNext,
                KeyEvent::BackTab => ComponentResult::FocusPrev,
                _ => ComponentResult::Ignored,
            }
        }

        fn render(&self, ctx: &RenderContext<'_>) -> Vec<String> {
            let style = if ctx.focused { "[*]" } else { "[ ]" };
            vec![format!("{} {}", style, self.label)]
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

        fn as_any(&self) -> &dyn Any {
            self
        }

        fn as_any_mut(&mut self) -> &mut dyn Any {
            self
        }
    }

    #[test]
    fn test_key_event_navigation() {
        assert!(KeyEvent::Up.is_navigation());
        assert!(KeyEvent::Tab.is_navigation());
        assert!(!KeyEvent::Enter.is_navigation());
        assert!(!KeyEvent::Char('a').is_navigation());
    }

    #[test]
    fn test_bounds_contains() {
        let bounds = Bounds::new(10, 10, 20, 10);
        assert!(bounds.contains(15, 15));
        assert!(!bounds.contains(5, 5));
        assert!(!bounds.contains(35, 15));
    }

    #[test]
    fn test_bounds_split() {
        let bounds = Bounds::new(0, 0, 100, 50);

        let (top, bottom) = bounds.split_horizontal(20);
        assert_eq!(top.height, 20);
        assert_eq!(bottom.height, 30);
        assert_eq!(bottom.y, 20);

        let (left, right) = bounds.split_vertical(30);
        assert_eq!(left.width, 30);
        assert_eq!(right.width, 70);
        assert_eq!(right.x, 30);
    }

    #[test]
    fn test_focus_manager() {
        let mut fm = FocusManager::new();

        fm.register("btn1");
        fm.register("btn2");
        fm.register("btn3");

        fm.set_focus("btn1");
        assert_eq!(fm.focused(), Some("btn1"));

        fm.move_focus(FocusDirection::Next);
        assert_eq!(fm.focused(), Some("btn2"));

        fm.move_focus(FocusDirection::Prev);
        assert_eq!(fm.focused(), Some("btn1"));

        // Wrap around
        fm.move_focus(FocusDirection::Prev);
        assert_eq!(fm.focused(), Some("btn3"));
    }

    #[test]
    fn test_component_handle_key() {
        let mut btn = TestButton {
            label: "Test".to_string(),
            focused: false,
            bounds: Bounds::default(),
        };

        assert_eq!(btn.handle_key(KeyEvent::Enter), ComponentResult::Action("clicked".into()));
        assert_eq!(btn.handle_key(KeyEvent::Tab), ComponentResult::FocusNext);
        assert_eq!(btn.handle_key(KeyEvent::Char('x')), ComponentResult::Ignored);
    }
}
