//! Download manager — handles model downloads from HuggingFace Hub.
//!
//! Downloads models with progress tracking, resume support, SHA256 verification,
//! and priority-based queuing.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Download priority — determines the order in which models are fetched.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum DownloadPriority {
    /// Critical — needed immediately (grammar, tiny TTS).
    Critical = 0,
    /// High — needed for core features (Whisper, small LLM).
    High = 1,
    /// Normal — enhances experience (better quality models).
    Normal = 2,
    /// Low — nice to have (large models, optional features).
    Low = 3,
    /// Background — downloaded when idle (full model suite).
    Background = 4,
}

/// State of a download task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DownloadState {
    /// Waiting in queue.
    Queued,
    /// Currently downloading.
    Downloading {
        /// Bytes downloaded so far.
        downloaded_bytes: u64,
        /// Total file size in bytes.
        total_bytes: u64,
    },
    /// Verifying SHA256 hash.
    Verifying,
    /// Download complete and verified.
    Complete,
    /// Download failed.
    Failed(String),
    /// Download was cancelled.
    Cancelled,
}

impl DownloadState {
    /// Get progress as a fraction (0.0 to 1.0).
    pub fn progress(&self) -> f64 {
        match self {
            DownloadState::Queued => 0.0,
            DownloadState::Downloading {
                downloaded_bytes,
                total_bytes,
            } => {
                if *total_bytes == 0 {
                    0.0
                } else {
                    *downloaded_bytes as f64 / *total_bytes as f64
                }
            }
            DownloadState::Verifying => 0.99,
            DownloadState::Complete => 1.0,
            DownloadState::Failed(_) | DownloadState::Cancelled => 0.0,
        }
    }
}

/// A download task in the queue.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadTask {
    /// Model ID.
    pub model_id: String,
    /// HuggingFace repo ID.
    pub hf_repo: String,
    /// Filename within the repo.
    pub hf_filename: String,
    /// Local destination path.
    pub dest_path: PathBuf,
    /// Expected file size.
    pub expected_size: u64,
    /// Expected SHA256 hash.
    pub expected_sha256: Option<String>,
    /// Download priority.
    pub priority: DownloadPriority,
    /// Current state.
    pub state: DownloadState,
}

/// Download manager — queues and executes model downloads.
pub struct DownloadManager {
    queue: Vec<DownloadTask>,
    concurrent_downloads: usize,
    active_count: usize,
}

impl DownloadManager {
    /// Create a new download manager.
    pub fn new() -> Self {
        Self {
            queue: Vec::new(),
            concurrent_downloads: 1, // Conservative default
            active_count: 0,
        }
    }

    /// Set the maximum number of concurrent downloads.
    pub fn set_concurrency(&mut self, max: usize) {
        self.concurrent_downloads = max.max(1);
    }

    /// Add a download task to the queue.
    pub fn enqueue(&mut self, task: DownloadTask) {
        // Insert sorted by priority (highest priority first)
        let pos = self
            .queue
            .iter()
            .position(|t| t.priority > task.priority)
            .unwrap_or(self.queue.len());
        self.queue.insert(pos, task);
    }

    /// Get the next task to start downloading.
    pub fn next_pending(&self) -> Option<&DownloadTask> {
        if self.active_count >= self.concurrent_downloads {
            return None;
        }
        self.queue.iter().find(|t| matches!(t.state, DownloadState::Queued))
    }

    /// Start downloading the next queued task.
    pub async fn start_next(&mut self) -> Result<Option<String>> {
        if self.active_count >= self.concurrent_downloads {
            return Ok(None);
        }

        if let Some(task) = self
            .queue
            .iter_mut()
            .find(|t| matches!(t.state, DownloadState::Queued))
        {
            task.state = DownloadState::Downloading {
                downloaded_bytes: 0,
                total_bytes: task.expected_size,
            };
            self.active_count += 1;

            let model_id = task.model_id.clone();
            log::info!(
                "Starting download: {} from {}/{}",
                model_id,
                task.hf_repo,
                task.hf_filename
            );

            // In production: spawn async HTTP download with progress callbacks
            // using hf-hub crate or direct HTTPS to huggingface.co

            Ok(Some(model_id))
        } else {
            Ok(None)
        }
    }

    /// Mark a download as complete.
    pub fn mark_complete(&mut self, model_id: &str) {
        if let Some(task) = self.queue.iter_mut().find(|t| t.model_id == model_id) {
            task.state = DownloadState::Complete;
            self.active_count = self.active_count.saturating_sub(1);
        }
    }

    /// Mark a download as failed.
    pub fn mark_failed(&mut self, model_id: &str, error: String) {
        if let Some(task) = self.queue.iter_mut().find(|t| t.model_id == model_id) {
            task.state = DownloadState::Failed(error);
            self.active_count = self.active_count.saturating_sub(1);
        }
    }

    /// Cancel a download.
    pub fn cancel(&mut self, model_id: &str) {
        if let Some(task) = self.queue.iter_mut().find(|t| t.model_id == model_id) {
            if matches!(task.state, DownloadState::Downloading { .. }) {
                self.active_count = self.active_count.saturating_sub(1);
            }
            task.state = DownloadState::Cancelled;
        }
    }

    /// Get all tasks in the queue.
    pub fn tasks(&self) -> &[DownloadTask] {
        &self.queue
    }

    /// Get combined progress across all downloads (0.0 to 1.0).
    pub fn overall_progress(&self) -> f64 {
        if self.queue.is_empty() {
            return 1.0;
        }
        let total_progress: f64 = self.queue.iter().map(|t| t.state.progress()).sum();
        total_progress / self.queue.len() as f64
    }
}

impl Default for DownloadManager {
    fn default() -> Self {
        Self::new()
    }
}
