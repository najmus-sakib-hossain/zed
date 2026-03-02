//! 📊 Charts / Plots Panel
//!
//! Stack: plotters + plotters-bitmap + resvg
//! Supports: bar, line, scatter, histogram, heatmap, 3D plots

use gpui::*;
use super::video::card;

pub struct ChartsPanel {
    status: String,
    features: Vec<&'static str>,
}

impl ChartsPanel {
    pub fn new() -> Self {
        Self {
            status: "Ready".to_string(),
            features: vec![
                "plotters — full charting library (pure Rust)",
                "plotters-bitmap — charts → in-memory bitmap",
                "plotters-svg — charts → SVG output",
                "resvg — SVG → bitmap (pure Rust)",
                "Bar, line, scatter, histogram, heatmap, 3D",
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
                    .child(format!("  📊 {}", f))
            })
            .collect::<Vec<_>>();

        card(
            "📊 Charts",
            &self.status,
            rgb(0xf39c12),
            "Pure Rust charting — render to bitmap → GPUI",
            items,
        )
    }
}
