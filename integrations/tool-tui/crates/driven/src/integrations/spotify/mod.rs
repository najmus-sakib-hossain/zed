//! # Spotify Integration
//!
//! Control Spotify playback via the Spotify Web API.
//!
//! ## Usage
//!
//! ```rust,ignore
//! use driven::integrations::spotify::{SpotifyClient, SpotifyConfig};
//!
//! let config = SpotifyConfig::from_file("~/.dx/config/spotify.sr")?;
//! let client = SpotifyClient::new(&config)?;
//!
//! // OAuth flow
//! client.authenticate().await?;
//!
//! // Control playback
//! client.play().await?;
//! client.pause().await?;
//! client.next().await?;
//! client.play_track("spotify:track:xxx").await?;
//! ```

use crate::error::{DrivenError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Spotify configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpotifyConfig {
    /// Spotify client ID
    #[serde(default)]
    pub client_id: String,
    /// Spotify client secret
    #[serde(default)]
    pub client_secret: String,
    /// OAuth redirect URI
    #[serde(default = "default_redirect_uri")]
    pub redirect_uri: String,
    /// OAuth scopes
    #[serde(default = "default_scopes")]
    pub scopes: Vec<String>,
    /// Cached access token
    #[serde(skip)]
    pub access_token: Option<String>,
    /// Cached refresh token
    #[serde(skip)]
    pub refresh_token: Option<String>,
}

fn default_redirect_uri() -> String {
    "http://localhost:8888/callback".to_string()
}

fn default_scopes() -> Vec<String> {
    vec![
        "user-read-playback-state".to_string(),
        "user-modify-playback-state".to_string(),
        "user-read-currently-playing".to_string(),
        "playlist-read-private".to_string(),
        "playlist-modify-public".to_string(),
        "playlist-modify-private".to_string(),
    ]
}

impl Default for SpotifyConfig {
    fn default() -> Self {
        Self {
            client_id: String::new(),
            client_secret: String::new(),
            redirect_uri: default_redirect_uri(),
            scopes: default_scopes(),
            access_token: None,
            refresh_token: None,
        }
    }
}

impl SpotifyConfig {
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
        if self.client_id.is_empty() || self.client_id.starts_with('$') {
            self.client_id = std::env::var("SPOTIFY_CLIENT_ID").unwrap_or_default();
        }
        if self.client_secret.is_empty() || self.client_secret.starts_with('$') {
            self.client_secret = std::env::var("SPOTIFY_CLIENT_SECRET").unwrap_or_default();
        }
    }
}

/// Playback state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaybackState {
    /// Whether currently playing
    pub is_playing: bool,
    /// Current track
    pub track: Option<SpotifyTrack>,
    /// Progress in milliseconds
    pub progress_ms: Option<u64>,
    /// Current device
    pub device: Option<SpotifyDevice>,
    /// Shuffle state
    pub shuffle: bool,
    /// Repeat mode
    pub repeat: RepeatMode,
}

/// Spotify track
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpotifyTrack {
    /// Track ID
    pub id: String,
    /// Track name
    pub name: String,
    /// Artists
    pub artists: Vec<String>,
    /// Album name
    pub album: String,
    /// Duration in milliseconds
    pub duration_ms: u64,
    /// Spotify URI
    pub uri: String,
}

/// Spotify device
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpotifyDevice {
    /// Device ID
    pub id: String,
    /// Device name
    pub name: String,
    /// Device type
    pub device_type: String,
    /// Volume (0-100)
    pub volume: u8,
    /// Is active
    pub is_active: bool,
}

/// Repeat mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RepeatMode {
    Off,
    Track,
    Context,
}

/// Spotify playlist
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpotifyPlaylist {
    /// Playlist ID
    pub id: String,
    /// Playlist name
    pub name: String,
    /// Owner
    pub owner: String,
    /// Track count
    pub track_count: u32,
    /// Spotify URI
    pub uri: String,
}

/// Spotify Web API client
pub struct SpotifyClient {
    config: SpotifyConfig,
    base_url: String,
}

impl SpotifyClient {
    /// API base URL
    const API_BASE: &'static str = "https://api.spotify.com/v1";
    /// Auth URL
    const AUTH_URL: &'static str = "https://accounts.spotify.com";

    /// Create a new Spotify client
    pub fn new(config: &SpotifyConfig) -> Result<Self> {
        let mut config = config.clone();
        config.resolve_env_vars();

        Ok(Self {
            config,
            base_url: Self::API_BASE.to_string(),
        })
    }

    /// Check if client is configured
    pub fn is_configured(&self) -> bool {
        !self.config.client_id.is_empty() && !self.config.client_secret.is_empty()
    }

    /// Check if authenticated
    pub fn is_authenticated(&self) -> bool {
        self.config.access_token.is_some()
    }

