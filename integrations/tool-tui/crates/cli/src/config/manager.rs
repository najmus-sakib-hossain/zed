//! Configuration Manager
//!
//! Central configuration management with loading, caching, hot-reload,
//! encryption, and validation support.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use tokio::sync::{RwLock, broadcast};

use super::config_validation::validate_config;
use super::defaults::{config_search_paths, find_config_file};
use super::encryption::ConfigEncryption;
use super::env::substitute_in_yaml_value;
use super::gateway_config::GatewayCliConfig;
use super::includes::process_includes;
use super::schema::{IssueSeverity, ValidationIssue};
use super::watcher::ConfigWatcher;

/// Configuration manager for the DX CLI and gateway
pub struct ConfigManager {
    /// Current configuration
    config: Arc<RwLock<GatewayCliConfig>>,
    /// Path to the active configuration file
    config_path: Option<PathBuf>,
    /// Encryption handler (if enabled)
    encryption: Option<ConfigEncryption>,
    /// File watcher for hot-reload
    watcher: Option<ConfigWatcher>,
    /// Config change broadcast channel
    change_tx: broadcast::Sender<Arc<GatewayCliConfig>>,
    /// Config version counter
    version: Arc<RwLock<u64>>,
}

/// Configuration load options
#[derive(Debug, Clone)]
pub struct LoadOptions {
    /// Path to config file (None = auto-detect)
    pub path: Option<PathBuf>,
    /// Enable environment variable substitution
    pub env_substitution: bool,
    /// Enable file includes
    pub process_includes: bool,
    /// Enable hot-reload watcher
    pub watch: bool,
    /// Debounce duration for watcher (ms)
    pub watch_debounce_ms: u64,
    /// Encryption key (for decrypting secrets)
    pub encryption_key: Option<[u8; 32]>,
    /// Validate config after loading
    pub validate: bool,
}

impl Default for LoadOptions {
    fn default() -> Self {
        Self {
            path: None,
            env_substitution: true,
            process_includes: true,
            watch: false,
            watch_debounce_ms: 500,
            encryption_key: None,
            validate: true,
        }
    }
}

/// Config manager errors
#[derive(Debug, thiserror::Error)]
pub enum ConfigManagerError {
    #[error("Config file not found. Searched: {0}")]
    NotFound(String),

    #[error("Failed to read config: {0}")]
    ReadError(String),

    #[error("Failed to parse config: {0}")]
    ParseError(String),

    #[error("Environment variable error: {0}")]
    EnvError(String),

    #[error("Include processing error: {0}")]
    IncludeError(String),

    #[error("Encryption error: {0}")]
    EncryptionError(String),

    #[error("Validation errors: {0:?}")]
    ValidationErrors(Vec<ValidationIssue>),

    #[error("Watcher error: {0}")]
    WatcherError(String),

    #[error("Serialization error: {0}")]
    SerializeError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

impl ConfigManager {
    /// Create a new ConfigManager with default configuration
    pub fn new() -> Self {
        let (change_tx, _) = broadcast::channel(16);
        Self {
            config: Arc::new(RwLock::new(GatewayCliConfig::default())),
            config_path: None,
            encryption: None,
            watcher: None,
            change_tx,
            version: Arc::new(RwLock::new(0)),
        }
    }

    /// Load configuration from file with the given options
    pub async fn load(&mut self, options: LoadOptions) -> Result<(), ConfigManagerError> {
        // Find config file
        let config_path = match options.path {
            Some(ref p) => {
                if p.exists() {
                    p.clone()
                } else {
                    return Err(ConfigManagerError::NotFound(p.display().to_string()));
                }
            }
            None => find_config_file().ok_or_else(|| {
                let paths = config_search_paths();
                let search_list =
                    paths.iter().map(|p| p.display().to_string()).collect::<Vec<_>>().join(", ");
                ConfigManagerError::NotFound(search_list)
            })?,
        };

        // Load and process config
        let config = self.load_from_path(&config_path, &options).await?;

        // Validate if requested
        if options.validate {
            let issues = validate_config(&config);
            let errors: Vec<_> =
                issues.iter().filter(|i| i.severity == IssueSeverity::Error).cloned().collect();
            if !errors.is_empty() {
                return Err(ConfigManagerError::ValidationErrors(errors));
            }
        }

        // Setup encryption
        if let Some(key) = options.encryption_key {
            self.encryption = Some(ConfigEncryption::new(&key));
        }

        // Store config
        *self.config.write().await = config.clone();
        self.config_path = Some(config_path.clone());
        *self.version.write().await += 1;

        // Broadcast change
        let _ = self.change_tx.send(Arc::new(config));

        // Setup watcher if requested
        if options.watch {
            self.start_watching(config_path, options).await?;
        }

        Ok(())
    }

