//! Pixabay provider implementation.
//!
//! [Pixabay API Documentation](https://pixabay.com/api/docs/)

use async_trait::async_trait;
use serde::Deserialize;
use std::time::Duration;

use crate::config::Config;
use crate::error::Result;
use crate::http::{HttpClient, ResponseExt};
use crate::providers::traits::{Provider, ProviderInfo};
use crate::types::{License, MediaAsset, MediaType, RateLimitConfig, SearchQuery, SearchResult};

/// Pixabay provider for free images and videos.
#[derive(Debug)]
pub struct PixabayProvider {
    api_key: Option<String>,
    client: HttpClient,
}

impl PixabayProvider {
    /// Create a new Pixabay provider.
    #[must_use]
    pub fn new(config: &Config) -> Self {
        let client = HttpClient::with_config(
            Self::RATE_LIMIT,
            config.retry_attempts,
            Duration::from_secs(config.timeout_secs),
        )
        .unwrap_or_default();

        Self {
            api_key: config.pixabay_api_key.clone(),
            client,
        }
    }

    /// Rate limit: 100 requests per minute (generous for free tier).
    const RATE_LIMIT: RateLimitConfig = RateLimitConfig::new(100, 60);
}

#[async_trait]
impl Provider for PixabayProvider {
    fn name(&self) -> &'static str {
        "pixabay"
    }

    fn display_name(&self) -> &'static str {
        "Pixabay"
    }

    fn supported_media_types(&self) -> &[MediaType] {
        &[MediaType::Image, MediaType::Video, MediaType::Vector]
    }

    fn requires_api_key(&self) -> bool {
        true
    }

    fn rate_limit(&self) -> RateLimitConfig {
        Self::RATE_LIMIT
    }

    fn is_available(&self) -> bool {
        self.api_key.is_some()
    }

    fn base_url(&self) -> &'static str {
        "https://pixabay.com/api/"
    }

    async fn search(&self, query: &SearchQuery) -> Result<SearchResult> {
        let Some(ref api_key) = self.api_key else {
            return Err(crate::error::DxError::MissingApiKey {
                provider: "pixabay".to_string(),
                env_var: "PIXABAY_API_KEY".to_string(),
            });
        };

        // Use video endpoint for video searches
        if query.media_type == Some(MediaType::Video) {
            return self.search_videos(api_key, query).await;
        }

        let image_type = match query.media_type {
            Some(MediaType::Vector) => "vector",
            Some(MediaType::Image) | None => "all",
            _ => "all",
        };

        let params = [
            ("key", api_key.as_str()),
            ("q", query.query.as_str()),
            ("page", &query.page.to_string()),
            ("per_page", &query.count.min(200).to_string()), // Pixabay max is 200
            ("image_type", image_type),
            ("safesearch", "true"),
        ];

        let response = self.client.get_with_query(self.base_url(), &params, &[]).await?;

        let api_response: PixabaySearchResponse = response.json_or_error().await?;

        let assets: Vec<MediaAsset> = api_response
            .hits
            .into_iter()
            .filter_map(|hit| {
                let media_type = if hit.hit_type == "vector/svg" {
                    MediaType::Vector
                } else {
                    MediaType::Image
                };

                // Prefer large image URL, fall back to web format
                let download_url = hit
                    .large_image_url
                    .or(Some(hit.web_format_url.clone()))
                    .unwrap_or_else(|| hit.preview_url.clone());

                let tags: Vec<String> =
                    hit.tags.split(", ").map(|s| s.trim().to_string()).collect();

                MediaAsset::builder()
                    .id(hit.id.to_string())
                    .provider("pixabay")
                    .media_type(media_type)
                    .title(
                        tags.first()
                            .cloned()
                            .unwrap_or_else(|| format!("Pixabay Image {}", hit.id)),
                    )
                    .download_url(download_url)
                    .preview_url(hit.preview_url)
                    .source_url(hit.page_url)
                    .author(hit.user.clone())
                    .author_url(format!("https://pixabay.com/users/{}-{}/", hit.user, hit.user_id))
                    .license(License::Pixabay)
                    .dimensions(hit.image_width, hit.image_height)
                    .tags(tags)
                    .build_or_log()
            })
            .collect();

        Ok(SearchResult {
            query: query.query.clone(),
            media_type: query.media_type,
            total_count: api_response.total_hits,
            assets,
            providers_searched: vec!["pixabay".to_string()],
            provider_errors: vec![],
            duration_ms: 0,
            provider_timings: Default::default(),
        })
    }
}

