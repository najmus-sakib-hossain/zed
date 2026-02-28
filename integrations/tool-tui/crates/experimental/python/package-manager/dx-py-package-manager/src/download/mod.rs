//! Async download manager with retry and parallel downloads
//!
//! Provides a high-performance download manager for fetching packages from PyPI
//! with support for concurrent downloads, retry logic, and SHA256 verification.

pub mod pypi;

pub use pypi::{
    FileDigestsInfo, PackageInfoDetails, PackageMetadata, PyPiDownloader, ReleaseFileInfo,
    PYPI_BASE_URL,
};

// Re-export wheel types for convenience
pub use dx_py_core::wheel::{PlatformEnvironment, WheelTag};

use std::sync::Arc;
use std::time::Duration;

use futures::stream::{self, StreamExt};
use reqwest::Client;
use sha2::{Digest, Sha256};
use tokio::sync::Semaphore;

use crate::{Error, Result};

/// Request for downloading a file
#[derive(Debug, Clone)]
pub struct DownloadRequest {
    /// URL to download from
    pub url: String,
    /// Expected SHA256 hash (hex-encoded)
    pub expected_sha256: String,
    /// Filename for identification
    pub filename: String,
}

/// Result of a download operation
#[derive(Debug)]
pub struct DownloadResult {
    /// The downloaded data
    pub data: Vec<u8>,
    /// The filename
    pub filename: String,
    /// Computed SHA256 hash (hex-encoded)
    pub sha256: String,
}

/// Progress callback type
pub type ProgressCallback = Box<dyn Fn(u64, u64) + Send + Sync>;

/// Async download manager with retry and parallelism
pub struct DownloadManager {
    /// HTTP client
    client: Client,
    /// Maximum concurrent downloads
    max_concurrent: usize,
    /// Number of retry attempts
    retry_count: u32,
    /// Base delay between retries (exponential backoff)
    retry_delay: Duration,
    /// Request timeout
    timeout: Duration,
}

impl Default for DownloadManager {
    fn default() -> Self {
        Self::new()
    }
}

impl DownloadManager {
    /// Create a new download manager with default settings
    pub fn new() -> Self {
        Self {
            client: Client::builder()
                .user_agent("dx-py/0.1.0")
                .timeout(Duration::from_secs(300))
                .connect_timeout(Duration::from_secs(30))
                .build()
                .expect("Failed to create HTTP client"),
            max_concurrent: 8,
            retry_count: 3,
            retry_delay: Duration::from_millis(500),
            timeout: Duration::from_secs(300),
        }
    }

    /// Set maximum concurrent downloads
    pub fn with_max_concurrent(mut self, max: usize) -> Self {
        self.max_concurrent = max;
        self
    }

    /// Set retry count
    pub fn with_retry_count(mut self, count: u32) -> Self {
        self.retry_count = count;
        self
    }

    /// Set base retry delay
    pub fn with_retry_delay(mut self, delay: Duration) -> Self {
        self.retry_delay = delay;
        self
    }

    /// Set request timeout
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Get the max concurrent downloads setting
    pub fn max_concurrent(&self) -> usize {
        self.max_concurrent
    }

    /// Get the retry count setting
    pub fn retry_count(&self) -> u32 {
        self.retry_count
    }

    /// Get the retry delay setting
    pub fn retry_delay(&self) -> Duration {
        self.retry_delay
    }

    /// Get the timeout setting
    pub fn timeout(&self) -> Duration {
        self.timeout
    }

    /// Download a single file with retry logic
    pub async fn download(&self, url: &str) -> Result<Vec<u8>> {
        let mut last_error = None;

        for attempt in 0..=self.retry_count {
            if attempt > 0 {
                // Exponential backoff
                let delay = self.retry_delay * 2u32.pow(attempt - 1);
                tokio::time::sleep(delay).await;
            }

            match self.download_once(url).await {
                Ok(data) => return Ok(data),
                Err(e) => {
                    last_error = Some(e);
                }
            }
        }

        Err(last_error.unwrap_or_else(|| {
            Error::Network(format!("Failed to download {} after {} retries", url, self.retry_count))
        }))
    }

    /// Single download attempt without retry
    async fn download_once(&self, url: &str) -> Result<Vec<u8>> {
        let response = self
            .client
            .get(url)
            .timeout(self.timeout)
            .send()
            .await
            .map_err(|e| Error::Network(format!("Request failed for {}: {}", url, e)))?;

        if !response.status().is_success() {
            return Err(Error::Network(format!("HTTP {} for {}", response.status(), url)));
        }

        let bytes = response
            .bytes()
            .await
            .map_err(|e| Error::Network(format!("Failed to read response body: {}", e)))?;

        Ok(bytes.to_vec())
    }

