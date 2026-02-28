//! # GIF Integration
//!
//! GIF search and creation via Giphy/Tenor APIs.
//!
//! ## Usage
//!
//! ```rust,ignore
//! use driven::integrations::gif::{GifClient, GifConfig};
//!
//! let config = GifConfig::from_file("~/.dx/config/gif.sr")?;
//! let client = GifClient::new(&config)?;
//!
//! // Search for GIFs
//! let gifs = client.search("happy cat").await?;
//!
//! // Get trending GIFs
//! let trending = client.trending(10).await?;
//! ```

use crate::error::{DrivenError, Result};
use serde::{Deserialize, Serialize};

/// GIF configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GifConfig {
    /// Whether GIF integration is enabled
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Giphy API key
    #[serde(default)]
    pub giphy_api_key: String,
    /// Tenor API key
    #[serde(default)]
    pub tenor_api_key: String,
    /// Preferred provider
    #[serde(default)]
    pub preferred_provider: GifProvider,
    /// Default limit for search results
    #[serde(default = "default_limit")]
    pub default_limit: u32,
    /// Content rating filter
    #[serde(default)]
    pub rating: ContentRating,
}

fn default_true() -> bool {
    true
}

fn default_limit() -> u32 {
    25
}

impl Default for GifConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            giphy_api_key: String::new(),
            tenor_api_key: String::new(),
            preferred_provider: GifProvider::Giphy,
            default_limit: default_limit(),
            rating: ContentRating::G,
        }
    }
}

impl GifConfig {
    /// Load from .sr config file
    pub fn from_file(path: impl AsRef<std::path::Path>) -> Result<Self> {
        let content = std::fs::read_to_string(path.as_ref())
            .map_err(|e| DrivenError::Io(e))?;
        Self::parse_sr(&content)
    }

    fn parse_sr(_content: &str) -> Result<Self> {
        Ok(Self::default())
    }

    /// Resolve environment variables
    pub fn resolve_env_vars(&mut self) {
        if self.giphy_api_key.is_empty() || self.giphy_api_key.starts_with('$') {
            self.giphy_api_key = std::env::var("GIPHY_API_KEY").unwrap_or_default();
        }
        if self.tenor_api_key.is_empty() || self.tenor_api_key.starts_with('$') {
            self.tenor_api_key = std::env::var("TENOR_API_KEY").unwrap_or_default();
        }
    }
}

/// GIF provider
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum GifProvider {
    #[default]
    Giphy,
    Tenor,
}

/// Content rating
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ContentRating {
    /// General audiences
    #[default]
    G,
    /// Parental guidance suggested
    PG,
    /// Parents strongly cautioned
    PG13,
    /// Restricted (Giphy only)
    R,
}

impl ContentRating {
    fn to_giphy_string(&self) -> &str {
        match self {
            ContentRating::G => "g",
            ContentRating::PG => "pg",
            ContentRating::PG13 => "pg-13",
            ContentRating::R => "r",
        }
    }

    fn to_tenor_string(&self) -> &str {
        match self {
            ContentRating::G => "off",
            ContentRating::PG => "low",
            ContentRating::PG13 => "medium",
            ContentRating::R => "high",
        }
    }
}

/// GIF result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Gif {
    /// Unique ID
    pub id: String,
    /// Title
    pub title: String,
    /// Original URL
    pub url: String,
    /// Source URL (webpage)
    pub source_url: String,
    /// GIF images in various formats
    pub images: GifImages,
    /// Username of uploader
    pub username: Option<String>,
    /// Import datetime
    pub import_datetime: Option<String>,
    /// Provider
    pub provider: GifProvider,
}

/// GIF images in various formats
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GifImages {
    /// Original size
    pub original: GifImage,
    /// Fixed height (200px)
    pub fixed_height: GifImage,
    /// Fixed width (200px)
    pub fixed_width: GifImage,
    /// Thumbnail/preview
    pub preview: Option<GifImage>,
    /// Still image
    pub still: Option<GifImage>,
}

/// Single GIF image
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GifImage {
    /// URL
    pub url: String,
    /// Width
    pub width: u32,
    /// Height
    pub height: u32,
    /// File size in bytes
    pub size: Option<u64>,
    /// MP4 URL (if available)
    pub mp4: Option<String>,
    /// WebP URL (if available)
    pub webp: Option<String>,
}

/// GIF client
pub struct GifClient {
    config: GifConfig,
}

impl GifClient {
    /// Create a new GIF client
    pub fn new(config: &GifConfig) -> Result<Self> {
        let mut config = config.clone();
        config.resolve_env_vars();

        Ok(Self { config })
    }

