//! Update downloader with progress tracking

use super::types::UpdateInfo;
use crate::utils::error::DxError;
use indicatif::{ProgressBar, ProgressStyle};
use std::time::Duration;

/// Update downloader with progress tracking
pub struct UpdateDownloader {
    /// HTTP client
    client: reqwest::blocking::Client,
}

impl UpdateDownloader {
    /// Create a new update downloader
    pub fn new() -> Result<Self, DxError> {
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(300))
            .user_agent(format!("dx-cli/{}", super::CURRENT_VERSION))
            .build()
            .map_err(|e| DxError::Network {
                message: format!("Failed to create HTTP client: {}", e),
            })?;

        Ok(Self { client })
    }

    /// Download update with progress bar
    pub fn download(&self, info: &UpdateInfo) -> Result<Vec<u8>, DxError> {
        let url = info.preferred_url();
        let size = info.preferred_size();

        let pb = ProgressBar::new(size);
        pb.set_style(
            ProgressStyle::default_bar()
                .template(
                    "  [{elapsed_precise}] [{bar:50.cyan/blue}] {bytes}/{total_bytes} ({eta})",
                )
                .unwrap()
                .progress_chars("█▓▒░ "),
        );

        let mut response =
            self.client.get(url).send().map_err(|e| DxError::UpdateDownloadFailed {
                message: format!("Failed to download: {}", e),
            })?;

        if !response.status().is_success() {
            return Err(DxError::UpdateDownloadFailed {
                message: format!(
                    "HTTP {}: {}",
                    response.status(),
                    response.status().canonical_reason().unwrap_or("Unknown")
                ),
            });
        }

        let mut buffer = Vec::with_capacity(size as usize);
        use std::io::Read;

        let mut chunk = [0u8; 8192];
        loop {
            match response.read(&mut chunk) {
                Ok(0) => break,
                Ok(n) => {
                    buffer.extend_from_slice(&chunk[..n]);
                    pb.inc(n as u64);
                }
                Err(e) => {
                    pb.finish_and_clear();
                    return Err(DxError::UpdateDownloadFailed {
                        message: format!("Download interrupted: {}", e),
                    });
                }
            }
        }

        pb.finish_and_clear();
        Ok(buffer)
    }

    /// Download signature file
    pub fn download_signature(&self, url: &str) -> Result<Vec<u8>, DxError> {
        let response = self.client.get(url).send().map_err(|e| DxError::UpdateDownloadFailed {
            message: format!("Failed to download signature: {}", e),
        })?;

        if !response.status().is_success() {
            return Err(DxError::UpdateDownloadFailed {
                message: format!("Signature download failed: HTTP {}", response.status()),
            });
        }

        response.bytes().map(|b| b.to_vec()).map_err(|e| DxError::UpdateDownloadFailed {
            message: format!("Failed to read signature: {}", e),
        })
    }
}

impl Default for UpdateDownloader {
    fn default() -> Self {
        Self::new().expect("Failed to create update downloader")
    }
}
