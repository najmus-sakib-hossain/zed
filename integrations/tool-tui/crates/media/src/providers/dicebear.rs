//! DiceBear Avatars provider implementation.
//!
//! [DiceBear](https://www.dicebear.com/)
//!
//! Free avatar generation API - unlimited SVG/PNG avatars, no API key required.

use async_trait::async_trait;
use std::time::Duration;

use crate::config::Config;
use crate::error::Result;
use crate::http::HttpClient;
use crate::providers::traits::{Provider, ProviderInfo};
use crate::types::{License, MediaAsset, MediaType, RateLimitConfig, SearchQuery, SearchResult};

/// DiceBear Avatars provider for generated avatars.
/// No API key required, unlimited generation.
#[derive(Debug)]
pub struct DiceBearProvider {
    #[allow(dead_code)]
    client: HttpClient,
}

impl DiceBearProvider {
    /// Create a new DiceBear provider.
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

    /// Rate limit: Generous
    const RATE_LIMIT: RateLimitConfig = RateLimitConfig::new(1000, 60);

    /// Available avatar styles
    const STYLES: &'static [&'static str] = &[
        "adventurer",
        "adventurer-neutral",
        "avataaars",
        "avataaars-neutral",
        "big-ears",
        "big-ears-neutral",
        "big-smile",
        "bottts",
        "bottts-neutral",
        "croodles",
        "croodles-neutral",
        "fun-emoji",
        "icons",
        "identicon",
        "initials",
        "lorelei",
        "lorelei-neutral",
        "micah",
        "miniavs",
        "notionists",
        "notionists-neutral",
        "open-peeps",
        "personas",
        "pixel-art",
        "pixel-art-neutral",
        "rings",
        "shapes",
        "thumbs",
    ];
}

#[async_trait]
impl Provider for DiceBearProvider {
    fn name(&self) -> &'static str {
        "dicebear"
    }

    fn display_name(&self) -> &'static str {
        "DiceBear Avatars"
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
        "https://api.dicebear.com/9.x"
    }

    async fn search(&self, query: &SearchQuery) -> Result<SearchResult> {
        let count = query.count.min(50);
        let seed_base = &query.query;

        // Generate avatars using different styles and seeds
        let mut assets = Vec::with_capacity(count);

        for i in 0..count {
            let style_idx = i % Self::STYLES.len();
            let style = Self::STYLES[style_idx];
            let seed = format!("{}_{}", seed_base, i);

            // SVG URL (default, smaller)
            let svg_url = format!("{}/{}/svg?seed={}", self.base_url(), style, seed);
            // PNG URL (for preview)
            let png_url = format!("{}/{}/png?seed={}&size=200", self.base_url(), style, seed);

            if let Some(asset) = MediaAsset::builder()
                .id(format!("dicebear_{}_{}", style, i))
                .provider("dicebear")
                .media_type(MediaType::Image)
                .title(format!("{} avatar - {}", style, seed))
                .download_url(svg_url)
                .preview_url(png_url)
                .source_url(format!("https://www.dicebear.com/styles/{}", style))
                .license(License::Cc0)
                .tags(vec![
                    "avatar".to_string(),
                    style.to_string(),
                    "generated".to_string(),
                    "svg".to_string(),
                ])
                .build_or_log()
            {
                assets.push(asset);
            }
        }

        let total = assets.len();

        Ok(SearchResult {
            query: query.query.clone(),
            media_type: query.media_type,
            total_count: total,
            assets,
            providers_searched: vec!["dicebear".to_string()],
            provider_errors: vec![],
            duration_ms: 0,
            provider_timings: Default::default(),
        })
    }
}

impl ProviderInfo for DiceBearProvider {
    fn description(&self) -> &'static str {
        "Free avatar generation - 25+ styles, unlimited SVG/PNG avatars"
    }

    fn api_key_url(&self) -> &'static str {
        "https://www.dicebear.com/"
    }

    fn default_license(&self) -> &'static str {
        "CC0 1.0 Public Domain"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_info() {
        let config = Config::default();
        let provider = DiceBearProvider::new(&config);
        assert_eq!(provider.name(), "dicebear");
        assert!(provider.is_available());
        assert!(!provider.requires_api_key());
    }

    #[test]
    fn test_styles_available() {
        assert!(!DiceBearProvider::STYLES.is_empty());
        assert!(DiceBearProvider::STYLES.contains(&"avataaars"));
    }
}
