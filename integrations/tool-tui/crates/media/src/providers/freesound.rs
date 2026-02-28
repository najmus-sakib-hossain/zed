//! Freesound provider implementation.
//!
//! [Freesound API Documentation](https://freesound.org/docs/api/)
//!
//! Provides access to 600,000+ sound effects and audio samples.

use async_trait::async_trait;
use serde::Deserialize;
use std::time::Duration;

use crate::config::Config;
use crate::error::Result;
use crate::http::{HttpClient, ResponseExt};
use crate::providers::traits::{Provider, ProviderInfo};
use crate::types::{License, MediaAsset, MediaType, RateLimitConfig, SearchQuery, SearchResult};

/// Freesound provider for sound effects and audio samples.
/// Access to 600K+ Creative Commons licensed sounds.
#[derive(Debug)]
pub struct FreesoundProvider {
    api_key: Option<String>,
    client: HttpClient,
}

impl FreesoundProvider {
    /// Create a new Freesound provider.
    #[must_use]
    pub fn new(config: &Config) -> Self {
        let client = HttpClient::with_config(
            Self::RATE_LIMIT,
            config.retry_attempts,
            Duration::from_secs(config.timeout_secs),
        )
        .unwrap_or_default();

        Self {
            api_key: config.freesound_api_key.clone(),
            client,
        }
    }

    /// Rate limit: 2000 requests per day
    const RATE_LIMIT: RateLimitConfig = RateLimitConfig::new(2000, 86400);

    /// Parse license from Freesound license string
    fn parse_license(license: &str) -> License {
        match license {
            "Creative Commons 0" => License::Cc0,
            "Attribution" => License::CcBy,
            "Attribution Noncommercial" => License::CcByNc,
            _ => License::Other(license.to_string()),
        }
    }
}

#[async_trait]
impl Provider for FreesoundProvider {
    fn name(&self) -> &'static str {
        "freesound"
    }

    fn display_name(&self) -> &'static str {
        "Freesound"
    }

    fn supported_media_types(&self) -> &[MediaType] {
        &[MediaType::Audio]
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
        "https://freesound.org/apiv2"
    }

    async fn search(&self, query: &SearchQuery) -> Result<SearchResult> {
        let Some(ref api_key) = self.api_key else {
            return Err(crate::error::DxError::MissingApiKey {
                provider: "freesound".to_string(),
                env_var: "FREESOUND_API_KEY".to_string(),
            });
        };

        let url = format!("{}/search/text/", self.base_url());

        let page_str = query.page.to_string();
        let page_size_str = query.count.min(150).to_string();

        let params = [
            ("query", query.query.as_str()),
            ("page", &page_str),
            ("page_size", &page_size_str),
            (
                "fields",
                "id,name,description,tags,license,username,previews,download,duration,filesize",
            ),
            ("token", api_key.as_str()),
        ];

        let response = self.client.get_with_query(&url, &params, &[]).await?;

        let api_response: FreesoundSearchResponse = response.json_or_error().await?;

        let assets: Vec<MediaAsset> = api_response
            .results
            .into_iter()
            .map(|sound| {
                let license = Self::parse_license(&sound.license);

                // Get preview URL (prefer HQ MP3)
                let preview_url = sound
                    .previews
                    .as_ref()
                    .and_then(|p| p.preview_hq_mp3.clone().or(p.preview_lq_mp3.clone()))
                    .unwrap_or_default();

                let author_url = format!("https://freesound.org/people/{}/", sound.username);

                MediaAsset::builder()
                    .id(sound.id.to_string())
                    .provider("freesound")
                    .media_type(MediaType::Audio)
                    .title(sound.name)
                    .download_url(sound.download.unwrap_or_else(|| preview_url.clone()))
                    .preview_url(preview_url)
                    .source_url(format!("https://freesound.org/s/{}/", sound.id))
                    .author(sound.username)
                    .author_url(author_url)
                    .license(license)
                    .tags(sound.tags)
                    .file_size(sound.filesize.unwrap_or(0))
                    .build_or_log()
            })
            .flatten()
            .collect();

        Ok(SearchResult {
            query: query.query.clone(),
            media_type: query.media_type,
            total_count: api_response.count,
            assets,
            providers_searched: vec!["freesound".to_string()],
            provider_errors: vec![],
            duration_ms: 0,
            provider_timings: Default::default(),
        })
    }
}

impl ProviderInfo for FreesoundProvider {
    fn description(&self) -> &'static str {
        "Collaborative database of Creative Commons licensed sounds"
    }

    fn api_key_url(&self) -> &'static str {
        "https://freesound.org/apiv2/apply/"
    }

    fn default_license(&self) -> &'static str {
        "Creative Commons (CC0, CC-BY, CC-BY-NC)"
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// API RESPONSE TYPES
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct FreesoundSearchResponse {
    count: usize,
    next: Option<String>,
    previous: Option<String>,
    results: Vec<FreesoundSound>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct FreesoundSound {
    id: u64,
    name: String,
    description: Option<String>,
    #[serde(default)]
    tags: Vec<String>,
    license: String,
    username: String,
    previews: Option<FreesoundPreviews>,
    download: Option<String>,
    duration: Option<f64>,
    filesize: Option<u64>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct FreesoundPreviews {
    #[serde(rename = "preview-hq-mp3")]
    preview_hq_mp3: Option<String>,
    #[serde(rename = "preview-lq-mp3")]
    preview_lq_mp3: Option<String>,
    #[serde(rename = "preview-hq-ogg")]
    preview_hq_ogg: Option<String>,
    #[serde(rename = "preview-lq-ogg")]
    preview_lq_ogg: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_metadata() {
        let config = Config::default_for_testing();
        let provider = FreesoundProvider::new(&config);

        assert_eq!(provider.name(), "freesound");
        assert_eq!(provider.display_name(), "Freesound");
        assert!(provider.requires_api_key());
    }

    #[test]
    fn test_license_parsing() {
        assert!(matches!(FreesoundProvider::parse_license("Creative Commons 0"), License::Cc0));
        assert!(matches!(FreesoundProvider::parse_license("Attribution"), License::CcBy));
    }

    #[test]
    fn test_supported_media_types() {
        let config = Config::default_for_testing();
        let provider = FreesoundProvider::new(&config);

        let types = provider.supported_media_types();
        assert!(types.contains(&MediaType::Audio));
        assert!(!types.contains(&MediaType::Image));
    }
}
