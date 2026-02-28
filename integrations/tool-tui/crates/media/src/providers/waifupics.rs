//! Waifu.pics provider - Anime images and GIFs.
//!
//! Waifu.pics is a free anime image API with:
//! - Unlimited anime images and GIFs
//! - Multiple categories (waifu, neko, shinobu, megumin, etc.)
//! - SFW and NSFW categories (we use SFW only)
//! - Bulk endpoint for multiple images at once
//! - No API key required
//!
//! API: <https://waifu.pics/docs>

use async_trait::async_trait;
use serde::Deserialize;
use std::time::Duration;

use crate::config::Config;
use crate::error::Result;
use crate::http::{HttpClient, ResponseExt};
use crate::providers::traits::Provider;
use crate::types::{License, MediaAsset, MediaType, RateLimitConfig, SearchQuery, SearchResult};

/// Waifu.pics provider for anime images and GIFs.
#[derive(Debug)]
pub struct WaifuPicsProvider {
    client: HttpClient,
}

/// API response for bulk images.
#[derive(Debug, Deserialize)]
struct WaifuBulkResponse {
    files: Vec<String>,
}

/// SFW categories available.
const SFW_CATEGORIES: &[&str] = &[
    "waifu", "neko", "shinobu", "megumin", "bully", "cuddle", "cry", "hug", "awoo", "kiss", "lick",
    "pat", "smug", "bonk", "yeet", "blush", "smile", "wave", "highfive", "handhold", "nom", "bite",
    "glomp", "slap", "kill", "kick", "happy", "wink", "poke", "dance", "cringe",
];

impl WaifuPicsProvider {
    /// Create a new Waifu.pics provider.
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

    /// Rate limit: generous (no official limit)
    const RATE_LIMIT: RateLimitConfig = RateLimitConfig::new(100, 60);

    /// Map search query to best matching category.
    fn map_query_to_category(query: &str) -> &'static str {
        let query_lower = query.to_lowercase();

        // Direct category matches
        for &cat in SFW_CATEGORIES {
            if query_lower.contains(cat) {
                return cat;
            }
        }

        // Keyword mappings
        if query_lower.contains("cat") || query_lower.contains("kitty") {
            return "neko";
        }
        if query_lower.contains("anime") || query_lower.contains("girl") {
            return "waifu";
        }
        if query_lower.contains("gif") || query_lower.contains("react") {
            return "smile";
        }
        if query_lower.contains("cute") {
            return "pat";
        }
        if query_lower.contains("love") || query_lower.contains("heart") {
            return "hug";
        }

        // Default
        "waifu"
    }

    /// Determine if URL is a GIF.
    fn is_gif(url: &str) -> bool {
        url.to_lowercase().ends_with(".gif")
    }
}

#[async_trait]
impl Provider for WaifuPicsProvider {
    fn name(&self) -> &'static str {
        "waifupics"
    }

    fn display_name(&self) -> &'static str {
        "Waifu.pics"
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
        "https://api.waifu.pics"
    }

    async fn search(&self, query: &SearchQuery) -> Result<SearchResult> {
        let category = Self::map_query_to_category(&query.query);
        let count = query.count.min(30); // API limit

        // Use bulk endpoint for multiple results
        let url = format!("{}/many/sfw/{}", self.base_url(), category);

        let response = self.client.post_json(&url, &serde_json::json!({})).await?;

        let bulk: WaifuBulkResponse = response.json_or_error().await?;

        let assets: Vec<MediaAsset> = bulk
            .files
            .into_iter()
            .take(count)
            .enumerate()
            .filter_map(|(i, url)| {
                let is_gif = Self::is_gif(&url);
                let id = format!("waifupics_{}_{}", category, i);

                MediaAsset::builder()
                    .id(id.clone())
                    .provider(self.name().to_string())
                    .title(format!("{} anime {}", category, if is_gif { "GIF" } else { "image" }))
                    .media_type(if is_gif {
                        MediaType::Gif
                    } else {
                        MediaType::Image
                    })
                    .download_url(url.clone())
                    .preview_url(url.clone())
                    .source_url(url)
                    .license(License::Other("Waifu.pics".to_string()))
                    .build_or_log()
            })
            .collect();

        let total = assets.len();

        Ok(SearchResult {
            query: query.query.clone(),
            media_type: query.media_type,
            total_count: total,
            assets,
            providers_searched: vec![self.name().to_string()],
            provider_errors: vec![],
            duration_ms: 0,
            provider_timings: Default::default(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_info() {
        let config = Config::default();
        let provider = WaifuPicsProvider::new(&config);
        assert_eq!(provider.name(), "waifupics");
        assert_eq!(provider.display_name(), "Waifu.pics");
        assert!(provider.is_available());
        assert!(!provider.requires_api_key());
    }

    #[test]
    fn test_category_mapping() {
        assert_eq!(WaifuPicsProvider::map_query_to_category("neko cat"), "neko");
        assert_eq!(WaifuPicsProvider::map_query_to_category("anime girl"), "waifu");
        assert_eq!(WaifuPicsProvider::map_query_to_category("hug love"), "hug");
        assert_eq!(WaifuPicsProvider::map_query_to_category("random"), "waifu");
    }

    #[test]
    fn test_gif_detection() {
        assert!(WaifuPicsProvider::is_gif("https://example.com/image.gif"));
        assert!(WaifuPicsProvider::is_gif("https://example.com/IMAGE.GIF"));
        assert!(!WaifuPicsProvider::is_gif("https://example.com/image.png"));
    }
}
