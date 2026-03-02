//! ModelDownloadUi — Model download progress with resume support.
//!
//! Part 16 UI: Shows download queue, per-model progress bars,
//! pause/resume/cancel, and disk space warnings.

use gpui::{
    div, prelude::*, px, FocusHandle, Focusable, ParentElement, Render, SharedString,
    Styled, ViewContext,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DownloadStatus {
    Queued,
    Downloading,
    Paused,
    Verifying,
    Complete,
    Failed,
}

impl DownloadStatus {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Queued => "Queued",
            Self::Downloading => "Downloading",
            Self::Paused => "Paused",
            Self::Verifying => "Verifying",
            Self::Complete => "Complete",
            Self::Failed => "Failed",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            Self::Queued => "⏳",
            Self::Downloading => "⬇️",
            Self::Paused => "⏸️",
            Self::Verifying => "🔍",
            Self::Complete => "✅",
            Self::Failed => "❌",
        }
    }
}

/// A single model download entry.
#[derive(Debug, Clone)]
pub struct ModelDownloadEntry {
    pub model_id: String,
    pub model_name: String,
    pub size_bytes: u64,
    pub downloaded_bytes: u64,
    pub status: DownloadStatus,
    pub speed_bytes_per_sec: Option<u64>,
    pub eta_secs: Option<u64>,
    pub error_message: Option<String>,
}

impl ModelDownloadEntry {
    pub fn progress_fraction(&self) -> f32 {
        if self.size_bytes == 0 {
            return 0.0;
        }
        (self.downloaded_bytes as f32 / self.size_bytes as f32).clamp(0.0, 1.0)
    }
}

#[derive(Debug, Clone)]
pub enum ModelDownloadEvent {
    PauseRequested(String),
    ResumeRequested(String),
    CancelRequested(String),
    RetryRequested(String),
    ClearCompleted,
}

pub struct ModelDownloadUi {
    focus_handle: FocusHandle,
    downloads: Vec<ModelDownloadEntry>,
    disk_free_bytes: u64,
    disk_warning_threshold_bytes: u64,
}

impl ModelDownloadUi {
    pub fn new(cx: &mut ViewContext<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            downloads: Vec::new(),
            disk_free_bytes: 0,
            disk_warning_threshold_bytes: 2 * 1024 * 1024 * 1024, // 2 GB
        }
    }

    pub fn set_downloads(
        &mut self,
        downloads: Vec<ModelDownloadEntry>,
        cx: &mut ViewContext<Self>,
    ) {
        self.downloads = downloads;
        cx.notify();
    }

    pub fn update_download(
        &mut self,
        model_id: &str,
        downloaded_bytes: u64,
        speed: Option<u64>,
        eta: Option<u64>,
        cx: &mut ViewContext<Self>,
    ) {
        if let Some(entry) = self.downloads.iter_mut().find(|d| d.model_id == model_id) {
            entry.downloaded_bytes = downloaded_bytes;
            entry.speed_bytes_per_sec = speed;
            entry.eta_secs = eta;
            if entry.status == DownloadStatus::Queued {
                entry.status = DownloadStatus::Downloading;
            }
        }
        cx.notify();
    }

    pub fn set_status(
        &mut self,
        model_id: &str,
        status: DownloadStatus,
        cx: &mut ViewContext<Self>,
    ) {
        if let Some(entry) = self.downloads.iter_mut().find(|d| d.model_id == model_id) {
            entry.status = status;
        }
        cx.notify();
    }

    pub fn set_disk_free(&mut self, bytes: u64, cx: &mut ViewContext<Self>) {
        self.disk_free_bytes = bytes;
        cx.notify();
    }

    pub fn is_disk_low(&self) -> bool {
        self.disk_free_bytes < self.disk_warning_threshold_bytes
    }

    pub fn active_count(&self) -> usize {
        self.downloads
            .iter()
            .filter(|d| d.status == DownloadStatus::Downloading)
            .count()
    }

    pub fn clear_completed(&mut self, cx: &mut ViewContext<Self>) {
        self.downloads
            .retain(|d| d.status != DownloadStatus::Complete);
        cx.emit(ModelDownloadEvent::ClearCompleted);
        cx.notify();
    }

    fn format_bytes(bytes: u64) -> String {
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

    fn format_speed(bytes_per_sec: u64) -> String {
        if bytes_per_sec < 1024 {
            format!("{} B/s", bytes_per_sec)
        } else if bytes_per_sec < 1024 * 1024 {
            format!("{:.1} KB/s", bytes_per_sec as f64 / 1024.0)
        } else {
            format!("{:.1} MB/s", bytes_per_sec as f64 / (1024.0 * 1024.0))
        }
    }

    fn format_eta(secs: u64) -> String {
        if secs < 60 {
            format!("{}s", secs)
        } else if secs < 3600 {
            format!("{}m {}s", secs / 60, secs % 60)
        } else {
            format!("{}h {}m", secs / 3600, (secs % 3600) / 60)
        }
    }
}

impl gpui::EventEmitter<ModelDownloadEvent> for ModelDownloadUi {}

