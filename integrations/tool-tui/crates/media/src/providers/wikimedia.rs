//! Wikimedia Commons provider implementation.
//!
//! [Wikimedia Commons API](https://commons.wikimedia.org/w/api.php)
//!
//! Provides access to 92+ million free-use media files.

use async_trait::async_trait;
use serde::Deserialize;
use std::collections::HashMap;
use std::time::Duration;

use crate::config::Config;
use crate::error::Result;
use crate::http::{HttpClient, ResponseExt};
use crate::providers::traits::{Provider, ProviderInfo};
use crate::types::{License, MediaAsset, MediaType, RateLimitConfig, SearchQuery, SearchResult};

/// Wikimedia Commons provider for free-use media.
/// Access to 92M+ images, videos, and audio files.
#[derive(Debug)]
pub struct WikimediaCommonsProvider {
    client: HttpClient,
}

impl WikimediaCommonsProvider {
    /// Create a new Wikimedia Commons provider.
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
    const RATE_LIMIT: RateLimitConfig = RateLimitConfig::new(200, 60);

    /// Determine media type from file extension
    fn media_type_from_title(title: &str) -> MediaType {
        let lower = title.to_lowercase();
        if lower.ends_with(".jpg")
            || lower.ends_with(".jpeg")
            || lower.ends_with(".png")
            || lower.ends_with(".gif")
            || lower.ends_with(".svg")
            || lower.ends_with(".webp")
        {
            MediaType::Image
        } else if lower.ends_with(".mp4") || lower.ends_with(".webm") || lower.ends_with(".ogv") {
            MediaType::Video
        } else if lower.ends_with(".mp3")
            || lower.ends_with(".ogg")
            || lower.ends_with(".wav")
            || lower.ends_with(".flac")
        {
            MediaType::Audio
        } else {
            MediaType::Image
        }
    }

    /// Clean title for display (remove File: prefix and extension)
    fn clean_title(title: &str) -> String {
        title
            .trim_start_matches("File:")
            .rsplit_once('.')
            .map(|(name, _)| name)
            .unwrap_or(title)
            .replace('_', " ")
    }
}

#[async_trait]
impl Provider for WikimediaCommonsProvider {
    fn name(&self) -> &'static str {
        "wikimedia"
    }

    fn display_name(&self) -> &'static str {
        "Wikimedia Commons"
    }

    fn supported_media_types(&self) -> &[MediaType] {
        &[MediaType::Image, MediaType::Video, MediaType::Audio]
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
        "https://commons.wikimedia.org/w/api.php"
    }

    async fn search(&self, query: &SearchQuery) -> Result<SearchResult> {
        let limit_str = query.count.min(50).to_string();
        let offset_str = ((query.page - 1) * query.count).to_string();

        let params = [
            ("action", "query"),
            ("format", "json"),
            ("generator", "search"),
            ("gsrnamespace", "6"), // File namespace
            ("gsrsearch", query.query.as_str()),
            ("gsrlimit", &limit_str),
            ("gsroffset", &offset_str),
            ("prop", "imageinfo"),
            ("iiprop", "url|size|mime|user|extmetadata"),
            ("iiurlwidth", "640"),
        ];

        let response = self.client.get_with_query(self.base_url(), &params, &[]).await?;

        let api_response: WikimediaSearchResponse = response.json_or_error().await?;

        let pages = api_response.query.map(|q| q.pages).unwrap_or_default();

        let assets: Vec<MediaAsset> = pages
            .into_values()
            .filter_map(|page| {
                let info = page.imageinfo?.into_iter().next()?;
                let media_type = Self::media_type_from_title(&page.title);

                // Filter by media type if specified
                if let Some(requested_type) = query.media_type {
                    if media_type != requested_type {
                        return None;
                    }
                }

                let license = info
                    .extmetadata
                    .as_ref()
                    .and_then(|m| m.license_short_name.as_ref())
                    .map(|l| {
                        let val = &l.value;
                        if val.contains("CC0") || val.contains("Public domain") {
                            License::Cc0
                        } else if val.contains("CC BY-SA") {
                            License::CcBySa
                        } else if val.contains("CC BY-NC") {
                            License::CcByNc
                        } else if val.contains("CC BY") {
                            License::CcBy
                        } else {
                            License::Other(val.clone())
                        }
                    })
                    .unwrap_or(License::Other("Various".to_string()));

                MediaAsset::builder()
                    .id(page.pageid.to_string())
                    .provider("wikimedia")
                    .media_type(media_type)
                    .title(Self::clean_title(&page.title))
                    .download_url(info.url.clone())
                    .preview_url(info.thumburl.unwrap_or_else(|| info.url.clone()))
                    .source_url(info.descriptionurl)
                    .author(info.user.unwrap_or_else(|| "Unknown".to_string()))
                    .license(license)
                    .dimensions(info.width.unwrap_or(0), info.height.unwrap_or(0))
                    .build_or_log()
            })
            .collect();

        Ok(SearchResult {
            query: query.query.clone(),
            media_type: query.media_type,
            total_count: assets.len(), // Wikimedia doesn't return total in this format
            assets,
            providers_searched: vec!["wikimedia".to_string()],
            provider_errors: vec![],
            duration_ms: 0,
            provider_timings: Default::default(),
        })
    }
}

