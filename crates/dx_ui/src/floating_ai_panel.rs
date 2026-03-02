//! FloatingAiPanel — Multi-mode floating panel for the DX AI assistant.
//!
//! Part 6: Three sizes — compact (320×480), medium (480×640), full (640×800).
//! Pinnable, draggable, with mode-dependent content.

use dx_core::mood::Mood;
use gpui::{
    div, prelude::*, px, FocusHandle, Focusable, ParentElement, Render, SharedString,
    Styled, ViewContext,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PanelSize {
    Compact,
    Medium,
    Full,
}

impl PanelSize {
    pub fn width(&self) -> f32 {
        match self {
            Self::Compact => 320.0,
            Self::Medium => 480.0,
            Self::Full => 640.0,
        }
    }

    pub fn height(&self) -> f32 {
        match self {
            Self::Compact => 480.0,
            Self::Medium => 640.0,
            Self::Full => 800.0,
        }
    }

    pub fn next(&self) -> Self {
        match self {
            Self::Compact => Self::Medium,
            Self::Medium => Self::Full,
            Self::Full => Self::Compact,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Compact => "Compact",
            Self::Medium => "Medium",
            Self::Full => "Full",
        }
    }
}

#[derive(Debug, Clone)]
pub enum FloatingPanelEvent {
    SizeChanged(PanelSize),
    PinnedChanged(bool),
    Dismissed,
    MessageSent(String),
}

pub struct FloatingAiPanel {
    focus_handle: FocusHandle,
    size: PanelSize,
    is_pinned: bool,
    is_visible: bool,
    current_mood: Mood,
    messages: Vec<PanelMessage>,
    input_text: String,
}

#[derive(Debug, Clone)]
pub struct PanelMessage {
    pub role: MessageRole,
    pub content: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageRole {
    User,
    Assistant,
    System,
}

impl FloatingAiPanel {
    pub fn new(cx: &mut ViewContext<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            size: PanelSize::Compact,
            is_pinned: false,
            is_visible: true,
            current_mood: Mood::Chat,
            messages: Vec::new(),
            input_text: String::new(),
        }
    }

    pub fn set_size(&mut self, size: PanelSize, cx: &mut ViewContext<Self>) {
        self.size = size;
        cx.emit(FloatingPanelEvent::SizeChanged(size));
        cx.notify();
    }

    pub fn cycle_size(&mut self, cx: &mut ViewContext<Self>) {
        let next = self.size.next();
        self.set_size(next, cx);
    }

    pub fn toggle_pinned(&mut self, cx: &mut ViewContext<Self>) {
        self.is_pinned = !self.is_pinned;
        cx.emit(FloatingPanelEvent::PinnedChanged(self.is_pinned));
        cx.notify();
    }

    pub fn show(&mut self, cx: &mut ViewContext<Self>) {
        self.is_visible = true;
        cx.notify();
    }

    pub fn hide(&mut self, cx: &mut ViewContext<Self>) {
        if !self.is_pinned {
            self.is_visible = false;
            cx.emit(FloatingPanelEvent::Dismissed);
            cx.notify();
        }
    }

    pub fn is_visible(&self) -> bool {
        self.is_visible
    }

    pub fn size(&self) -> PanelSize {
        self.size
    }

    pub fn set_mood(&mut self, mood: Mood, cx: &mut ViewContext<Self>) {
        self.current_mood = mood;
        cx.notify();
    }

    pub fn push_message(&mut self, message: PanelMessage, cx: &mut ViewContext<Self>) {
        self.messages.push(message);
        cx.notify();
    }

    pub fn clear_messages(&mut self, cx: &mut ViewContext<Self>) {
        self.messages.clear();
        cx.notify();
    }
}

impl gpui::EventEmitter<FloatingPanelEvent> for FloatingAiPanel {}

impl Focusable for FloatingAiPanel {
    fn focus_handle(&self, _cx: &gpui::App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for FloatingAiPanel {
    fn render(&mut self, _cx: &mut ViewContext<Self>) -> impl IntoElement {
        let size = self.size;
        let is_pinned = self.is_pinned;
        let is_visible = self.is_visible;
        let messages = self.messages.clone();
        let mood = self.current_mood;

        div()
            .when(!is_visible, |d| d.invisible())
            .w(px(size.width()))
            .h(px(size.height()))
            .rounded(px(16.0))
            .border_1()
            .flex()
            .flex_col()
            .overflow_hidden()
            // Title bar
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .px(px(12.0))
                    .py(px(8.0))
                    .border_b_1()
                    // Left: mood + title
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.0))
                            .child(
                                div()
                                    .text_sm()
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .child(SharedString::from(format!("DX AI — {:?}", mood)))
                            )
                    )
                    // Right: size toggle + pin + close
                    .child(
                        div()
                            .flex()
                            .gap(px(4.0))
                            .child(
                                div()
                                    .cursor_pointer()
                                    .text_xs()
                                    .px(px(6.0))
                                    .py(px(2.0))
                                    .rounded(px(4.0))
                                    .child(SharedString::from(size.label()))
                            )
                            .child(
                                div()
                                    .cursor_pointer()
                                    .text_xs()
                                    .px(px(6.0))
                                    .py(px(2.0))
                                    .rounded(px(4.0))
                                    .child(SharedString::from(
                                        if is_pinned { "📌" } else { "📍" }
                                    ))
                            )
                            .child(
                                div()
                                    .cursor_pointer()
                                    .text_xs()
                                    .px(px(6.0))
                                    .py(px(2.0))
                                    .rounded(px(4.0))
                                    .child(SharedString::from("✕"))
                            )
                    )
            )
            // Message area
            .child(
                div()
                    .flex_1()
                    .overflow_y_scroll()
                    .p(px(12.0))
                    .flex()
                    .flex_col()
                    .gap(px(8.0))
                    .children(messages.iter().map(|msg| {
                        let is_user = msg.role == MessageRole::User;
                        div()
                            .flex()
                            .when(is_user, |d| d.justify_end())
                            .child(
                                div()
                                    .max_w(px(size.width() * 0.75))
                                    .px(px(12.0))
                                    .py(px(8.0))
                                    .rounded(px(12.0))
                                    .text_sm()
                                    .child(SharedString::from(msg.content.clone()))
                            )
                    }))
            )
            // Input area
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .p(px(12.0))
                    .border_t_1()
                    .child(
                        div()
                            .flex_1()
                            .px(px(12.0))
                            .py(px(8.0))
                            .rounded(px(8.0))
                            .border_1()
                            .text_sm()
                            .child(SharedString::from("Type a message..."))
                    )
                    .child(
                        div()
                            .cursor_pointer()
                            .px(px(12.0))
                            .py(px(8.0))
                            .rounded(px(8.0))
                            .text_sm()
                            .child(SharedString::from("Send"))
                    )
            )
    }
}
