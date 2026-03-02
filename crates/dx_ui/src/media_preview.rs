//! MediaPreview — Image / Video / Audio / 3D / PDF preview panels.
//!
//! Parts 10-13 UI: Unified preview component that renders the appropriate
//! preview widget based on media type.

use gpui::{
    div, prelude::*, px, FocusHandle, Focusable, ParentElement, Render, SharedString,
    Styled, ViewContext,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaType {
    Image,
    Video,
    Audio,
    ThreeD,
    Pdf,
    Document,
}

impl MediaType {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Image => "Image",
            Self::Video => "Video",
            Self::Audio => "Audio",
            Self::ThreeD => "3D Model",
            Self::Pdf => "PDF",
            Self::Document => "Document",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            Self::Image => "🖼️",
            Self::Video => "🎬",
            Self::Audio => "🎵",
            Self::ThreeD => "🧊",
            Self::Pdf => "📄",
            Self::Document => "📝",
        }
    }
}

/// Metadata about a media output.
#[derive(Debug, Clone)]
pub struct MediaMetadata {
    pub media_type: MediaType,
    pub title: String,
    pub file_path: Option<String>,
    pub file_size_bytes: Option<u64>,
    pub provider_name: String,
    pub generation_time_ms: Option<u64>,
    pub cost_usd: Option<f64>,
    /// Dimensions for image/video.
    pub dimensions: Option<(u32, u32)>,
    /// Duration for audio/video in seconds.
    pub duration_secs: Option<f64>,
}

#[derive(Debug, Clone)]
pub enum MediaPreviewEvent {
    DownloadRequested(String),
    ShareRequested(MediaMetadata),
    RegenerateRequested(MediaMetadata),
    FullscreenRequested,
    Dismissed,
}

pub struct MediaPreview {
    focus_handle: FocusHandle,
    metadata: Option<MediaMetadata>,
    is_loading: bool,
    error_message: Option<String>,
    progress: Option<f32>,
}

impl MediaPreview {
    pub fn new(cx: &mut ViewContext<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            metadata: None,
            is_loading: false,
            error_message: None,
            progress: None,
        }
    }

    pub fn set_metadata(&mut self, metadata: MediaMetadata, cx: &mut ViewContext<Self>) {
        self.metadata = Some(metadata);
        self.is_loading = false;
        self.error_message = None;
        cx.notify();
    }

    pub fn set_loading(&mut self, cx: &mut ViewContext<Self>) {
        self.is_loading = true;
        self.error_message = None;
        cx.notify();
    }

    pub fn set_progress(&mut self, progress: f32, cx: &mut ViewContext<Self>) {
        self.progress = Some(progress.clamp(0.0, 1.0));
        cx.notify();
    }

    pub fn set_error(&mut self, message: String, cx: &mut ViewContext<Self>) {
        self.error_message = Some(message);
        self.is_loading = false;
        cx.notify();
    }

    pub fn clear(&mut self, cx: &mut ViewContext<Self>) {
        self.metadata = None;
        self.is_loading = false;
        self.error_message = None;
        self.progress = None;
        cx.notify();
    }

    fn format_file_size(bytes: u64) -> String {
        if bytes < 1024 {
            format!("{} B", bytes)
        } else if bytes < 1024 * 1024 {
            format!("{:.1} KB", bytes as f64 / 1024.0)
        } else if bytes < 1024 * 1024 * 1024 {
            format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
        } else {
            format!("{:.2} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
        }
    }
}

impl gpui::EventEmitter<MediaPreviewEvent> for MediaPreview {}

