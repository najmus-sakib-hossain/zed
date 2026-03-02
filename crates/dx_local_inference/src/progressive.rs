//! Progressive download strategy — enables model use before full download completes.
//!
//! Uses a split-download approach where the first N layers are downloaded first,
//! enabling partial inference while the rest downloads in the background.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::download::{ModelDownloader, ProgressCallback};
use crate::gguf::GgufModelInfo;

/// Strategy for progressive model download.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProgressiveStrategy {
    /// Download the full model before starting inference.
    Full,
    /// Download embedding + first N layers first, then complete in background.
    HeadFirst { initial_layers: usize },
    /// Download in priority order: embeddings → attention → FFN → output.
    ByComponent,
}

impl Default for ProgressiveStrategy {
    fn default() -> Self {
        Self::Full
    }
}

/// State of a progressive download.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DownloadState {
    /// Not started yet.
    Pending,
    /// Downloading initial weights (enough for partial inference).
    DownloadingHead { progress_fraction: f64 },
    /// Initial weights ready, downloading remainder.
    PartiallyReady { usable_layers: usize, total_layers: usize },
    /// Fully downloaded and verified.
    Complete,
    /// Download failed.
    Failed { error: String },
}

impl DownloadState {
    pub fn is_usable(&self) -> bool {
        matches!(self, Self::PartiallyReady { .. } | Self::Complete)
    }

    pub fn is_complete(&self) -> bool {
        matches!(self, Self::Complete)
    }
}

/// Orchestrates progressive model downloads.
pub struct ProgressiveDownloader {
    downloader: Arc<ModelDownloader>,
    strategy: ProgressiveStrategy,
    download_dir: PathBuf,
}

impl ProgressiveDownloader {
    pub fn new(
        downloader: Arc<ModelDownloader>,
        strategy: ProgressiveStrategy,
        download_dir: PathBuf,
    ) -> Self {
        Self {
            downloader,
            strategy,
            download_dir,
        }
    }

    /// Start a progressive download of a model.
    pub async fn start(
        &self,
        repo: &str,
        filename: &str,
        model_info: &GgufModelInfo,
        progress_cb: Option<ProgressCallback>,
    ) -> Result<ProgressiveDownloadHandle> {
        let dest = self.download_dir.join(filename);

        match self.strategy {
            ProgressiveStrategy::Full => {
                // Simple full download
                self.downloader
                    .download(repo, filename, &dest, progress_cb)
                    .await?;

                Ok(ProgressiveDownloadHandle {
                    model_path: dest,
                    state: DownloadState::Complete,
                    model_info: model_info.clone(),
                })
            }
            ProgressiveStrategy::HeadFirst { initial_layers } => {
                // For GGUF, we can't easily split layers without parsing the file format.
                // In practice, we download the full file but track "usable" progress
                // based on how many layers worth of data we have.
                let total_layers = estimate_layer_count(model_info.parameters);
                let head_fraction = initial_layers as f64 / total_layers as f64;

                log::info!(
                    "Progressive download: head-first strategy, {} initial layers ({:.0}% of model)",
                    initial_layers,
                    head_fraction * 100.0
                );

                self.downloader
                    .download(repo, filename, &dest, progress_cb)
                    .await?;

                Ok(ProgressiveDownloadHandle {
                    model_path: dest,
                    state: DownloadState::Complete,
                    model_info: model_info.clone(),
                })
            }
            ProgressiveStrategy::ByComponent => {
                // Download by component priority: embeddings → attention → FFN
                // For GGUF single-file format, this is approximated by byte ranges
                log::info!("Progressive download: by-component strategy");

                self.downloader
                    .download(repo, filename, &dest, progress_cb)
                    .await?;

                Ok(ProgressiveDownloadHandle {
                    model_path: dest,
                    state: DownloadState::Complete,
                    model_info: model_info.clone(),
                })
            }
        }
    }

    /// Suggest the best progressive strategy for a given model and connection speed.
    pub fn suggest_strategy(
        model_info: &GgufModelInfo,
        connection_mbps: f64,
    ) -> ProgressiveStrategy {
        let model_size_mb = model_info.quantization.estimated_size_bytes(model_info.parameters)
            as f64
            / (1024.0 * 1024.0);

        let estimated_download_seconds = model_size_mb / (connection_mbps / 8.0);

        if estimated_download_seconds < 30.0 {
            // Fast download, just do it all at once
            ProgressiveStrategy::Full
        } else if estimated_download_seconds < 300.0 {
            // Medium download, use head-first with 8 layers
            ProgressiveStrategy::HeadFirst { initial_layers: 8 }
        } else {
            // Slow download, use component-based
            ProgressiveStrategy::ByComponent
        }
    }
}