    /// Download with SHA256 verification
    pub async fn download_verified(&self, url: &str, expected_sha256: &str) -> Result<Vec<u8>> {
        let data = self.download(url).await?;
        verify_sha256(&data, expected_sha256)?;
        Ok(data)
    }

    /// Download multiple files in parallel
    pub async fn download_many(
        &self,
        requests: Vec<DownloadRequest>,
    ) -> Vec<Result<DownloadResult>> {
        let semaphore = Arc::new(Semaphore::new(self.max_concurrent));

        let futures = requests.into_iter().map(|req| {
            let sem = Arc::clone(&semaphore);
            let client = self.clone();

            async move {
                let _permit = sem
                    .acquire()
                    .await
                    .map_err(|e| Error::Network(format!("Semaphore error: {}", e)))?;

                let data = client.download_verified(&req.url, &req.expected_sha256).await?;
                let sha256 = compute_sha256(&data);

                Ok(DownloadResult {
                    data,
                    filename: req.filename,
                    sha256,
                })
            }
        });

        stream::iter(futures).buffer_unordered(self.max_concurrent).collect().await
    }

    /// Download with progress callback
    pub async fn download_with_progress<F>(
        &self,
        url: &str,
        expected_sha256: &str,
        on_progress: F,
    ) -> Result<Vec<u8>>
    where
        F: Fn(u64, u64) + Send + Sync,
    {
        let response = self
            .client
            .get(url)
            .timeout(self.timeout)
            .send()
            .await
            .map_err(|e| Error::Network(format!("Request failed for {}: {}", url, e)))?;

        if !response.status().is_success() {
            return Err(Error::Network(format!("HTTP {} for {}", response.status(), url)));
        }

        let total_size = response.content_length().unwrap_or(0);
        let mut downloaded: u64 = 0;
        let mut data = Vec::with_capacity(total_size as usize);

        let mut stream = response.bytes_stream();
        while let Some(chunk_result) = stream.next().await {
            let chunk =
                chunk_result.map_err(|e| Error::Network(format!("Failed to read chunk: {}", e)))?;
            data.extend_from_slice(&chunk);
            downloaded += chunk.len() as u64;
            on_progress(downloaded, total_size);
        }

        verify_sha256(&data, expected_sha256)?;
        Ok(data)
    }
}

impl Clone for DownloadManager {
    fn clone(&self) -> Self {
        Self {
            client: self.client.clone(),
            max_concurrent: self.max_concurrent,
            retry_count: self.retry_count,
            retry_delay: self.retry_delay,
            timeout: self.timeout,
        }
    }
}

/// Compute SHA256 hash of data and return hex-encoded string
pub fn compute_sha256(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hex::encode(hasher.finalize())
}

/// Verify SHA256 hash of data matches expected value
pub fn verify_sha256(data: &[u8], expected: &str) -> Result<()> {
    let computed = compute_sha256(data);
    let expected_lower = expected.to_lowercase();

    if computed != expected_lower {
        return Err(Error::Cache(format!(
            "SHA256 mismatch: expected {}, got {}",
            expected_lower, computed
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_sha256() {
        let data = b"hello world";
        let hash = compute_sha256(data);
        assert_eq!(hash, "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9");
    }

    #[test]
    fn test_verify_sha256_success() {
        let data = b"hello world";
        let hash = "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9";
        assert!(verify_sha256(data, hash).is_ok());
    }

    #[test]
    fn test_verify_sha256_case_insensitive() {
        let data = b"hello world";
        let hash = "B94D27B9934D3E08A52E52D7DA7DABFAC484EFE37A5380EE9088F7ACE2EFCDE9";
        assert!(verify_sha256(data, hash).is_ok());
    }

    #[test]
    fn test_verify_sha256_failure() {
        let data = b"hello world";
        let wrong_hash = "0000000000000000000000000000000000000000000000000000000000000000";
        assert!(verify_sha256(data, wrong_hash).is_err());
    }

    #[test]
    fn test_download_manager_defaults() {
        let dm = DownloadManager::new();
        assert_eq!(dm.max_concurrent(), 8);
        assert_eq!(dm.retry_count(), 3);
    }

    #[test]
    fn test_download_manager_builder() {
        let dm = DownloadManager::new()
            .with_max_concurrent(4)
            .with_retry_count(5)
            .with_retry_delay(Duration::from_secs(1))
            .with_timeout(Duration::from_secs(60));

        assert_eq!(dm.max_concurrent(), 4);
        assert_eq!(dm.retry_count(), 5);
        assert_eq!(dm.retry_delay(), Duration::from_secs(1));
        assert_eq!(dm.timeout(), Duration::from_secs(60));
    }
}