impl Focusable for MediaPreview {
    fn focus_handle(&self, _cx: &gpui::App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for MediaPreview {
    fn render(&mut self, _cx: &mut ViewContext<Self>) -> impl IntoElement {
        let is_loading = self.is_loading;
        let error = self.error_message.clone();
        let metadata = self.metadata.clone();
        let progress = self.progress;

        div()
            .flex()
            .flex_col()
            .gap(px(12.0))
            .p(px(16.0))
            .rounded(px(12.0))
            .border_1()
            // Loading state
            .when(is_loading, |container| {
                container.child(
                    div()
                        .flex()
                        .flex_col()
                        .items_center()
                        .justify_center()
                        .h(px(200.0))
                        .gap(px(12.0))
                        .child(
                            div()
                                .text_sm()
                                .child(SharedString::from("Generating..."))
                        )
                        .when_some(progress, |d, p| {
                            d.child(
                                div()
                                    .w_full()
                                    .h(px(4.0))
                                    .rounded(px(2.0))
                                    .child(
                                        div()
                                            .h_full()
                                            .w(px(300.0 * p))
                                            .rounded(px(2.0))
                                    )
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .child(SharedString::from(format!("{:.0}%", p * 100.0)))
                            )
                        })
                )
            })
            // Error state
            .when_some(error, |container, err| {
                container.child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(8.0))
                        .p(px(12.0))
                        .rounded(px(8.0))
                        .child(
                            div()
                                .text_sm()
                                .child(SharedString::from("❌"))
                        )
                        .child(
                            div()
                                .text_sm()
                                .child(SharedString::from(err))
                        )
                )
            })
            // Content preview
            .when_some(metadata, |container, meta| {
                let media_type = meta.media_type;
                container
                    // Header with type + title
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.0))
                            .child(
                                div()
                                    .text_size(px(20.0))
                                    .child(SharedString::from(media_type.icon()))
                            )
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .child(
                                        div()
                                            .text_sm()
                                            .font_weight(gpui::FontWeight::SEMIBOLD)
                                            .child(SharedString::from(meta.title.clone()))
                                    )
                                    .child(
                                        div()
                                            .text_xs()
                                            .child(SharedString::from(format!(
                                                "{} • {}",
                                                media_type.label(),
                                                meta.provider_name
                                            )))
                                    )
                            )
                    )
                    // Preview area (placeholder — actual rendering depends on media type)
                    .child(
                        div()
                            .w_full()
                            .h(px(240.0))
                            .rounded(px(8.0))
                            .border_1()
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(
                                div()
                                    .text_size(px(48.0))
                                    .child(SharedString::from(media_type.icon()))
                            )
                    )
                    // Metadata row
                    .child(
                        div()
                            .flex()
                            .flex_wrap()
                            .gap(px(12.0))
                            .text_xs()
                            .when_some(meta.dimensions, |d, (w, h)| {
                                d.child(
                                    div().child(SharedString::from(format!("{}×{}", w, h)))
                                )
                            })
                            .when_some(meta.duration_secs, |d, dur| {
                                d.child(
                                    div().child(SharedString::from(format!("{:.1}s", dur)))
                                )
                            })
                            .when_some(meta.file_size_bytes, |d, bytes| {
                                d.child(
                                    div().child(SharedString::from(
                                        Self::format_file_size(bytes)
                                    ))
                                )
                            })
                            .when_some(meta.generation_time_ms, |d, ms| {
                                d.child(
                                    div().child(SharedString::from(format!("{}ms", ms)))
                                )
                            })
                            .when_some(meta.cost_usd, |d, cost| {
                                d.child(
                                    div().child(SharedString::from(format!("${:.4}", cost)))
                                )
                            })
                    )
                    // Action buttons
                    .child(
                        div()
                            .flex()
                            .gap(px(6.0))
                            .child(action_button("Download"))
                            .child(action_button("Share"))
                            .child(action_button("Regenerate"))
                            .child(action_button("Fullscreen"))
                    )
            })
    }
}

fn action_button(label: &str) -> gpui::Div {
    div()
        .px(px(10.0))
        .py(px(6.0))
        .rounded(px(6.0))
        .border_1()
        .cursor_pointer()
        .text_xs()
        .child(SharedString::from(label.to_string()))
}
