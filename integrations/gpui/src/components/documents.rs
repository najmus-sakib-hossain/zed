//! 📝 Documents Panel
//!
//! Stack: docx-rs (DOCX) + calamine (Excel/ODS)
//! Pipeline: Parse → extract text/styles → GPUI layout  OR  → genpdf → hayro → bitmap

use gpui::*;
use super::video::card;

pub struct DocumentsPanel {
    status: String,
    features: Vec<&'static str>,
}

impl DocumentsPanel {
    pub fn new() -> Self {
        Self {
            status: "Ready".to_string(),
            features: vec![
                "docx-rs — Read/Write DOCX (pure Rust)",
                "calamine — Excel .xlsx, .xls, .ods (pure Rust)",
                "Pipeline A: parse → GPUI layout elements",
                "Pipeline B: parse → genpdf → hayro → bitmap",
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
                    .child(format!("  📝 {}", f))
            })
            .collect::<Vec<_>>();

        card(
            "📝 Documents",
            &self.status,
            rgb(0x9b59b6),
            "DOCX + Excel/ODS — all pure Rust",
            items,
        )
    }
}