    /// Load configuration from a specific path
    async fn load_from_path(
        &self,
        path: &Path,
        options: &LoadOptions,
    ) -> Result<GatewayCliConfig, ConfigManagerError> {
        // Read file
        let content = std::fs::read_to_string(path)
            .map_err(|e| ConfigManagerError::ReadError(e.to_string()))?;

        // Check if it's TOML (legacy) or YAML
        let is_toml = path.extension().and_then(|e| e.to_str()) == Some("toml");

        let mut yaml_value = if is_toml {
            // Convert TOML to YAML value
            let yaml_str = super::migration::migrate_toml_to_yaml(&content)
                .map_err(|e| ConfigManagerError::ParseError(e.to_string()))?;
            serde_yaml::from_str(&yaml_str)
                .map_err(|e| ConfigManagerError::ParseError(e.to_string()))?
        } else {
            serde_yaml::from_str::<serde_yaml::Value>(&content)
                .map_err(|e| ConfigManagerError::ParseError(e.to_string()))?
        };

        // Process file includes
        if options.process_includes {
            process_includes(path, &mut yaml_value)
                .map_err(|e| ConfigManagerError::IncludeError(e.to_string()))?;
        }

        // Substitute environment variables
        if options.env_substitution {
            substitute_in_yaml_value(&mut yaml_value)
                .map_err(|e| ConfigManagerError::EnvError(e.to_string()))?;
        }

        // Decrypt secrets if encryption is configured
        if let Some(ref enc) = self.encryption {
            enc.decrypt_secrets(&mut yaml_value)
                .map_err(|e| ConfigManagerError::EncryptionError(e.to_string()))?;
        }

        // Deserialize into config struct
        let config: GatewayCliConfig = serde_yaml::from_value(yaml_value)
            .map_err(|e| ConfigManagerError::ParseError(e.to_string()))?;

        Ok(config)
    }

    /// Start file watching for hot-reload
    async fn start_watching(
        &mut self,
        config_path: PathBuf,
        options: LoadOptions,
    ) -> Result<(), ConfigManagerError> {
        let mut watcher = ConfigWatcher::new(options.watch_debounce_ms);
        watcher.watch_path(config_path);

        let mut reload_rx = watcher.subscribe();
        let config = Arc::clone(&self.config);
        let version = Arc::clone(&self.version);
        let change_tx = self.change_tx.clone();
        let encryption = self.encryption.as_ref().map(|_| {
            // We can't clone ConfigEncryption easily, so we create a flag
            // indicating encryption is enabled. The reload will use the
            // manager's encryption instance.
            true
        });

        watcher
            .start()
            .await
            .map_err(|e| ConfigManagerError::WatcherError(e.to_string()))?;

        // Spawn reload handler
        tokio::spawn(async move {
            while let Ok(event) = reload_rx.recv().await {
                tracing::info!("Config file changed: {:?}", event.path);

                // Reload config
                let path = &event.path;
                let content = match std::fs::read_to_string(path) {
                    Ok(c) => c,
                    Err(e) => {
                        tracing::error!("Failed to read config: {}", e);
                        continue;
                    }
                };

                let yaml_value: serde_yaml::Value = match serde_yaml::from_str(&content) {
                    Ok(v) => v,
                    Err(e) => {
                        tracing::error!("Failed to parse config: {}", e);
                        continue;
                    }
                };

                let new_config: GatewayCliConfig = match serde_yaml::from_value(yaml_value) {
                    Ok(c) => c,
                    Err(e) => {
                        tracing::error!("Failed to deserialize config: {}", e);
                        continue;
                    }
                };

                // Validate before applying
                let issues = validate_config(&new_config);
                let errors: Vec<_> =
                    issues.iter().filter(|i| i.severity == IssueSeverity::Error).collect();
                if !errors.is_empty() {
                    tracing::error!("Config validation failed: {:?}", errors);
                    continue;
                }

                // Apply new config
                *config.write().await = new_config.clone();
                *version.write().await += 1;
                let _ = change_tx.send(Arc::new(new_config));
                tracing::info!("Configuration reloaded successfully");
            }
        });

        self.watcher = Some(watcher);
        Ok(())
    }

