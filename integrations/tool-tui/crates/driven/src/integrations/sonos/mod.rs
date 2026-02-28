//! # Sonos Integration
//!
//! Control Sonos speakers via SSDP discovery and SOAP API.
//!
//! ## Usage
//!
//! ```rust,ignore
//! use driven::integrations::sonos::{SonosClient, SonosConfig};
//!
//! let config = SonosConfig::from_file("~/.dx/config/sonos.sr")?;
//! let client = SonosClient::new(&config)?;
//!
//! // Discover speakers
//! let rooms = client.discover().await?;
//!
//! // Control playback
//! client.play("Living Room").await?;
//! client.set_volume("Living Room", 50).await?;
//! ```

use crate::error::{DrivenError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::IpAddr;

/// Sonos configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SonosConfig {
    /// Discovery timeout in seconds
    #[serde(default = "default_discovery_timeout")]
    pub discovery_timeout: u64,
    /// Cached speaker IPs
    #[serde(default)]
    pub known_speakers: HashMap<String, String>,
    /// Default room
    pub default_room: Option<String>,
}

fn default_discovery_timeout() -> u64 {
    5
}

impl Default for SonosConfig {
    fn default() -> Self {
        Self {
            discovery_timeout: default_discovery_timeout(),
            known_speakers: HashMap::new(),
            default_room: None,
        }
    }
}

impl SonosConfig {
    /// Load from .sr config file
    pub fn from_file(path: impl AsRef<std::path::Path>) -> Result<Self> {
        let content = std::fs::read_to_string(path.as_ref())
            .map_err(|e| DrivenError::Io(e))?;
        Self::parse_sr(&content)
    }

    fn parse_sr(_content: &str) -> Result<Self> {
        Ok(Self::default())
    }
}

/// Sonos room/speaker
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SonosRoom {
    /// Room name
    pub name: String,
    /// Speaker IP address
    pub ip: IpAddr,
    /// Speaker model
    pub model: String,
    /// Is coordinator (group leader)
    pub is_coordinator: bool,
    /// Group ID
    pub group_id: Option<String>,
    /// Current volume (0-100)
    pub volume: u8,
    /// Is muted
    pub muted: bool,
}

/// Sonos playback state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SonosPlaybackState {
    Playing,
    Paused,
    Stopped,
    Transitioning,
}

/// Currently playing track info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SonosTrack {
    /// Track title
    pub title: String,
    /// Artist
    pub artist: String,
    /// Album
    pub album: String,
    /// Album art URL
    pub album_art: Option<String>,
    /// Duration in seconds
    pub duration: u32,
    /// Current position in seconds
    pub position: u32,
}

/// Sonos queue item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SonosQueueItem {
    /// Position in queue (1-indexed)
    pub position: u32,
    /// Track info
    pub track: SonosTrack,
}

/// Sonos client
pub struct SonosClient {
    config: SonosConfig,
    rooms: HashMap<String, SonosRoom>,
}

impl SonosClient {
    /// Create a new Sonos client
    pub fn new(config: &SonosConfig) -> Result<Self> {
        Ok(Self {
            config: config.clone(),
            rooms: HashMap::new(),
        })
    }

    /// Discover Sonos speakers on the network
    pub async fn discover(&mut self) -> Result<Vec<SonosRoom>> {
        // SSDP M-SEARCH for Sonos devices
        let rooms = self.ssdp_discover().await?;
        
        for room in &rooms {
            self.rooms.insert(room.name.clone(), room.clone());
        }

        Ok(rooms)
    }

    /// Get a room by name
    pub fn get_room(&self, name: &str) -> Option<&SonosRoom> {
        self.rooms.get(name)
    }

    /// List discovered rooms
    pub fn rooms(&self) -> Vec<&SonosRoom> {
        self.rooms.values().collect()
    }

    /// Play on a room
    pub async fn play(&self, room: &str) -> Result<()> {
        let room = self.get_room_or_default(room)?;
        self.soap_action(&room.ip, "Play").await
    }

    /// Pause on a room
    pub async fn pause(&self, room: &str) -> Result<()> {
        let room = self.get_room_or_default(room)?;
        self.soap_action(&room.ip, "Pause").await
    }

    /// Stop on a room
    pub async fn stop(&self, room: &str) -> Result<()> {
        let room = self.get_room_or_default(room)?;
        self.soap_action(&room.ip, "Stop").await
    }

    /// Next track
    pub async fn next(&self, room: &str) -> Result<()> {
        let room = self.get_room_or_default(room)?;
        self.soap_action(&room.ip, "Next").await
    }

    /// Previous track
    pub async fn previous(&self, room: &str) -> Result<()> {
        let room = self.get_room_or_default(room)?;
        self.soap_action(&room.ip, "Previous").await
    }

    /// Set volume (0-100)
    pub async fn set_volume(&self, room: &str, volume: u8) -> Result<()> {
        let room = self.get_room_or_default(room)?;
        let volume = volume.min(100);
        self.soap_action_with_args(&room.ip, "SetVolume", &[("DesiredVolume", &volume.to_string())]).await
    }

    /// Get current volume
    pub async fn get_volume(&self, room: &str) -> Result<u8> {
        let room = self.get_room_or_default(room)?;
        let response = self.soap_request(&room.ip, "GetVolume", &[]).await?;
        // Parse response XML for CurrentVolume
        self.parse_volume_response(&response)
    }