    /// Check if client is configured
    pub fn is_configured(&self) -> bool {
        self.config.enabled
            && (!self.config.giphy_api_key.is_empty()
                || !self.config.tenor_api_key.is_empty())
    }

    /// Search for GIFs
    pub async fn search(&self, query: &str) -> Result<Vec<Gif>> {
        self.search_with_limit(query, self.config.default_limit).await
    }

    /// Search for GIFs with limit
    pub async fn search_with_limit(&self, query: &str, limit: u32) -> Result<Vec<Gif>> {
        match self.config.preferred_provider {
            GifProvider::Giphy => self.search_giphy(query, limit).await,
            GifProvider::Tenor => self.search_tenor(query, limit).await,
        }
    }

    /// Get trending GIFs
    pub async fn trending(&self, limit: u32) -> Result<Vec<Gif>> {
        match self.config.preferred_provider {
            GifProvider::Giphy => self.trending_giphy(limit).await,
            GifProvider::Tenor => self.trending_tenor(limit).await,
        }
    }

    /// Get a random GIF
    pub async fn random(&self, tag: Option<&str>) -> Result<Gif> {
        match self.config.preferred_provider {
            GifProvider::Giphy => self.random_giphy(tag).await,
            GifProvider::Tenor => {
                // Tenor doesn't have random, so search and pick first
                let query = tag.unwrap_or("funny");
                let results = self.search_tenor(query, 1).await?;
                results.into_iter().next().ok_or_else(|| {
                    DrivenError::NotFound("No GIF found".into())
                })
            }
        }
    }

    /// Get GIF by ID
    pub async fn get_by_id(&self, id: &str) -> Result<Gif> {
        self.get_giphy_by_id(id).await
    }

    // Giphy API

    async fn search_giphy(&self, query: &str, limit: u32) -> Result<Vec<Gif>> {
        let url = format!(
            "https://api.giphy.com/v1/gifs/search?api_key={}&q={}&limit={}&rating={}",
            self.config.giphy_api_key,
            urlencoding::encode(query),
            limit,
            self.config.rating.to_giphy_string()
        );

        let response: serde_json::Value = self.api_get(&url).await?;
        self.parse_giphy_results(response)
    }

    async fn trending_giphy(&self, limit: u32) -> Result<Vec<Gif>> {
        let url = format!(
            "https://api.giphy.com/v1/gifs/trending?api_key={}&limit={}&rating={}",
            self.config.giphy_api_key,
            limit,
            self.config.rating.to_giphy_string()
        );

        let response: serde_json::Value = self.api_get(&url).await?;
        self.parse_giphy_results(response)
    }

    async fn random_giphy(&self, tag: Option<&str>) -> Result<Gif> {
        let mut url = format!(
            "https://api.giphy.com/v1/gifs/random?api_key={}&rating={}",
            self.config.giphy_api_key,
            self.config.rating.to_giphy_string()
        );

        if let Some(t) = tag {
            url.push_str(&format!("&tag={}", urlencoding::encode(t)));
        }

        let response: serde_json::Value = self.api_get(&url).await?;
        self.parse_giphy_gif(&response["data"])
    }

    async fn get_giphy_by_id(&self, id: &str) -> Result<Gif> {
        let url = format!(
            "https://api.giphy.com/v1/gifs/{}?api_key={}",
            id,
            self.config.giphy_api_key
        );

        let response: serde_json::Value = self.api_get(&url).await?;
        self.parse_giphy_gif(&response["data"])
    }

    fn parse_giphy_results(&self, response: serde_json::Value) -> Result<Vec<Gif>> {
        let data = response["data"]
            .as_array()
            .ok_or_else(|| DrivenError::Parse("Invalid Giphy response".into()))?;

        data.iter()
            .map(|g| self.parse_giphy_gif(g))
            .collect()
    }

    fn parse_giphy_gif(&self, gif: &serde_json::Value) -> Result<Gif> {
        let images = &gif["images"];

        Ok(Gif {
            id: gif["id"].as_str().unwrap_or_default().to_string(),
            title: gif["title"].as_str().unwrap_or_default().to_string(),
            url: gif["url"].as_str().unwrap_or_default().to_string(),
            source_url: gif["source"].as_str().unwrap_or_default().to_string(),
            username: gif["username"].as_str().map(String::from),
            import_datetime: gif["import_datetime"].as_str().map(String::from),
            provider: GifProvider::Giphy,
            images: GifImages {
                original: self.parse_giphy_image(&images["original"]),
                fixed_height: self.parse_giphy_image(&images["fixed_height"]),
                fixed_width: self.parse_giphy_image(&images["fixed_width"]),
                preview: images["preview_gif"].as_object().map(|_| {
                    self.parse_giphy_image(&images["preview_gif"])
                }),
                still: images["original_still"].as_object().map(|_| {
                    self.parse_giphy_image(&images["original_still"])
                }),
            },
        })
    }

