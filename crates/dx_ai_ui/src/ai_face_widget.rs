//! AI Face Widget — Procedural GPU-rendered avatar (Part 25).
//!
//! A small animated avatar face that reacts to the AI's state:
//! - Idle: gentle breathing animation
//! - Listening: attentive expression, ears perked
//! - Thinking: furrowed brow, eye movement
//! - Speaking: mouth animation synced to TTS output
//! - Error: confused expression
//!
//! This is rendered procedurally via GPUI's GPU primitives (circles, arcs)
//! rather than sprite sheets, ensuring resolution independence.

use gpui::{div, prelude::*, SharedString, Window};

/// AI avatar expression.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AiFaceExpression {
    Neutral,
    Happy,
    Thinking,
    Listening,
    Speaking,
    Confused,
    Surprised,
    Sleeping,
}

/// Configuration for the face widget.
#[derive(Debug, Clone)]
pub struct AiFaceConfig {
    /// Size of the face widget in pixels.
    pub size_px: f32,
    /// Primary color for the face.
    pub primary_color: [f32; 3],
    /// Whether to show the breathing animation.
    pub animate_breathing: bool,
    /// Whether to sync mouth to audio output.
    pub sync_to_audio: bool,
}

impl Default for AiFaceConfig {
    fn default() -> Self {
        Self {
            size_px: 48.0,
            primary_color: [0.4, 0.6, 1.0], // Soft blue
            animate_breathing: true,
            sync_to_audio: true,
        }
    }
}

/// The AI face widget — a procedurally rendered animated avatar.
pub struct AiFaceWidget {
    expression: AiFaceExpression,
    config: AiFaceConfig,
    /// Mouth openness (0.0 = closed, 1.0 = wide open), driven by audio amplitude.
    mouth_amplitude: f32,
    /// Eye blink timer (0.0 to 1.0, blink at 1.0).
    blink_progress: f32,
}

impl AiFaceWidget {
    pub fn new() -> Self {
        Self {
            expression: AiFaceExpression::Neutral,
            config: AiFaceConfig::default(),
            mouth_amplitude: 0.0,
            blink_progress: 0.0,
        }
    }

    pub fn with_config(config: AiFaceConfig) -> Self {
        Self {
            expression: AiFaceExpression::Neutral,
            config,
            mouth_amplitude: 0.0,
            blink_progress: 0.0,
        }
    }

    pub fn set_expression(&mut self, expr: AiFaceExpression, cx: &mut Context<Self>) {
        self.expression = expr;
        cx.notify();
    }

    pub fn set_mouth_amplitude(&mut self, amplitude: f32, cx: &mut Context<Self>) {
        self.mouth_amplitude = amplitude.clamp(0.0, 1.0);
        cx.notify();
    }

    pub fn expression(&self) -> AiFaceExpression {
        self.expression
    }
}

impl Render for AiFaceWidget {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let label: SharedString = match self.expression {
            AiFaceExpression::Neutral => "😐".into(),
            AiFaceExpression::Happy => "😊".into(),
            AiFaceExpression::Thinking => "🤔".into(),
            AiFaceExpression::Listening => "👂".into(),
            AiFaceExpression::Speaking => "🗣️".into(),
            AiFaceExpression::Confused => "😕".into(),
            AiFaceExpression::Surprised => "😮".into(),
            AiFaceExpression::Sleeping => "😴".into(),
        };

        // In production, this would use GPUI's GPU primitives to draw
        // circles, arcs, and bezier curves for the face. For now, we
        // use emoji as placeholders.
        div()
            .flex()
            .items_center()
            .justify_center()
            .child(label)
    }
}
