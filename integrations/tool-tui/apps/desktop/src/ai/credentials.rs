use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Credential store for AI provider API keys.
///
/// Stores API keys in a JSON config file in the user's data directory.
/// In production, this should use the OS keychain (Windows Credential Manager,
/// macOS Keychain, Linux Secret Service) via the credentials_provider crate
/// from Zed's codebase.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CredentialStore {
    #[serde(default)]
    credentials: HashMap<String, String>,
    #[serde(skip)]
    config_path: PathBuf,
}

impl CredentialStore {
    /// Load credentials from the config file, or create a new store.
    pub fn load() -> Self {
        let config_path = Self::config_file_path();
        if config_path.exists() {
            if let Ok(data) = std::fs::read_to_string(&config_path) {
                if let Ok(mut store) = serde_json::from_str::<CredentialStore>(&data) {
                    store.config_path = config_path;
                    return store;
                }
            }
        }
        Self {
            credentials: HashMap::new(),
            config_path,
        }
    }

    /// Get the config file path in the user's data directory.
    fn config_file_path() -> PathBuf {
        let base = dirs::data_dir().or_else(dirs::config_dir).unwrap_or_else(|| PathBuf::from("."));
        let dir = base.join("dx").join("desktop");
        let _ = std::fs::create_dir_all(&dir);
        dir.join("credentials.json")
    }

    /// Get the API key for a provider.
    pub fn get_api_key(&self, provider_id: &str) -> Option<&str> {
        self.credentials.get(provider_id).map(|s| s.as_str())
    }

    /// Check if a provider has an API key stored.
    pub fn has_api_key(&self, provider_id: &str) -> bool {
        self.credentials.get(provider_id).map_or(false, |k| !k.is_empty())
    }

    /// Store an API key for a provider and persist to disk.
    pub fn set_api_key(&mut self, provider_id: &str, key: String) {
        if key.is_empty() {
            self.credentials.remove(provider_id);
        } else {
            self.credentials.insert(provider_id.to_string(), key);
        }
        let _ = self.save();
    }

    /// Remove the API key for a provider.
    #[allow(dead_code)]
    pub fn remove_api_key(&mut self, provider_id: &str) {
        self.credentials.remove(provider_id);
        let _ = self.save();
    }

    /// Persist credentials to disk.
    fn save(&self) -> anyhow::Result<()> {
        let json = serde_json::to_string_pretty(self)?;
        if let Some(parent) = self.config_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&self.config_path, json)?;
        Ok(())
    }
}

impl Default for CredentialStore {
    fn default() -> Self {
        Self::load()
    }
}