    fn parse_giphy_image(&self, img: &serde_json::Value) -> GifImage {
        GifImage {
            url: img["url"].as_str().unwrap_or_default().to_string(),
            width: img["width"]
                .as_str()
                .and_then(|s| s.parse().ok())
                .unwrap_or(0),
            height: img["height"]
                .as_str()
                .and_then(|s| s.parse().ok())
                .unwrap_or(0),
            size: img["size"]
                .as_str()
                .and_then(|s| s.parse().ok()),
            mp4: img["mp4"].as_str().map(String::from),
            webp: img["webp"].as_str().map(String::from),
        }
    }

    // Tenor API

    async fn search_tenor(&self, query: &str, limit: u32) -> Result<Vec<Gif>> {
        let url = format!(
            "https://tenor.googleapis.com/v2/search?key={}&q={}&limit={}&contentfilter={}",
            self.config.tenor_api_key,
            urlencoding::encode(query),
            limit,
            self.config.rating.to_tenor_string()
        );

        let response: serde_json::Value = self.api_get(&url).await?;
        self.parse_tenor_results(response)
    }

    async fn trending_tenor(&self, limit: u32) -> Result<Vec<Gif>> {
        let url = format!(
            "https://tenor.googleapis.com/v2/featured?key={}&limit={}&contentfilter={}",
            self.config.tenor_api_key,
            limit,
            self.config.rating.to_tenor_string()
        );

        let response: serde_json::Value = self.api_get(&url).await?;
        self.parse_tenor_results(response)
    }

    fn parse_tenor_results(&self, response: serde_json::Value) -> Result<Vec<Gif>> {
        let results = response["results"]
            .as_array()
            .ok_or_else(|| DrivenError::Parse("Invalid Tenor response".into()))?;

        Ok(results
            .iter()
            .map(|g| self.parse_tenor_gif(g))
            .collect())
    }

    fn parse_tenor_gif(&self, gif: &serde_json::Value) -> Gif {
        let media = &gif["media_formats"];

        Gif {
            id: gif["id"].as_str().unwrap_or_default().to_string(),
            title: gif["content_description"].as_str().unwrap_or_default().to_string(),
            url: gif["url"].as_str().unwrap_or_default().to_string(),
            source_url: gif["itemurl"].as_str().unwrap_or_default().to_string(),
            username: None,
            import_datetime: gif["created"].as_f64().map(|t| t.to_string()),
            provider: GifProvider::Tenor,
            images: GifImages {
                original: self.parse_tenor_image(&media["gif"]),
                fixed_height: self.parse_tenor_image(&media["tinygif"]),
                fixed_width: self.parse_tenor_image(&media["tinygif"]),
                preview: media["nanogif"].as_object().map(|_| {
                    self.parse_tenor_image(&media["nanogif"])
                }),
                still: None,
            },
        }
    }

    fn parse_tenor_image(&self, media: &serde_json::Value) -> GifImage {
        GifImage {
            url: media["url"].as_str().unwrap_or_default().to_string(),
            width: media["dims"][0].as_u64().unwrap_or(0) as u32,
            height: media["dims"][1].as_u64().unwrap_or(0) as u32,
            size: media["size"].as_u64(),
            mp4: None,
            webp: None,
        }
    }

    async fn api_get(&self, url: &str) -> Result<serde_json::Value> {
        let client = reqwest::Client::new();
        let response = client
            .get(url)
            .send()
            .await
            .map_err(|e| DrivenError::Network(e.to_string()))?;

        if !response.status().is_success() {
            let error = response.text().await.unwrap_or_default();
            return Err(DrivenError::Api(format!("GIF API error: {}", error)));
        }

        response
            .json()
            .await
            .map_err(|e| DrivenError::Parse(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = GifConfig::default();
        assert!(config.enabled);
        assert_eq!(config.default_limit, 25);
    }

    #[test]
    fn test_content_rating_strings() {
        assert_eq!(ContentRating::G.to_giphy_string(), "g");
        assert_eq!(ContentRating::PG13.to_giphy_string(), "pg-13");
        assert_eq!(ContentRating::G.to_tenor_string(), "off");
    }
}
