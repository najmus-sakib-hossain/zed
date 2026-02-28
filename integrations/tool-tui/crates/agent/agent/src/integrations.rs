//! # Integration System
//!
//! Connect to ANY app: WhatsApp, Telegram, Discord, GitHub, Notion, Spotify, etc.
//! Each integration is defined in DX Serializer format and can be hot-reloaded.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::info;

use crate::Result;

/// Configuration for an integration in DX Serializer format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrationConfig {
    /// Unique name of the integration
    pub name: String,

    /// Type of integration (messaging, tool, service)
    pub kind: IntegrationKind,

    /// API endpoint or connection string
    pub endpoint: Option<String>,

    /// Required authentication method
    pub auth_method: AuthMethod,

    /// Capabilities this integration provides
    pub capabilities: Vec<String>,

    /// Path to the WASM plugin (if custom)
    pub wasm_path: Option<PathBuf>,

    /// Whether the integration is enabled
    pub enabled: bool,
}

/// Types of integrations
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum IntegrationKind {
    /// Messaging platforms (WhatsApp, Telegram, Discord, etc.)
    Messaging,
    /// Developer tools (GitHub, GitLab, etc.)
    DevTool,
    /// Productivity (Notion, Todoist, etc.)
    Productivity,
    /// Media (Spotify, YouTube, etc.)
    Media,
    /// Browser control
    Browser,
    /// Custom/other
    Custom,
}

/// Authentication methods supported
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthMethod {
    /// OAuth 2.0 flow
    OAuth2 {
        client_id: String,
        scopes: Vec<String>,
    },
    /// API key
    ApiKey,
    /// Username/password
    Basic,
    /// No authentication needed
    None,
}

/// A single integration instance
pub struct Integration {
    config: IntegrationConfig,
    connected: bool,
    wasm_module: Option<Arc<dyn IntegrationModule>>,
}

/// Trait for integration modules (implemented by WASM plugins)
#[async_trait::async_trait]
pub trait IntegrationModule: Send + Sync {
    /// Initialize the integration
    async fn init(&self) -> Result<()>;

    /// Connect to the service
    async fn connect(&self, auth_token: &str) -> Result<()>;

    /// Disconnect from the service
    async fn disconnect(&self) -> Result<()>;

    /// Send a message
    async fn send_message(&self, message: &str) -> Result<()>;

    /// Poll for new messages
    async fn poll_messages(&self) -> Result<Vec<String>>;

    /// Execute a custom action
    async fn execute_action(&self, action: &str, params: &str) -> Result<String>;
}

impl Integration {
    pub fn new(config: IntegrationConfig) -> Self {
        Self {
            config,
            connected: false,
            wasm_module: None,
        }
    }

    pub fn name(&self) -> &str {
        &self.config.name
    }

    pub fn kind(&self) -> &IntegrationKind {
        &self.config.kind
    }

    pub fn is_connected(&self) -> bool {
        self.connected
    }

    pub async fn connect(&mut self, auth_token: &str) -> Result<()> {
        if let Some(module) = &self.wasm_module {
            module.connect(auth_token).await?;
        }
        self.connected = true;
        Ok(())
    }

    pub async fn disconnect(&mut self) -> Result<()> {
        if let Some(module) = &self.wasm_module {
            module.disconnect().await?;
        }
        self.connected = false;
        Ok(())
    }

    pub async fn send_message(&self, message: &str) -> Result<()> {
        if let Some(module) = &self.wasm_module {
            module.send_message(message).await?;
        }
        Ok(())
    }

    pub async fn poll_messages(&self) -> Result<Vec<String>> {
        if let Some(module) = &self.wasm_module {
            return module.poll_messages().await;
        }
        Ok(vec![])
    }

    pub fn set_module(&mut self, module: Arc<dyn IntegrationModule>) {
        self.wasm_module = Some(module);
    }
}

/// Manages all integrations
pub struct IntegrationManager {
    integrations: HashMap<String, Integration>,
    config_path: PathBuf,
}

impl IntegrationManager {
    pub async fn new(dx_path: &Path) -> Result<Self> {
        let config_path = dx_path.join("integrations");
        std::fs::create_dir_all(&config_path)?;

        Ok(Self {
            integrations: HashMap::new(),
            config_path,
        })
    }

