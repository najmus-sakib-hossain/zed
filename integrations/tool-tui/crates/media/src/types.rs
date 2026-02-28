//! Core types for DX Media.
//!
//! This module defines the fundamental data structures used throughout the library.

use crate::error::DxError;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use strum::{Display, EnumString};

// ═══════════════════════════════════════════════════════════════════════════════
// SEARCH MODE
// ═══════════════════════════════════════════════════════════════════════════════

/// Search mode for controlling how providers are queried.
///
/// - **Quantity**: Fast mode with early-exit optimization. Returns results as soon as
///   enough are gathered (3x requested count). Ideal for quick searches.
/// - **Quality**: Waits for ALL providers to respond (or timeout). Gathers the most
///   comprehensive results from all sources. Better for thorough searches.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, Display, EnumString,
)]
#[strum(serialize_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum SearchMode {
    /// Fast mode: Early exit after gathering enough results (3x count).
    /// Skips slow providers for speed. DEFAULT mode.
    #[default]
    Quantity,
    /// Thorough mode: Wait for ALL providers to respond (or timeout).
    /// Gets comprehensive results from every source.
    Quality,
}

impl SearchMode {
    /// Returns true if this is quantity (fast) mode.
    #[must_use]
    pub fn is_quantity(&self) -> bool {
        matches!(self, Self::Quantity)
    }

    /// Returns true if this is quality (thorough) mode.
    #[must_use]
    pub fn is_quality(&self) -> bool {
        matches!(self, Self::Quality)
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// MEDIA TYPE
// ═══════════════════════════════════════════════════════════════════════════════

/// Supported media types for search and download.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Display, EnumString)]
#[strum(serialize_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum MediaType {
    /// Photographs and images (JPEG, PNG, WebP, etc.)
    Image,
    /// Video files (MP4, WebM, etc.)
    Video,
    /// Audio files (MP3, WAV, FLAC, etc.)
    Audio,
    /// GIF animations
    Gif,
    /// Vector graphics (SVG)
    Vector,
    /// Documents (PDF, Word, etc.)
    Document,
    /// Data files (JSON, CSV, datasets)
    Data,
    /// 3D models (OBJ, FBX, GLTF)
    Model3D,
    /// Code snippets and templates
    Code,
    /// Text content (articles, quotes)
    Text,
}

impl MediaType {
    /// Returns all available media types.
    #[must_use]
    pub fn all() -> &'static [MediaType] {
        &[
            Self::Image,
            Self::Video,
            Self::Audio,
            Self::Gif,
            Self::Vector,
            Self::Document,
            Self::Data,
            Self::Model3D,
            Self::Code,
            Self::Text,
        ]
    }

