//! FlowBarUi — Persistent bottom-center pill widget for voice interaction.
//!
//! Part 24: States — idle, listening, transcribing, post-processing, result, speaking.
//! Expanding pill animation, waveform visualisation stub, push-to-talk support.

use gpui::{
    div, prelude::*, px, FocusHandle, Focusable, ParentElement, Render, SharedString,
    Styled, ViewContext,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlowBarState {
    Idle,
    Listening,
    Transcribing,
    PostProcessing,
    Result,
    Speaking,
}

impl FlowBarState {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Idle => "Tap to speak",
            Self::Listening => "Listening...",
            Self::Transcribing => "Transcribing...",
            Self::PostProcessing => "Thinking...",
            Self::Result => "Done",
            Self::Speaking => "Speaking...",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            Self::Idle => "🎤",
            Self::Listening => "🔴",
            Self::Transcribing => "📝",
            Self::PostProcessing => "⚙️",
            Self::Result => "✅",
            Self::Speaking => "🔊",
        }
    }

    pub fn is_active(&self) -> bool {
        !matches!(self, Self::Idle | Self::Result)
    }

    /// Pill width at each state (expanded when active).
    pub fn pill_width(&self) -> f32 {
        match self {
            Self::Idle => 160.0,
            Self::Listening => 280.0,
            Self::Transcribing => 240.0,
            Self::PostProcessing => 220.0,
            Self::Result => 320.0,
            Self::Speaking => 260.0,
        }
    }
}

#[derive(Debug, Clone)]
pub enum FlowBarEvent {
    StateChanged(FlowBarState),
    PushToTalkStarted,
    PushToTalkEnded,
    ResultDismissed,
    ResultCopied(String),
}

pub struct FlowBarUi {
    focus_handle: FocusHandle,
    state: FlowBarState,
    transcript_text: String,
    result_text: String,
    /// Waveform amplitude samples for visualisation (0.0..1.0).
    waveform_samples: Vec<f32>,
    push_to_talk_active: bool,
}

impl FlowBarUi {
    pub fn new(cx: &mut ViewContext<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            state: FlowBarState::Idle,
            transcript_text: String::new(),
            result_text: String::new(),
            waveform_samples: Vec::new(),
            push_to_talk_active: false,
        }
    }

    pub fn state(&self) -> FlowBarState {
        self.state
    }

    pub fn set_state(&mut self, state: FlowBarState, cx: &mut ViewContext<Self>) {
        self.state = state;
        cx.emit(FlowBarEvent::StateChanged(state));
        cx.notify();
    }

    pub fn set_transcript(&mut self, text: String, cx: &mut ViewContext<Self>) {
        self.transcript_text = text;
        cx.notify();
    }

    pub fn set_result(&mut self, text: String, cx: &mut ViewContext<Self>) {
        self.result_text = text;
        self.state = FlowBarState::Result;
        cx.notify();
    }

    pub fn update_waveform(&mut self, samples: Vec<f32>, cx: &mut ViewContext<Self>) {
        self.waveform_samples = samples;
        cx.notify();
    }

    pub fn start_push_to_talk(&mut self, cx: &mut ViewContext<Self>) {
        self.push_to_talk_active = true;
        self.set_state(FlowBarState::Listening, cx);
        cx.emit(FlowBarEvent::PushToTalkStarted);
    }

    pub fn end_push_to_talk(&mut self, cx: &mut ViewContext<Self>) {
        self.push_to_talk_active = false;
        self.set_state(FlowBarState::Transcribing, cx);
        cx.emit(FlowBarEvent::PushToTalkEnded);
    }

    pub fn dismiss_result(&mut self, cx: &mut ViewContext<Self>) {
        self.result_text.clear();
        self.transcript_text.clear();
        self.set_state(FlowBarState::Idle, cx);
        cx.emit(FlowBarEvent::ResultDismissed);
    }

    fn render_waveform(&self) -> gpui::Div {
        let bar_count = 24;
        let samples = &self.waveform_samples;

        div()
            .flex()
            .items_end()
            .gap(px(2.0))
            .h(px(32.0))
            .children((0..bar_count).map(|i| {
                let amplitude = samples
                    .get(i)
                    .copied()
                    .unwrap_or(0.1);
                let bar_height = (amplitude * 28.0).max(4.0);
                div()
                    .w(px(3.0))
                    .h(px(bar_height))
                    .rounded(px(1.5))
            }))
    }
}

impl gpui::EventEmitter<FlowBarEvent> for FlowBarUi {}

impl Focusable for FlowBarUi {
    fn focus_handle(&self, _cx: &gpui::App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for FlowBarUi {
    fn render(&mut self, _cx: &mut ViewContext<Self>) -> impl IntoElement {
        let state = self.state;
        let transcript = self.transcript_text.clone();
        let result = self.result_text.clone();

        div()
            .flex()
            .justify_center()
            .w_full()
            .pb(px(16.0))
            // Pill container
            .child(
                div()
                    .w(px(state.pill_width()))
                    .rounded(px(24.0))
                    .border_1()
                    .px(px(16.0))
                    .py(px(10.0))
                    .flex()
                    .items_center()
                    .gap(px(10.0))
                    .cursor_pointer()
                    // State icon
                    .child(
                        div()
                            .text_size(px(18.0))
                            .child(SharedString::from(state.icon()))
                    )
                    // Content depends on state
                    .child(
                        div()
                            .flex_1()
                            .flex()
                            .flex_col()
                            .gap(px(2.0))
                            // State label
                            .child(
                                div()
                                    .text_xs()
                                    .font_weight(gpui::FontWeight::MEDIUM)
                                    .child(SharedString::from(state.label()))
                            )
                            // Waveform when listening
                            .when(state == FlowBarState::Listening, |d| {
                                d.child(self.render_waveform())
                            })
                            // Transcript when transcribing
                            .when(
                                state == FlowBarState::Transcribing && !transcript.is_empty(),
                                |d| {
                                    d.child(
                                        div()
                                            .text_xs()
                                            .child(SharedString::from(transcript.clone()))
                                    )
                                }
                            )
                            // Result text
                            .when(
                                state == FlowBarState::Result && !result.is_empty(),
                                |d| {
                                    d.child(
                                        div()
                                            .text_sm()
                                            .child(SharedString::from(
                                                if result.len() > 100 {
                                                    format!("{}...", &result[..100])
                                                } else {
                                                    result.clone()
                                                }
                                            ))
                                    )
                                }
                            )
                    )
                    // Dismiss button on result state
                    .when(state == FlowBarState::Result, |d| {
                        d.child(
                            div()
                                .cursor_pointer()
                                .text_xs()
                                .child(SharedString::from("✕"))
                        )
                    })
            )
    }
}
