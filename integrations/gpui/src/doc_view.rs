//! 📝 Document Viewer Component
//!
//! Strategy: `docx-rs` parses DOCX → extracts text/styles → renders with GPUI div elements.
//! Pipeline: DOCX → parse → extract text/styles/images → custom GPUI layout elements.

use docx_rs::*;
use gpui::*;
use gpui_component::StyledExt;
use gpui_component::scroll::ScrollableElement;

pub struct DocView {
    paragraphs: Vec<DocParagraph>,
    file_path: String,
    loaded: bool,
    status: String,
}

#[derive(Clone)]
struct DocParagraph {
    text: String,
    is_heading: bool,
    is_bold: bool,
    font_size: f32,
}

impl DocView {
    pub fn new(file_path: &str, _window: &Window, cx: &mut Context<Self>) -> Self {
        let path = file_path.to_string();
        let view = Self {
            paragraphs: Vec::new(),
            file_path: path.clone(),
            loaded: false,
            status: if crate::remote::is_url(file_path) {
                "Downloading document...".into()
            } else if file_path.is_empty() {
                "No file loaded".into()
            } else {
                "Loading...".into()
            },
        };

        // Load & parse in background (handles both URLs and local paths)
        cx.spawn(async move |this: WeakEntity<DocView>, cx| {
            let result = smol::unblock(move || {
                let data = crate::remote::resolve(&path)?;
                if data.is_empty() { return Ok(Vec::new()); }
                parse_docx_bytes(&data)
            }).await;

            cx.update(|cx| {
                this.update(cx, |view, cx: &mut Context<DocView>| {
                    match result {
                        Ok(paras) => {
                            view.paragraphs = paras;
                            view.loaded = true;
                            view.status = format!("{} paragraphs loaded", view.paragraphs.len());
                        }
                        Err(e) => {
                            view.status = format!("Error: {e}");
                        }
                    }
                    cx.notify();
                })
            }).ok();
        })
        .detach();

        view
    }
}

impl Render for DocView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .v_flex()
            .gap_3()
            .p_4()
            .size_full()
            .child(
                div()
                    .text_color(rgb(0xcdd6f4))
                    .text_xl()
                    .child("📝 Document Viewer (docx-rs — pure Rust)"),
            )
            .child(
                // File info
                div()
                    .text_color(rgb(0x6c7086))
                    .text_sm()
                    .child(format!("Source: {}", self.file_path)),
            )
            .child(
                div()
                    .text_color(rgb(0x6c7086))
                    .text_sm()
                    .child(self.status.clone()),
            )
            .child(
                // Document content area
                div()
                    .v_flex()
                    .gap_2()
                    .p_6()
                    .w(px(700.))
                    .bg(rgb(0xffffff))
                    .rounded(px(8.))
                    .shadow_lg()
                    .overflow_y_scrollbar()
                    .max_h(px(600.))
                    .children(
                        self.paragraphs
                            .iter()
                            .map(|para| {
                                let mut el = div()
                                    .w_full()
                                    .text_color(rgb(0x1e1e2e));

                                if para.is_heading {
                                    el = el
                                        .text_xl()
                                        .font_weight(FontWeight::BOLD)
                                        .pb_2()
                                        .border_b_1()
                                        .border_color(rgb(0xcccccc));
                                } else if para.is_bold {
                                    el = el
                                        .text_sm()
                                        .font_weight(FontWeight::BOLD);
                                } else {
                                    el = el.text_sm();
                                    // font_size hint (visual only — GPUI uses text_sm/text_base etc)
                                    let _ = para.font_size; // acknowledged
                                }

                                el.child(para.text.clone())
                            })
                            .collect::<Vec<_>>(),
                    ),
            )
    }
}

/// Parse a DOCX byte slice and extract paragraphs
fn parse_docx_bytes(data: &[u8]) -> anyhow::Result<Vec<DocParagraph>> {
    let doc = docx_rs::read_docx(data)?;
    let mut paragraphs = Vec::new();

    for child in doc.document.children {
        if let DocumentChild::Paragraph(para) = child {
            let mut text = String::new();
            let mut is_bold = false;
            let mut is_heading = false;
            let mut font_size = 12.0;

            if let Some(ref style) = para.property.style
                && style.val.starts_with("Heading") {
                    is_heading = true;
                    font_size = 18.0;
            }

            for content in &para.children {
                if let ParagraphChild::Run(run) = content {
                    let rp = &run.run_property;
                    if rp.bold.is_some() {
                        is_bold = true;
                    }
                    for run_child in &run.children {
                        if let RunChild::Text(t) = run_child {
                            text.push_str(&t.text);
                        }
                    }
                }
            }

            if !text.is_empty() {
                paragraphs.push(DocParagraph {
                    text,
                    is_heading,
                    is_bold,
                    font_size,
                });
            }
        }
    }

    Ok(paragraphs)
}