impl PixabayProvider {
    /// Search for videos using Pixabay video API endpoint.
    async fn search_videos(&self, api_key: &str, query: &SearchQuery) -> Result<SearchResult> {
        let video_url = "https://pixabay.com/api/videos/";

        let params = [
            ("key", api_key),
            ("q", query.query.as_str()),
            ("page", &query.page.to_string()),
            ("per_page", &query.count.min(200).to_string()),
            ("safesearch", "true"),
        ];

        let response = self.client.get_with_query(video_url, &params, &[]).await?;

        let api_response: PixabayVideoSearchResponse = response.json_or_error().await?;

        let assets: Vec<MediaAsset> = api_response
            .hits
            .into_iter()
            .filter_map(|hit| {
                let tags: Vec<String> =
                    hit.tags.split(", ").map(|s| s.trim().to_string()).collect();

                // Get the best quality video URL available
                let download_url = hit
                    .videos
                    .large
                    .as_ref()
                    .or(hit.videos.medium.as_ref())
                    .or(hit.videos.small.as_ref())
                    .map(|v| v.url.clone())
                    .unwrap_or_default();

                let (width, height) = hit
                    .videos
                    .large
                    .as_ref()
                    .or(hit.videos.medium.as_ref())
                    .or(hit.videos.small.as_ref())
                    .map(|v| (v.width, v.height))
                    .unwrap_or((0, 0));

                // Use tiny video as preview
                let preview_url = hit.videos.tiny.as_ref().map(|v| v.url.clone());

                MediaAsset::builder()
                    .id(hit.id.to_string())
                    .provider("pixabay")
                    .media_type(MediaType::Video)
                    .title(
                        tags.first()
                            .cloned()
                            .unwrap_or_else(|| format!("Pixabay Video {}", hit.id)),
                    )
                    .download_url(download_url)
                    .preview_url(preview_url.unwrap_or_default())
                    .source_url(hit.page_url)
                    .author(hit.user.clone())
                    .author_url(format!("https://pixabay.com/users/{}-{}/", hit.user, hit.user_id))
                    .license(License::Pixabay)
                    .dimensions(width, height)
                    .tags(tags)
                    .build_or_log()
            })
            .collect();

        Ok(SearchResult {
            query: query.query.clone(),
            media_type: query.media_type,
            total_count: api_response.total_hits,
            assets,
            providers_searched: vec!["pixabay".to_string()],
            provider_errors: vec![],
            duration_ms: 0,
            provider_timings: Default::default(),
        })
    }
}

impl ProviderInfo for PixabayProvider {
    fn description(&self) -> &'static str {
        "Stunning royalty-free images & royalty-free stock"
    }

    fn api_key_url(&self) -> &'static str {
        "https://pixabay.com/api/docs/"
    }

    fn default_license(&self) -> &'static str {
        "Pixabay License (free for commercial use, no attribution required)"
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// API RESPONSE TYPES
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Deserialize)]
#[allow(dead_code)] // Fields used for JSON deserialization
struct PixabaySearchResponse {
    total: usize,
    #[serde(rename = "totalHits")]
    total_hits: usize,
    hits: Vec<PixabayHit>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)] // Fields used for JSON deserialization
struct PixabayHit {
    id: u64,

    #[serde(rename = "pageURL")]
    page_url: String,

    #[serde(rename = "type", default)]
    hit_type: String,

    tags: String,

    #[serde(rename = "previewURL")]
    preview_url: String,

    #[serde(rename = "previewWidth")]
    preview_width: u32,

    #[serde(rename = "previewHeight")]
    preview_height: u32,

    #[serde(rename = "webformatURL")]
    web_format_url: String,

    #[serde(rename = "webformatWidth")]
    web_format_width: u32,

    #[serde(rename = "webformatHeight")]
    web_format_height: u32,

    #[serde(rename = "largeImageURL")]
    large_image_url: Option<String>,

    #[serde(rename = "imageWidth")]
    image_width: u32,

    #[serde(rename = "imageHeight")]
    image_height: u32,

    #[serde(rename = "imageSize")]
    image_size: u64,

    views: u64,
    downloads: u64,
    likes: u64,

    user: String,

    #[serde(rename = "user_id")]
    user_id: u64,

    #[serde(rename = "userImageURL")]
    user_image_url: String,
}

// ═══════════════════════════════════════════════════════════════════════════════
// VIDEO API RESPONSE TYPES
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct PixabayVideoSearchResponse {
    total: usize,
    #[serde(rename = "totalHits")]
    total_hits: usize,
    hits: Vec<PixabayVideoHit>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct PixabayVideoHit {
    id: u64,

    #[serde(rename = "pageURL")]
    page_url: String,

    #[serde(rename = "type", default)]
    hit_type: String,

    tags: String,

    duration: u32,

    videos: PixabayVideoSizes,

    views: u64,
    downloads: u64,
    likes: u64,

    user: String,

    #[serde(rename = "user_id")]
    user_id: u64,

    #[serde(rename = "userImageURL")]
    user_image_url: String,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct PixabayVideoSizes {
    large: Option<PixabayVideoSize>,
    medium: Option<PixabayVideoSize>,
    small: Option<PixabayVideoSize>,
    tiny: Option<PixabayVideoSize>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct PixabayVideoSize {
    url: String,
    width: u32,
    height: u32,
    size: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_metadata() {
        let config = Config::default_for_testing();
        let provider = PixabayProvider::new(&config);

        assert_eq!(provider.name(), "pixabay");
        assert_eq!(provider.display_name(), "Pixabay");
        assert!(provider.requires_api_key());
        assert!(!provider.is_available());
    }

    #[test]
    fn test_supported_media_types() {
        let config = Config::default_for_testing();
        let provider = PixabayProvider::new(&config);

        let types = provider.supported_media_types();
        assert!(types.contains(&MediaType::Image));
        assert!(types.contains(&MediaType::Video));
        assert!(types.contains(&MediaType::Vector));
    }
}
