//! Messaging Channels
//!
//! Provides a unified interface for messaging channels including WhatsApp,
//! Telegram, Discord, Slack, and webhooks.
//!
//! # Architecture
//!
//! - [`Channel`] trait for unified messaging
//! - [`ChannelRegistry`] for discovery and management
//! - Hot-reload support for `.dx/channels/*.sr` configs
//!
//! # Example
//!
//! ```rust,ignore
//! use dx_cli::channels::{ChannelRegistry, MessageContent, ChannelMessage};
//!
//! let mut registry = ChannelRegistry::new();
//! registry.discover().await?;
//!
//! if let Some(channel) = registry.get("whatsapp") {
//!     channel.send(ChannelMessage {
//!         to: "+123456789".to_string(),
//!         content: MessageContent::Text("Hello!".to_string()),
//!         metadata: Default::default(),
//!     }).await?;
//! }
//! ```

pub mod credentials;
pub mod discord;
pub mod executor;
pub mod imessage;
pub mod queue;
pub mod signal;
pub mod slack;
pub mod telegram;
pub mod trait_def;
pub mod webhook;
pub mod whatsapp;

pub use credentials::{ChannelCredentials, CredentialsStore};
pub use executor::{ChannelExecutor, ChannelManager, ChannelType};
pub use queue::{MessageQueue, QueuedMessage};
pub use trait_def::{Channel, ChannelConfig};

use anyhow::{Context, Result};
use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock, mpsc::channel};

use discord::DiscordChannel;
use slack::SlackChannel;
use telegram::TelegramChannel;
use webhook::WebhookChannel;
use whatsapp::WhatsAppChannel;

/// Channel registry for discovery and management
pub struct ChannelRegistry {
    /// Registered channels
    channels: HashMap<String, Arc<dyn Channel>>,
    /// Channel configurations
    configs: Arc<RwLock<HashMap<String, ChannelConfig>>>,
    /// Config directory
    config_dir: PathBuf,
    /// Hot-reload watcher
    watcher: Option<RecommendedWatcher>,
}

impl ChannelRegistry {
    /// Create a new registry
    pub fn new() -> Self {
        let config_dir = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("dx")
            .join("channels");

        Self {
            channels: HashMap::new(),
            configs: Arc::new(RwLock::new(HashMap::new())),
            config_dir,
            watcher: None,
        }
    }

    /// Initialize registry with built-in channels
    pub async fn initialize(&mut self) -> Result<()> {
        // Create config directory if it doesn't exist
        if !self.config_dir.exists() {
            std::fs::create_dir_all(&self.config_dir)
                .context("Failed to create channels config directory")?;
        }

        // Register built-in channels
        self.register_builtin_channels().await?;

        Ok(())
    }

    /// Discover channels from config files
    pub async fn discover(&mut self) -> Result<()> {
        self.initialize().await?;

        // Load all .sr files in config directory
        if self.config_dir.exists() {
            for entry in std::fs::read_dir(&self.config_dir)? {
                let entry = entry?;
                let path = entry.path();

                if path.extension().and_then(|s| s.to_str()) == Some("sr") {
                    self.load_config(&path)?;
                }
            }
        }

        Ok(())
    }

    /// Register built-in channels
    async fn register_builtin_channels(&mut self) -> Result<()> {
        // WhatsApp
        self.register_channel(Arc::new(WhatsAppChannel::default()));

        // Telegram
        self.register_channel(Arc::new(TelegramChannel::default()));

        // Discord
        self.register_channel(Arc::new(DiscordChannel::default()));

        // Slack
        self.register_channel(Arc::new(SlackChannel::default()));

        // Webhook
        self.register_channel(Arc::new(WebhookChannel::default()));

        Ok(())
    }

    /// Register a channel
    pub fn register_channel(&mut self, channel: Arc<dyn Channel>) {
        self.channels.insert(channel.name().to_string(), channel);
    }

