//! # DX Integrations Module
//!
//! Universal control plane for all external service integrations.
//! Each integration is built with token efficiency and performance in mind.
//!
//! ## Categories
//!
//! - **Voice & Audio**: TTS (ElevenLabs, OpenAI, Edge), Voice Wake, Spotify, Sonos, Shazam
//! - **Automation**: Webhooks, Cron, Zapier, N8N, Gmail Pub/Sub, Answer Call
//! - **Productivity**: Notion, GitHub, Obsidian, Trello, Things 3, Bear, Apple Notes/Reminders
//! - **Media & Capture**: Camera, Screen, GIF Finder, Weather
//! - **Security & Home**: 1Password, Smart Home (HomeAssistant, Hue)
//!
//! ## Usage
//!
//! ```rust,ignore
//! use driven::integrations::{TtsProvider, VoiceWake, SpotifyClient};
//!
//! // Text-to-speech
//! let tts = TtsProvider::elevenlabs(&config)?;
//! tts.speak("Hello, world!").await?;
//!
//! // Voice activation
//! let wake = VoiceWake::new(&config)?;
//! wake.start_listening().await?;
//!
//! // Music control
//! let spotify = SpotifyClient::new(&config)?;
//! spotify.play("spotify:track:xxx").await?;
//! ```

// Phase 14: Voice & Audio Integrations
pub mod tts;
pub mod voice;
pub mod spotify;
pub mod sonos;
pub mod shazam;

// Phase 15: Automation Integrations
pub mod webhooks;
pub mod cron;
pub mod zapier;
pub mod n8n;
pub mod gmail;
pub mod calls;

// Phase 16: Productivity Integrations
pub mod notion;
pub mod obsidian;
pub mod github;
pub mod trello;
pub mod things;
pub mod apple;
pub mod bear;

// Phase 17: Media & Security Integrations
pub mod camera;
pub mod screen;
pub mod gif;
pub mod weather;
pub mod twitter;
pub mod onepassword;
pub mod smarthome;

// Re-export commonly used types
pub use tts::{TtsConfig, TtsClient, Voice};
pub use voice::{VoiceWakeConfig, VoiceWakeClient, WakeWordDetection};
pub use spotify::{SpotifyClient, SpotifyConfig, Track, PlaybackState};
pub use sonos::{SonosClient, SonosConfig, SonosDevice};
pub use shazam::{ShazamClient, ShazamConfig, SongMatch};
pub use webhooks::{WebhookClient, WebhookConfig, WebhookEndpoint};
pub use cron::{CronClient, CronConfig, CronJob};
pub use zapier::{ZapierClient, ZapierConfig};
pub use n8n::{N8nClient, N8nConfig, Workflow};
pub use gmail::{GmailClient, GmailConfig, Email};
pub use calls::{CallClient, CallConfig, CallStatus};
pub use notion::{NotionClient, NotionConfig, Page as NotionPage};
pub use obsidian::{ObsidianClient, ObsidianConfig, Note as ObsidianNote};
pub use github::{GitHubClient, GitHubConfig, Issue as GitHubIssue};
pub use trello::{TrelloClient, TrelloConfig, Card as TrelloCard};
pub use things::{ThingsClient, ThingsConfig, Task as ThingsTask};
pub use apple::{AppleClient, AppleConfig};
pub use bear::{BearClient, BearConfig, Note as BearNote};
pub use camera::{CameraClient, CameraConfig, CapturedPhoto};
pub use screen::{ScreenClient, ScreenConfig, Screenshot};
pub use gif::{GifClient, GifConfig, Gif};
pub use weather::{WeatherClient, WeatherConfig, CurrentWeather};
pub use twitter::{TwitterClient, TwitterConfig, Tweet};
pub use onepassword::{OnePasswordClient, OnePasswordConfig, Item as OnePasswordItem};
pub use smarthome::{SmartHomeClient, SmartHomeConfig, Device as SmartHomeDevice};

use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Common trait for all integrations
#[async_trait::async_trait]
pub trait Integration: Send + Sync {
    /// Integration name for logging and identification
    fn name(&self) -> &'static str;

    /// Check if the integration is properly configured
    fn is_configured(&self) -> bool;

    /// Initialize the integration
    async fn initialize(&mut self) -> Result<()>;

    /// Shutdown the integration gracefully
    async fn shutdown(&mut self) -> Result<()>;

    /// Get the configuration file path
    fn config_path(&self) -> PathBuf;
}

/// Integration health status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IntegrationHealth {
    /// Integration is healthy and operational
    Healthy,
    /// Integration has warnings but is functional
    Degraded,
    /// Integration is not working
    Unhealthy,
    /// Integration status is unknown
    Unknown,
}

/// Integration capability flags
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IntegrationCapabilities {
    /// Requires network access
    pub network: bool,
    /// Requires filesystem access
    pub filesystem: bool,
    /// Requires audio access
    pub audio: bool,
    /// Requires camera access
    pub camera: bool,
    /// Requires screen capture
    pub screen: bool,
    /// Requires location access
    pub location: bool,
}

impl IntegrationCapabilities {
    /// Create capabilities with all permissions
    pub const fn all() -> Self {
        Self {
            network: true,
            filesystem: true,
            audio: true,
            camera: true,
            screen: true,
            location: true,
        }
    }

    /// Create capabilities with only network permission
    pub const fn network_only() -> Self {
        Self {
            network: true,
            filesystem: false,
            audio: false,
            camera: false,
            screen: false,
            location: false,
        }
    }
}