    /// Get current configuration
    pub async fn get(&self) -> GatewayCliConfig {
        self.config.read().await.clone()
    }

    /// Get a specific config value by dotted path
    pub async fn get_value(&self, path: &str) -> Option<serde_json::Value> {
        let config = self.config.read().await;
        let json = serde_json::to_value(&*config).ok()?;
        get_nested_value(&json, path)
    }

    /// Set a specific config value by dotted path
    pub async fn set_value(
        &self,
        path: &str,
        value: serde_json::Value,
    ) -> Result<(), ConfigManagerError> {
        let mut config = self.config.write().await;
        let mut json = serde_json::to_value(&*config)
            .map_err(|e| ConfigManagerError::SerializeError(e.to_string()))?;

        set_nested_value(&mut json, path, value);

        *config = serde_json::from_value(json)
            .map_err(|e| ConfigManagerError::ParseError(e.to_string()))?;

        *self.version.write().await += 1;
        let _ = self.change_tx.send(Arc::new(config.clone()));
        Ok(())
    }

    /// Save current configuration to file
    pub async fn save(&self) -> Result<(), ConfigManagerError> {
        let config_path = self
            .config_path
            .as_ref()
            .ok_or_else(|| ConfigManagerError::NotFound("No config path set".to_string()))?;

        let config = self.config.read().await;
        let mut yaml_value = serde_yaml::to_value(&*config)
            .map_err(|e| ConfigManagerError::SerializeError(e.to_string()))?;

        // Re-encrypt secrets before saving
        if let Some(ref enc) = self.encryption {
            enc.encrypt_secrets(&mut yaml_value)
                .map_err(|e| ConfigManagerError::EncryptionError(e.to_string()))?;
        }

        let yaml = serde_yaml::to_string(&yaml_value)
            .map_err(|e| ConfigManagerError::SerializeError(e.to_string()))?;

        // Atomic write: write to temp file then rename
        let temp_path = config_path.with_extension("yaml.tmp");
        std::fs::write(&temp_path, &yaml)?;
        std::fs::rename(&temp_path, config_path)?;

        Ok(())
    }

    /// Subscribe to configuration changes
    pub fn subscribe(&self) -> broadcast::Receiver<Arc<GatewayCliConfig>> {
        self.change_tx.subscribe()
    }

    /// Get the current config version
    pub async fn version(&self) -> u64 {
        *self.version.read().await
    }

    /// Get the config file path
    pub fn config_path(&self) -> Option<&Path> {
        self.config_path.as_deref()
    }

    /// Reload configuration from disk
    pub async fn reload(&mut self) -> Result<(), ConfigManagerError> {
        if let Some(path) = self.config_path.clone() {
            let options = LoadOptions {
                path: Some(path),
                env_substitution: true,
                process_includes: true,
                watch: false,
                validate: true,
                ..Default::default()
            };
            let config = self.load_from_path(options.path.as_ref().unwrap(), &options).await?;

            let issues = validate_config(&config);
            let errors: Vec<_> =
                issues.iter().filter(|i| i.severity == IssueSeverity::Error).cloned().collect();
            if !errors.is_empty() {
                return Err(ConfigManagerError::ValidationErrors(errors));
            }

            *self.config.write().await = config.clone();
            *self.version.write().await += 1;
            let _ = self.change_tx.send(Arc::new(config));
        }
        Ok(())
    }

