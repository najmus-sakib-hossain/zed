//! AiFaceWidget — Procedural GPU-rendered avatar with eye tracking,
//! blinking, mouth animation, and glow ring.
//!
//! Part 25: Visual AI face that animates in response to voice activity,
//! processing state, and user gaze direction.

use gpui::{
    div, prelude::*, px, FocusHandle, Focusable, ParentElement, Render, SharedString,
    Styled, ViewContext,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

impl Vec2 {
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    pub fn zero() -> Self {
        Self { x: 0.0, y: 0.0 }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FaceExpression {
    Neutral,
    Happy,
    Thinking,
    Speaking,
    Listening,
    Surprised,
    Focused,
}

impl FaceExpression {
    pub fn eye_emoji(&self) -> &'static str {
        match self {
            Self::Neutral => "😐",
            Self::Happy => "😊",
            Self::Thinking => "🤔",
            Self::Speaking => "🗣️",
            Self::Listening => "👂",
            Self::Surprised => "😮",
            Self::Focused => "🧐",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FaceAnimationState {
    /// Eye gaze direction, normalised -1..1 on each axis.
    pub gaze_direction: Vec2,
    /// Blink progress 0.0 (open) to 1.0 (closed).
    pub blink_progress: f32,
    /// Mouth openness 0.0 (closed) to 1.0 (fully open).
    pub mouth_openness: f32,
    /// Glow ring intensity 0.0..1.0.
    pub glow_intensity: f32,
    /// Current expression.
    pub expression: FaceExpression,
}

impl Default for FaceAnimationState {
    fn default() -> Self {
        Self {
            gaze_direction: Vec2::zero(),
            blink_progress: 0.0,
            mouth_openness: 0.0,
            glow_intensity: 0.3,
            expression: FaceExpression::Neutral,
        }
    }
}

#[derive(Debug, Clone)]
pub enum AiFaceEvent {
    ExpressionChanged(FaceExpression),
    GazeChanged(Vec2),
}

pub struct AiFaceWidget {
    focus_handle: FocusHandle,
    animation: FaceAnimationState,
    size: f32,
    /// Whether to track user's cursor position for eye gaze.
    pub track_cursor: bool,
}

impl AiFaceWidget {
    pub fn new(cx: &mut ViewContext<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            animation: FaceAnimationState::default(),
            size: 120.0,
            track_cursor: true,
        }
    }

    pub fn with_size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }

    pub fn set_expression(&mut self, expression: FaceExpression, cx: &mut ViewContext<Self>) {
        self.animation.expression = expression;
        cx.emit(AiFaceEvent::ExpressionChanged(expression));
        cx.notify();
    }

    pub fn set_gaze(&mut self, direction: Vec2, cx: &mut ViewContext<Self>) {
        self.animation.gaze_direction = direction;
        cx.emit(AiFaceEvent::GazeChanged(direction));
        cx.notify();
    }

    pub fn set_blink(&mut self, progress: f32, cx: &mut ViewContext<Self>) {
        self.animation.blink_progress = progress.clamp(0.0, 1.0);
        cx.notify();
    }

    pub fn set_mouth_openness(&mut self, openness: f32, cx: &mut ViewContext<Self>) {
        self.animation.mouth_openness = openness.clamp(0.0, 1.0);
        cx.notify();
    }

    pub fn set_glow_intensity(&mut self, intensity: f32, cx: &mut ViewContext<Self>) {
        self.animation.glow_intensity = intensity.clamp(0.0, 1.0);
        cx.notify();
    }

    pub fn set_speaking(&mut self, mouth_openness: f32, cx: &mut ViewContext<Self>) {
        self.animation.expression = FaceExpression::Speaking;
        self.animation.mouth_openness = mouth_openness.clamp(0.0, 1.0);
        self.animation.glow_intensity = 0.6 + mouth_openness * 0.4;
        cx.notify();
    }

    pub fn set_listening(&mut self, cx: &mut ViewContext<Self>) {
        self.animation.expression = FaceExpression::Listening;
        self.animation.glow_intensity = 0.5;
        self.animation.mouth_openness = 0.0;
        cx.notify();
    }

    pub fn set_thinking(&mut self, cx: &mut ViewContext<Self>) {
        self.animation.expression = FaceExpression::Thinking;
        self.animation.glow_intensity = 0.4;
        self.animation.mouth_openness = 0.0;
        cx.notify();
    }

    pub fn animation_state(&self) -> &FaceAnimationState {
        &self.animation
    }
}

impl gpui::EventEmitter<AiFaceEvent> for AiFaceWidget {}

impl Focusable for AiFaceWidget {
    fn focus_handle(&self, _cx: &gpui::App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for AiFaceWidget {
    fn render(&mut self, _cx: &mut ViewContext<Self>) -> impl IntoElement {
        let anim = self.animation;
        let size = self.size;
        let eye_offset_x = anim.gaze_direction.x * size * 0.08;
        let eye_offset_y = anim.gaze_direction.y * size * 0.06;

        // Eye openness (inverted blink)
        let eye_height = (1.0 - anim.blink_progress) * (size * 0.18);
        let mouth_height = anim.mouth_openness * (size * 0.12);

        div()
            .w(px(size))
            .h(px(size))
            .rounded_full()
            .border_1()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap(px(size * 0.06))
            .overflow_hidden()
            // Glow ring (outer border effect via nested container)
            .child(
                div()
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap(px(size * 0.08))
                    // Eyes row
                    .child(
                        div()
                            .flex()
                            .gap(px(size * 0.2))
                            // Left eye
                            .child(
                                div()
                                    .w(px(size * 0.16))
                                    .h(px(eye_height.max(2.0)))
                                    .rounded_full()
                            )
                            // Right eye
                            .child(
                                div()
                                    .w(px(size * 0.16))
                                    .h(px(eye_height.max(2.0)))
                                    .rounded_full()
                            )
                    )
                    // Mouth
                    .child(
                        div()
                            .w(px(size * 0.25))
                            .h(px(mouth_height.max(3.0)))
                            .rounded(px(size * 0.04))
                    )
                    // Expression label (debug/fallback)
                    .child(
                        div()
                            .text_size(px(size * 0.12))
                            .child(SharedString::from(anim.expression.eye_emoji()))
                    )
            )
    }
}
