//! Dialog component for modal interactions
//!
//! A modal dialog for confirmations, alerts, and user input.
//!
//! # Example
//!
//! ```rust,ignore
//! use dx_cli::ui::components::{Dialog, DialogVariant};
//!
//! let dialog = Dialog::new("Confirm", "Are you sure?")
//!     .variant(DialogVariant::Warning)
//!     .buttons(vec!["Cancel", "Confirm"]);
//! ```

use super::traits::{Bounds, ComponentResult, DxComponent, KeyEvent, MouseEvent, RenderContext};
use std::any::Any;

/// Dialog variant for different styles
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DialogVariant {
    /// Default informational dialog
    #[default]
    Default,
    /// Warning dialog
    Warning,
    /// Error/destructive dialog
    Error,
    /// Success dialog
    Success,
    /// Input dialog
    Input,
}

/// Dialog result
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DialogResult {
    /// Dialog confirmed with selected button
    Confirmed(String),
    /// Dialog cancelled
    Cancelled,
    /// Dialog dismissed (Escape)
    Dismissed,
}

/// Modal dialog component
pub struct Dialog {
    /// Dialog title
    title: String,
    /// Dialog message
    message: String,
    /// Dialog buttons
    buttons: Vec<String>,
    /// Currently selected button
    selected_button: usize,
    /// Dialog width
    width: u16,
    /// Dialog height
    height: u16,
    /// Dialog variant
    variant: DialogVariant,
    /// Component bounds
    bounds: Bounds,
    /// Component ID
    id: Option<String>,
    /// Whether focused
    focused: bool,
    /// Whether visible
    visible: bool,
    /// Input value (for Input variant)
    input_value: String,
    /// Input cursor position
    input_cursor: usize,
}

impl Dialog {
    /// Create a new dialog
    pub fn new(title: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            message: message.into(),
            buttons: vec!["OK".to_string()],
            selected_button: 0,
            width: 50,
            height: 10,
            variant: DialogVariant::Default,
            bounds: Bounds::default(),
            id: None,
            focused: true,
            visible: true,
            input_value: String::new(),
            input_cursor: 0,
        }
    }

    /// Set the dialog buttons
    pub fn buttons<S: Into<String>>(mut self, buttons: Vec<S>) -> Self {
        self.buttons = buttons.into_iter().map(|s| s.into()).collect();
        self.selected_button = 0;
        self
    }

    /// Set dialog size
    pub fn size(mut self, width: u16, height: u16) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    /// Set dialog variant
    pub fn variant(mut self, variant: DialogVariant) -> Self {
        self.variant = variant;
        self
    }

    /// Set component ID
    pub fn with_id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// Set visibility
    pub fn visible(mut self, visible: bool) -> Self {
        self.visible = visible;
        self
    }

    /// Set initial input value (for Input variant)
    pub fn input_value(mut self, value: impl Into<String>) -> Self {
        self.input_value = value.into();
        self.input_cursor = self.input_value.len();
        self
    }

    /// Get the selected button text
    pub fn selected_button(&self) -> &str {
        &self.buttons[self.selected_button]
    }

    /// Get the input value
    pub fn get_input(&self) -> &str {
        &self.input_value
    }

    /// Show the dialog
    pub fn show(&mut self) {
        self.visible = true;
        self.focused = true;
    }

    /// Hide the dialog
    pub fn hide(&mut self) {
        self.visible = false;
    }

    /// Get variant indicator
    fn variant_indicator(&self) -> &'static str {
        match self.variant {
            DialogVariant::Default => "ℹ",
            DialogVariant::Warning => "⚠",
            DialogVariant::Error => "✗",
            DialogVariant::Success => "✓",
            DialogVariant::Input => "✎",
        }
    }

    /// Handle input key
    fn handle_input_key(&mut self, key: KeyEvent) -> ComponentResult {
        match key {
            KeyEvent::Char(c) => {
                self.input_value.insert(self.input_cursor, c);
                self.input_cursor += 1;
                ComponentResult::Consumed
            }
            KeyEvent::Backspace => {
                if self.input_cursor > 0 {
                    self.input_cursor -= 1;
                    self.input_value.remove(self.input_cursor);
                }
                ComponentResult::Consumed
            }
            KeyEvent::Delete => {
                if self.input_cursor < self.input_value.len() {
                    self.input_value.remove(self.input_cursor);
                }
                ComponentResult::Consumed
            }
            KeyEvent::Left => {
                self.input_cursor = self.input_cursor.saturating_sub(1);
                ComponentResult::Consumed
            }
            KeyEvent::Right => {
                self.input_cursor = (self.input_cursor + 1).min(self.input_value.len());
                ComponentResult::Consumed
            }
            KeyEvent::Home => {
                self.input_cursor = 0;
                ComponentResult::Consumed
            }
            KeyEvent::End => {
                self.input_cursor = self.input_value.len();
                ComponentResult::Consumed
            }
            _ => ComponentResult::Ignored,
        }
    }
}

