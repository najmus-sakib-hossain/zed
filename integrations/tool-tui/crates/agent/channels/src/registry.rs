//! Channel registry for managing multiple channel integrations.

use anyhow::Result;
use dashmap::DashMap;
use std::sync::Arc;
use tracing::{info, warn};

use crate::traits::Channel;

/// Registry that manages all channel integrations
pub struct ChannelRegistry {
    channels: DashMap<String, Arc<dyn Channel>>,
}

impl ChannelRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            channels: DashMap::new(),
        }
    }

    /// Register a channel integration
    pub fn register(&self, channel: Arc<dyn Channel>) {
        let name = channel.name().to_string();
        info!("Registering channel: {}", name);
        self.channels.insert(name, channel);
    }

    /// Get a channel by name
    pub fn get(&self, name: &str) -> Option<Arc<dyn Channel>> {
        self.channels.get(name).map(|r| r.value().clone())
    }

    /// List all registered channel names
    pub fn list(&self) -> Vec<String> {
        self.channels.iter().map(|r| r.key().clone()).collect()
    }

    /// List all enabled channels
    pub fn list_enabled(&self) -> Vec<String> {
        self.channels
            .iter()
            .filter(|r| r.value().is_enabled())
            .map(|r| r.key().clone())
            .collect()
    }

    /// List all connected channels
    pub fn list_connected(&self) -> Vec<String> {
        self.channels
            .iter()
            .filter(|r| r.value().is_connected())
            .map(|r| r.key().clone())
            .collect()
    }

    /// Remove a channel
    pub fn remove(&self, name: &str) -> Option<Arc<dyn Channel>> {
        self.channels.remove(name).map(|(_, v)| v)
    }

    /// Get the number of registered channels
    pub fn count(&self) -> usize {
        self.channels.len()
    }

    /// Connect all enabled channels
    pub async fn connect_all(&self) -> Result<()> {
        let names: Vec<String> = self.list_enabled();
        for name in names {
            if let Some(mut channel) = self.channels.get_mut(&name) {
                let channel = Arc::get_mut(&mut channel)
                    .ok_or_else(|| anyhow::anyhow!("Channel {} is in use, cannot connect", name))?;
                match channel.connect().await {
                    Ok(()) => info!("Connected channel: {}", name),
                    Err(e) => warn!("Failed to connect channel {}: {}", name, e),
                }
            }
        }
        Ok(())
    }

    /// Disconnect all connected channels
    pub async fn disconnect_all(&self) -> Result<()> {
        let names: Vec<String> = self.list_connected();
        for name in names {
            if let Some(mut channel) = self.channels.get_mut(&name) {
                if let Some(channel) = Arc::get_mut(&mut channel) {
                    match channel.disconnect().await {
                        Ok(()) => info!("Disconnected channel: {}", name),
                        Err(e) => warn!("Failed to disconnect channel {}: {}", name, e),
                    }
                }
            }
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
    use crate::message::*;
    use crate::traits::*;
    use async_trait::async_trait;

    struct MockChannel {
        name: String,
        enabled: bool,
        connected: bool,
    }

    #[async_trait]
    impl Channel for MockChannel {
        fn name(&self) -> &str {
            &self.name
        }
        fn display_name(&self) -> &str {
            &self.name
        }
        fn is_enabled(&self) -> bool {
            self.enabled
        }
        fn is_connected(&self) -> bool {
            self.connected
        }
        fn capabilities(&self) -> ChannelCapabilities {
            ChannelCapabilities::default()
        }
        fn registration(&self) -> ChannelRegistration {
            ChannelRegistration {
                name: self.name.clone(),
                display_name: self.name.clone(),
                description: "Mock".into(),
                version: "0.1.0".into(),
                author: "test".into(),
                icon: None,
                capabilities: ChannelCapabilities::default(),
            }
        }
        async fn connect(&mut self) -> Result<()> {
            self.connected = true;
            Ok(())
        }
        async fn disconnect(&mut self) -> Result<()> {
            self.connected = false;
            Ok(())
        }
        async fn send(&self, _message: ChannelMessage) -> Result<DeliveryStatus> {
            Ok(DeliveryStatus::Sent)
        }
        async fn receive(&self) -> Result<Vec<IncomingMessage>> {
            Ok(vec![])
        }
        async fn handle_webhook(&self, _payload: serde_json::Value) -> Result<()> {
            Ok(())
        }
    }

    #[test]
    fn test_registry_register_and_get() {
        let registry = ChannelRegistry::new();
        let channel = Arc::new(MockChannel {
            name: "test".into(),
            enabled: true,
            connected: false,
        });
        registry.register(channel);
        assert!(registry.get("test").is_some());
        assert!(registry.get("nonexistent").is_none());
    }

    #[test]
    fn test_registry_list() {
        let registry = ChannelRegistry::new();
        registry.register(Arc::new(MockChannel {
            name: "a".into(),
            enabled: true,
            connected: false,
        }));
        registry.register(Arc::new(MockChannel {
            name: "b".into(),
            enabled: false,
            connected: false,
        }));

        assert_eq!(registry.count(), 2);
        assert_eq!(registry.list_enabled().len(), 1);
    }
}
