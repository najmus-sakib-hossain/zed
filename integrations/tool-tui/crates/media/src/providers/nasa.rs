//! NASA Images provider implementation.
//!
//! [NASA Images API Documentation](https://images.nasa.gov/docs/images.nasa.gov_api_docs.pdf)
//!
//! Provides access to NASA's image and video library with 140,000+ public domain assets.

use async_trait::async_trait;
use serde::Deserialize;
use std::time::Duration;

use crate::config::Config;
use crate::error::Result;
use crate::http::{HttpClient, ResponseExt};
use crate::providers::traits::{Provider, ProviderInfo};
use crate::types::{License, MediaAsset, MediaType, RateLimitConfig, SearchQuery, SearchResult};

/// NASA Images provider for space and science media.
/// Access to 140K+ public domain images and videos.
#[derive(Debug)]
pub struct NasaImagesProvider {
    client: HttpClient,
    /// Base URL for API requests (configurable for testing)
    base_url: String,
}

impl NasaImagesProvider {
    /// Default base URL for NASA Images API.
    const DEFAULT_BASE_URL: &'static str = "https://images-api.nasa.gov";

    /// Create a new NASA Images provider.
    #[must_use]
    pub fn new(config: &Config) -> Self {
        let client = HttpClient::with_config(
            Self::RATE_LIMIT,
            config.retry_attempts,
            Duration::from_secs(config.timeout_secs),
        )
        .unwrap_or_default();

        Self {
            client,
            base_url: Self::DEFAULT_BASE_URL.to_string(),
        }
    }

    /// Create a new NASA Images provider with a custom base URL.
    ///
    /// This is primarily useful for testing with mock servers.
    ///
    /// # Arguments
    ///
    /// * `base_url` - The base URL to use for API requests.
    #[must_use]
    pub fn with_base_url(base_url: &str) -> Self {
        let client = HttpClient::with_config(
            Self::RATE_LIMIT,
            0,                      // No retries for testing
            Duration::from_secs(5), // Short timeout for testing
        )
        .unwrap_or_default();

        Self {
            client,
            base_url: base_url.to_string(),
        }
    }

    /// Rate limit: Unlimited (but be respectful)
    const RATE_LIMIT: RateLimitConfig = RateLimitConfig::new(1000, 3600);

    /// Get the media type filter string for the API
    fn media_type_filter(media_type: Option<MediaType>) -> &'static str {
        match media_type {
            Some(MediaType::Image) => "image",
            Some(MediaType::Video) => "video",
            Some(MediaType::Audio) => "audio",
            _ => "image",
        }
    }

    /// Parse media type from string
    fn parse_media_type(s: &str) -> MediaType {
        match s {
            "video" => MediaType::Video,
            "audio" => MediaType::Audio,
            _ => MediaType::Image,
        }
    }
}

