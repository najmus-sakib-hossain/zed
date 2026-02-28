//! Toast component
//!
//! Shows transient notification messages.
//!
//! # Example
//!
//! ```rust,ignore
//! use dx_cli::ui::components::toast::{Toast, ToastType};
//!
//! let toast = Toast::new("Saved", ToastType::Success);
//! ```

use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};
use std::time::{Duration, Instant};

use super::traits::{Bounds, ComponentResult, DxComponent, KeyEvent, MouseEvent, RenderContext};

/// Toast notification severity level.
///
/// Determines the icon and color scheme of the toast.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ToastType {
    /// Informational message (blue, ℹ icon)
    Info,
    /// Success message (green, ✓ icon)
    Success,
    /// Warning message (yellow, ⚠ icon)
    Warning,
    /// Error message (red, ✗ icon)
    Error,
}

/// A transient notification component that auto-dismisses.
///
/// Toasts appear in the corner of the screen to show brief
/// messages. They automatically disappear after a configurable
/// duration.
///
/// # Features
///
/// - Auto-dismiss after timeout
/// - Four severity levels with distinct styling
/// - Positioned in top-right corner
/// - Convenience constructors for each type
///
/// # Example
///
/// ```rust,ignore
/// use dx_cli::ui::components::toast::{Toast, ToastType};
/// use std::time::Duration;
///
/// // Using type-specific constructors
/// let success = Toast::success("File saved successfully!");
/// let error = Toast::error("Failed to connect");
///
/// // Custom duration
/// let toast = Toast::new("Processing...", ToastType::Info)
///     .duration(Duration::from_secs(5));
///
/// // Check if toast should be removed
/// if toast.is_expired() {
///     // Remove from toast queue
/// }
/// ```
pub struct Toast {
    /// The notification message
    message: String,
    /// Severity level (Info, Success, Warning, Error)
    toast_type: ToastType,
    /// When the toast was created
    created_at: Instant,
    /// How long the toast should be visible
    duration: Duration,
    /// Whether the component has focus
    focused: bool,
    /// Component bounds for positioning
    bounds: Bounds,
}

impl Toast {
    /// Create a new toast
    pub fn new(message: impl Into<String>, toast_type: ToastType) -> Self {
        Self {
            message: message.into(),
            toast_type,
            created_at: Instant::now(),
            duration: Duration::from_secs(3),
            focused: false,
            bounds: Bounds::default(),
        }
    }

    pub fn info(message: impl Into<String>) -> Self {
        Self::new(message, ToastType::Info)
    }

    pub fn success(message: impl Into<String>) -> Self {
        Self::new(message, ToastType::Success)
    }

    pub fn warning(message: impl Into<String>) -> Self {
        Self::new(message, ToastType::Warning)
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self::new(message, ToastType::Error)
    }

    pub fn duration(mut self, duration: Duration) -> Self {
        self.duration = duration;
        self
    }

    pub fn is_expired(&self) -> bool {
        self.created_at.elapsed() > self.duration
    }

    pub fn render(&self, f: &mut Frame, screen_area: Rect) {
        let (icon, color) = match self.toast_type {
            ToastType::Info => ("ℹ", Color::Blue),
            ToastType::Success => ("✓", Color::Green),
            ToastType::Warning => ("⚠", Color::Yellow),
            ToastType::Error => ("✗", Color::Red),
        };

        let width = (self.message.len() + 6).min(60) as u16;
        let height = 3;

        let x = screen_area.width.saturating_sub(width + 2);
        let y = 1;

        let area = Rect {
            x,
            y,
            width,
            height,
        };

        f.render_widget(Clear, area);

        let line = Line::from(vec![
            Span::styled(
                format!("{} ", icon),
                Style::default().fg(color).add_modifier(Modifier::BOLD),
            ),
            Span::styled(&self.message, Style::default().fg(Color::White)),
        ]);

        let block = Block::default().borders(Borders::ALL).border_style(Style::default().fg(color));

        let paragraph = Paragraph::new(vec![line]).block(block).alignment(Alignment::Left);

        f.render_widget(paragraph, area);
    }
}

pub struct ToastManager {
    toasts: Vec<Toast>,
}

impl ToastManager {
    pub fn new() -> Self {
        Self { toasts: Vec::new() }
    }

    pub fn add(&mut self, toast: Toast) {
        self.toasts.push(toast);
    }

    pub fn update(&mut self) {
        self.toasts.retain(|toast| !toast.is_expired());
    }

    pub fn render(&self, f: &mut Frame, screen_area: Rect) {
        for (i, toast) in self.toasts.iter().enumerate() {
            let mut area = screen_area;
            area.y += (i * 4) as u16;
            toast.render(f, area);
        }
    }
}

impl Default for ToastManager {
    fn default() -> Self {
        Self::new()
    }
}

impl DxComponent for Toast {
    fn handle_key(&mut self, key: KeyEvent) -> ComponentResult {
        match key {
            KeyEvent::Escape | KeyEvent::Enter => {
                ComponentResult::Action("toast:dismiss".to_string())
            }
            _ => ComponentResult::Ignored,
        }
    }

    fn handle_mouse(&mut self, event: MouseEvent) -> ComponentResult {
        match event {
            MouseEvent::Click { x, y } => {
                if self.bounds.contains(x, y) {
                    return ComponentResult::Action("toast:dismiss".to_string());
                }
                ComponentResult::Ignored
            }
            _ => ComponentResult::Ignored,
        }
    }

    fn render(&self, _ctx: &RenderContext<'_>) -> Vec<String> {
        let icon = match self.toast_type {
            ToastType::Info => "ℹ",
            ToastType::Success => "✓",
            ToastType::Warning => "⚠",
            ToastType::Error => "✗",
        };
        vec![format!("{} {}", icon, self.message)]
    }

    fn is_focusable(&self) -> bool {
        false
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
        ((self.message.len() + 4).min(60) as u16, 3)
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