impl DxComponent for Dialog {
    fn handle_key(&mut self, key: KeyEvent) -> ComponentResult {
        if !self.visible {
            return ComponentResult::Ignored;
        }

        // Handle input for Input variant
        if self.variant == DialogVariant::Input {
            match key {
                KeyEvent::Tab => {
                    // Move to buttons
                }
                KeyEvent::Enter => {
                    let button = self.buttons[self.selected_button].clone();
                    return ComponentResult::ActionWithData(
                        format!("dialog:{}:{}", button, self.input_value),
                        self.input_value.clone(),
                    );
                }
                KeyEvent::Escape => {
                    self.visible = false;
                    return ComponentResult::Action("dialog:cancelled".into());
                }
                _ => {
                    let result = self.handle_input_key(key.clone());
                    if result != ComponentResult::Ignored {
                        return result;
                    }
                }
            }
        }

        match key {
            KeyEvent::Left | KeyEvent::Char('h') => {
                if self.selected_button > 0 {
                    self.selected_button -= 1;
                } else {
                    self.selected_button = self.buttons.len() - 1;
                }
                ComponentResult::Consumed
            }
            KeyEvent::Right | KeyEvent::Char('l') => {
                self.selected_button = (self.selected_button + 1) % self.buttons.len();
                ComponentResult::Consumed
            }
            KeyEvent::Tab => {
                self.selected_button = (self.selected_button + 1) % self.buttons.len();
                ComponentResult::Consumed
            }
            KeyEvent::BackTab => {
                if self.selected_button > 0 {
                    self.selected_button -= 1;
                } else {
                    self.selected_button = self.buttons.len() - 1;
                }
                ComponentResult::Consumed
            }
            KeyEvent::Enter => {
                let button = self.buttons[self.selected_button].clone();
                self.visible = false;
                ComponentResult::Action(format!("dialog:{}", button))
            }
            KeyEvent::Escape => {
                self.visible = false;
                ComponentResult::Action("dialog:cancelled".into())
            }
            _ => ComponentResult::Ignored,
        }
    }

    fn handle_mouse(&mut self, event: MouseEvent) -> ComponentResult {
        if !self.visible {
            return ComponentResult::Ignored;
        }

        match event {
            MouseEvent::Click { x, y } => {
                // Check if click is on a button
                // For now, just consume clicks within bounds
                if self.bounds.contains(x, y) {
                    ComponentResult::Consumed
                } else {
                    // Click outside dialog
                    ComponentResult::Ignored
                }
            }
            _ => ComponentResult::Ignored,
        }
    }

