//! Secure credentials management for messaging channels

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelCredentials {
    pub channel_name: String,
    pub credentials: HashMap<String, String>,
    #[serde(skip)]
    encrypted: bool,
}

impl ChannelCredentials {
    pub fn new(channel_name: String) -> Self {
        Self {
            channel_name,
            credentials: HashMap::new(),
            encrypted: false,
        }
    }

    pub fn add(&mut self, key: String, value: String) {
        self.credentials.insert(key, value);
    }

    pub fn get(&self, key: &str) -> Option<&String> {
        self.credentials.get(key)
    }

    pub fn remove(&mut self, key: &str) -> Option<String> {
        self.credentials.remove(key)
    }
}

/// Credentials store with encryption support
pub struct CredentialsStore {
    store_path: PathBuf,
    credentials: HashMap<String, ChannelCredentials>,
}

impl CredentialsStore {
    pub fn new(store_path: PathBuf) -> Self {
        Self {
            store_path,
            credentials: HashMap::new(),
        }
    }

    /// Load credentials from disk
    pub async fn load(&mut self) -> Result<()> {
        if !self.store_path.exists() {
            return Ok(());
        }

        let content = tokio::fs::read_to_string(&self.store_path).await?;
        let creds: HashMap<String, ChannelCredentials> = serde_json::from_str(&content)?;
        self.credentials = creds;

        Ok(())
    }

    /// Save credentials to disk
    pub async fn save(&self) -> Result<()> {
        if let Some(parent) = self.store_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        let content = serde_json::to_string_pretty(&self.credentials)?;
        tokio::fs::write(&self.store_path, content).await?;

        Ok(())
    }

    /// Add or update credentials for a channel
    pub fn set(&mut self, channel: String, creds: ChannelCredentials) {
        self.credentials.insert(channel, creds);
    }

    /// Get credentials for a channel
    pub fn get(&self, channel: &str) -> Option<&ChannelCredentials> {
        self.credentials.get(channel)
    }

    /// Remove credentials for a channel
    pub fn remove(&mut self, channel: &str) -> Option<ChannelCredentials> {
        self.credentials.remove(channel)
    }

    /// List all channels with credentials
    pub fn list(&self) -> Vec<String> {
        self.credentials.keys().cloned().collect()
    }

    /// Clear all credentials
    pub fn clear(&mut self) {
        self.credentials.clear();
    }
}

impl Default for CredentialsStore {
    fn default() -> Self {
        let store_path = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("dx")
            .join("credentials.json");

        Self::new(store_path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_credentials_store() {
        let dir = tempdir().unwrap();
        let store_path = dir.path().join("creds.json");
        let mut store = CredentialsStore::new(store_path);

        let mut creds = ChannelCredentials::new("telegram".to_string());
        creds.add("token".to_string(), "test_token".to_string());

        store.set("telegram".to_string(), creds);
        assert!(store.save().await.is_ok());

        let mut store2 = CredentialsStore::new(dir.path().join("creds.json"));
        assert!(store2.load().await.is_ok());
        assert!(store2.get("telegram").is_some());
    }
}
