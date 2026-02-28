//! Unsplash provider implementation.
//!
//! [Unsplash API Documentation](https://unsplash.com/documentation)

use async_trait::async_trait;
use serde::Deserialize;
use std::time::Duration;

use crate::config::Config;
use crate::error::Result;
use crate::http::{HttpClient, ResponseExt};
use crate::providers::traits::{Provider, ProviderInfo};
use crate::types::{License, MediaAsset, MediaType, RateLimitConfig, SearchQuery, SearchResult};

/// Unsplash provider for high-resolution photography.
#[derive(Debug)]
pub struct UnsplashProvider {
    api_key: Option<String>,
    client: HttpClient,
}

impl UnsplashProvider {
    /// Create a new Unsplash provider.
    #[must_use]
    pub fn new(config: &Config) -> Self {
        let client = HttpClient::with_config(
            Self::RATE_LIMIT,
            config.retry_attempts,
            Duration::from_secs(config.timeout_secs),
        )
        .unwrap_or_default();

        Self {
            api_key: config.unsplash_api_key.clone(),
            client,
        }
    }

    /// Rate limit: 50 requests per hour
    const RATE_LIMIT: RateLimitConfig = RateLimitConfig::new(50, 3600);

    /// Build authorization header.
    #[allow(dead_code)] // May be used in future methods
    fn auth_header(&self) -> Option<(&'static str, String)> {
        self.api_key.as_ref().map(|key| ("Authorization", format!("Client-ID {key}")))
    }
}

#[async_trait]
impl Provider for UnsplashProvider {
    fn name(&self) -> &'static str {
        "unsplash"
    }

    fn display_name(&self) -> &'static str {
        "Unsplash"
    }

    fn supported_media_types(&self) -> &[MediaType] {
        &[MediaType::Image]
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
        "https://api.unsplash.com"
    }

    async fn search(&self, query: &SearchQuery) -> Result<SearchResult> {
        let Some(ref api_key) = self.api_key else {
            return Err(crate::error::DxError::MissingApiKey {
                provider: "unsplash".to_string(),
                env_var: "UNSPLASH_ACCESS_KEY".to_string(),
            });
        };

        let url = format!("{}/search/photos", self.base_url());

        let params = UnsplashSearchParams {
            query: &query.query,
            page: query.page,
            per_page: query.count.min(30), // Unsplash max is 30
            orientation: query.orientation.as_ref().map(|o| o.to_string()),
            color: query.color.clone(),
        };

        let headers = [("Authorization", format!("Client-ID {api_key}"))];
        let headers_ref: Vec<(&str, &str)> =
            headers.iter().map(|(k, v)| (*k, v.as_str())).collect();

        let response = self.client.get_with_query(&url, &params, &headers_ref).await?;

        let api_response: UnsplashSearchResponse = response.json_or_error().await?;

        let assets: Vec<MediaAsset> = api_response
            .results
            .into_iter()
            .filter_map(|photo| {
                MediaAsset::builder()
                    .id(photo.id)
                    .provider("unsplash")
                    .media_type(MediaType::Image)
                    .title(
                        photo
                            .description
                            .or(photo.alt_description)
                            .unwrap_or_else(|| "Unsplash Photo".to_string()),
                    )
                    .download_url(photo.urls.full)
                    .preview_url(photo.urls.small)
                    .source_url(photo.links.html)
                    .author(photo.user.name)
                    .author_url(photo.user.links.html)
                    .license(License::Unsplash)
                    .dimensions(photo.width, photo.height)
                    .tags(photo.tags.into_iter().map(|t| t.title).collect())
                    .build_or_log()
            })
            .collect();

        Ok(SearchResult {
            query: query.query.clone(),
            media_type: query.media_type,
            total_count: api_response.total,
            assets,
            providers_searched: vec!["unsplash".to_string()],
            provider_errors: vec![],
            duration_ms: 0,
            provider_timings: Default::default(),
        })
    }
}

impl ProviderInfo for UnsplashProvider {
    fn description(&self) -> &'static str {
        "High-resolution photography from talented photographers worldwide"
    }

    fn api_key_url(&self) -> &'static str {
        "https://unsplash.com/developers"
    }

    fn default_license(&self) -> &'static str {
        "Unsplash License (free for commercial and personal use)"
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// API RESPONSE TYPES
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Deserialize)]
struct UnsplashSearchParams<'a> {
    query: &'a str,
    page: usize,
    per_page: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    orientation: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    color: Option<String>,
}

impl serde::Serialize for UnsplashSearchParams<'_> {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(None)?;
        map.serialize_entry("query", self.query)?;
        map.serialize_entry("page", &self.page)?;
        map.serialize_entry("per_page", &self.per_page)?;
        if let Some(ref o) = self.orientation {
            map.serialize_entry("orientation", o)?;
        }
        if let Some(ref c) = self.color {
            map.serialize_entry("color", c)?;
        }
        map.end()
    }
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)] // Fields used for JSON deserialization
struct UnsplashSearchResponse {
    total: usize,
    total_pages: usize,
    results: Vec<UnsplashPhoto>,
}

#[derive(Debug, Deserialize)]
struct UnsplashPhoto {
    id: String,
    width: u32,
    height: u32,
    description: Option<String>,
    alt_description: Option<String>,
    urls: UnsplashUrls,
    links: UnsplashLinks,
    user: UnsplashUser,
    #[serde(default)]
    tags: Vec<UnsplashTag>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)] // Fields used for JSON deserialization
struct UnsplashUrls {
    raw: String,
    full: String,
    regular: String,
    small: String,
    thumb: String,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)] // Fields used for JSON deserialization
struct UnsplashLinks {
    html: String,
    download: String,
}

#[derive(Debug, Deserialize)]
struct UnsplashUser {
    name: String,
    links: UnsplashUserLinks,
}

#[derive(Debug, Deserialize)]
struct UnsplashUserLinks {
    html: String,
}

#[derive(Debug, Deserialize)]
struct UnsplashTag {
    title: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_metadata() {
        let config = Config::default_for_testing();
        let provider = UnsplashProvider::new(&config);

        assert_eq!(provider.name(), "unsplash");
        assert_eq!(provider.display_name(), "Unsplash");
        assert!(provider.requires_api_key());
        assert!(!provider.is_available()); // No API key in test config
    }
}