impl ProviderInfo for WikimediaCommonsProvider {
    fn description(&self) -> &'static str {
        "Free media repository with 92M+ images, videos, and audio files"
    }

    fn api_key_url(&self) -> &'static str {
        "https://commons.wikimedia.org/wiki/Commons:API"
    }

    fn default_license(&self) -> &'static str {
        "Various Creative Commons and Public Domain licenses"
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// API RESPONSE TYPES
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct WikimediaSearchResponse {
    query: Option<WikimediaQuery>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct WikimediaQuery {
    pages: HashMap<String, WikimediaPage>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct WikimediaPage {
    pageid: u64,
    title: String,
    imageinfo: Option<Vec<WikimediaImageInfo>>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct WikimediaImageInfo {
    url: String,
    descriptionurl: String,
    thumburl: Option<String>,
    width: Option<u32>,
    height: Option<u32>,
    size: Option<u64>,
    mime: Option<String>,
    user: Option<String>,
    extmetadata: Option<WikimediaExtMetadata>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct WikimediaExtMetadata {
    #[serde(rename = "LicenseShortName")]
    license_short_name: Option<WikimediaMetadataValue>,
    #[serde(rename = "Artist")]
    artist: Option<WikimediaMetadataValue>,
    #[serde(rename = "ImageDescription")]
    image_description: Option<WikimediaMetadataValue>,
}

#[derive(Debug, Deserialize)]
struct WikimediaMetadataValue {
    value: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_metadata() {
        let config = Config::default_for_testing();
        let provider = WikimediaCommonsProvider::new(&config);

        assert_eq!(provider.name(), "wikimedia");
        assert_eq!(provider.display_name(), "Wikimedia Commons");
        assert!(!provider.requires_api_key());
        assert!(provider.is_available());
    }

    #[test]
    fn test_media_type_detection() {
        assert_eq!(
            WikimediaCommonsProvider::media_type_from_title("File:Test.jpg"),
            MediaType::Image
        );
        assert_eq!(
            WikimediaCommonsProvider::media_type_from_title("File:Test.mp4"),
            MediaType::Video
        );
        assert_eq!(
            WikimediaCommonsProvider::media_type_from_title("File:Test.mp3"),
            MediaType::Audio
        );
    }

    #[test]
    fn test_title_cleaning() {
        assert_eq!(WikimediaCommonsProvider::clean_title("File:Test_Image.jpg"), "Test Image");
        assert_eq!(WikimediaCommonsProvider::clean_title("File:My_Photo.png"), "My Photo");
    }
}
