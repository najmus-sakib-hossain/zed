//! 📄 PDF Viewer Component
//!
//! Strategy: `hayro` renders PDF pages to PNG bitmaps → GPUI image element.
//! 100% pure Rust. No PDFium. No C deps.

use gpui::*;
use gpui_component::StyledExt;
use image::RgbaImage;
use std::sync::{Arc, Mutex};

use crate::bitmap_utils;

pub struct PdfView {
    pages: Arc<Mutex<Vec<RgbaImage>>>,
    current_page: usize,
    total_pages: usize,
    /// None = still loading, Some(Ok(n)) = n pages loaded, Some(Err(msg)) = failed
    status: Arc<Mutex<Option<Result<usize, String>>>>,
    file_path: String,
}

impl PdfView {
    pub fn new(file_path: &str, _window: &Window, cx: &mut Context<Self>) -> Self {
        let pages = Arc::new(Mutex::new(Vec::new()));
        let pages_clone = pages.clone();
        let path = file_path.to_string();

        let status: Arc<Mutex<Option<Result<usize, String>>>> = Arc::new(Mutex::new(None));
        let status_clone = status.clone();

        // Render PDF pages in background
        cx.spawn(async move |this: WeakEntity<PdfView>, cx| {
            let result = smol::unblock(move || {
                let data = crate::remote::resolve(&path)
                    .map_err(|e| anyhow::anyhow!("Fetch failed: {e}"))?;
                if data.is_empty() {
                    return Err(anyhow::anyhow!("Empty response — check the URL or file path"));
                }
                render_pdf_pages(data)
            }).await;

            match result {
                Ok(rendered_pages) => {
                    let count = rendered_pages.len();
                    *pages_clone.lock().unwrap() = rendered_pages;
                    *status_clone.lock().unwrap() = Some(Ok(count));
                    cx.update(|cx| {
                        this.update(cx, |view, cx: &mut Context<PdfView>| {
                            view.total_pages = count;
                            cx.notify();
                        })
                    }).ok();
                }
                Err(e) => {
                    *status_clone.lock().unwrap() = Some(Err(e.to_string()));
                    cx.update(|cx| {
                        this.update(cx, |_, cx: &mut Context<PdfView>| cx.notify())
                    }).ok();
                }
            }
        })
        .detach();

        Self {
            pages,
            current_page: 0,
            total_pages: 0,
            status,
            file_path: file_path.to_string(),
        }
    }

    fn next_page(&mut self, cx: &mut Context<Self>) {
        if self.current_page + 1 < self.total_pages {
            self.current_page += 1;
            cx.notify();
        }
    }

    fn prev_page(&mut self, cx: &mut Context<Self>) {
        if self.current_page > 0 {
            self.current_page -= 1;
            cx.notify();
        }
    }
}

