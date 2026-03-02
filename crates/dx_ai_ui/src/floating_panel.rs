//! Floating AI Panel — Compact / medium / full mode popup (Part 6).
//!
//! A draggable, resizable floating panel that provides quick AI access
//! without leaving the current editor context.
//!
//! ## Modes
//!
//! - **Compact**: Single-line input bar (like Spotlight/Alfred)
//! - **Medium**: Input + small response area
//! - **Full**: Complete panel with history, tools, and sidebar

use gpui::{div, prelude::*, SharedString, Window};

/// Floating panel display mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PanelMode {
    /// Single-line quick input (like Spotlight).
    Compact,
    /// Input + small response area.
    Medium,
    /// Full-featured panel.
    Full,
}

/// The floating AI panel component.
pub struct FloatingAiPanel {
    mode: PanelMode,
    visible: bool,
    input_text: String,
    response_text: String,
    position_x: f32,
    position_y: f32,
}

impl FloatingAiPanel {
    pub fn new() -> Self {
        Self {
            mode: PanelMode::Compact,
            visible: false,
            input_text: String::new(),
            response_text: String::new(),
            position_x: 0.0,
            position_y: 0.0,
        }
    }

    pub fn toggle_visibility(&mut self, cx: &mut Context<Self>) {
        self.visible = !self.visible;
        cx.notify();
    }

    pub fn set_mode(&mut self, mode: PanelMode, cx: &mut Context<Self>) {
        self.mode = mode;
        cx.notify();
    }

    pub fn cycle_mode(&mut self, cx: &mut Context<Self>) {
        self.mode = match self.mode {
            PanelMode::Compact => PanelMode::Medium,
            PanelMode::Medium => PanelMode::Full,
            PanelMode::Full => PanelMode::Compact,
        };
        cx.notify();
    }

    pub fn set_input(&mut self, text: String, cx: &mut Context<Self>) {
        self.input_text = text;
        cx.notify();
    }

    pub fn set_response(&mut self, text: String, cx: &mut Context<Self>) {
        self.response_text = text;
        cx.notify();
    }

    pub fn is_visible(&self) -> bool {
        self.visible
    }

    pub fn mode(&self) -> PanelMode {
        self.mode
    }
}

impl Render for FloatingAiPanel {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        if !self.visible {
            return div();
        }

        let mode_label: SharedString = match self.mode {
            PanelMode::Compact => "Compact".into(),
            PanelMode::Medium => "Medium".into(),
            PanelMode::Full => "Full".into(),
        };

        div()
            .flex()
            .flex_col()
            .child(mode_label)
    }
}
