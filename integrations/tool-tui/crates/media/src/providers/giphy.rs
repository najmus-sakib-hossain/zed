//! Giphy provider implementation.
//!
//! [Giphy API Documentation](https://developers.giphy.com/docs/api)
//!
//! Provides access to millions of GIFs and stickers.

use async_trait::async_trait;
use serde::Deserialize;
use std::time::Duration;

use crate::config::Config;
use crate::error::Result;
use crate::http::{HttpClient, ResponseExt};
use crate::providers::traits::{Provider, ProviderInfo};
use crate::types::{License, MediaAsset, MediaType, RateLimitConfig, SearchQuery, SearchResult};

/// Giphy provider for GIFs and stickers.
/// Access to millions of animated GIFs.
#[derive(Debug)]
pub struct GiphyProvider {
    api_key: Option<String>,
    client: HttpClient,
}

impl GiphyProvider {
    /// Create a new Giphy provider.
    #[must_use]
    pub fn new(config: &Config) -> Self {
        let client = HttpClient::with_config(
            Self::RATE_LIMIT,
            config.retry_attempts,
            Duration::from_secs(config.timeout_secs),
        )
        .unwrap_or_default();

        Self {
            api_key: config.giphy_api_key.clone(),
            client,
        }
    }

    /// Rate limit: 42 requests per hour for free tier, 1000 for production
    const RATE_LIMIT: RateLimitConfig = RateLimitConfig::new(42, 3600);
}

#[async_trait]
impl Provider for GiphyProvider {
    fn name(&self) -> &'static str {
        "giphy"
    }

    fn display_name(&self) -> &'static str {
        "Giphy"
    }

    fn supported_media_types(&self) -> &[MediaType] {
        &[MediaType::Gif]
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
        "https://api.giphy.com/v1"
    }

    async fn search(&self, query: &SearchQuery) -> Result<SearchResult> {
        let Some(ref api_key) = self.api_key else {
            return Err(crate::error::DxError::MissingApiKey {
                provider: "giphy".to_string(),
                env_var: "GIPHY_API_KEY".to_string(),
            });
        };

        let url = format!("{}/gifs/search", self.base_url());

        let offset = ((query.page - 1) * query.count).to_string();
        let limit = query.count.min(50).to_string();

        let params = [
            ("api_key", api_key.as_str()),
            ("q", query.query.as_str()),
            ("offset", &offset),
            ("limit", &limit),
            ("rating", "g"), // Safe for work
        ];

        let response = self.client.get_with_query(&url, &params, &[]).await?;

        let api_response: GiphySearchResponse = response.json_or_error().await?;

        let assets: Vec<MediaAsset> = api_response
            .data
            .into_iter()
            .map(|gif| {
                // Prefer original size, fall back to downsized
                let original = &gif.images.original;
                let download_url = original.url.clone().unwrap_or_default();
                let preview_url = gif.images.fixed_height.url.clone().unwrap_or_default();

                let width =
                    original.width.as_ref().and_then(|w| w.parse::<u32>().ok()).unwrap_or(0);
                let height =
                    original.height.as_ref().and_then(|h| h.parse::<u32>().ok()).unwrap_or(0);

                MediaAsset::builder()
                    .id(gif.id)
                    .provider("giphy")
                    .media_type(MediaType::Gif)
                    .title(gif.title.unwrap_or_else(|| "Giphy GIF".to_string()))
                    .download_url(download_url)
                    .preview_url(preview_url)
                    .source_url(gif.url)
                    .author(gif.username.unwrap_or_else(|| "Unknown".to_string()))
                    .license(License::Other("Giphy".to_string()))
                    .dimensions(width, height)
                    .build_or_log()
            })
            .flatten()
            .collect();

        Ok(SearchResult {
            query: query.query.clone(),
            media_type: query.media_type,
            total_count: api_response.pagination.total_count,
            assets,
            providers_searched: vec!["giphy".to_string()],
            provider_errors: vec![],
            duration_ms: 0,
            provider_timings: Default::default(),
        })
    }
}

impl ProviderInfo for GiphyProvider {
    fn description(&self) -> &'static str {
        "World's largest library of animated GIFs and stickers"
    }

    fn api_key_url(&self) -> &'static str {
        "https://developers.giphy.com/"
    }

    fn default_license(&self) -> &'static str {
        "Giphy Terms of Service"
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// API RESPONSE TYPES
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct GiphySearchResponse {
    data: Vec<GiphyGif>,
    pagination: GiphyPagination,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct GiphyGif {
    id: String,
    url: String,
    title: Option<String>,
    username: Option<String>,
    images: GiphyImages,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct GiphyImages {
    original: GiphyImage,
    fixed_height: GiphyImage,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct GiphyImage {
    url: Option<String>,
    width: Option<String>,
    height: Option<String>,
    size: Option<String>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct GiphyPagination {
    total_count: usize,
    count: usize,
    offset: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_metadata() {
        let config = Config::default_for_testing();
        let provider = GiphyProvider::new(&config);

        assert_eq!(provider.name(), "giphy");
        assert_eq!(provider.display_name(), "Giphy");
        assert!(provider.requires_api_key());
    }

    #[test]
    fn test_supported_media_types() {
        let config = Config::default_for_testing();
        let provider = GiphyProvider::new(&config);

        let types = provider.supported_media_types();
        assert!(types.contains(&MediaType::Gif));
    }
}