    fn render(&self, ctx: &RenderContext<'_>) -> Vec<String> {
        if !self.visible {
            return Vec::new();
        }

        let mut lines = Vec::new();
        let width = self.width as usize;
        let indicator = self.variant_indicator();

        // Top border with title
        let title_text = format!("{} {}", indicator, self.title);
        let title_padding = width.saturating_sub(title_text.len() + 4);
        lines.push(format!("╔═ {} {}╗", title_text, "═".repeat(title_padding)));

        // Empty line
        lines.push(format!("║{}║", " ".repeat(width - 2)));

        // Message (centered)
        let msg_padding = width.saturating_sub(self.message.len() + 2);
        let left_pad = msg_padding / 2;
        let right_pad = msg_padding - left_pad;
        lines.push(format!("║{}{}{}║", " ".repeat(left_pad), self.message, " ".repeat(right_pad)));

        // Input field for Input variant
        if self.variant == DialogVariant::Input {
            lines.push(format!("║{}║", " ".repeat(width - 2)));

            let input_width = width - 6;
            let input_display = if self.input_value.len() > input_width {
                &self.input_value[self.input_value.len() - input_width..]
            } else {
                &self.input_value
            };
            let cursor = if ctx.focused { "█" } else { "" };
            let input_padding = input_width.saturating_sub(input_display.len() + cursor.len());
            lines.push(format!("║ [{}{}{}] ║", input_display, cursor, " ".repeat(input_padding)));
        }

        // Empty line before buttons
        lines.push(format!("║{}║", " ".repeat(width - 2)));

        // Buttons
        let mut button_line = String::new();
        for (i, button) in self.buttons.iter().enumerate() {
            let is_selected = i == self.selected_button;
            if is_selected {
                button_line.push_str(&format!(" [*{}*] ", button));
            } else {
                button_line.push_str(&format!(" [ {} ] ", button));
            }
        }
        let btn_padding = width.saturating_sub(button_line.len() + 2);
        let btn_left = btn_padding / 2;
        let btn_right = btn_padding - btn_left;
        lines.push(format!("║{}{}{}║", " ".repeat(btn_left), button_line, " ".repeat(btn_right)));

        // Bottom border
        lines.push(format!("╚{}╝", "═".repeat(width - 2)));

        lines
    }

    fn is_focusable(&self) -> bool {
        self.visible
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
    }

    fn min_size(&self) -> (u16, u16) {
        (self.width, self.height)
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
    fn test_dialog_creation() {
        let dialog = Dialog::new("Test", "Test message").buttons(vec!["Cancel", "OK"]);

        assert_eq!(dialog.title, "Test");
        assert_eq!(dialog.message, "Test message");
        assert_eq!(dialog.buttons.len(), 2);
    }

    #[test]
    fn test_dialog_navigation() {
        let mut dialog = Dialog::new("Test", "Message").buttons(vec!["A", "B", "C"]);

        assert_eq!(dialog.selected_button, 0);

        dialog.handle_key(KeyEvent::Right);
        assert_eq!(dialog.selected_button, 1);

        dialog.handle_key(KeyEvent::Right);
        assert_eq!(dialog.selected_button, 2);

        dialog.handle_key(KeyEvent::Right);
        assert_eq!(dialog.selected_button, 0); // Wrap
    }

    #[test]
    fn test_dialog_confirm() {
        let mut dialog = Dialog::new("Test", "Message").buttons(vec!["Cancel", "OK"]);

        dialog.handle_key(KeyEvent::Right); // Select "OK"
        let result = dialog.handle_key(KeyEvent::Enter);

        assert!(matches!(result, ComponentResult::Action(s) if s == "dialog:OK"));
        assert!(!dialog.visible);
    }

    #[test]
    fn test_dialog_cancel() {
        let mut dialog = Dialog::new("Test", "Message");

        let result = dialog.handle_key(KeyEvent::Escape);

        assert!(matches!(result, ComponentResult::Action(s) if s == "dialog:cancelled"));
        assert!(!dialog.visible);
    }

    #[test]
    fn test_input_dialog() {
        let mut dialog = Dialog::new("Input", "Enter value:").variant(DialogVariant::Input);

        dialog.handle_key(KeyEvent::Char('t'));
        dialog.handle_key(KeyEvent::Char('e'));
        dialog.handle_key(KeyEvent::Char('s'));
        dialog.handle_key(KeyEvent::Char('t'));

        assert_eq!(dialog.get_input(), "test");
    }
}
