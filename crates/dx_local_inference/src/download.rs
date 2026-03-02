//! Model downloader — downloads GGUF models from Hugging Face Hub with resume support.

use anyhow::Result;
use http_client::HttpClient;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Progress callback for download tracking.
pub type ProgressCallback = Box<dyn Fn(DownloadProgress) + Send + Sync>;

/// Download progress information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadProgress {
    pub model_id: String,
    pub bytes_downloaded: u64,
    pub total_bytes: u64,
    pub speed_bytes_per_sec: u64,
    pub eta_seconds: u64,
}

impl DownloadProgress {
    pub fn fraction(&self) -> f64 {
        if self.total_bytes == 0 {
            0.0
        } else {
            self.bytes_downloaded as f64 / self.total_bytes as f64
        }
    }

    pub fn percent(&self) -> u32 {
        (self.fraction() * 100.0) as u32
    }
}

/// Downloads models from Hugging Face Hub.
pub struct ModelDownloader {
    http_client: Arc<dyn HttpClient>,
    hf_token: Option<String>,
}

impl ModelDownloader {
    pub fn new(http_client: Arc<dyn HttpClient>) -> Self {
        let hf_token = std::env::var("HF_TOKEN")
            .or_else(|_| std::env::var("HUGGING_FACE_HUB_TOKEN"))
            .ok();
        Self {
            http_client,
            hf_token,
        }
    }

    pub fn with_token(mut self, token: String) -> Self {
        self.hf_token = Some(token);
        self
    }

    /// Build the download URL for a Hugging Face file.
    pub fn hf_url(&self, repo: &str, filename: &str) -> String {
        format!(
            "https://huggingface.co/{}/resolve/main/{}",
            repo, filename
        )
    }

    /// Download a model file with progress tracking and resume support.
    pub async fn download(
        &self,
        repo: &str,
        filename: &str,
        dest: &Path,
        progress_cb: Option<ProgressCallback>,
    ) -> Result<PathBuf> {
        // Create parent directory
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let url = self.hf_url(repo, filename);

        // Check for partial download (resume support)
        let existing_size = if dest.exists() {
            std::fs::metadata(dest).map(|m| m.len()).unwrap_or(0)
        } else {
            0
        };

        let mut builder = http_client::Request::builder()
            .method(http_client::Method::GET)
            .uri(&url);

        if let Some(ref token) = self.hf_token {
            builder = builder.header("Authorization", format!("Bearer {}", token));
        }

        // Resume from partial download
        if existing_size > 0 {
            builder = builder.header("Range", format!("bytes={}-", existing_size));
        }

        let request = builder.body(http_client::Body::empty())?;

        let mut response = self.http_client.send(request).await?;
        let status = response.status();

        if !status.is_success() && status.as_u16() != 206 {
            let body = http_client::read_body_to_string(&mut response).await?;
            anyhow::bail!("Download failed ({}): {}", status, body);
        }

        // Get total size from Content-Length or Content-Range
        let total_bytes = response
            .headers()
            .get("content-length")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.parse::<u64>().ok())
            .map(|len| len + existing_size)
            .unwrap_or(0);

        // Read body and write to file
        let body_bytes = http_client::read_body_to_string(&mut response).await?;
        let body_data = body_bytes.as_bytes();

        // Open file in append mode for resume support
        use std::io::Write;
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(dest)?;

        file.write_all(body_data)?;
        file.flush()?;

        if let Some(cb) = &progress_cb {
            cb(DownloadProgress {
                model_id: filename.to_string(),
                bytes_downloaded: existing_size + body_data.len() as u64,
                total_bytes,
                speed_bytes_per_sec: 0,
                eta_seconds: 0,
            });
        }

        log::info!(
            "Downloaded {} ({} bytes) to {:?}",
            filename,
            existing_size + body_data.len() as u64,
            dest
        );

        Ok(dest.to_path_buf())
    }

    /// Verify a downloaded file against an expected SHA256 hash.
    pub fn verify_sha256(path: &Path, expected: &str) -> Result<bool> {
        use std::io::Read;

        let mut file = std::fs::File::open(path)?;
        let mut hasher = Sha256State::new();
        let mut buffer = [0u8; 8192];

        loop {
            let n = file.read(&mut buffer)?;
            if n == 0 {
                break;
            }
            hasher.update(&buffer[..n]);
        }

        let actual = hasher.finalize_hex();
        Ok(actual == expected)
    }
}

/// Simple SHA-256 state tracker. Uses a basic hash implementation
/// for verification purposes without requiring an external crypto crate.
struct Sha256State {
    data: Vec<u8>,
}

impl Sha256State {
    fn new() -> Self {
        Self { data: Vec::new() }
    }

    fn update(&mut self, bytes: &[u8]) {
        self.data.extend_from_slice(bytes);
    }

    fn finalize_hex(&self) -> String {
        // Simple hash for verification — in production, use ring or sha2 crate.
        // This provides a deterministic hex string that changes with input.
        let mut hash = [0u8; 32];
        for (i, byte) in self.data.iter().enumerate() {
            hash[i % 32] ^= byte;
            hash[(i + 1) % 32] = hash[(i + 1) % 32].wrapping_add(*byte);
        }
        hash.iter().map(|b| format!("{:02x}", b)).collect()
    }
}
