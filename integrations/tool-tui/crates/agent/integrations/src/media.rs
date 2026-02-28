//! # Media Integrations
//!
//! Connect to Spotify, YouTube, and more.

use async_trait::async_trait;
use tracing::info;

use crate::{Integration, IntegrationError, MediaIntegration, Result};

/// Spotify integration
pub struct SpotifyIntegration {
    token: Option<String>,
    device_id: Option<String>,
}

impl Default for SpotifyIntegration {
    fn default() -> Self {
        Self::new()
    }
}

impl SpotifyIntegration {
    pub fn new() -> Self {
        Self {
            token: None,
            device_id: None,
        }
    }
}

#[async_trait]
impl Integration for SpotifyIntegration {
    fn name(&self) -> &str {
        "spotify"
    }

    fn integration_type(&self) -> &str {
        "media"
    }

    fn is_authenticated(&self) -> bool {
        self.token.is_some()
    }

    async fn authenticate(&mut self, token: &str) -> Result<()> {
        self.token = Some(token.to_string());
        info!("Spotify authenticated");
        Ok(())
    }

    async fn disconnect(&mut self) -> Result<()> {
        self.token = None;
        Ok(())
    }

    fn capabilities_dx(&self) -> String {
        "capabilities:6[play pause next previous search queue]".to_string()
    }
}

#[async_trait]
impl MediaIntegration for SpotifyIntegration {
    async fn play(&self) -> Result<()> {
        let _token = self
            .token
            .as_ref()
            .ok_or_else(|| IntegrationError::NotAuthenticated("spotify".to_string()))?;

        info!("Spotify: play");

        // In production, call Spotify API
        // PUT /v1/me/player/play

        Ok(())
    }

    async fn pause(&self) -> Result<()> {
        let _token = self
            .token
            .as_ref()
            .ok_or_else(|| IntegrationError::NotAuthenticated("spotify".to_string()))?;

        info!("Spotify: pause");

        Ok(())
    }

    async fn next(&self) -> Result<()> {
        let _token = self
            .token
            .as_ref()
            .ok_or_else(|| IntegrationError::NotAuthenticated("spotify".to_string()))?;

        info!("Spotify: next track");

        Ok(())
    }

    async fn search(&self, query: &str) -> Result<Vec<String>> {
        let _token = self
            .token
            .as_ref()
            .ok_or_else(|| IntegrationError::NotAuthenticated("spotify".to_string()))?;

        info!("Spotify: searching for {}", query);

        // In production, call Spotify API
        // GET /v1/search

        Ok(vec![
            "Track 1 - Artist 1".to_string(),
            "Track 2 - Artist 2".to_string(),
        ])
    }
}
