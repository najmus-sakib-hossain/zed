//! Digital Public Library of America (DPLA) provider implementation.
//!
//! [DPLA API](https://pro.dp.la/developers)
//!
//! Provides access to 40+ million items from US libraries, archives, and museums.

use async_trait::async_trait;
use serde::Deserialize;
use std::time::Duration;

use crate::config::Config;
use crate::error::Result;
use crate::http::{HttpClient, ResponseExt};
use crate::providers::traits::{Provider, ProviderInfo};
use crate::types::{License, MediaAsset, MediaType, RateLimitConfig, SearchQuery, SearchResult};

/// DPLA provider for American cultural heritage.
/// Access to 40M+ items from US libraries, archives, and museums.
#[derive(Debug)]
pub struct DplaProvider {
    client: HttpClient,
}

impl DplaProvider {
    /// Create a new DPLA provider.
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

    /// Parse license from rights field
    fn parse_license(rights: Option<&str>) -> License {
        match rights {
            Some(r) if r.to_lowercase().contains("public domain") => License::PublicDomain,
            Some(r) if r.contains("CC0") => License::Cc0,
            Some(r) if r.contains("CC BY-SA") => License::CcBySa,
            Some(r) if r.contains("CC BY-NC") => License::CcByNc,
            Some(r) if r.contains("CC BY") => License::CcBy,
            _ => License::Other("Various".to_string()),
        }
    }
}

#[async_trait]
impl Provider for DplaProvider {
    fn name(&self) -> &'static str {
        "dpla"
    }

    fn display_name(&self) -> &'static str {
        "Digital Public Library of America"
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
        true // DPLA requires API key (free to obtain at https://pro.dp.la/developers/policies)
    }

    fn rate_limit(&self) -> RateLimitConfig {
        Self::RATE_LIMIT
    }

    fn is_available(&self) -> bool {
        // Requires DPLA_API_KEY environment variable
        std::env::var("DPLA_API_KEY").is_ok()
    }

    fn base_url(&self) -> &'static str {
        "https://api.dp.la/v2"
    }

    async fn search(&self, query: &SearchQuery) -> Result<SearchResult> {
        let api_key =
            std::env::var("DPLA_API_KEY").map_err(|_| crate::error::DxError::MissingApiKey {
                provider: "dpla".to_string(),
                env_var: "DPLA_API_KEY".to_string(),
            })?;

        let url = format!("{}/items", self.base_url());

        let page_size = query.count.min(500).to_string();
        let page_str = query.page.to_string();

        let params = [
            ("q", query.query.as_str()),
            ("page_size", page_size.as_str()),
            ("page", page_str.as_str()),
            ("api_key", api_key.as_str()),
        ];

        let response = self.client.get_with_query(&url, &params, &[]).await?;

        let api_response: DplaSearchResponse = response.json_or_error().await?;

        let assets: Vec<MediaAsset> = api_response
            .docs
            .into_iter()
            .filter_map(|doc| {
                let preview = doc.object.as_ref()?.clone();
                let title = doc.sourceResource.title.as_ref()?.first()?.clone();

                Some(
                    MediaAsset::builder()
                        .id(doc.id.clone())
                        .provider("dpla")
                        .media_type(MediaType::Image)
                        .title(title)
                        .download_url(preview.clone())
                        .preview_url(preview)
                        .source_url(doc.isShownAt.unwrap_or_default())
                        .author(doc.sourceResource.creator.unwrap_or_default().join(", "))
                        .license(Self::parse_license(
                            doc.sourceResource
                                .rights
                                .as_ref()
                                .and_then(|v| v.first().map(|s| s.as_str())),
                        ))
                        .build_or_log(),
                )
            })
            .flatten()
            .collect();

        Ok(SearchResult {
            query: query.query.clone(),
            media_type: query.media_type,
            total_count: api_response.count.unwrap_or(0),
            assets,
            providers_searched: vec!["dpla".to_string()],
            provider_errors: vec![],
            duration_ms: 0,
            provider_timings: Default::default(),
        })
    }
}

impl ProviderInfo for DplaProvider {
    fn description(&self) -> &'static str {
        "Digital Public Library of America - 40M+ items from US libraries and museums"
    }

    fn api_key_url(&self) -> &'static str {
        "https://pro.dp.la/developers/policies#get-a-key"
    }

    fn default_license(&self) -> &'static str {
        "Various (Public Domain, CC)"
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// API RESPONSE TYPES
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Deserialize)]
struct DplaSearchResponse {
    count: Option<usize>,
    docs: Vec<DplaDoc>,
}

#[derive(Debug, Deserialize)]
#[allow(non_snake_case)]
struct DplaDoc {
    id: String,
    object: Option<String>,
    isShownAt: Option<String>,
    sourceResource: DplaSourceResource,
}

#[derive(Debug, Deserialize)]
struct DplaSourceResource {
    title: Option<Vec<String>>,
    creator: Option<Vec<String>>,
    rights: Option<Vec<String>>,
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
        let provider = DplaProvider::new(&config);

        assert_eq!(provider.name(), "dpla");
        assert_eq!(provider.display_name(), "Digital Public Library of America");
        assert!(provider.requires_api_key());
        // Without API key, provider is not available
        assert!(!provider.is_available());
    }

    #[test]
    fn test_supported_media_types() {
        let config = Config::default();
        let provider = DplaProvider::new(&config);

        let types = provider.supported_media_types();
        assert!(types.contains(&MediaType::Image));
        assert!(types.contains(&MediaType::Document));
    }

    #[test]
    fn test_license_parsing() {
        assert!(matches!(
            DplaProvider::parse_license(Some("Public Domain")),
            License::PublicDomain
        ));
        assert!(matches!(DplaProvider::parse_license(Some("CC0")), License::Cc0));
        assert!(matches!(DplaProvider::parse_license(Some("CC BY 4.0")), License::CcBy));
    }
}
