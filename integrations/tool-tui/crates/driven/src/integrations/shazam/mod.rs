//! # Shazam Integration
//!
//! Audio fingerprinting for song recognition.
//!
//! ## Usage
//!
//! ```rust,ignore
//! use driven::integrations::shazam::{ShazamClient, ShazamConfig};
//!
//! let config = ShazamConfig::from_file("~/.dx/config/shazam.sr")?;
//! let client = ShazamClient::new(&config)?;
//!
//! // Recognize song from audio
//! let match_ = client.recognize_file("song.mp3").await?;
//! println!("Song: {} by {}", match_.title, match_.artist);
//! ```

use crate::error::{DrivenError, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Shazam configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShazamConfig {
    /// RapidAPI key for Shazam API
    #[serde(default)]
    pub api_key: String,
    /// Sample duration in seconds (default: 5)
    #[serde(default = "default_sample_duration")]
    pub sample_duration: u32,
}

fn default_sample_duration() -> u32 {
    5
}

impl Default for ShazamConfig {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            sample_duration: default_sample_duration(),
        }
    }
}

impl ShazamConfig {
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
        if self.api_key.is_empty() || self.api_key.starts_with('$') {
            self.api_key = std::env::var("SHAZAM_API_KEY")
                .or_else(|_| std::env::var("RAPIDAPI_KEY"))
                .unwrap_or_default();
        }
    }
}

/// Song match result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SongMatch {
    /// Song title
    pub title: String,
    /// Artist name
    pub artist: String,
    /// Album name
    pub album: Option<String>,
    /// Release year
    pub year: Option<u32>,
    /// Genre
    pub genre: Option<String>,
    /// Cover art URL
    pub cover_art: Option<String>,
    /// Shazam track key
    pub track_key: String,
    /// Spotify URI (if available)
    pub spotify_uri: Option<String>,
    /// Apple Music ID (if available)
    pub apple_music_id: Option<String>,
    /// Match confidence (0.0 - 1.0)
    pub confidence: f32,
}

/// Shazam client
pub struct ShazamClient {
    config: ShazamConfig,
    base_url: String,
}

impl ShazamClient {
    /// API base URL (RapidAPI)
    const API_BASE: &'static str = "https://shazam.p.rapidapi.com";

    /// Create a new Shazam client
    pub fn new(config: &ShazamConfig) -> Result<Self> {
        let mut config = config.clone();
        config.resolve_env_vars();

        Ok(Self {
            config,
            base_url: Self::API_BASE.to_string(),
        })
    }

    /// Check if client is configured
    pub fn is_configured(&self) -> bool {
        !self.config.api_key.is_empty()
    }

    /// Recognize song from audio file
    pub async fn recognize_file(&self, path: impl AsRef<Path>) -> Result<Option<SongMatch>> {
        let audio_data = tokio::fs::read(path.as_ref())
            .await
            .map_err(|e| DrivenError::Io(e))?;
        
        self.recognize(&audio_data).await
    }

