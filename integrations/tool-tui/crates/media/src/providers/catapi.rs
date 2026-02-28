//! The Cat API provider implementation.
//!
//! [The Cat API](https://thecatapi.com/)
//!
//! Free cat images API - 60K+ images, no API key required.

use async_trait::async_trait;
use serde::Deserialize;
use std::time::Duration;

use crate::config::Config;
use crate::error::Result;
use crate::http::{HttpClient, ResponseExt};
use crate::providers::traits::{Provider, ProviderInfo};
use crate::types::{License, MediaAsset, MediaType, RateLimitConfig, SearchQuery, SearchResult};

/// The Cat API provider for cat images.
/// No API key required, 60K+ images.
#[derive(Debug)]
pub struct CatApiProvider {
    client: HttpClient,
}

impl CatApiProvider {
    /// Create a new Cat API provider.
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

    /// Rate limit
    const RATE_LIMIT: RateLimitConfig = RateLimitConfig::new(60, 60);
}

#[async_trait]
impl Provider for CatApiProvider {
    fn name(&self) -> &'static str {
        "catapi"
    }

    fn display_name(&self) -> &'static str {
        "The Cat API"
    }

    fn supported_media_types(&self) -> &[MediaType] {
        &[MediaType::Image]
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
        "https://api.thecatapi.com/v1"
    }

    async fn search(&self, query: &SearchQuery) -> Result<SearchResult> {
        let count = query.count.min(25); // API max is 25 per request

        let url = format!("{}/images/search?limit={}", self.base_url(), count);

        let response = self.client.get(&url).await?;
        let cats: Vec<CatImage> = response.json_or_error().await?;

        let assets: Vec<MediaAsset> = cats
            .into_iter()
            .enumerate()
            .filter_map(|(idx, cat)| {
                let breeds = cat.breeds.unwrap_or_default();
                let breed_name =
                    breeds.first().map(|b| b.name.clone()).unwrap_or_else(|| "Cat".to_string());
                let breed_tags: Vec<String> = breeds.iter().map(|b| b.name.clone()).collect();

                MediaAsset::builder()
                    .id(format!("catapi_{}", cat.id))
                    .provider("catapi")
                    .media_type(MediaType::Image)
                    .title(format!("{} photo #{}", breed_name, idx + 1))
                    .download_url(cat.url.clone())
                    .preview_url(cat.url.clone())
                    .source_url(cat.url)
                    .license(License::Other("The Cat API - Free".to_string()))
                    .tags(
                        vec!["cat".to_string(), "animal".to_string(), "pet".to_string()]
                            .into_iter()
                            .chain(breed_tags)
                            .collect(),
                    )
                    .dimensions(cat.width.unwrap_or(0) as u32, cat.height.unwrap_or(0) as u32)
                    .build_or_log()
            })
            .collect();

        let total = assets.len();

        Ok(SearchResult {
            query: query.query.clone(),
            media_type: query.media_type,
            total_count: total,
            assets,
            providers_searched: vec!["catapi".to_string()],
            provider_errors: vec![],
            duration_ms: 0,
            provider_timings: Default::default(),
        })
    }
}

impl ProviderInfo for CatApiProvider {
    fn description(&self) -> &'static str {
        "The Cat API - 60K+ cat images, no API key required"
    }

    fn api_key_url(&self) -> &'static str {
        "https://thecatapi.com/"
    }

    fn default_license(&self) -> &'static str {
        "Free for any use"
    }
}

/// Cat API response structures
#[derive(Debug, Deserialize)]
struct CatImage {
    id: String,
    url: String,
    width: Option<i32>,
    height: Option<i32>,
    breeds: Option<Vec<CatBreed>>,
}

#[derive(Debug, Deserialize)]
struct CatBreed {
    name: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_info() {
        let config = Config::default();
        let provider = CatApiProvider::new(&config);
        assert_eq!(provider.name(), "catapi");
        assert!(provider.is_available());
        assert!(!provider.requires_api_key());
    }
}