    /// Get a channel by name
    pub fn get(&self, name: &str) -> Option<Arc<dyn Channel>> {
        self.channels.get(name).cloned()
    }

    /// List all channels
    pub fn list(&self) -> Vec<String> {
        self.channels.keys().cloned().collect()
    }

    /// Load a channel config file
    fn load_config(&mut self, path: &Path) -> Result<()> {
        let content = std::fs::read_to_string(path)?;

        // For now, parse as JSON (placeholder for .sr)
        let config: ChannelConfig = serde_json::from_str(&content).unwrap_or_else(|_| {
            // Fallback to default config with name from filename
            let name = path.file_stem().and_then(|s| s.to_str()).unwrap_or("unknown");
            ChannelConfig::new(name)
        });

        if let Ok(mut configs) = self.configs.write() {
            configs.insert(config.name.clone(), config);
        }
        Ok(())
    }

    /// Start hot-reload watcher
    pub async fn start_hot_reload(&mut self) -> Result<()> {
        let config_dir = self.config_dir.clone();
        let configs = Arc::clone(&self.configs);
        let (tx, rx) = channel();

        let mut watcher = RecommendedWatcher::new(
            move |res| {
                if let Ok(event) = res {
                    let _ = tx.send(event);
                }
            },
            Config::default(),
        )?;

        watcher.watch(&config_dir, RecursiveMode::NonRecursive)?;
        self.watcher = Some(watcher);

        std::thread::spawn(move || {
            while let Ok(event) = rx.recv() {
                if let Some(path) = event.paths.first() {
                    if path.extension().and_then(|s| s.to_str()) == Some("sr") {
                        if let Ok(content) = std::fs::read_to_string(path) {
                            if let Ok(config) = serde_json::from_str::<ChannelConfig>(&content) {
                                if let Ok(mut map) = configs.write() {
                                    map.insert(config.name.clone(), config);
                                }
                            }
                        }
                    }
                }
            }
        });

        Ok(())
    }

    /// Stop hot-reload watcher
    pub fn stop_hot_reload(&mut self) {
        self.watcher = None;
    }

    /// Reload all configs
    pub fn reload(&mut self) -> Result<()> {
        if let Ok(mut configs) = self.configs.write() {
            configs.clear();
        }
        if self.config_dir.exists() {
            for entry in std::fs::read_dir(&self.config_dir)? {
                let entry = entry?;
                let path = entry.path();

                if path.extension().and_then(|s| s.to_str()) == Some("sr") {
                    self.load_config(&path)?;
                }
            }
        }
        Ok(())
    }

    /// Get config for a channel
    pub fn config(&self, name: &str) -> Option<ChannelConfig> {
        self.configs.read().ok().and_then(|configs| configs.get(name).cloned())
    }

    /// Update config for a channel
    pub fn update_config(&mut self, config: ChannelConfig) -> Result<()> {
        let path = self.config_dir.join(format!("{}.sr", config.name));
        let content = serde_json::to_string_pretty(&config)?;
        std::fs::write(&path, content)?;

        if let Ok(mut configs) = self.configs.write() {
            configs.insert(config.name.clone(), config);
        }
        Ok(())
    }
}

impl Default for ChannelRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_registry_creation() {
        let mut registry = ChannelRegistry::new();
        assert!(registry.initialize().await.is_ok());
    }

    #[test]
    fn test_config_serialization() {
        let config = ChannelConfig::new("test");
        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("test"));
    }

    #[tokio::test]
    async fn test_discovery_with_temp_dir() {
        let dir = tempdir().unwrap();
        let mut registry = ChannelRegistry {
            config_dir: dir.path().to_path_buf(),
            ..ChannelRegistry::new()
        };

        // Create a fake config
        let config = ChannelConfig::new("telegram");
        let content = serde_json::to_string(&config).unwrap();
        std::fs::write(dir.path().join("telegram.sr"), content).unwrap();

        registry.discover().await.unwrap();
        assert!(registry.config("telegram").is_some());
    }
}