    /// Returns file extensions typically associated with this media type.
    #[must_use]
    pub fn extensions(&self) -> &'static [&'static str] {
        match self {
            Self::Image => &["jpg", "jpeg", "png", "webp", "avif", "bmp", "tiff"],
            Self::Video => &["mp4", "webm", "mov", "avi", "mkv"],
            Self::Audio => &["mp3", "wav", "flac", "ogg", "aac", "m4a"],
            Self::Gif => &["gif"],
            Self::Vector => &["svg", "eps", "ai"],
            Self::Document => &["pdf", "doc", "docx", "ppt", "pptx", "xls", "xlsx"],
            Self::Data => &["json", "csv", "xml", "yaml", "parquet"],
            Self::Model3D => &["obj", "fbx", "gltf", "glb", "blend"],
            Self::Code => &["rs", "py", "js", "ts", "go", "java", "cpp"],
            Self::Text => &["txt", "md", "rst"],
        }
    }

    /// Returns the media type as a lowercase string.
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Image => "image",
            Self::Video => "video",
            Self::Audio => "audio",
            Self::Gif => "gif",
            Self::Vector => "vector",
            Self::Document => "document",
            Self::Data => "data",
            Self::Model3D => "model3d",
            Self::Code => "code",
            Self::Text => "text",
        }
    }

    /// Returns the plural form of the media type (for directory names).
    #[must_use]
    pub fn as_plural_str(&self) -> &'static str {
        match self {
            Self::Image => "images",
            Self::Video => "videos",
            Self::Audio => "audio",
            Self::Gif => "gifs",
            Self::Vector => "vectors",
            Self::Document => "documents",
            Self::Data => "data",
            Self::Model3D => "models",
            Self::Code => "code",
            Self::Text => "text",
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// LICENSE
// ═══════════════════════════════════════════════════════════════════════════════

/// License types for media assets.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Display)]
pub enum License {
    /// Creative Commons Zero - Public Domain
    #[strum(serialize = "CC0")]
    Cc0,
    /// Creative Commons Attribution
    #[strum(serialize = "CC-BY")]
    CcBy,
    /// Creative Commons Attribution ShareAlike
    #[strum(serialize = "CC-BY-SA")]
    CcBySa,
    /// Creative Commons Attribution NonCommercial
    #[strum(serialize = "CC-BY-NC")]
    CcByNc,
    /// Public Domain
    PublicDomain,
    /// Unsplash License (free for commercial use with attribution)
    Unsplash,
    /// Pexels License (free for commercial use)
    Pexels,
    /// Pixabay License (free for commercial use)
    Pixabay,
    /// Custom license with description
    Custom(String),
    /// Other or unspecified license
    Other(String),
}

impl License {
    /// Returns the license as a string.
    #[must_use]
    pub fn as_str(&self) -> &str {
        match self {
            Self::Cc0 => "CC0",
            Self::CcBy => "CC-BY",
            Self::CcBySa => "CC-BY-SA",
            Self::CcByNc => "CC-BY-NC",
            Self::PublicDomain => "Public Domain",
            Self::Unsplash => "Unsplash License",
            Self::Pexels => "Pexels License",
            Self::Pixabay => "Pixabay License",
            Self::Custom(s) => s.as_str(),
            Self::Other(s) => s.as_str(),
        }
    }
}

impl Default for License {
    fn default() -> Self {
        Self::Other("Unknown".to_string())
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// MEDIA ASSET
// ═══════════════════════════════════════════════════════════════════════════════

/// A downloadable media asset from a provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaAsset {
    /// Unique identifier from the provider.
    pub id: String,
    /// Source provider name.
    pub provider: String,
    /// Media type.
    pub media_type: MediaType,
    /// Asset title or description.
    pub title: String,
    /// Direct download URL.
    pub download_url: String,
    /// Preview/thumbnail URL.
    pub preview_url: Option<String>,
    /// Web page URL on provider site.
    pub source_url: String,
    /// Author/creator name.
    pub author: Option<String>,
    /// Author profile URL.
    pub author_url: Option<String>,
    /// License information.
    pub license: License,
    /// Width in pixels (for images/videos).
    pub width: Option<u32>,
    /// Height in pixels (for images/videos).
    pub height: Option<u32>,
    /// File size in bytes.
    pub file_size: Option<u64>,
    /// MIME type.
    pub mime_type: Option<String>,
    /// Tags/keywords.
    pub tags: Vec<String>,
    /// When the asset was indexed.
    pub indexed_at: DateTime<Utc>,
}

impl MediaAsset {
    /// Create a new media asset builder.
    #[must_use]
    pub fn builder() -> MediaAssetBuilder {
        MediaAssetBuilder::default()
    }

    /// Get a safe filename for this asset.
    #[must_use]
    pub fn safe_filename(&self) -> String {
        let title = sanitize_filename::sanitize(&self.title);
        let title = if title.len() > 50 {
            &title[..50]
        } else {
            &title
        };

        let ext = self
            .download_url
            .split('.')
            .last()
            .and_then(|e| e.split('?').next())
            .unwrap_or("bin");

        format!("{}_{}_{}.{}", title, self.provider, &self.id[..8.min(self.id.len())], ext)
            .replace(' ', "_")
            .to_lowercase()
    }
}

/// Builder for [`MediaAsset`].
#[derive(Debug, Default)]
pub struct MediaAssetBuilder {
    id: Option<String>,
    provider: Option<String>,
    media_type: Option<MediaType>,
    title: Option<String>,
    download_url: Option<String>,
    preview_url: Option<String>,
    source_url: Option<String>,
    author: Option<String>,
    author_url: Option<String>,
    license: Option<License>,
    width: Option<u32>,
    height: Option<u32>,
    file_size: Option<u64>,
    mime_type: Option<String>,
    tags: Vec<String>,
}

impl MediaAssetBuilder {
    /// Set the asset ID.
    #[must_use]
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// Set the provider name.
    #[must_use]
    pub fn provider(mut self, provider: impl Into<String>) -> Self {
        self.provider = Some(provider.into());
        self
    }

    /// Set the media type.
    #[must_use]
    pub fn media_type(mut self, media_type: MediaType) -> Self {
        self.media_type = Some(media_type);
        self
    }

    /// Set the title.
    #[must_use]
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Set the download URL.
    #[must_use]
    pub fn download_url(mut self, url: impl Into<String>) -> Self {
        self.download_url = Some(url.into());
        self
    }

    /// Set the preview URL.
    #[must_use]
    pub fn preview_url(mut self, url: impl Into<String>) -> Self {
        self.preview_url = Some(url.into());
        self
    }

    /// Set the source URL.
    #[must_use]
    pub fn source_url(mut self, url: impl Into<String>) -> Self {
        self.source_url = Some(url.into());
        self
    }

    /// Set the author.
    #[must_use]
    pub fn author(mut self, author: impl Into<String>) -> Self {
        self.author = Some(author.into());
        self
    }

    /// Set the author URL.
    #[must_use]
    pub fn author_url(mut self, url: impl Into<String>) -> Self {
        self.author_url = Some(url.into());
        self
    }

    /// Set the license.
    #[must_use]
    pub fn license(mut self, license: License) -> Self {
        self.license = Some(license);
        self
    }

    /// Set the dimensions.
    #[must_use]
    pub fn dimensions(mut self, width: u32, height: u32) -> Self {
        self.width = Some(width);
        self.height = Some(height);
        self
    }

    /// Set the file size.
    #[must_use]
    pub fn file_size(mut self, size: u64) -> Self {
        self.file_size = Some(size);
        self
    }

    /// Set the MIME type.
    #[must_use]
    pub fn mime_type(mut self, mime: impl Into<String>) -> Self {
        self.mime_type = Some(mime.into());
        self
    }

    /// Set the tags.
    #[must_use]
    pub fn tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    /// Build the media asset.
    ///
    /// # Errors
    ///
    /// Returns `DxError::BuilderValidation` if required fields (id, provider, media_type,
    /// title, download_url, source_url) are not set.
    pub fn build(self) -> Result<MediaAsset, DxError> {
        Ok(MediaAsset {
            id: self.id.ok_or(DxError::builder_validation("id"))?,
            provider: self.provider.ok_or(DxError::builder_validation("provider"))?,
            media_type: self.media_type.ok_or(DxError::builder_validation("media_type"))?,
            title: self.title.ok_or(DxError::builder_validation("title"))?,
            download_url: self.download_url.ok_or(DxError::builder_validation("download_url"))?,
            preview_url: self.preview_url,
            source_url: self.source_url.ok_or(DxError::builder_validation("source_url"))?,
            author: self.author,
            author_url: self.author_url,
            license: self.license.unwrap_or_default(),
            width: self.width,
            height: self.height,
            file_size: self.file_size,
            mime_type: self.mime_type,
            tags: self.tags,
            indexed_at: Utc::now(),
        })
    }

    /// Build the media asset, returning None if required fields are missing.
    ///
    /// This is useful in iterator chains where you want to silently skip invalid assets.
    ///
    /// # Deprecated
    ///
    /// This method is deprecated. Use [`build()`](Self::build) for explicit error handling
    /// or [`build_or_log()`](Self::build_or_log) for logging failures at debug level.
    #[deprecated(
        since = "1.0.0",
        note = "Use `build()` for explicit error handling or `build_or_log()` for logging failures"
    )]
    #[must_use]
    pub fn try_build(self) -> Option<MediaAsset> {
        match self.build() {
            Ok(asset) => Some(asset),
            Err(DxError::BuilderValidation { field }) => {
                tracing::warn!(
                    missing_field = %field,
                    "MediaAssetBuilder.build_or_log() failed: missing required field"
                );
                None
            }
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    "MediaAssetBuilder.build_or_log() failed"
                );
                None
            }
        }
    }

    /// Build the media asset, logging at debug level if required fields are missing.
    ///
    /// This is useful in iterator chains where you want to skip invalid assets
    /// while still having visibility into failures via debug logs.
    ///
    /// # Returns
    ///
    /// - `Some(MediaAsset)` if all required fields are set
    /// - `None` if any required field is missing (logs at debug level)
    ///
    /// # Example
    ///
    /// ```
    /// use dx_media::types::{MediaAsset, MediaType};
    ///
    /// let asset = MediaAsset::builder()
    ///     .id("123")
    ///     .provider("test")
    ///     .media_type(MediaType::Image)
    ///     .title("Test")
    ///     .download_url("https://example.com/image.jpg")
    ///     .source_url("https://example.com")
    ///     .build_or_log();
    ///
    /// assert!(asset.is_some());
    /// ```
    #[must_use]
    pub fn build_or_log(self) -> Option<MediaAsset> {
        match self.build() {
            Ok(asset) => Some(asset),
            Err(DxError::BuilderValidation { field }) => {
                tracing::debug!(
                    missing_field = %field,
                    "MediaAssetBuilder.build_or_log() skipping asset: missing required field"
                );
                None
            }
            Err(e) => {
                tracing::debug!(
                    error = %e,
                    "MediaAssetBuilder.build_or_log() skipping asset"
                );
                None
            }
        }
    }

    /// Build the media asset, panicking if required fields are missing.
    ///
    /// Use only in tests or when you've validated inputs.
    ///
    /// # Panics
    ///
    /// Panics if required fields (id, provider, media_type, title, download_url, source_url)
    /// are not set.
    #[cfg(test)]
    #[must_use]
    pub fn build_unchecked(self) -> MediaAsset {
        self.build().expect("MediaAssetBuilder missing required fields")
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// SEARCH QUERY
// ═══════════════════════════════════════════════════════════════════════════════

/// Search query parameters.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchQuery {
    /// Search query string.
    pub query: String,
    /// Media type to search for (None = all types).
    pub media_type: Option<MediaType>,
    /// Maximum number of results.
    pub count: usize,
    /// Page number (1-indexed).
    pub page: usize,
    /// Specific providers to search (empty = all).
    pub providers: Vec<String>,
    /// Minimum width filter.
    pub min_width: Option<u32>,
    /// Minimum height filter.
    pub min_height: Option<u32>,
    /// Orientation filter.
    pub orientation: Option<Orientation>,
    /// Color filter (hex or name).
    pub color: Option<String>,
    /// Search mode (Quantity=fast early-exit, Quality=wait for all).
    #[serde(default)]
    pub mode: SearchMode,
}