    /// Load all integration configurations from DX Serializer format
    pub async fn load_all(&mut self) -> Result<usize> {
        let mut count = 0;

        // Load built-in integrations
        self.load_builtin_integrations().await?;

        // Load custom integrations from .sr files
        if let Ok(entries) = std::fs::read_dir(&self.config_path) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().is_some_and(|e| e == "sr") {
                    if let Ok(config) = self.load_config(&path).await {
                        let name = config.name.clone();
                        self.integrations.insert(name, Integration::new(config));
                        count += 1;
                    }
                }
            }
        }

        Ok(count)
    }

    /// Load built-in integrations (pre-configured)
    async fn load_builtin_integrations(&mut self) -> Result<()> {
        // WhatsApp
        self.integrations.insert(
            "whatsapp".to_string(),
            Integration::new(IntegrationConfig {
                name: "whatsapp".to_string(),
                kind: IntegrationKind::Messaging,
                endpoint: Some("https://api.whatsapp.com".to_string()),
                auth_method: AuthMethod::OAuth2 {
                    client_id: String::new(),
                    scopes: vec!["messages.read".to_string(), "messages.write".to_string()],
                },
                capabilities: vec!["send_message".to_string(), "receive_message".to_string()],
                wasm_path: None,
                enabled: false,
            }),
        );

        // Telegram
        self.integrations.insert(
            "telegram".to_string(),
            Integration::new(IntegrationConfig {
                name: "telegram".to_string(),
                kind: IntegrationKind::Messaging,
                endpoint: Some("https://api.telegram.org".to_string()),
                auth_method: AuthMethod::ApiKey,
                capabilities: vec![
                    "send_message".to_string(),
                    "receive_message".to_string(),
                    "send_file".to_string(),
                ],
                wasm_path: None,
                enabled: false,
            }),
        );

        // Discord
        self.integrations.insert(
            "discord".to_string(),
            Integration::new(IntegrationConfig {
                name: "discord".to_string(),
                kind: IntegrationKind::Messaging,
                endpoint: Some("https://discord.com/api".to_string()),
                auth_method: AuthMethod::OAuth2 {
                    client_id: String::new(),
                    scopes: vec!["bot".to_string(), "messages.read".to_string()],
                },
                capabilities: vec![
                    "send_message".to_string(),
                    "receive_message".to_string(),
                    "manage_channels".to_string(),
                ],
                wasm_path: None,
                enabled: false,
            }),
        );

        // GitHub
        self.integrations.insert(
            "github".to_string(),
            Integration::new(IntegrationConfig {
                name: "github".to_string(),
                kind: IntegrationKind::DevTool,
                endpoint: Some("https://api.github.com".to_string()),
                auth_method: AuthMethod::OAuth2 {
                    client_id: String::new(),
                    scopes: vec!["repo".to_string(), "user".to_string()],
                },
                capabilities: vec![
                    "create_pr".to_string(),
                    "create_issue".to_string(),
                    "list_repos".to_string(),
                ],
                wasm_path: None,
                enabled: false,
            }),
        );

        // Notion
        self.integrations.insert(
            "notion".to_string(),
            Integration::new(IntegrationConfig {
                name: "notion".to_string(),
                kind: IntegrationKind::Productivity,
                endpoint: Some("https://api.notion.com".to_string()),
                auth_method: AuthMethod::OAuth2 {
                    client_id: String::new(),
                    scopes: vec!["read_content".to_string(), "write_content".to_string()],
                },
                capabilities: vec![
                    "read_page".to_string(),
                    "write_page".to_string(),
                    "query_database".to_string(),
                ],
                wasm_path: None,
                enabled: false,
            }),
        );

        // Spotify
        self.integrations.insert(
            "spotify".to_string(),
            Integration::new(IntegrationConfig {
                name: "spotify".to_string(),
                kind: IntegrationKind::Media,
                endpoint: Some("https://api.spotify.com".to_string()),
                auth_method: AuthMethod::OAuth2 {
                    client_id: String::new(),
                    scopes: vec![
                        "user-read-playback-state".to_string(),
                        "user-modify-playback-state".to_string(),
                    ],
                },
                capabilities: vec![
                    "play".to_string(),
                    "pause".to_string(),
                    "next".to_string(),
                    "search".to_string(),
                ],
                wasm_path: None,
                enabled: false,
            }),
        );

        // Slack
        self.integrations.insert(
            "slack".to_string(),
            Integration::new(IntegrationConfig {
                name: "slack".to_string(),
                kind: IntegrationKind::Messaging,
                endpoint: Some("https://slack.com/api".to_string()),
                auth_method: AuthMethod::OAuth2 {
                    client_id: String::new(),
                    scopes: vec!["chat:write".to_string(), "channels:read".to_string()],
                },
                capabilities: vec!["send_message".to_string(), "receive_message".to_string()],
                wasm_path: None,
                enabled: false,
            }),
        );

        // X (Twitter)
        self.integrations.insert(
            "x".to_string(),
            Integration::new(IntegrationConfig {
                name: "x".to_string(),
                kind: IntegrationKind::Messaging,
                endpoint: Some("https://api.twitter.com".to_string()),
                auth_method: AuthMethod::OAuth2 {
                    client_id: String::new(),
                    scopes: vec![
                        "tweet.read".to_string(),
                        "tweet.write".to_string(),
                        "dm.read".to_string(),
                    ],
                },
                capabilities: vec![
                    "post_tweet".to_string(),
                    "send_dm".to_string(),
                    "read_dm".to_string(),
                ],
                wasm_path: None,
                enabled: false,
            }),
        );

        // Browser
        self.integrations.insert(
            "browser".to_string(),
            Integration::new(IntegrationConfig {
                name: "browser".to_string(),
                kind: IntegrationKind::Browser,
                endpoint: None,
                auth_method: AuthMethod::None,
                capabilities: vec![
                    "navigate".to_string(),
                    "click".to_string(),
                    "type".to_string(),
                    "screenshot".to_string(),
                ],
                wasm_path: None,
                enabled: true,
            }),
        );

        Ok(())
    }

    /// Load a config from a .sr file (DX Serializer format)
    async fn load_config(&self, path: &Path) -> Result<IntegrationConfig> {
        let content = std::fs::read_to_string(path)?;
        // Parse DX Serializer format
        // Format: name=value key=value ...
        let mut config = IntegrationConfig {
            name: String::new(),
            kind: IntegrationKind::Custom,
            endpoint: None,
            auth_method: AuthMethod::None,
            capabilities: vec![],
            wasm_path: None,
            enabled: true,
        };

        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            if let Some((key, value)) = line.split_once('=') {
                match key.trim() {
                    "name" => config.name = value.trim().to_string(),
                    "kind" => {
                        config.kind = match value.trim() {
                            "messaging" => IntegrationKind::Messaging,
                            "devtool" => IntegrationKind::DevTool,
                            "productivity" => IntegrationKind::Productivity,
                            "media" => IntegrationKind::Media,
                            "browser" => IntegrationKind::Browser,
                            _ => IntegrationKind::Custom,
                        }
                    }
                    "endpoint" => config.endpoint = Some(value.trim().to_string()),
                    "enabled" => config.enabled = value.trim() == "true",
                    "wasm_path" => config.wasm_path = Some(PathBuf::from(value.trim())),
                    _ => {}
                }
            }
        }

        Ok(config)
    }

    /// Get an integration by name
    pub fn get(&self, name: &str) -> Option<&Integration> {
        self.integrations.get(name)
    }

    /// Get a mutable integration by name
    pub fn get_mut(&mut self, name: &str) -> Option<&mut Integration> {
        self.integrations.get_mut(name)
    }

    /// Iterate over all integrations
    pub fn iter(&self) -> impl Iterator<Item = &Integration> {
        self.integrations.values()
    }

    /// Register a new integration from a plugin
    pub async fn register_from_plugin(&mut self, name: &str) -> Result<()> {
        info!("Registering integration from plugin: {}", name);

        let config = IntegrationConfig {
            name: name.to_string(),
            kind: IntegrationKind::Custom,
            endpoint: None,
            auth_method: AuthMethod::None,
            capabilities: vec![],
            wasm_path: Some(PathBuf::from(format!(".dx/plugins/{}.wasm", name))),
            enabled: true,
        };

        self.integrations
            .insert(name.to_string(), Integration::new(config));

        Ok(())
    }

    /// Save an integration config to a .sr file
    pub async fn save_config(&self, name: &str) -> Result<()> {
        if let Some(int) = self.integrations.get(name) {
            let path = self.config_path.join(format!("{}.sr", name));

            // Generate DX Serializer format
            let content = format!(
                "# DX Integration: {}\n\
                 name={}\n\
                 kind={:?}\n\
                 endpoint={}\n\
                 enabled={}\n",
                name,
                int.config.name,
                int.config.kind,
                int.config.endpoint.as_deref().unwrap_or(""),
                int.config.enabled
            );

            std::fs::write(path, content)?;
        }

        Ok(())
    }
}
