//! Library of Congress provider implementation.
//!
//! [Library of Congress API](https://loc.gov/apis)
//!
//! Provides access to 3+ million public domain images from the Library of Congress.

use async_trait::async_trait;
use serde::Deserialize;
use std::time::Duration;

use crate::config::Config;
use crate::error::Result;
use crate::http::{HttpClient, ResponseExt};
use crate::providers::traits::{Provider, ProviderInfo};
use crate::types::{License, MediaAsset, MediaType, RateLimitConfig, SearchQuery, SearchResult};

/// Library of Congress provider for public domain historical media.
/// Access to 3M+ public domain images, documents, and historical materials.
#[derive(Debug)]
pub struct LibraryOfCongressProvider {
    client: HttpClient,
}

impl LibraryOfCongressProvider {
    /// Create a new Library of Congress provider.
    #[must_use]
    pub fn new(config: &Config) -> Self {
        let client = HttpClient::with_config(
            Self::RATE_LIMIT,
            config.retry_attempts,
            Duration::from_secs(config.timeout_secs),
        )
        .unwrap_or_default();

        Self { client }
    }

    /// Rate limit: Unlimited but be respectful
    const RATE_LIMIT: RateLimitConfig = RateLimitConfig::new(100, 60);
}

#[async_trait]
impl Provider for LibraryOfCongressProvider {
    fn name(&self) -> &'static str {
        "loc"
    }

    fn display_name(&self) -> &'static str {
        "Library of Congress"
    }

    fn supported_media_types(&self) -> &[MediaType] {
        &[
            MediaType::Image,
            MediaType::Document,
            MediaType::Audio,
            MediaType::Video,
        ]
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
        "https://loc.gov"
    }

    async fn search(&self, query: &SearchQuery) -> Result<SearchResult> {
        let url = format!("{}/search/", self.base_url());

        let format_filter = match query.media_type {
            Some(MediaType::Image) => "photo,print,drawing",
            Some(MediaType::Audio) => "audio",
            Some(MediaType::Video) => "film,video",
            Some(MediaType::Document) => "manuscript,book",
            _ => "photo,print,drawing",
        };

        let count_str = query.count.min(100).to_string();
        let page_str = query.page.to_string();

        let params = [
            ("q", query.query.as_str()),
            ("fo", "json"),
            ("fa", format_filter),
            ("c", count_str.as_str()),
            ("sp", page_str.as_str()),
        ];

        let response = self.client.get_with_query(&url, &params, &[]).await?;

        let api_response: LocSearchResponse = response.json_or_error().await?;

        let assets: Vec<MediaAsset> = api_response
            .results
            .into_iter()
            .filter_map(|item| {
                let image_url = item.image_url.first().cloned()?;

                Some(
                    MediaAsset::builder()
                        .id(item.id.unwrap_or_default())
                        .provider("loc")
                        .media_type(MediaType::Image)
                        .title(item.title.unwrap_or_else(|| "Library of Congress Item".to_string()))
                        .download_url(image_url.clone())
                        .preview_url(image_url)
                        .source_url(item.url.unwrap_or_default())
                        .author(item.contributor.unwrap_or_default().join(", "))
                        .license(License::PublicDomain)
                        .build_or_log(),
                )
            })
            .flatten()
            .collect();

        Ok(SearchResult {
            query: query.query.clone(),
            media_type: query.media_type,
            total_count: api_response.pagination.total.unwrap_or(0),
            assets,
            providers_searched: vec!["loc".to_string()],
            provider_errors: vec![],
            duration_ms: 0,
            provider_timings: Default::default(),
        })
    }
}

impl ProviderInfo for LibraryOfCongressProvider {
    fn description(&self) -> &'static str {
        "The Library of Congress - 3M+ public domain images, maps, and historical documents"
    }

    fn api_key_url(&self) -> &'static str {
        "https://loc.gov/apis"
    }

    fn default_license(&self) -> &'static str {
        "Public Domain"
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// API RESPONSE TYPES
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Deserialize)]
struct LocSearchResponse {
    results: Vec<LocItem>,
    pagination: LocPagination,
}

#[derive(Debug, Deserialize)]
struct LocItem {
    id: Option<String>,
    title: Option<String>,
    url: Option<String>,
    #[serde(default)]
    image_url: Vec<String>,
    #[serde(default)]
    contributor: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct LocPagination {
    total: Option<usize>,
}

// ═══════════════════════════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_metadata() {
        let config = Config::default();
        let provider = LibraryOfCongressProvider::new(&config);

        assert_eq!(provider.name(), "loc");
        assert_eq!(provider.display_name(), "Library of Congress");
        assert!(!provider.requires_api_key());
        assert!(provider.is_available());
    }

    #[test]
    fn test_supported_media_types() {
        let config = Config::default();
        let provider = LibraryOfCongressProvider::new(&config);

        let types = provider.supported_media_types();
        assert!(types.contains(&MediaType::Image));
        assert!(types.contains(&MediaType::Document));
    }
}