impl SearchQuery {
    /// Create a new search query.
    #[must_use]
    pub fn new(query: impl Into<String>) -> Self {
        Self {
            query: query.into(),
            media_type: None,
            count: 10,
            page: 1,
            providers: Vec::new(),
            min_width: None,
            min_height: None,
            orientation: None,
            color: None,
            mode: SearchMode::default(),
        }
    }

    /// Create a search query for a specific media type.
    #[must_use]
    pub fn for_type(query: impl Into<String>, media_type: MediaType) -> Self {
        Self {
            query: query.into(),
            media_type: Some(media_type),
            count: 10,
            page: 1,
            providers: Vec::new(),
            min_width: None,
            min_height: None,
            orientation: None,
            color: None,
            mode: SearchMode::default(),
        }
    }

    /// Set the media type filter.
    #[must_use]
    pub fn media_type(mut self, media_type: MediaType) -> Self {
        self.media_type = Some(media_type);
        self
    }

    /// Set the result count.
    #[must_use]
    pub fn count(mut self, count: usize) -> Self {
        self.count = count;
        self
    }

    /// Set the page number.
    #[must_use]
    pub fn page(mut self, page: usize) -> Self {
        self.page = page;
        self
    }

    /// Set the search mode.
    #[must_use]
    pub fn mode(mut self, mode: SearchMode) -> Self {
        self.mode = mode;
        self
    }

