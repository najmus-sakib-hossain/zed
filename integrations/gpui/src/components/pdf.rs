//! 📄 PDF Panel
//!
//! Stack: hayro (render) + genpdf (generate) + printpdf (low-level)
//! Pipeline: PDF → hayro → PNG/SVG bitmap → GPUI element

use gpui::*;
use super::video::card;

pub struct PdfPanel {
    status: String,
    features: Vec<&'static str>,
}

impl PdfPanel {
    pub fn new() -> Self {
        Self {
            status: "Ready".to_string(),
            features: vec![
                "hayro — PDF → PNG/SVG (pure Rust, no PDFium)",
                "pdfium-render — static link alternative",
                "genpdf — PDF generation (pure Rust)",
                "printpdf — low-level PDF creation",
                "100% pure Rust. No C deps.",
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
                    .child(format!("  📄 {}", f))
            })
            .collect::<Vec<_>>();

        card(
            "📄 PDF",
            &self.status,
            rgb(0xe67e22),
            "Render + generate PDFs — all pure Rust",
            items,
        )
    }
}