impl Render for PdfView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let pages = self.pages.lock().unwrap();
        let status = self.status.lock().unwrap().clone();
        let prev = cx.listener(|this, _: &MouseDownEvent, _, cx| this.prev_page(cx));
        let next = cx.listener(|this, _: &MouseDownEvent, _, cx| this.next_page(cx));

        let (status_text, status_color): (String, Hsla) = match &status {
            None => ("⏳ Fetching & rendering…".into(), rgb(0x89b4fa).into()),
            Some(Ok(n)) => (format!("✅ {n} pages rendered"), rgb(0xa6e3a1).into()),
            Some(Err(e)) => (format!("❌ {e}"), rgb(0xf38ba8).into()),
        };

        div()
            .v_flex()
            .gap_3()
            .items_center()
            .p_4()
            .child(div().text_color(rgb(0xcdd6f4)).text_xl().child("📄 PDF Viewer (hayro — pure Rust)"))
            .child(div().text_color(status_color).text_sm().child(status_text))
            .child(
                div()
                    .text_color(rgb(0x6c7086))
                    .text_sm()
                    .child(self.file_path.clone()),
            )
            .child(
                // PDF page display
                div()
                    .w(px(620.))
                    .min_h(px(800.))
                    .bg(rgb(0xffffff))
                    .rounded(px(4.))
                    .shadow_lg()
                    .overflow_hidden()
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(if let Some(page_img) = pages.get(self.current_page) {
                        let source = bitmap_utils::rgba_to_gpui_image(page_img);
                        div().child(
                            img(source)
                                .w(px(600.))
                                .object_fit(ObjectFit::Contain),
                        )
                    } else if let Some(Err(_)) = &status {
                        div().text_color(rgb(0xf38ba8)).child("Failed to render — see error above")
                    } else {
                        div().text_color(rgb(0x89b4fa)).child("⏳ Rendering pages…")
                    }),
            )
            // Page navigation
            .child(
                div()
                    .h_flex()
                    .gap_3()
                    .items_center()
                    .child(
                        div()
                            .px_3()
                            .py_2()
                            .bg(rgb(0x585b70))
                            .text_color(rgb(0xcdd6f4))
                            .rounded(px(6.))
                            .cursor_pointer()
                            .child("◀ Prev")
                            .on_mouse_down(MouseButton::Left, prev),
                    )
                    .child(
                        div()
                            .text_color(rgb(0xa6adc8))
                            .child(format!(
                                "Page {} of {}",
                                self.current_page + 1,
                                self.total_pages
                            )),
                    )
                    .child(
                        div()
                            .px_3()
                            .py_2()
                            .bg(rgb(0x585b70))
                            .text_color(rgb(0xcdd6f4))
                            .rounded(px(6.))
                            .cursor_pointer()
                            .child("Next ▶")
                            .on_mouse_down(MouseButton::Left, next),
                    ),
            )
    }
}

/// Render all pages of a PDF to RGBA images using hayro (pure Rust)
fn render_pdf_pages(data: Vec<u8>) -> anyhow::Result<Vec<RgbaImage>> {
    let pdf = hayro::Pdf::new(std::sync::Arc::new(data))
        .map_err(|e| anyhow::anyhow!("Failed to parse PDF: {:?}", e))?;

    let page_count = pdf.pages().len();
    if page_count == 0 {
        return Err(anyhow::anyhow!("PDF has 0 pages"));
    }

    let mut pages = Vec::with_capacity(page_count);
    let interp = hayro::InterpreterSettings::default();
    // Render at 1.5× scale for sharper text on HiDPI displays
    let render_settings = hayro::RenderSettings { x_scale: 1.5, y_scale: 1.5, ..Default::default() };

    for (i, page) in pdf.pages().iter().enumerate() {
        let pixmap = hayro::render(page, &interp, &render_settings);
        let width  = pixmap.width()  as u32;
        let height = pixmap.height() as u32;

        // hayro returns premultiplied RGBA8.  Un-premultiply to straight RGBA8
        // before storing in image::RgbaImage, which assumes straight alpha.
        let premul = pixmap.take_u8(); // RGBA, premultiplied
        let straight: Vec<u8> = premul
            .chunks_exact(4)
            .flat_map(|px| {
                let (r, g, b, a) = (px[0], px[1], px[2], px[3]);
                if a == 0 {
                    [0u8, 0, 0, 0]
                } else {
                    let f = 255.0 / a as f32;
                    [
                        (r as f32 * f).min(255.0) as u8,
                        (g as f32 * f).min(255.0) as u8,
                        (b as f32 * f).min(255.0) as u8,
                        a,
                    ]
                }
            })
            .collect();

        match RgbaImage::from_raw(width, height, straight) {
            Some(img) => pages.push(img),
            None => log::warn!("Page {i}: RgbaImage::from_raw failed (w={width} h={height})"),
        }
    }

    if pages.is_empty() {
        return Err(anyhow::anyhow!("Rendered 0 pages from {page_count}-page PDF"));
    }
    Ok(pages)
}