    /// Get OAuth authorization URL
    pub fn get_auth_url(&self) -> String {
        let scopes = self.config.scopes.join(" ");
        format!(
            "{}/authorize?client_id={}&response_type=code&redirect_uri={}&scope={}",
            Self::AUTH_URL,
            self.config.client_id,
            urlencoding::encode(&self.config.redirect_uri),
            urlencoding::encode(&scopes)
        )
    }

    /// Exchange authorization code for tokens
    pub async fn exchange_code(&mut self, code: &str) -> Result<()> {
        let client = reqwest::Client::new();
        
        let mut params = HashMap::new();
        params.insert("grant_type", "authorization_code");
        params.insert("code", code);
        params.insert("redirect_uri", &self.config.redirect_uri);

        let response = client
            .post(format!("{}/api/token", Self::AUTH_URL))
            .basic_auth(&self.config.client_id, Some(&self.config.client_secret))
            .form(&params)
            .send()
            .await
            .map_err(|e| DrivenError::Network(e.to_string()))?;

        if !response.status().is_success() {
            return Err(DrivenError::Api("Failed to exchange auth code".into()));
        }

        #[derive(Deserialize)]
        struct TokenResponse {
            access_token: String,
            refresh_token: String,
        }

        let tokens: TokenResponse = response
            .json()
            .await
            .map_err(|e| DrivenError::Parse(e.to_string()))?;

        self.config.access_token = Some(tokens.access_token);
        self.config.refresh_token = Some(tokens.refresh_token);

        Ok(())
    }

    /// Refresh access token
    pub async fn refresh_token(&mut self) -> Result<()> {
        let refresh_token = self.config.refresh_token.as_ref()
            .ok_or_else(|| DrivenError::Config("No refresh token".into()))?;

        let client = reqwest::Client::new();
        
        let mut params = HashMap::new();
        params.insert("grant_type", "refresh_token");
        params.insert("refresh_token", refresh_token.as_str());

        let response = client
            .post(format!("{}/api/token", Self::AUTH_URL))
            .basic_auth(&self.config.client_id, Some(&self.config.client_secret))
            .form(&params)
            .send()
            .await
            .map_err(|e| DrivenError::Network(e.to_string()))?;

        if !response.status().is_success() {
            return Err(DrivenError::Api("Failed to refresh token".into()));
        }

        #[derive(Deserialize)]
        struct TokenResponse {
            access_token: String,
        }

        let tokens: TokenResponse = response
            .json()
            .await
            .map_err(|e| DrivenError::Parse(e.to_string()))?;

        self.config.access_token = Some(tokens.access_token);

        Ok(())
    }

    /// Get current playback state
    pub async fn get_playback(&self) -> Result<Option<PlaybackState>> {
        let response = self.api_get("/me/player").await?;
        
        if response.status() == reqwest::StatusCode::NO_CONTENT {
            return Ok(None);
        }

        let state = response
            .json()
            .await
            .map_err(|e| DrivenError::Parse(e.to_string()))?;

        Ok(Some(state))
    }

    /// Start/resume playback
    pub async fn play(&self) -> Result<()> {
        self.api_put("/me/player/play", None).await?;
        Ok(())
    }

    /// Play a specific track/album/playlist
    pub async fn play_uri(&self, uri: &str) -> Result<()> {
        let body = if uri.contains(":track:") {
            serde_json::json!({ "uris": [uri] })
        } else {
            serde_json::json!({ "context_uri": uri })
        };
        
        self.api_put("/me/player/play", Some(body)).await?;
        Ok(())
    }

    /// Pause playback
    pub async fn pause(&self) -> Result<()> {
        self.api_put("/me/player/pause", None).await?;
        Ok(())
    }

    /// Skip to next track
    pub async fn next(&self) -> Result<()> {
        self.api_post("/me/player/next", None).await?;
        Ok(())
    }

    /// Skip to previous track
    pub async fn previous(&self) -> Result<()> {
        self.api_post("/me/player/previous", None).await?;
        Ok(())
    }

    /// Set volume (0-100)
    pub async fn set_volume(&self, volume: u8) -> Result<()> {
        let volume = volume.min(100);
        self.api_put(&format!("/me/player/volume?volume_percent={}", volume), None).await?;
        Ok(())
    }

    /// Seek to position
    pub async fn seek(&self, position_ms: u64) -> Result<()> {
        self.api_put(&format!("/me/player/seek?position_ms={}", position_ms), None).await?;
        Ok(())
    }

    /// Set repeat mode
    pub async fn set_repeat(&self, mode: RepeatMode) -> Result<()> {
        let state = match mode {
            RepeatMode::Off => "off",
            RepeatMode::Track => "track",
            RepeatMode::Context => "context",
        };
        self.api_put(&format!("/me/player/repeat?state={}", state), None).await?;
        Ok(())
    }