    /// Set specific providers to search.
    #[must_use]
    pub fn providers(mut self, providers: Vec<String>) -> Self {
        self.providers = providers;
        self
    }

    /// Set minimum dimensions.
    #[must_use]
    pub fn min_dimensions(mut self, width: u32, height: u32) -> Self {
        self.min_width = Some(width);
        self.min_height = Some(height);
        self
    }

    /// Set orientation filter.
    #[must_use]
    pub fn orientation(mut self, orientation: Orientation) -> Self {
        self.orientation = Some(orientation);
        self
    }
}

/// Image orientation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Display, EnumString)]
#[strum(serialize_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum Orientation {
    /// Landscape (wider than tall).
    Landscape,
    /// Portrait (taller than wide).
    Portrait,
    /// Square (equal dimensions).
    Square,
}

// ═══════════════════════════════════════════════════════════════════════════════
// SEARCH RESULT
// ═══════════════════════════════════════════════════════════════════════════════

/// Results from a search operation.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SearchResult {
    /// The original query.
    #[serde(default)]
    pub query: String,
    /// Media type searched (None if all types).
    #[serde(default)]
    pub media_type: Option<MediaType>,
    /// Total results available (across all pages).
    #[serde(default)]
    pub total_count: usize,
    /// Assets returned in this page.
    #[serde(default)]
    pub assets: Vec<MediaAsset>,
    /// Providers that were searched.
    #[serde(default)]
    pub providers_searched: Vec<String>,
    /// Providers that failed (with error messages).
    #[serde(default)]
    pub provider_errors: Vec<(String, String)>,
    /// Search duration in milliseconds.
    #[serde(default)]
    pub duration_ms: u64,
    /// Per-provider timing in milliseconds for debugging.
    #[serde(default)]
    pub provider_timings: std::collections::HashMap<String, u64>,
}

