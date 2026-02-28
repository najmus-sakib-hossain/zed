//! Pexels provider implementation.
//!
//! [Pexels API Documentation](https://www.pexels.com/api/documentation/)

use async_trait::async_trait;
use serde::Deserialize;
use std::time::Duration;

use crate::config::Config;
use crate::error::Result;
use crate::http::{HttpClient, ResponseExt};
use crate::providers::traits::{Provider, ProviderInfo};
use crate::types::{License, MediaAsset, MediaType, RateLimitConfig, SearchQuery, SearchResult};

/// Pexels provider for stock photos and videos.
#[derive(Debug)]
pub struct PexelsProvider {
    api_key: Option<String>,
    client: HttpClient,
}

impl PexelsProvider {
    /// Create a new Pexels provider.
    #[must_use]
    pub fn new(config: &Config) -> Self {
        let client = HttpClient::with_config(
            Self::RATE_LIMIT,
            config.retry_attempts,
            Duration::from_secs(config.timeout_secs),
        )
        .unwrap_or_default();

        Self {
            api_key: config.pexels_api_key.clone(),
            client,
        }
    }

    /// Rate limit: 200 requests per hour
    const RATE_LIMIT: RateLimitConfig = RateLimitConfig::new(200, 3600);
}

#[async_trait]
impl Provider for PexelsProvider {
    fn name(&self) -> &'static str {
        "pexels"
    }

    fn display_name(&self) -> &'static str {
        "Pexels"
    }

    fn supported_media_types(&self) -> &[MediaType] {
        &[MediaType::Image, MediaType::Video]
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
        "https://api.pexels.com/v1"
    }

    async fn search(&self, query: &SearchQuery) -> Result<SearchResult> {
        let Some(ref api_key) = self.api_key else {
            return Err(crate::error::DxError::MissingApiKey {
                provider: "pexels".to_string(),
                env_var: "PEXELS_API_KEY".to_string(),
            });
        };

        // Use video endpoint for video searches
        if query.media_type == Some(MediaType::Video) {
            return self.search_videos(api_key, query).await;
        }

        let url = format!("{}/search", self.base_url());

        let params = [
            ("query", query.query.as_str()),
            ("page", &query.page.to_string()),
            ("per_page", &query.count.min(80).to_string()), // Pexels max is 80
        ];

        let headers = [("Authorization", api_key.as_str())];

        let response = self.client.get_with_query(&url, &params, &headers).await?;

        let api_response: PexelsSearchResponse = response.json_or_error().await?;

        let assets: Vec<MediaAsset> = api_response
            .photos
            .into_iter()
            .filter_map(|photo| {
                // Choose the best quality available
                let download_url = photo.src.original.clone();
                let preview_url = photo.src.medium.clone();

                MediaAsset::builder()
                    .id(photo.id.to_string())
                    .provider("pexels")
                    .media_type(MediaType::Image)
                    .title(
                        photo
                            .alt
                            .filter(|s| !s.is_empty())
                            .unwrap_or_else(|| format!("Pexels Photo {}", photo.id)),
                    )
                    .download_url(download_url)
                    .preview_url(preview_url)
                    .source_url(photo.url)
                    .author(photo.photographer)
                    .author_url(photo.photographer_url)
                    .license(License::Pexels)
                    .dimensions(photo.width, photo.height)
                    .build_or_log()
            })
            .collect();

        Ok(SearchResult {
            query: query.query.clone(),
            media_type: query.media_type,
            total_count: api_response.total_results,
            assets,
            providers_searched: vec!["pexels".to_string()],
            provider_errors: vec![],
            duration_ms: 0,
            provider_timings: Default::default(),
        })
    }
}

