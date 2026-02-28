//! Nekos.best provider - High-quality anime images and GIFs.
//!
//! Nekos.best offers:
//! - High-quality anime images and GIFs
//! - Multiple categories (neko, kitsune, waifu, husbando, etc.)
//! - Proper API with pagination
//! - No API key required
//! - Artist attribution included
//!
//! API: <https://docs.nekos.best/>

use async_trait::async_trait;
use serde::Deserialize;
use std::time::Duration;

use crate::config::Config;
use crate::error::Result;
use crate::http::HttpClient;
use crate::providers::traits::Provider;
use crate::types::{License, MediaAsset, MediaType, RateLimitConfig, SearchQuery, SearchResult};

/// Nekos.best anime image provider.
#[derive(Debug)]
pub struct NekosBestProvider {
    client: HttpClient,
}

/// Individual result from API.
#[derive(Debug, Deserialize)]
struct NekoResult {
    url: String,
    #[serde(default)]
    artist_name: Option<String>,
    #[serde(default)]
    source_url: Option<String>,
}

/// API response.
#[derive(Debug, Deserialize)]
struct NekoResponse {
    results: Vec<NekoResult>,
}

/// Available image categories.
const IMAGE_CATEGORIES: &[&str] = &["neko", "kitsune", "waifu", "husbando"];

/// Available GIF categories (reactions).
const GIF_CATEGORIES: &[&str] = &[
    "baka",
    "bite",
    "blush",
    "bored",
    "cry",
    "cuddle",
    "dance",
    "facepalm",
    "feed",
    "handhold",
    "handshake",
    "happy",
    "highfive",
    "hug",
    "kick",
    "kiss",
    "laugh",
    "lurk",
    "nod",
    "nom",
    "nope",
    "pat",
    "peck",
    "poke",
    "pout",
    "punch",
    "shoot",
    "shrug",
    "slap",
    "sleep",
    "smile",
    "smug",
    "stare",
    "think",
    "thumbsup",
    "tickle",
    "wave",
    "wink",
    "yawn",
    "yeet",
];

impl NekosBestProvider {
    /// Create a new Nekos.best provider.
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

    /// Rate limit: generous
    const RATE_LIMIT: RateLimitConfig = RateLimitConfig::new(100, 60);

    /// Map search query to categories.
    fn map_query_to_categories(query: &str, prefer_gif: bool) -> Vec<&'static str> {
        let query_lower = query.to_lowercase();
        let mut categories = Vec::new();

        // Check for direct category matches
        for &cat in IMAGE_CATEGORIES {
            if query_lower.contains(cat) {
                categories.push(cat);
            }
        }
        for &cat in GIF_CATEGORIES {
            if query_lower.contains(cat) {
                categories.push(cat);
            }
        }

        // Keyword mappings
        if query_lower.contains("cat") || query_lower.contains("kitty") {
            categories.push("neko");
        }
        if query_lower.contains("fox") {
            categories.push("kitsune");
        }
        if query_lower.contains("anime") || query_lower.contains("girl") {
            categories.push("waifu");
        }
        if query_lower.contains("boy") || query_lower.contains("guy") {
            categories.push("husbando");
        }
        if query_lower.contains("hug") || query_lower.contains("love") {
            categories.push("hug");
        }
        if query_lower.contains("cute") || query_lower.contains("happy") {
            categories.push("happy");
        }

        // If no matches, default based on preference
        if categories.is_empty() {
            if prefer_gif {
                categories.push("smile");
                categories.push("wave");
            } else {
                categories.push("neko");
                categories.push("waifu");
            }
        }

        categories.into_iter().take(3).collect()
    }

    /// Check if category is a GIF category.
    fn is_gif_category(category: &str) -> bool {
        GIF_CATEGORIES.contains(&category)
    }
}

#[async_trait]
impl Provider for NekosBestProvider {
    fn name(&self) -> &'static str {
        "nekosbest"
    }

    fn display_name(&self) -> &'static str {
        "Nekos.best"
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
        // Disabled: nekos.best has Cloudflare bot detection
        false
    }

    fn base_url(&self) -> &'static str {
        "https://nekos.best"
    }

    async fn search(&self, query: &SearchQuery) -> Result<SearchResult> {
        let prefer_gif = query.media_type == Some(MediaType::Video); // Treat video as GIF request
        let categories = Self::map_query_to_categories(&query.query, prefer_gif);
        let per_category = (query.count / categories.len().max(1)).max(1).min(20);

        let mut all_assets = Vec::new();

        for category in &categories {
            let url = format!("{}/api/v2/{}?amount={}", self.base_url(), category, per_category);

            // Debug: log URL
            tracing::debug!("Nekos.best fetching: {}", url);

            let response = match self.client.get(&url).await {
                Ok(r) => r,
                Err(e) => {
                    tracing::debug!("Nekos.best error for {}: {}", category, e);
                    continue;
                }
            };

            let text = match response.text().await {
                Ok(t) => t,
                Err(e) => {
                    tracing::debug!("Nekos.best text error: {}", e);
                    continue;
                }
            };

            let data: NekoResponse = match serde_json::from_str(&text) {
                Ok(d) => d,
                Err(e) => {
                    tracing::debug!(
                        "Nekos.best parse error: {} - text: {}",
                        e,
                        &text[..text.len().min(200)]
                    );
                    continue;
                }
            };

            let is_gif = Self::is_gif_category(category);

            for (i, result) in data.results.into_iter().enumerate() {
                let id = format!("nekosbest_{}_{}", category, i);
                let source_url = result.source_url.clone().unwrap_or_else(|| result.url.clone());

                let mut builder = MediaAsset::builder()
                    .id(id)
                    .provider(self.name().to_string())
                    .title(format!("{} anime {}", category, if is_gif { "GIF" } else { "image" }))
                    .media_type(MediaType::Image)
                    .download_url(result.url.clone())
                    .preview_url(result.url.clone())
                    .source_url(source_url)
                    .license(License::Other("Nekos.best".to_string()));

                if let Some(artist) = result.artist_name {
                    builder = builder.author(artist);
                }

                if let Some(asset) = builder.build_or_log() {
                    all_assets.push(asset);
                }
            }
        }

        let total = all_assets.len();

        Ok(SearchResult {
            query: query.query.clone(),
            media_type: query.media_type,
            total_count: total,
            assets: all_assets.into_iter().take(query.count).collect(),
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
        let provider = NekosBestProvider::new(&config);
        assert_eq!(provider.name(), "nekosbest");
        assert_eq!(provider.display_name(), "Nekos.best");
        // NOTE: Provider is disabled due to Cloudflare bot detection
        assert!(!provider.is_available());
        assert!(!provider.requires_api_key());
    }

    #[test]
    fn test_category_mapping() {
        let cats = NekosBestProvider::map_query_to_categories("neko cat", false);
        assert!(cats.contains(&"neko"));

        let cats = NekosBestProvider::map_query_to_categories("anime girl", false);
        assert!(cats.contains(&"waifu"));
    }
}