impl SearchResult {
    /// Create a new empty search result.
    #[must_use]
    pub fn new(query: impl Into<String>) -> Self {
        Self {
            query: query.into(),
            media_type: None,
            total_count: 0,
            assets: Vec::new(),
            providers_searched: Vec::new(),
            provider_errors: Vec::new(),
            duration_ms: 0,
            provider_timings: std::collections::HashMap::new(),
        }
    }

    /// Create a new empty search result for a specific media type.
    #[must_use]
    pub fn for_type(query: impl Into<String>, media_type: MediaType) -> Self {
        Self {
            query: query.into(),
            media_type: Some(media_type),
            total_count: 0,
            assets: Vec::new(),
            providers_searched: Vec::new(),
            provider_errors: Vec::new(),
            duration_ms: 0,
            provider_timings: std::collections::HashMap::new(),
        }
    }

    /// Check if the search returned any results.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.assets.is_empty()
    }

    /// Get the number of assets in this result.
    #[must_use]
    pub fn len(&self) -> usize {
        self.assets.len()
    }

    /// Merge another search result into this one.
    pub fn merge(&mut self, other: SearchResult) {
        self.total_count += other.total_count;
        self.assets.extend(other.assets);
        self.providers_searched.extend(other.providers_searched);
        self.provider_errors.extend(other.provider_errors);
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// RATE LIMIT CONFIG
// ═══════════════════════════════════════════════════════════════════════════════

/// Rate limit configuration for a provider.
#[derive(Debug, Clone, Copy)]
pub struct RateLimitConfig {
    /// Maximum requests allowed.
    pub requests: u32,
    /// Time period in seconds.
    pub period_secs: u64,
}

impl RateLimitConfig {
    /// Create a new rate limit configuration.
    #[must_use]
    pub const fn new(requests: u32, period_secs: u64) -> Self {
        Self {
            requests,
            period_secs,
        }
    }

    /// No rate limiting.
    #[must_use]
    pub const fn unlimited() -> Self {
        Self {
            requests: u32::MAX,
            period_secs: 1,
        }
    }

    /// Calculate delay between requests in milliseconds.
    #[must_use]
    pub const fn delay_ms(&self) -> u64 {
        if self.requests == 0 {
            return 0;
        }
        (self.period_secs * 1000) / self.requests as u64
    }

    /// Check if rate limiting is enabled.
    #[must_use]
    pub const fn is_limited(&self) -> bool {
        self.requests != u32::MAX
    }

    /// Get the number of requests per window (alias for requests).
    #[must_use]
    pub const fn requests_per_window(&self) -> u32 {
        self.requests
    }

    /// Get the window duration in seconds (alias for period_secs).
    #[must_use]
    pub const fn window_secs(&self) -> u64 {
        self.period_secs
    }
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self::new(100, 60) // 100 requests per minute
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// HEALTH CHECK
// ═══════════════════════════════════════════════════════════════════════════════

/// Result of a health check for a single provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckResult {
    /// Provider name.
    pub provider: String,
    /// Whether the provider is available and responding.
    pub available: bool,
    /// Response latency in milliseconds (if available).
    pub latency_ms: Option<u64>,
    /// Error message if the check failed.
    pub error: Option<String>,
    /// Circuit breaker state.
    pub circuit_state: String,
}

impl HealthCheckResult {
    /// Create a successful health check result.
    #[must_use]
    pub fn success(provider: impl Into<String>, latency_ms: u64, circuit_state: &str) -> Self {
        Self {
            provider: provider.into(),
            available: true,
            latency_ms: Some(latency_ms),
            error: None,
            circuit_state: circuit_state.to_string(),
        }
    }

    /// Create a failed health check result.
    #[must_use]
    pub fn failure(
        provider: impl Into<String>,
        error: impl Into<String>,
        circuit_state: &str,
    ) -> Self {
        Self {
            provider: provider.into(),
            available: false,
            latency_ms: None,
            error: Some(error.into()),
            circuit_state: circuit_state.to_string(),
        }
    }
}

/// Comprehensive health report for all providers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthReport {
    /// Health check results for each provider.
    pub providers: Vec<HealthCheckResult>,
    /// Timestamp when the health check was performed.
    pub timestamp: DateTime<Utc>,
    /// Total number of providers checked.
    pub total_providers: usize,
    /// Number of healthy providers.
    pub healthy_count: usize,
    /// Number of unhealthy providers.
    pub unhealthy_count: usize,
    /// Overall health check duration in milliseconds.
    pub duration_ms: u64,
}