    /// Toggle shuffle
    pub async fn set_shuffle(&self, shuffle: bool) -> Result<()> {
        self.api_put(&format!("/me/player/shuffle?state={}", shuffle), None).await?;
        Ok(())
    }

    /// Search for tracks
    pub async fn search(&self, query: &str, limit: u32) -> Result<Vec<SpotifyTrack>> {
        let response = self
            .api_get(&format!(
                "/search?q={}&type=track&limit={}",
                urlencoding::encode(query),
                limit
            ))
            .await?;

        #[derive(Deserialize)]
        struct SearchResponse {
            tracks: TracksContainer,
        }

        #[derive(Deserialize)]
        struct TracksContainer {
            items: Vec<TrackItem>,
        }

        #[derive(Deserialize)]
        struct TrackItem {
            id: String,
            name: String,
            artists: Vec<ArtistItem>,
            album: AlbumItem,
            duration_ms: u64,
            uri: String,
        }

        #[derive(Deserialize)]
        struct ArtistItem {
            name: String,
        }

        #[derive(Deserialize)]
        struct AlbumItem {
            name: String,
        }

        let search: SearchResponse = response
            .json()
            .await
            .map_err(|e| DrivenError::Parse(e.to_string()))?;

        Ok(search
            .tracks
            .items
            .into_iter()
            .map(|t| SpotifyTrack {
                id: t.id,
                name: t.name,
                artists: t.artists.into_iter().map(|a| a.name).collect(),
                album: t.album.name,
                duration_ms: t.duration_ms,
                uri: t.uri,
            })
            .collect())
    }

    /// Get user's playlists
    pub async fn get_playlists(&self) -> Result<Vec<SpotifyPlaylist>> {
        let response = self.api_get("/me/playlists?limit=50").await?;

        #[derive(Deserialize)]
        struct PlaylistsResponse {
            items: Vec<PlaylistItem>,
        }

        #[derive(Deserialize)]
        struct PlaylistItem {
            id: String,
            name: String,
            owner: OwnerItem,
            tracks: TracksInfo,
            uri: String,
        }

        #[derive(Deserialize)]
        struct OwnerItem {
            display_name: Option<String>,
        }

        #[derive(Deserialize)]
        struct TracksInfo {
            total: u32,
        }

        let playlists: PlaylistsResponse = response
            .json()
            .await
            .map_err(|e| DrivenError::Parse(e.to_string()))?;

        Ok(playlists
            .items
            .into_iter()
            .map(|p| SpotifyPlaylist {
                id: p.id,
                name: p.name,
                owner: p.owner.display_name.unwrap_or_default(),
                track_count: p.tracks.total,
                uri: p.uri,
            })
            .collect())
    }

    /// Make authenticated GET request
    async fn api_get(&self, endpoint: &str) -> Result<reqwest::Response> {
        let token = self.config.access_token.as_ref()
            .ok_or_else(|| DrivenError::Config("Not authenticated".into()))?;

        let client = reqwest::Client::new();
        let response = client
            .get(format!("{}{}", self.base_url, endpoint))
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await
            .map_err(|e| DrivenError::Network(e.to_string()))?;

        Ok(response)
    }

    /// Make authenticated PUT request
    async fn api_put(&self, endpoint: &str, body: Option<serde_json::Value>) -> Result<reqwest::Response> {
        let token = self.config.access_token.as_ref()
            .ok_or_else(|| DrivenError::Config("Not authenticated".into()))?;

        let client = reqwest::Client::new();
        let mut request = client
            .put(format!("{}{}", self.base_url, endpoint))
            .header("Authorization", format!("Bearer {}", token));

        if let Some(b) = body {
            request = request.json(&b);
        }

        let response = request
            .send()
            .await
            .map_err(|e| DrivenError::Network(e.to_string()))?;

        Ok(response)
    }

    /// Make authenticated POST request
    async fn api_post(&self, endpoint: &str, body: Option<serde_json::Value>) -> Result<reqwest::Response> {
        let token = self.config.access_token.as_ref()
            .ok_or_else(|| DrivenError::Config("Not authenticated".into()))?;

        let client = reqwest::Client::new();
        let mut request = client
            .post(format!("{}{}", self.base_url, endpoint))
            .header("Authorization", format!("Bearer {}", token));

        if let Some(b) = body {
            request = request.json(&b);
        }

        let response = request
            .send()
            .await
            .map_err(|e| DrivenError::Network(e.to_string()))?;

        Ok(response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = SpotifyConfig::default();
        assert!(config.scopes.contains(&"user-read-playback-state".to_string()));
    }

    #[test]
    fn test_auth_url() {
        let mut config = SpotifyConfig::default();
        config.client_id = "test_client_id".to_string();
        let client = SpotifyClient::new(&config).unwrap();
        let url = client.get_auth_url();
        assert!(url.contains("test_client_id"));
        assert!(url.contains("authorize"));
    }
}