impl Focusable for ModelDownloadUi {
    fn focus_handle(&self, _cx: &gpui::App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for ModelDownloadUi {
    fn render(&mut self, _cx: &mut ViewContext<Self>) -> impl IntoElement {
        let downloads = self.downloads.clone();
        let disk_low = self.is_disk_low();
        let disk_free = self.disk_free_bytes;
        let active = self.active_count();

        div()
            .flex()
            .flex_col()
            .gap(px(12.0))
            .p(px(16.0))
            // Header
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .text_sm()
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .child(SharedString::from(format!(
                                "Model Downloads{}",
                                if active > 0 {
                                    format!(" ({})", active)
                                } else {
                                    String::new()
                                }
                            )))
                    )
                    .child(
                        div()
                            .cursor_pointer()
                            .text_xs()
                            .px(px(8.0))
                            .py(px(4.0))
                            .rounded(px(4.0))
                            .border_1()
                            .child(SharedString::from("Clear Completed"))
                    )
            )
            // Disk space warning
            .when(disk_low, |container| {
                container.child(
                    div()
                        .p(px(10.0))
                        .rounded(px(8.0))
                        .border_1()
                        .flex()
                        .items_center()
                        .gap(px(8.0))
                        .child(
                            div()
                                .text_sm()
                                .child(SharedString::from("⚠️"))
                        )
                        .child(
                            div()
                                .text_xs()
                                .child(SharedString::from(format!(
                                    "Low disk space: {} free",
                                    Self::format_bytes(disk_free)
                                )))
                        )
                )
            })
            // Download entries
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(8.0))
                    .children(downloads.iter().map(|entry| {
                        let progress = entry.progress_fraction();
                        let status = entry.status;

                        div()
                            .p(px(12.0))
                            .rounded(px(8.0))
                            .border_1()
                            .flex()
                            .flex_col()
                            .gap(px(6.0))
                            // Model name + status
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .child(
                                        div()
                                            .text_sm()
                                            .font_weight(gpui::FontWeight::MEDIUM)
                                            .child(SharedString::from(
                                                entry.model_name.clone(),
                                            ))
                                    )
                                    .child(
                                        div()
                                            .flex()
                                            .items_center()
                                            .gap(px(4.0))
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .child(SharedString::from(
                                                        status.icon(),
                                                    ))
                                            )
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .child(SharedString::from(
                                                        status.label(),
                                                    ))
                                            )
                                    )
                            )
                            // Progress bar
                            .when(
                                status == DownloadStatus::Downloading
                                    || status == DownloadStatus::Paused,
                                |d| {
                                    d.child(
                                        div()
                                            .w_full()
                                            .h(px(4.0))
                                            .rounded(px(2.0))
                                            .child(
                                                div()
                                                    .h_full()
                                                    .w(px(280.0 * progress))
                                                    .rounded(px(2.0))
                                            )
                                    )
                                }
                            )
                            // Stats row
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .text_xs()
                                    .child(SharedString::from(format!(
                                        "{} / {}",
                                        Self::format_bytes(entry.downloaded_bytes),
                                        Self::format_bytes(entry.size_bytes)
                                    )))
                                    .child(
                                        div()
                                            .flex()
                                            .gap(px(8.0))
                                            .when_some(
                                                entry.speed_bytes_per_sec,
                                                |d, speed| {
                                                    d.child(SharedString::from(
                                                        Self::format_speed(speed),
                                                    ))
                                                },
                                            )
                                            .when_some(entry.eta_secs, |d, eta| {
                                                d.child(SharedString::from(
                                                    Self::format_eta(eta),
                                                ))
                                            })
                                    )
                            )
                            // Action buttons
                            .child(
                                div()
                                    .flex()
                                    .gap(px(4.0))
                                    .when(status == DownloadStatus::Downloading, |d| {
                                        d.child(small_button("Pause"))
                                    })
                                    .when(status == DownloadStatus::Paused, |d| {
                                        d.child(small_button("Resume"))
                                    })
                                    .when(
                                        status == DownloadStatus::Downloading
                                            || status == DownloadStatus::Paused
                                            || status == DownloadStatus::Queued,
                                        |d| d.child(small_button("Cancel")),
                                    )
                                    .when(status == DownloadStatus::Failed, |d| {
                                        d.child(small_button("Retry"))
                                    })
                            )
                            // Error message
                            .when_some(entry.error_message.clone(), |d, err| {
                                d.child(
                                    div()
                                        .text_xs()
                                        .child(SharedString::from(err))
                                )
                            })
                    }))
            )
            // Empty state
            .when(downloads.is_empty(), |container| {
                container.child(
                    div()
                        .flex()
                        .items_center()
                        .justify_center()
                        .h(px(80.0))
                        .text_sm()
                        .child(SharedString::from("No active downloads"))
                )
            })
    }
}

fn small_button(label: &str) -> gpui::Div {
    div()
        .px(px(8.0))
        .py(px(3.0))
        .rounded(px(4.0))
        .border_1()
        .cursor_pointer()
        .text_xs()
        .child(SharedString::from(label.to_string()))
}
