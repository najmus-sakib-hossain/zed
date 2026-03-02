//! Flow Bar Widget — Persistent bottom-center pill widget (Part 24).
//!
//! A small, always-visible widget anchored to the bottom of the window.
//! Shows the current AI state and provides quick-action triggers.
//!
//! ## States
//!
//! - **Idle**: Subtle pill with current mood icon
//! - **Listening**: Pulsing microphone animation
//! - **Transcribing**: Waveform display
//! - **Thinking**: Animated dots / loading spinner
//! - **Speaking**: Audio waveform playback indicator
//! - **Result**: Expandable preview of the AI's response
//! - **Error**: Red indicator with retry button

use gpui::{div, prelude::*, SharedString, Window};

/// The state of the flow bar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlowBarState {
    Idle,
    Listening,
    Transcribing,
    Thinking,
    Speaking,
    Result,
    Error,
}

/// The persistent bottom-center flow bar widget.
pub struct FlowBarWidget {
    state: FlowBarState,
    status_text: SharedString,
    expanded: bool,
}

impl FlowBarWidget {
    pub fn new() -> Self {
        Self {
            state: FlowBarState::Idle,
            status_text: "Ready".into(),
            expanded: false,
        }
    }

    pub fn set_state(&mut self, state: FlowBarState, cx: &mut Context<Self>) {
        self.state = state;
        self.status_text = match state {
            FlowBarState::Idle => "Ready".into(),
            FlowBarState::Listening => "Listening...".into(),
            FlowBarState::Transcribing => "Transcribing...".into(),
            FlowBarState::Thinking => "Thinking...".into(),
            FlowBarState::Speaking => "Speaking...".into(),
            FlowBarState::Result => "Done".into(),
            FlowBarState::Error => "Error — tap to retry".into(),
        };
        cx.notify();
    }

    pub fn toggle_expand(&mut self, cx: &mut Context<Self>) {
        self.expanded = !self.expanded;
        cx.notify();
    }

    pub fn state(&self) -> FlowBarState {
        self.state
    }

    pub fn is_expanded(&self) -> bool {
        self.expanded
    }
}

impl Render for FlowBarWidget {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_row()
            .items_center()
            .justify_center()
            .child(self.status_text.clone())
    }
}
