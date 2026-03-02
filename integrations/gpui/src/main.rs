//! 🏗️ GPUI Single-Binary Desktop App
//!
//! A pure-Rust multimedia powerhouse showcasing:
//!   🎬 Video   🧊 3D   🔊 Audio   📄 PDF   📝 Docs   📐 LaTeX   📊 Charts
//!
//! Rule: The user downloads ONE binary. Nothing else. Ever.

use gpui::*;
use gpui_component::StyledExt;

mod bitmap_utils;
mod remote;
mod video_view;
mod three_d_view;
mod audio_view;
mod pdf_view;
mod doc_view;
mod latex_view;
mod chart_view;

use audio_view::AudioPlayerView;
use chart_view::ChartView;
use doc_view::DocView;
use latex_view::LatexView;
use pdf_view::PdfView;
use three_d_view::ThreeDView;
use video_view::VideoPlayerView;

fn main() {
    let app = Application::new().with_assets(gpui_component_assets::Assets);
    app.run(move |cx| {
        gpui_component::init(cx);
        cx.spawn(async move |cx| {
            cx.open_window(WindowOptions::default(), |window, cx| {
                let view = cx.new(|cx| AppRoot::new(window, cx));
                cx.new(|cx| gpui_component::Root::new(view, window, cx))
            })?;
            Ok::<_, anyhow::Error>(())
        })
        .detach();
    });
}

/// Top-level application view hosting all media components
struct AppRoot {
    current_tab: Tab,
    // Persistent view entities — created once, kept alive the whole session
    video_view:  Entity<VideoPlayerView>,
    three_d_view: Entity<ThreeDView>,
    audio_view:  Entity<AudioPlayerView>,
    pdf_view:    Entity<PdfView>,
    doc_view:    Entity<DocView>,
    latex_view:  Entity<LatexView>,
    chart_view:  Entity<ChartView>,
}

#[derive(Clone, Copy, PartialEq)]
enum Tab {
    Video,
    ThreeD,
    Audio,
    Pdf,
    Doc,
    Latex,
    Chart,
}

impl AppRoot {
    fn new(window: &Window, cx: &mut Context<Self>) -> Self {
        Self {
            current_tab: Tab::Chart,
            video_view:   cx.new(|cx| VideoPlayerView::new(
                // Short (15 s) H.264 / AAC clip from Google's public sample bucket
                "https://commondatastorage.googleapis.com/gtv-videos-bucket/sample/ForBiggerEscapes.mp4",
                window, cx,
            )),
            three_d_view: cx.new(|cx| ThreeDView::new(window, cx)),
            audio_view:   cx.new(|cx| AudioPlayerView::new(
                "https://www.soundhelix.com/examples/mp3/SoundHelix-Song-1.mp3",
                window, cx,
            )),
            pdf_view:     cx.new(|cx| PdfView::new(
                // Reliable W3C test PDF (tiny, always accessible)
                "https://www.w3.org/WAI/ER/tests/xhtml/testfiles/resources/pdf/dummy.pdf",
                window, cx,
            )),
            doc_view:     cx.new(|cx| DocView::new(
                "https://calibre-ebook.com/downloads/demos/demo.docx",
                window, cx,
            )),
            latex_view:   cx.new(|cx| LatexView::new(window, cx)),
            chart_view:   cx.new(|cx| ChartView::new(window, cx)),
        }
    }
}

impl Render for AppRoot {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let tab = self.current_tab;

        div()
            .size_full()
            .v_flex()
            .bg(rgb(0x1e1e2e))
            .child(
                // ── Tab bar ──
                div()
                    .h_flex()
                    .gap_1()
                    .p_2()
                    .bg(rgb(0x181825))
                    .children(
                        [
                            ("🎬 Video",  Tab::Video),
                            ("🧊 3D",     Tab::ThreeD),
                            ("🔊 Audio",  Tab::Audio),
                            ("📄 PDF",    Tab::Pdf),
                            ("📝 Docs",   Tab::Doc),
                            ("📐 LaTeX",  Tab::Latex),
                            ("📊 Chart",  Tab::Chart),
                        ]
                        .into_iter()
                        .map(|(label, t)| {
                            let is_active = tab == t;
                            let switch = cx.listener(move |this, _: &MouseDownEvent, _win, cx| {
                                this.current_tab = t;
                                cx.notify();
                            });
                            div()
                                .px_4()
                                .py_2()
                                .rounded(px(6.))
                                .cursor_pointer()
                                .bg(if is_active { rgb(0x585b70) } else { rgb(0x313244) })
                                .text_color(rgb(0xcdd6f4))
                                .child(label)
                                .on_mouse_down(MouseButton::Left, switch)
                        }),
                    ),
            )
            .child(
                // ── Content area ──
                div()
                    .flex_1()
                    .overflow_hidden()
                    .child(match self.current_tab {
                        Tab::Video  => AnyView::from(self.video_view.clone()),
                        Tab::ThreeD => AnyView::from(self.three_d_view.clone()),
                        Tab::Audio  => AnyView::from(self.audio_view.clone()),
                        Tab::Pdf    => AnyView::from(self.pdf_view.clone()),
                        Tab::Doc    => AnyView::from(self.doc_view.clone()),
                        Tab::Latex  => AnyView::from(self.latex_view.clone()),
                        Tab::Chart  => AnyView::from(self.chart_view.clone()),
                    }),
            )
    }
}
