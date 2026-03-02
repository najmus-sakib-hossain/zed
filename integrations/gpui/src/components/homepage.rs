//! Homepage — grid layout showcasing all 7 component panels.

use gpui::*;

use super::{
    video::VideoPanel,
    three_d::ThreeDPanel,
    audio::AudioPanel,
    pdf::PdfPanel,
    documents::DocumentsPanel,
    latex::LatexPanel,
    charts::ChartsPanel,
};

pub struct Homepage {
    video: VideoPanel,
    three_d: ThreeDPanel,
    audio: AudioPanel,
    pdf: PdfPanel,
    documents: DocumentsPanel,
    latex: LatexPanel,
    charts: ChartsPanel,
}

impl Homepage {
    pub fn new() -> Self {
        Self {
            video: VideoPanel::new(),
            three_d: ThreeDPanel::new(),
            audio: AudioPanel::new(),
            pdf: PdfPanel::new(),
            documents: DocumentsPanel::new(),
            latex: LatexPanel::new(),
            charts: ChartsPanel::new(),
        }
    }
}

impl Render for Homepage {
    fn render(&mut self, _window: &mut Window, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let header = div()
            .flex()
            .flex_col()
            .items_center()
            .py(px(24.0))
            .gap(px(8.0))
            .child(
                div()
                    .text_size(px(32.0))
                    .font_weight(FontWeight::EXTRA_BOLD)
                    .text_color(rgb(0xFFFFFF))
                    .child("🏗️ GPUI Desktop — Single Binary Multimedia"),
            )
            .child(
                div()
                    .text_size(px(14.0))
                    .text_color(rgb(0x999999))
                    .child("Ship ONE binary. Zero external dependencies. Pure Rust. 🚀"),
            );

        let row1 = div()
            .flex()
            .flex_row()
            .gap(px(16.0))
            .px(px(24.0))
            .child(self.video.render_card())
            .child(self.three_d.render_card())
            .child(self.audio.render_card())
            .child(self.pdf.render_card());

        let row2 = div()
            .flex()
            .flex_row()
            .gap(px(16.0))
            .px(px(24.0))
            .child(self.documents.render_card())
            .child(self.latex.render_card())
            .child(self.charts.render_card());

        div()
            .size_full()
            .bg(rgb(0x1a1a2e))
            .flex()
            .flex_col()
            .gap(px(16.0))
            .overflow_y_scroll()
            .child(header)
            .child(row1)
            .child(row2)
            .child(
                div()
                    .flex()
                    .justify_center()
                    .py(px(16.0))
                    .child(
                        div()
                            .text_size(px(12.0))
                            .text_color(rgb(0x666666))
                            .child("cargo build --release → one binary → ship it. Done. 🎉"),
                    ),
            )
    }
}