#[async_trait]
impl Provider for NasaImagesProvider {
    fn name(&self) -> &'static str {
        "nasa"
    }

    fn display_name(&self) -> &'static str {
        "NASA Images"
    }

    fn supported_media_types(&self) -> &[MediaType] {
        &[MediaType::Image, MediaType::Video, MediaType::Audio]
    }

    fn requires_api_key(&self) -> bool {
        false
    }

    fn rate_limit(&self) -> RateLimitConfig {
        Self::RATE_LIMIT
    }

    fn is_available(&self) -> bool {
        true
    }

    fn base_url(&self) -> &'static str {
        // Note: This returns the default static URL for trait compliance.
        // The actual search method uses self.base_url field which may be customized.
        Self::DEFAULT_BASE_URL
    }

    async fn search(&self, query: &SearchQuery) -> Result<SearchResult> {
        let url = format!("{}/search", self.base_url);

        let media_type = Self::media_type_filter(query.media_type);
        let page_str = query.page.to_string();
        let count_str = query.count.min(100).to_string();

        let params = [
            ("q", query.query.as_str()),
            ("media_type", media_type),
            ("page", &page_str),
            ("page_size", &count_str),
        ];

        let response = self.client.get_with_query(&url, &params, &[]).await?;

        let api_response: NasaSearchResponse = response.json_or_error().await?;

        let assets: Vec<MediaAsset> = api_response
            .collection
            .items
            .into_iter()
            .filter_map(|item| {
                let data = item.data.into_iter().next()?;
                let link = item.links.and_then(|l| l.into_iter().next());

                let preview_url = link.as_ref().map(|l| l.href.clone());

                // NASA assets are all public domain

                MediaAsset::builder()
                    .id(data.nasa_id)
                    .provider("nasa")
                    .media_type(Self::parse_media_type(&data.media_type))
                    .title(data.title)
                    .download_url(preview_url.clone().unwrap_or_default())
                    .preview_url(preview_url.unwrap_or_default())
                    .source_url(item.href)
                    .author(data.center.unwrap_or_else(|| "NASA".to_string()))
                    .license(License::PublicDomain)
                    .tags(data.keywords.unwrap_or_default())
                    .build_or_log()
            })
            .collect();

        Ok(SearchResult {
            query: query.query.clone(),
            media_type: query.media_type,
            total_count: api_response.collection.metadata.total_hits,
            assets,
            providers_searched: vec!["nasa".to_string()],
            provider_errors: vec![],
            duration_ms: 0,
            provider_timings: Default::default(),
        })
    }

    async fn get_by_id(&self, id: &str) -> Result<Option<MediaAsset>> {
        // NASA API: GET /search?nasa_id={id}
        let url = format!("{}/search", self.base_url);
        let params = [("nasa_id", id)];

        let response = self.client.get_with_query(&url, &params, &[]).await?;
        let api_response: NasaSearchResponse = response.json_or_error().await?;

        // Get the first (and should be only) result
        let item = api_response.collection.items.into_iter().next();

        if let Some(item) = item {
            let data = item.data.into_iter().next();
            if let Some(data) = data {
                let media_type = Self::parse_media_type(&data.media_type);
                let preview_url =
                    item.links.as_ref().and_then(|l| l.first()).map(|l| l.href.clone());

                // For videos/audio, fetch the actual media URL from the asset manifest
                let download_url = if matches!(media_type, MediaType::Video | MediaType::Audio) {
                    // Fetch the asset manifest from item.href
                    match self.client.get(&item.href).await {
                        Ok(manifest_response) => {
                            match manifest_response.json_or_error::<Vec<String>>().await {
                                Ok(urls) => {
                                    // Find the actual video/audio file (not thumbnail)
                                    urls.into_iter()
                                        .find(|u| {
                                            let lower = u.to_lowercase();
                                            if media_type == MediaType::Video {
                                                lower.ends_with(".mp4") || lower.ends_with(".mov")
                                            } else {
                                                lower.ends_with(".mp3") || lower.ends_with(".wav")
                                            }
                                        })
                                        .unwrap_or_else(|| item.href.clone())
                                }
                                Err(_) => item.href.clone(),
                            }
                        }
                        Err(_) => item.href.clone(),
                    }
                } else {
                    // For images, use the preview link
                    preview_url.clone().unwrap_or_else(|| item.href.clone())
                };

                return Ok(MediaAsset::builder()
                    .id(data.nasa_id)
                    .provider("nasa")
                    .media_type(media_type)
                    .title(data.title)
                    .download_url(download_url)
                    .preview_url(preview_url.unwrap_or_default())
                    .source_url(item.href)
                    .author(data.center.unwrap_or_else(|| "NASA".to_string()))
                    .license(License::PublicDomain)
                    .tags(data.keywords.unwrap_or_default())
                    .build_or_log());
            }
        }

        Ok(None)
    }
}

impl ProviderInfo for NasaImagesProvider {
    fn description(&self) -> &'static str {
        "NASA's official image and video library with space and science media"
    }

    fn api_key_url(&self) -> &'static str {
        "https://images.nasa.gov/docs/images.nasa.gov_api_docs.pdf"
    }

    fn default_license(&self) -> &'static str {
        "Public Domain (NASA media is not copyrighted)"
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// API RESPONSE TYPES
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct NasaSearchResponse {
    collection: NasaCollection,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct NasaCollection {
    items: Vec<NasaItem>,
    metadata: NasaMetadata,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct NasaMetadata {
    total_hits: usize,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct NasaItem {
    href: String,
    data: Vec<NasaItemData>,
    links: Option<Vec<NasaLink>>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct NasaItemData {
    nasa_id: String,
    title: String,
    media_type: String,
    description: Option<String>,
    center: Option<String>,
    date_created: Option<String>,
    #[serde(default)]
    keywords: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct NasaLink {
    href: String,
    rel: String,
    render: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_metadata() {
        let config = Config::default_for_testing();
        let provider = NasaImagesProvider::new(&config);

        assert_eq!(provider.name(), "nasa");
        assert_eq!(provider.display_name(), "NASA Images");
        assert!(!provider.requires_api_key());
        assert!(provider.is_available());
    }

    #[test]
    fn test_supported_media_types() {
        let config = Config::default_for_testing();
        let provider = NasaImagesProvider::new(&config);

        let types = provider.supported_media_types();
        assert!(types.contains(&MediaType::Image));
        assert!(types.contains(&MediaType::Video));
        assert!(types.contains(&MediaType::Audio));
    }
}
