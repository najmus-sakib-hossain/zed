//! RoboHash provider implementation.
//!
//! [RoboHash](https://robohash.org/)
//!
//! Free robot/monster avatar generation - unlimited, no API key required.

use async_trait::async_trait;

use crate::config::Config;
use crate::error::Result;
use crate::providers::traits::{Provider, ProviderInfo};
use crate::types::{License, MediaAsset, MediaType, RateLimitConfig, SearchQuery, SearchResult};

/// RoboHash provider for generated robot avatars.
/// No API key required, unlimited generation.
#[derive(Debug)]
pub struct RoboHashProvider {
    // Note: No HTTP client needed - RoboHash generates URLs directly without API calls
}

impl RoboHashProvider {
    /// Create a new RoboHash provider.
    #[must_use]
    pub fn new(_config: &Config) -> Self {
        Self {}
    }

    /// Rate limit: Generous
    const RATE_LIMIT: RateLimitConfig = RateLimitConfig::new(1000, 60);

    /// Available sets
    /// set1 = Robots, set2 = Monsters, set3 = Robot Heads, set4 = Cats, set5 = Humans
    const SETS: &'static [(&'static str, &'static str)] = &[
        ("set1", "robots"),
        ("set2", "monsters"),
        ("set3", "robot-heads"),
        ("set4", "cats"),
        ("set5", "humans"),
    ];
}

#[async_trait]
impl Provider for RoboHashProvider {
    fn name(&self) -> &'static str {
        "robohash"
    }

    fn display_name(&self) -> &'static str {
        "RoboHash"
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
        "https://robohash.org"
    }

    async fn search(&self, query: &SearchQuery) -> Result<SearchResult> {
        let count = query.count.min(50);
        let seed_base = &query.query;

        // Determine which set to use based on query
        let query_lower = query.query.to_lowercase();
        let set = if query_lower.contains("monster") {
            "set2"
        } else if query_lower.contains("cat") {
            "set4"
        } else if query_lower.contains("human") || query_lower.contains("person") {
            "set5"
        } else if query_lower.contains("head") {
            "set3"
        } else {
            "set1" // Default to robots
        };

        // Generate avatars with different seeds
        let mut assets = Vec::with_capacity(count);

        for i in 0..count {
            let seed = format!("{}_{}", seed_base, i);

            // Generate URLs with size
            let url_300 = format!("{}/{}?set={}&size=300x300", self.base_url(), seed, set);
            let url_preview = format!("{}/{}?set={}&size=150x150", self.base_url(), seed, set);

            let set_name = Self::SETS
                .iter()
                .find(|(s, _)| *s == set)
                .map(|(_, name)| *name)
                .unwrap_or("robots");

            if let Some(asset) = MediaAsset::builder()
                .id(format!("robohash_{}_{}_{}", set, seed_base, i))
                .provider("robohash")
                .media_type(MediaType::Image)
                .title(format!("RoboHash {} - {}", set_name, seed))
                .download_url(url_300)
                .preview_url(url_preview)
                .source_url(format!("{}/{}", self.base_url(), seed))
                .license(License::CcBy)
                .tags(vec![
                    "avatar".to_string(),
                    set_name.to_string(),
                    "generated".to_string(),
                    "robohash".to_string(),
                ])
                .dimensions(300, 300)
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
            providers_searched: vec!["robohash".to_string()],
            provider_errors: vec![],
            duration_ms: 0,
            provider_timings: Default::default(),
        })
    }
}

impl ProviderInfo for RoboHashProvider {
    fn description(&self) -> &'static str {
        "Free robot/monster avatar generation - 5 sets, unlimited images"
    }

    fn api_key_url(&self) -> &'static str {
        "https://robohash.org/"
    }

    fn default_license(&self) -> &'static str {
        "CC-BY 4.0"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_info() {
        let config = Config::default();
        let provider = RoboHashProvider::new(&config);
        assert_eq!(provider.name(), "robohash");
        assert!(provider.is_available());
        assert!(!provider.requires_api_key());
    }

    #[test]
    fn test_sets_available() {
        assert_eq!(RoboHashProvider::SETS.len(), 5);
    }
}
