//! 🎬 VIDEO — Decoding & Playback Panel
//!
//! Stack: symphonia (demux) + rav1d (AV1) + openh264 (H.264)
//! Pipeline: Decode frames → RGBA buffer → paint as GPUI custom Element texture

use gpui::*;

pub struct VideoPanel {
    status: String,
    supported_formats: Vec<&'static str>,
}

impl VideoPanel {
    pub fn new() -> Self {
        Self {
            status: "Ready".to_string(),
            supported_formats: vec![
                "AV1 (rav1d — pure Rust)",
                "H.264 (openh264 — static link)",
                "VP8/VP9 (vpx-rs — static link)",
                "MP4/MKV/WebM (symphonia — pure Rust)",
                "AV1 encode (rav1e — pure Rust)",
                "MP4 mux (muxide — pure Rust)",
                "Fallback (ffmpeg-sidecar — bundled)",
            ],
        }
    }

    pub fn render_card(&self) -> impl IntoElement {
        let formats = self
            .supported_formats
            .iter()
            .map(|f| {
                div()
                    .text_size(px(11.0))
                    .text_color(rgb(0xBBBBBB))
                    .child(format!("  ✅ {}", f))
            })
            .collect::<Vec<_>>();

        card(
            "🎬 Video",
            &self.status,
            rgb(0xe74c3c),
            "Decode frames → RGBA → GPUI texture",
            formats,
        )
    }
}

// ────────────────────────────────────────────────────────────
// Shared card builder used by all panels
// ────────────────────────────────────────────────────────────
pub fn card(
    title: &str,
    status: &str,
    accent: Rgba,
    description: &str,
    items: Vec<Div>,
) -> Div {
    let mut container = div()
        .flex_1()
        .min_w(px(250.0))
        .bg(rgb(0x16213e))
        .rounded(px(12.0))
        .border_1()
        .border_color(rgb(0x2a2a4a))
        .p(px(16.0))
        .flex()
        .flex_col()
        .gap(px(8.0))
        .child(
            div()
                .flex()
                .flex_row()
                .justify_between()
                .child(
                    div()
                        .text_size(px(18.0))
                        .font_weight(FontWeight::BOLD)
                        .text_color(rgb(0xFFFFFF))
                        .child(title.to_string()),
                )
                .child(
                    div()
                        .text_size(px(11.0))
                        .text_color(accent)
                        .bg(rgba(accent.r, accent.g, accent.b, 0.15))
                        .rounded(px(4.0))
                        .px(px(8.0))
                        .py(px(2.0))
                        .child(status.to_string()),
                ),
        )
        .child(
            div()
                .text_size(px(12.0))
                .text_color(rgb(0x888888))
                .child(description.to_string()),
        );

    for item in items {
        container = container.child(item);
    }

    container
}

/// Helper to create a rgba from components (used by accent badge backgrounds)
fn rgba(r: f32, g: f32, b: f32, a: f32) -> Rgba {
    Rgba { r, g, b, a }
}
