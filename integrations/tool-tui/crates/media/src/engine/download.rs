//! Download functionality for fetching media assets.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use crate::config::Config;
use crate::error::{DxError, Result};
use crate::http::{HttpClient, validate_url, verify_content_type};
use crate::types::{MediaAsset, MediaType, RateLimitConfig};

/// Progress callback type for download progress updates.
pub type ProgressCallback = Arc<dyn Fn(u64, u64) + Send + Sync>;

/// Downloader for fetching media assets.
#[derive(Debug, Clone)]
pub struct Downloader {
    client: HttpClient,
    download_dir: PathBuf,
}

impl Downloader {
    /// Create a new downloader with default settings.
    #[must_use]
    pub fn new(config: &Config) -> Self {
        // No rate limiting for downloads - providers handle this
        let rate_limit = RateLimitConfig::unlimited();
        let client = HttpClient::with_config(
            rate_limit,
            config.retry_attempts,
            Duration::from_secs(config.timeout_secs),
        )
        .unwrap_or_default();

        Self {
            client,
            download_dir: config.download_dir.clone(),
        }
    }

    /// Set the download directory.
    #[must_use]
    pub fn with_download_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.download_dir = dir.into();
        self
    }

    /// Download a media asset to the default download directory.
    pub async fn download(&self, asset: &MediaAsset) -> Result<PathBuf> {
        self.download_to(&self.download_dir, asset).await
    }

    /// Download a media asset to a specific directory.
    pub async fn download_to(&self, dir: &Path, asset: &MediaAsset) -> Result<PathBuf> {
        let filename = self.generate_filename(asset);
        let filepath = dir.join(&filename);

        // Ensure directory exists
        if let Some(parent) = filepath.parent() {
            tokio::fs::create_dir_all(parent).await.map_err(|e| DxError::FileIo {
                path: parent.to_path_buf(),
                message: format!("Failed to create directory: {}", e),
                source: Some(e),
            })?;
        }

        // Download the file with URL validation and content-type verification
        self.download_file(&asset.download_url, &filepath, asset.media_type).await?;

        Ok(filepath)
    }

    /// Download a media asset with progress callback.
    pub async fn download_with_progress(
        &self,
        asset: &MediaAsset,
        _on_progress: ProgressCallback,
    ) -> Result<PathBuf> {
        // For now, we don't have streaming progress - just download
        // Future enhancement: implement streaming download with progress
        self.download(asset).await
    }

    /// Download a file from URL to a path.
    async fn download_file(&self, url: &str, path: &Path, media_type: MediaType) -> Result<()> {
        // Validate URL before making request (SSRF prevention)
        validate_url(url)?;

        let response = self.client.get_raw(url).await?;

        if !response.status().is_success() {
            return Err(DxError::Download {
                url: url.to_string(),
                message: format!("HTTP {}", response.status()),
            });
        }

        // Verify content-type matches expected media type
        if let Some(content_type) = response.headers().get("content-type") {
            if let Ok(ct_str) = content_type.to_str() {
                // Log warning but don't fail - some servers return wrong content-types
                if let Err(e) = verify_content_type(ct_str, media_type) {
                    tracing::warn!("Content-type mismatch for {}: {}", url, e);
                }
            }
        }

        let bytes = response.bytes().await.map_err(|e| DxError::Download {
            url: url.to_string(),
            message: format!("Failed to read response body: {}", e),
        })?;

        tokio::fs::write(path, &bytes).await.map_err(|e| DxError::FileIo {
            path: path.to_path_buf(),
            message: format!("Failed to write file: {}", e),
            source: Some(e),
        })?;

        Ok(())
    }

    /// Generate a filename for an asset.
    fn generate_filename(&self, asset: &MediaAsset) -> String {
        // Sanitize the ID to be a valid filename
        let sanitized_id = self.sanitize_filename(&asset.id);
        let extension = self.guess_extension(asset);
        format!("{}-{}.{}", asset.provider, sanitized_id, extension)
    }

    /// Sanitize a string to be a valid filename.
    fn sanitize_filename(&self, input: &str) -> String {
        // Replace invalid characters with underscores
        input
            .chars()
            .map(|c| match c {
                '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
                c if c.is_ascii_control() => '_',
                c => c,
            })
            .collect::<String>()
            // Limit length to avoid filesystem issues
            .chars()
            .take(100)
            .collect()
    }

    /// Guess the file extension from the asset.
    fn guess_extension(&self, asset: &MediaAsset) -> &'static str {
        // Try to extract from URL first
        if let Some(ext) = self.extension_from_url(&asset.download_url) {
            return ext;
        }

        // Fall back to media type default
        match asset.media_type {
            crate::types::MediaType::Image => "jpg",
            crate::types::MediaType::Video => "mp4",
            crate::types::MediaType::Audio => "mp3",
            crate::types::MediaType::Gif => "gif",
            crate::types::MediaType::Vector => "svg",
            crate::types::MediaType::Document => "pdf",
            crate::types::MediaType::Data => "json",
            crate::types::MediaType::Model3D => "glb",
            crate::types::MediaType::Code => "txt",
            crate::types::MediaType::Text => "txt",
        }
    }

    /// Extract extension from URL.
    fn extension_from_url(&self, url: &str) -> Option<&'static str> {
        let url_lower = url.to_lowercase();

        // Check common image formats
        if url_lower.contains(".jpg") || url_lower.contains(".jpeg") {
            return Some("jpg");
        }
        if url_lower.contains(".png") {
            return Some("png");
        }
        if url_lower.contains(".gif") {
            return Some("gif");
        }
        if url_lower.contains(".webp") {
            return Some("webp");
        }
        if url_lower.contains(".svg") {
            return Some("svg");
        }

        // Check video formats
        if url_lower.contains(".mp4") {
            return Some("mp4");
        }
        if url_lower.contains(".webm") {
            return Some("webm");
        }
        if url_lower.contains(".mov") {
            return Some("mov");
        }

        None
    }

    /// Get the default download directory.
    #[must_use]
    pub fn download_dir(&self) -> &Path {
        &self.download_dir
    }
}

impl Default for Downloader {
    fn default() -> Self {
        Self::new(&Config::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::MediaType;

    #[test]
    fn test_generate_filename() {
        let downloader = Downloader::default();
        let asset = MediaAsset::builder()
            .id("12345")
            .provider("unsplash")
            .media_type(MediaType::Image)
            .title("Test Image")
            .download_url("https://example.com/image.jpg")
            .source_url("https://unsplash.com/photos/12345")
            .build()
            .expect("test asset should build");

        let filename = downloader.generate_filename(&asset);
        assert_eq!(filename, "unsplash-12345.jpg");
    }

    #[test]
    fn test_extension_from_url() {
        let downloader = Downloader::default();

        assert_eq!(downloader.extension_from_url("https://example.com/image.jpg"), Some("jpg"));
        assert_eq!(downloader.extension_from_url("https://example.com/image.PNG"), Some("png"));
        assert_eq!(downloader.extension_from_url("https://example.com/video.mp4"), Some("mp4"));
        assert_eq!(downloader.extension_from_url("https://example.com/unknown"), None);
    }
}