    /// Mute/unmute
    pub async fn set_mute(&self, room: &str, mute: bool) -> Result<()> {
        let room = self.get_room_or_default(room)?;
        self.soap_action_with_args(&room.ip, "SetMute", &[("DesiredMute", if mute { "1" } else { "0" })]).await
    }

    /// Get current playback state
    pub async fn get_state(&self, room: &str) -> Result<SonosPlaybackState> {
        let room = self.get_room_or_default(room)?;
        let response = self.soap_request(&room.ip, "GetTransportInfo", &[]).await?;
        self.parse_transport_state(&response)
    }

    /// Get currently playing track
    pub async fn get_current_track(&self, room: &str) -> Result<Option<SonosTrack>> {
        let room = self.get_room_or_default(room)?;
        let response = self.soap_request(&room.ip, "GetPositionInfo", &[]).await?;
        self.parse_track_info(&response)
    }

    /// Play a URI (radio station, playlist, etc.)
    pub async fn play_uri(&self, room: &str, uri: &str) -> Result<()> {
        let room = self.get_room_or_default(room)?;
        self.soap_action_with_args(&room.ip, "SetAVTransportURI", &[
            ("CurrentURI", uri),
            ("CurrentURIMetaData", ""),
        ]).await?;
        self.play(&room.name).await
    }

    /// Group rooms together
    pub async fn group(&self, coordinator: &str, members: &[&str]) -> Result<()> {
        let coord = self.get_room_or_default(coordinator)?;
        let coord_uuid = format!("RINCON_{}", coord.ip.to_string().replace('.', ""));

        for member in members {
            let room = self.get_room_or_default(member)?;
            self.soap_action_with_args(
                &room.ip,
                "SetAVTransportURI",
                &[("CurrentURI", &format!("x-rincon:{}", coord_uuid))],
            ).await?;
        }

        Ok(())
    }

    /// Ungroup a room
    pub async fn ungroup(&self, room: &str) -> Result<()> {
        let room = self.get_room_or_default(room)?;
        self.soap_action(&room.ip, "BecomeCoordinatorOfStandaloneGroup").await
    }

    /// Get room or default
    fn get_room_or_default(&self, name: &str) -> Result<SonosRoom> {
        if let Some(room) = self.rooms.get(name) {
            return Ok(room.clone());
        }
        
        if let Some(ref default) = self.config.default_room {
            if let Some(room) = self.rooms.get(default) {
                return Ok(room.clone());
            }
        }

        Err(DrivenError::NotFound(format!("Room '{}' not found", name)))
    }

    /// SSDP discovery
    async fn ssdp_discover(&self) -> Result<Vec<SonosRoom>> {
        // Placeholder for SSDP discovery
        // In production, would use M-SEARCH to find Sonos devices
        tracing::debug!("Discovering Sonos devices...");
        Ok(Vec::new())
    }

    /// SOAP action (no args)
    async fn soap_action(&self, ip: &IpAddr, action: &str) -> Result<()> {
        self.soap_action_with_args(ip, action, &[]).await
    }

    /// SOAP action with args
    async fn soap_action_with_args(&self, ip: &IpAddr, action: &str, args: &[(&str, &str)]) -> Result<()> {
        let _ = self.soap_request(ip, action, args).await?;
        Ok(())
    }

    /// Make SOAP request
    async fn soap_request(&self, ip: &IpAddr, action: &str, args: &[(&str, &str)]) -> Result<String> {
        let url = format!("http://{}:1400/MediaRenderer/AVTransport/Control", ip);
        
        // Build SOAP envelope
        let mut args_xml = String::new();
        for (name, value) in args {
            args_xml.push_str(&format!("<{}>{}</{}>", name, value, name));
        }

        let body = format!(
            r#"<?xml version="1.0" encoding="utf-8"?>
            <s:Envelope xmlns:s="http://schemas.xmlsoap.org/soap/envelope/" s:encodingStyle="http://schemas.xmlsoap.org/soap/encoding/">
                <s:Body>
                    <u:{action} xmlns:u="urn:schemas-upnp-org:service:AVTransport:1">
                        <InstanceID>0</InstanceID>
                        {args}
                    </u:{action}>
                </s:Body>
            </s:Envelope>"#,
            action = action,
            args = args_xml
        );

        let client = reqwest::Client::new();
        let response = client
            .post(&url)
            .header("Content-Type", "text/xml; charset=\"utf-8\"")
            .header("SOAPACTION", format!("\"urn:schemas-upnp-org:service:AVTransport:1#{}\"", action))
            .body(body)
            .send()
            .await
            .map_err(|e| DrivenError::Network(e.to_string()))?;

        response
            .text()
            .await
            .map_err(|e| DrivenError::Network(e.to_string()))
    }

    /// Parse volume from response
    fn parse_volume_response(&self, _response: &str) -> Result<u8> {
        // Parse XML response for CurrentVolume
        Ok(50) // Placeholder
    }

    /// Parse transport state
    fn parse_transport_state(&self, _response: &str) -> Result<SonosPlaybackState> {
        // Parse XML response for CurrentTransportState
        Ok(SonosPlaybackState::Stopped) // Placeholder
    }

    /// Parse track info
    fn parse_track_info(&self, _response: &str) -> Result<Option<SonosTrack>> {
        // Parse XML response for track metadata
        Ok(None) // Placeholder
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = SonosConfig::default();
        assert_eq!(config.discovery_timeout, 5);
    }

    #[test]
    fn test_client_creation() {
        let config = SonosConfig::default();
        let client = SonosClient::new(&config);
        assert!(client.is_ok());
    }
}