    /// Recognize song from raw audio bytes
    pub async fn recognize(&self, audio_data: &[u8]) -> Result<Option<SongMatch>> {
        if !self.is_configured() {
            return Err(DrivenError::Config("Shazam API key not configured".into()));
        }

        // Convert audio to base64 for API
        let audio_base64 = base64::encode(audio_data);

        let client = reqwest::Client::new();
        let response = client
            .post(format!("{}/songs/v2/detect", self.base_url))
            .header("X-RapidAPI-Key", &self.config.api_key)
            .header("X-RapidAPI-Host", "shazam.p.rapidapi.com")
            .header("Content-Type", "text/plain")
            .body(audio_base64)
            .send()
            .await
            .map_err(|e| DrivenError::Network(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(DrivenError::Api(format!(
                "Shazam API error ({}): {}",
                status, error_text
            )));
        }

        let result: ShazamResponse = response
            .json()
            .await
            .map_err(|e| DrivenError::Parse(e.to_string()))?;

        Ok(result.track.map(|t| SongMatch {
            title: t.title,
            artist: t.subtitle,
            album: t.sections
                .and_then(|s| s.first().and_then(|sec| sec.metadata.as_ref()))
                .and_then(|m| m.iter().find(|i| i.title == "Album"))
                .map(|i| i.text.clone()),
            year: t.sections
                .and_then(|s| s.first().and_then(|sec| sec.metadata.as_ref()))
                .and_then(|m| m.iter().find(|i| i.title == "Released"))
                .and_then(|i| i.text.parse().ok()),
            genre: t.genres.and_then(|g| g.primary),
            cover_art: t.images.map(|i| i.coverart),
            track_key: t.key,
            spotify_uri: t.hub.and_then(|h| h.providers)
                .and_then(|p| p.iter().find(|pr| pr.r#type == "SPOTIFY"))
                .map(|pr| pr.actions.first().map(|a| a.uri.clone()))
                .flatten(),
            apple_music_id: t.hub.and_then(|h| h.providers)
                .and_then(|p| p.iter().find(|pr| pr.r#type == "APPLEMUSIC"))
                .map(|pr| pr.actions.first().map(|a| a.id.clone()))
                .flatten(),
            confidence: 1.0, // Shazam doesn't return confidence
        }))
    }

    /// Search for songs by text
    pub async fn search(&self, query: &str, limit: u32) -> Result<Vec<SongMatch>> {
        if !self.is_configured() {
            return Err(DrivenError::Config("Shazam API key not configured".into()));
        }

        let client = reqwest::Client::new();
        let response = client
            .get(format!(
                "{}/search?term={}&limit={}",
                self.base_url,
                urlencoding::encode(query),
                limit
            ))
            .header("X-RapidAPI-Key", &self.config.api_key)
            .header("X-RapidAPI-Host", "shazam.p.rapidapi.com")
            .send()
            .await
            .map_err(|e| DrivenError::Network(e.to_string()))?;

        if !response.status().is_success() {
            return Err(DrivenError::Api("Shazam search failed".into()));
        }

        let result: SearchResponse = response
            .json()
            .await
            .map_err(|e| DrivenError::Parse(e.to_string()))?;

        Ok(result
            .tracks
            .unwrap_or_default()
            .hits
            .into_iter()
            .map(|h| SongMatch {
                title: h.track.title,
                artist: h.track.subtitle,
                album: None,
                year: None,
                genre: None,
                cover_art: h.track.images.map(|i| i.coverart),
                track_key: h.track.key,
                spotify_uri: None,
                apple_music_id: None,
                confidence: 1.0,
            })
            .collect())
    }
}

// API Response types

#[derive(Debug, Deserialize)]
struct ShazamResponse {
    track: Option<ShazamTrack>,
}

#[derive(Debug, Deserialize)]
struct ShazamTrack {
    key: String,
    title: String,
    subtitle: String,
    images: Option<ShazamImages>,
    genres: Option<ShazamGenres>,
    sections: Option<Vec<ShazamSection>>,
    hub: Option<ShazamHub>,
}

#[derive(Debug, Deserialize)]
struct ShazamImages {
    coverart: String,
}

#[derive(Debug, Deserialize)]
struct ShazamGenres {
    primary: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ShazamSection {
    metadata: Option<Vec<ShazamMetadata>>,
}

#[derive(Debug, Deserialize)]
struct ShazamMetadata {
    title: String,
    text: String,
}

#[derive(Debug, Clone, Deserialize)]
struct ShazamHub {
    providers: Option<Vec<ShazamProvider>>,
}

#[derive(Debug, Clone, Deserialize)]
struct ShazamProvider {
    r#type: String,
    actions: Vec<ShazamAction>,
}

#[derive(Debug, Clone, Deserialize)]
struct ShazamAction {
    uri: Option<String>,
    id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SearchResponse {
    tracks: Option<SearchTracks>,
}

#[derive(Debug, Deserialize)]
struct SearchTracks {
    hits: Vec<SearchHit>,
}

#[derive(Debug, Deserialize)]
struct SearchHit {
    track: ShazamTrack,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = ShazamConfig::default();
        assert_eq!(config.sample_duration, 5);
    }

    #[test]
    fn test_client_not_configured() {
        let config = ShazamConfig::default();
        let client = ShazamClient::new(&config).unwrap();
        assert!(!client.is_configured());
    }
}
