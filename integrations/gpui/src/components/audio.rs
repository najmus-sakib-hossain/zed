//! 🔊 Audio Panel
//!
//! Stack: rodio (playback) + symphonia (decode) + cpal (low-level I/O)
//! Talks directly to OS audio APIs — CoreAudio / WASAPI / ALSA

use gpui::*;
use super::video::card;

pub struct AudioPanel {
    status: String,
    features: Vec<&'static str>,
}

impl AudioPanel {
    pub fn new() -> Self {
        Self {
            status: "Ready".to_string(),
            features: vec![
                "rodio — high-level playback (pure Rust)",
                "symphonia — AAC, FLAC, MP3, OGG, WAV, Vorbis",
                "cpal — low-level I/O to OS audio APIs",
                "macOS: CoreAudio ✅",
                "Linux: ALSA ✅",
                "Windows: WASAPI ✅",
            ],
        }
    }

    pub fn render_card(&self) -> impl IntoElement {
        let items = self
            .features
            .iter()
            .map(|f| {
                div()
                    .text_size(px(11.0))
                    .text_color(rgb(0xBBBBBB))
                    .child(format!("  🔊 {}", f))
            })
            .collect::<Vec<_>>();

        card(
            "🔊 Audio",
            &self.status,
            rgb(0x2ecc71),
            "OS-native audio APIs — zero user dependencies",
            items,
        )
    }
}
