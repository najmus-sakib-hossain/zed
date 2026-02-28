//! Unified channel configuration management.
//!
//! Loads and saves per-channel configuration from TOML files,
//! combining security policies, threading, and media limits.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::media::MediaLimits;
use crate::security::SecurityPolicy;
use crate::threading::ReplyMode;

/// Threading configuration for a channel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreadingConfig {
    /// How outgoing replies behave.
    #[serde(default)]
    pub reply_mode: ReplyMode,
    /// Whether to allow topic-based threading.
    #[serde(default)]
    pub allow_threads: bool,
}

impl Default for ThreadingConfig {
    fn default() -> Self {
        Self {
            reply_mode: ReplyMode::All,
            allow_threads: true,
        }
    }
}

/// Complete configuration for one channel instance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelConfig {
    /// Channel identifier.
    pub channel_id: String,
    /// Whether this channel is enabled.
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Display name override.
    pub display_name: Option<String>,
    /// Security policies.
    #[serde(default)]
    pub security: SecurityPolicy,
    /// Threading configuration.
    #[serde(default)]
    pub threading: ThreadingConfig,
    /// Media upload limits.
    #[serde(default)]
    pub media_limits: MediaLimits,
    /// Streaming / coalescing configuration.
    #[serde(default)]
    pub streaming: StreamingConfig,
}

fn default_true() -> bool {
    true
}

/// Streaming settings within a channel config.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamingConfig {
    /// Minimum characters before flush.
    #[serde(default = "default_min_chars")]
    pub min_chars: usize,
    /// Idle timeout in ms.
    #[serde(default = "default_idle_ms")]
    pub idle_ms: u64,
    /// Hard character limit per message part.
    #[serde(default = "default_max_chars")]
    pub max_chars: usize,
}

fn default_min_chars() -> usize {
    80
}
fn default_idle_ms() -> u64 {
    500
}
fn default_max_chars() -> usize {
    4000
}

impl Default for StreamingConfig {
    fn default() -> Self {
        Self {
            min_chars: default_min_chars(),
            idle_ms: default_idle_ms(),
            max_chars: default_max_chars(),
        }
    }
}

impl Default for ChannelConfig {
    fn default() -> Self {
        Self {
            channel_id: String::new(),
            enabled: true,
            display_name: None,
            security: SecurityPolicy::default(),
            threading: ThreadingConfig::default(),
            media_limits: MediaLimits::default(),
            streaming: StreamingConfig::default(),
        }
    }
}

impl ChannelConfig {
    /// Create a minimal config for a channel.
    pub fn new(channel_id: impl Into<String>) -> Self {
        Self {
            channel_id: channel_id.into(),
            ..Default::default()
        }
    }

    /// Set security policy.
    pub fn with_security(mut self, policy: SecurityPolicy) -> Self {
        self.security = policy;
        self
    }

    /// Set threading config.
    pub fn with_threading(mut self, config: ThreadingConfig) -> Self {
        self.threading = config;
        self
    }

    /// Set media limits.
    pub fn with_media_limits(mut self, limits: MediaLimits) -> Self {
        self.media_limits = limits;
        self
    }
}

/// Load a channel config from a TOML file.
pub fn load_config(path: &str) -> Result<ChannelConfig> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read config file: {}", path))?;
    let config: ChannelConfig = toml::from_str(&content)
        .with_context(|| format!("Failed to parse config file: {}", path))?;
    Ok(config)
}

/// Save a channel config to a TOML file.
pub fn save_config(path: &str, config: &ChannelConfig) -> Result<()> {
    let content = toml::to_string_pretty(config).context("Failed to serialize config to TOML")?;
    std::fs::write(path, content)
        .with_context(|| format!("Failed to write config file: {}", path))?;
    Ok(())
}

/// Load config from a JSON string.
pub fn from_json(json: &str) -> Result<ChannelConfig> {
    serde_json::from_str(json).context("Failed to parse channel config from JSON")
}

/// Serialize config to a JSON string.
pub fn to_json(config: &ChannelConfig) -> Result<String> {
    serde_json::to_string_pretty(config).context("Failed to serialize channel config to JSON")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::security::DmPolicy;

    #[test]
    fn test_default_config() {
        let cfg = ChannelConfig::default();
        assert!(cfg.enabled);
        assert!(cfg.channel_id.is_empty());
        assert_eq!(cfg.threading.reply_mode, ReplyMode::All);
    }

    #[test]
    fn test_new_config() {
        let cfg = ChannelConfig::new("telegram");
        assert_eq!(cfg.channel_id, "telegram");
        assert!(cfg.enabled);
    }

    #[test]
    fn test_builder_methods() {
        let cfg = ChannelConfig::new("tg")
            .with_security(SecurityPolicy {
                dm_policy: DmPolicy::Implicit,
                ..Default::default()
            })
            .with_threading(ThreadingConfig {
                reply_mode: ReplyMode::First,
                allow_threads: false,
            })
            .with_media_limits(MediaLimits::telegram());

        assert_eq!(cfg.security.dm_policy, DmPolicy::Implicit);
        assert_eq!(cfg.threading.reply_mode, ReplyMode::First);
        assert!(!cfg.threading.allow_threads);
    }

    #[test]
    fn test_json_roundtrip() {
        let cfg = ChannelConfig::new("discord").with_media_limits(MediaLimits::discord());

        let json = to_json(&cfg).expect("serialize");
        let restored = from_json(&json).expect("deserialize");
        assert_eq!(restored.channel_id, "discord");
    }

    #[test]
    fn test_toml_serialization() {
        let cfg = ChannelConfig::new("slack");
        let toml_str = toml::to_string_pretty(&cfg).expect("serialize to TOML");
        assert!(toml_str.contains("slack"));

        let restored: ChannelConfig = toml::from_str(&toml_str).expect("parse TOML");
        assert_eq!(restored.channel_id, "slack");
    }

    #[test]
    fn test_load_config_file_not_found() {
        let result = load_config("/nonexistent/path.toml");
        assert!(result.is_err());
    }

    #[test]
    fn test_save_and_load_config() {
        let dir = std::env::temp_dir();
        let path = dir.join("dx_test_channel_config.toml").to_string_lossy().to_string();

        let cfg = ChannelConfig::new("test-channel");
        save_config(&path, &cfg).expect("save");

        let loaded = load_config(&path).expect("load");
        assert_eq!(loaded.channel_id, "test-channel");

        // Cleanup
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_streaming_defaults() {
        let s = StreamingConfig::default();
        assert_eq!(s.min_chars, 80);
        assert_eq!(s.idle_ms, 500);
        assert_eq!(s.max_chars, 4000);
    }
}