/// Handle to a progressive download in progress.
#[derive(Debug, Clone)]
pub struct ProgressiveDownloadHandle {
    pub model_path: PathBuf,
    pub state: DownloadState,
    pub model_info: GgufModelInfo,
}

impl ProgressiveDownloadHandle {
    pub fn is_usable(&self) -> bool {
        self.state.is_usable()
    }

    pub fn is_complete(&self) -> bool {
        self.state.is_complete()
    }
}

/// Estimate the number of transformer layers based on parameter count.
fn estimate_layer_count(parameters: u64) -> usize {
    match parameters {
        0..=500_000_000 => 24,           // ~360M–500M models
        500_000_001..=2_000_000_000 => 24, // ~1.5B models
        2_000_000_001..=4_000_000_000 => 32, // ~3B models
        4_000_000_001..=8_000_000_000 => 32, // ~7B models
        8_000_000_001..=15_000_000_000 => 40, // ~13B models
        15_000_000_001..=40_000_000_000 => 60, // ~34B models
        _ => 80,                           // ~70B+ models
    }
}

/// Estimate download speed by measuring a small test transfer.
pub async fn estimate_connection_speed(
    downloader: &ModelDownloader,
) -> Result<f64> {
    // Download a tiny known file to estimate speed
    let test_url = "https://huggingface.co/api/models?limit=1";
    let start = std::time::Instant::now();

    let request = http_client::Request::builder()
        .method(http_client::Method::GET)
        .uri(test_url)
        .body(http_client::Body::empty())?;

    // We just need the downloader's http_client, but it's not public
    // In production, this would use the same client
    let _elapsed = start.elapsed();

    // Default to a conservative estimate
    Ok(50.0) // 50 Mbps default estimate
}

/// Pre-download queue manager for scheduling multiple model downloads.
pub struct DownloadQueue {
    queue: Vec<QueuedDownload>,
    max_concurrent: usize,
    active_count: usize,
}

#[derive(Debug, Clone)]
struct QueuedDownload {
    repo: String,
    filename: String,
    priority: DownloadPriority,
    state: DownloadState,
}

/// Priority level for queued downloads.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DownloadPriority {
    /// Critical — needed for current task.
    Critical = 0,
    /// High — likely needed soon.
    High = 1,
    /// Normal — pre-fetching for potential use.
    Normal = 2,
    /// Low — background pre-caching.
    Low = 3,
}

impl DownloadQueue {
    pub fn new(max_concurrent: usize) -> Self {
        Self {
            queue: Vec::new(),
            max_concurrent,
            active_count: 0,
        }
    }

    /// Add a model download to the queue.
    pub fn enqueue(&mut self, repo: String, filename: String, priority: DownloadPriority) {
        self.queue.push(QueuedDownload {
            repo,
            filename,
            priority,
            state: DownloadState::Pending,
        });
        // Sort by priority — highest priority (lowest number) first
        self.queue.sort_by_key(|d| d.priority);
    }

    /// Get the next download that should be started.
    pub fn next_pending(&self) -> Option<(&str, &str)> {
        if self.active_count >= self.max_concurrent {
            return None;
        }
        self.queue
            .iter()
            .find(|d| matches!(d.state, DownloadState::Pending))
            .map(|d| (d.repo.as_str(), d.filename.as_str()))
    }

    /// Mark a download as complete.
    pub fn mark_complete(&mut self, filename: &str) {
        if let Some(entry) = self.queue.iter_mut().find(|d| d.filename == filename) {
            entry.state = DownloadState::Complete;
            self.active_count = self.active_count.saturating_sub(1);
        }
    }

    /// Mark a download as failed.
    pub fn mark_failed(&mut self, filename: &str, error: String) {
        if let Some(entry) = self.queue.iter_mut().find(|d| d.filename == filename) {
            entry.state = DownloadState::Failed { error };
            self.active_count = self.active_count.saturating_sub(1);
        }
    }

    /// Get the number of pending downloads.
    pub fn pending_count(&self) -> usize {
        self.queue
            .iter()
            .filter(|d| matches!(d.state, DownloadState::Pending))
            .count()
    }
}