impl PexelsProvider {
    /// Search for videos using Pexels video API endpoint.
    async fn search_videos(&self, api_key: &str, query: &SearchQuery) -> Result<SearchResult> {
        let url = "https://api.pexels.com/videos/search";

        let params = [
            ("query", query.query.as_str()),
            ("page", &query.page.to_string()),
            ("per_page", &query.count.min(80).to_string()),
        ];

        let headers = [("Authorization", api_key)];

        let response = self.client.get_with_query(url, &params, &headers).await?;

        let api_response: PexelsVideoSearchResponse = response.json_or_error().await?;

        let assets: Vec<MediaAsset> = api_response
            .videos
            .into_iter()
            .filter_map(|video| {
                // Get the best quality video file available (prefer HD)
                let best_file = video
                    .video_files
                    .iter()
                    .filter(|f| f.quality == "hd" || f.quality == "sd")
                    .max_by_key(|f| f.width.unwrap_or(0))
                    .or_else(|| video.video_files.first());

                let download_url = best_file.map(|f| f.link.clone()).unwrap_or_default();

                let (width, height) = best_file
                    .map(|f| (f.width.unwrap_or(video.width), f.height.unwrap_or(video.height)))
                    .unwrap_or((video.width, video.height));

                // Use video picture as preview
                let preview_url = video.video_pictures.first().map(|p| p.picture.clone());

                MediaAsset::builder()
                    .id(video.id.to_string())
                    .provider("pexels")
                    .media_type(MediaType::Video)
                    .title(format!("Pexels Video {}", video.id))
                    .download_url(download_url)
                    .preview_url(preview_url.unwrap_or_default())
                    .source_url(video.url)
                    .author(video.user.name.clone())
                    .author_url(video.user.url.clone())
                    .license(License::Pexels)
                    .dimensions(width, height)
                    .build_or_log()
            })
            .collect();

        Ok(SearchResult {
            query: query.query.clone(),
            media_type: query.media_type,
            total_count: api_response.total_results,
            assets,
            providers_searched: vec!["pexels".to_string()],
            provider_errors: vec![],
            duration_ms: 0,
            provider_timings: Default::default(),
        })
    }
}
impl ProviderInfo for PexelsProvider {
    fn description(&self) -> &'static str {
        "Free stock photos and videos shared by talented creators"
    }

    fn api_key_url(&self) -> &'static str {
        "https://www.pexels.com/api/"
    }

    fn default_license(&self) -> &'static str {
        "Pexels License (free for personal and commercial use)"
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// API RESPONSE TYPES
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Deserialize)]
#[allow(dead_code)] // Fields used for JSON deserialization
struct PexelsSearchResponse {
    total_results: usize,
    page: usize,
    per_page: usize,
    photos: Vec<PexelsPhoto>,
    #[serde(default)]
    next_page: Option<String>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)] // Fields used for JSON deserialization
struct PexelsPhoto {
    id: u64,
    width: u32,
    height: u32,
    url: String,
    photographer: String,
    photographer_url: String,
    #[serde(default)]
    photographer_id: u64,
    avg_color: Option<String>,
    src: PexelsPhotoSrc,
    alt: Option<String>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)] // Fields used for JSON deserialization
struct PexelsPhotoSrc {
    original: String,
    large2x: String,
    large: String,
    medium: String,
    small: String,
    portrait: String,
    landscape: String,
    tiny: String,
}

// ═══════════════════════════════════════════════════════════════════════════════
// VIDEO API RESPONSE TYPES
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct PexelsVideoSearchResponse {
    total_results: usize,
    page: usize,
    per_page: usize,
    videos: Vec<PexelsVideo>,
    #[serde(default)]
    next_page: Option<String>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct PexelsVideo {
    id: u64,
    width: u32,
    height: u32,
    url: String,
    duration: u32,
    user: PexelsVideoUser,
    video_files: Vec<PexelsVideoFile>,
    video_pictures: Vec<PexelsVideoPicture>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct PexelsVideoUser {
    id: u64,
    name: String,
    url: String,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct PexelsVideoFile {
    id: u64,
    quality: String,
    file_type: String,
    width: Option<u32>,
    height: Option<u32>,
    link: String,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct PexelsVideoPicture {
    id: u64,
    picture: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_metadata() {
        let config = Config::default_for_testing();
        let provider = PexelsProvider::new(&config);

        assert_eq!(provider.name(), "pexels");
        assert_eq!(provider.display_name(), "Pexels");
        assert!(provider.requires_api_key());
        assert!(!provider.is_available());
    }
}