impl HealthReport {
    /// Create a new health report.
    #[must_use]
    pub fn new(providers: Vec<HealthCheckResult>, duration_ms: u64) -> Self {
        let total_providers = providers.len();
        let healthy_count = providers.iter().filter(|p| p.available).count();
        let unhealthy_count = total_providers - healthy_count;

        Self {
            providers,
            timestamp: Utc::now(),
            total_providers,
            healthy_count,
            unhealthy_count,
            duration_ms,
        }
    }

    /// Check if all providers are healthy.
    #[must_use]
    pub fn all_healthy(&self) -> bool {
        self.unhealthy_count == 0
    }

    /// Get the list of unhealthy providers.
    #[must_use]
    pub fn unhealthy_providers(&self) -> Vec<&HealthCheckResult> {
        self.providers.iter().filter(|p| !p.available).collect()
    }

    /// Get the list of healthy providers.
    #[must_use]
    pub fn healthy_providers(&self) -> Vec<&HealthCheckResult> {
        self.providers.iter().filter(|p| p.available).collect()
    }

    /// Get the average latency of healthy providers.
    #[must_use]
    pub fn average_latency_ms(&self) -> Option<u64> {
        let latencies: Vec<u64> = self.providers.iter().filter_map(|p| p.latency_ms).collect();

        if latencies.is_empty() {
            None
        } else {
            Some(latencies.iter().sum::<u64>() / latencies.len() as u64)
        }
    }
}