    /// Stop the config watcher
    pub async fn stop_watching(&mut self) {
        if let Some(ref mut watcher) = self.watcher {
            watcher.stop().await;
        }
        self.watcher = None;
    }
}

/// Get a nested value from a JSON object using dot notation
fn get_nested_value(value: &serde_json::Value, path: &str) -> Option<serde_json::Value> {
    let parts: Vec<&str> = path.split('.').collect();
    let mut current = value;

    for part in parts {
        current = current.get(part)?;
    }

    Some(current.clone())
}

/// Set a nested value in a JSON object using dot notation
fn set_nested_value(value: &mut serde_json::Value, path: &str, new_value: serde_json::Value) {
    let parts: Vec<&str> = path.split('.').collect();

    if parts.is_empty() {
        return;
    }

    if parts.len() == 1 {
        if let Some(obj) = value.as_object_mut() {
            obj.insert(parts[0].to_string(), new_value);
        }
        return;
    }

    let mut current = value;
    for part in &parts[..parts.len() - 1] {
        current = current
            .as_object_mut()
            .and_then(|obj| {
                if !obj.contains_key(*part) {
                    obj.insert(part.to_string(), serde_json::json!({}));
                }
                obj.get_mut(*part)
            })
            .expect("path traversal failed");
    }

    if let Some(obj) = current.as_object_mut() {
        obj.insert(parts.last().unwrap().to_string(), new_value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_nested_value() {
        let json = serde_json::json!({
            "gateway": {
                "port": 31337,
                "host": "0.0.0.0"
            },
            "llm": {
                "temperature": 0.7
            }
        });

        assert_eq!(get_nested_value(&json, "gateway.port"), Some(serde_json::json!(31337)));
        assert_eq!(get_nested_value(&json, "llm.temperature"), Some(serde_json::json!(0.7)));
        assert_eq!(get_nested_value(&json, "nonexistent.path"), None);
    }

    #[test]
    fn test_set_nested_value() {
        let mut json = serde_json::json!({
            "gateway": {
                "port": 31337
            }
        });

        set_nested_value(&mut json, "gateway.port", serde_json::json!(8080));
        assert_eq!(json["gateway"]["port"], 8080);
    }

    #[tokio::test]
    async fn test_new_manager() {
        let manager = ConfigManager::new();
        let config = manager.get().await;
        assert_eq!(config.gateway.port, 31337);
    }

    #[tokio::test]
    async fn test_get_set_value() {
        let manager = ConfigManager::new();

        let port = manager.get_value("gateway.port").await;
        assert_eq!(port, Some(serde_json::json!(31337)));

        manager.set_value("gateway.port", serde_json::json!(8080)).await.unwrap();

        let port = manager.get_value("gateway.port").await;
        assert_eq!(port, Some(serde_json::json!(8080)));
    }

    #[tokio::test]
    async fn test_version_increments() {
        let manager = ConfigManager::new();
        let v1 = manager.version().await;

        manager.set_value("gateway.port", serde_json::json!(9999)).await.unwrap();

        let v2 = manager.version().await;
        assert_eq!(v2, v1 + 1);
    }

    #[tokio::test]
    async fn test_subscribe() {
        let manager = ConfigManager::new();
        let mut rx = manager.subscribe();

        manager.set_value("gateway.port", serde_json::json!(9999)).await.unwrap();

        let new_config = rx.recv().await.unwrap();
        assert_eq!(new_config.gateway.port, 9999);
    }

    #[tokio::test]
    async fn test_load_from_yaml_file() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("config.yaml");
        std::fs::write(&config_path, "gateway:\n  port: 9999\nllm:\n  temperature: 0.5\n").unwrap();

        let mut manager = ConfigManager::new();
        manager
            .load(LoadOptions {
                path: Some(config_path),
                env_substitution: false,
                process_includes: false,
                watch: false,
                validate: true,
                ..Default::default()
            })
            .await
            .unwrap();

        let config = manager.get().await;
        assert_eq!(config.gateway.port, 9999);
        assert_eq!(config.llm.temperature, 0.5);
    }

    #[tokio::test]
    async fn test_save_and_reload() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("config.yaml");
        std::fs::write(&config_path, "gateway:\n  port: 31337\n").unwrap();

        let mut manager = ConfigManager::new();
        manager
            .load(LoadOptions {
                path: Some(config_path.clone()),
                validate: false,
                ..Default::default()
            })
            .await
            .unwrap();

        // Modify and save
        manager.set_value("gateway.port", serde_json::json!(8080)).await.unwrap();
        manager.save().await.unwrap();

        // Reload and verify
        manager.reload().await.unwrap();
        let config = manager.get().await;
        assert_eq!(config.gateway.port, 8080);
    }
}
