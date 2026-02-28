//! Gateway integration — multi-node message routing.
//!
//! Routes messages between the channel layer and the
//! agent gateway (WebSocket server), supporting health
//! checks and basic load-balancing hints.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};

use crate::message::ChannelMessage;

/// Gateway connection configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayConfig {
    /// Gateway WebSocket/HTTP URL.
    pub url: String,
    /// Authentication token.
    pub token: String,
    /// Request timeout in milliseconds.
    pub timeout_ms: u64,
    /// Number of retries on failure.
    pub max_retries: u32,
}

impl Default for GatewayConfig {
    fn default() -> Self {
        Self {
            url: "http://localhost:8080".into(),
            token: String::new(),
            timeout_ms: 30_000,
            max_retries: 3,
        }
    }
}

/// Connection state for a single gateway node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayNode {
    /// Node URL.
    pub url: String,
    /// Whether the node is currently reachable.
    pub healthy: bool,
    /// Number of messages routed through this node.
    pub messages_routed: u64,
    /// Last health check timestamp.
    pub last_check: Option<chrono::DateTime<chrono::Utc>>,
}

/// Gateway router — picks a healthy node and forwards messages.
pub struct Gateway {
    config: GatewayConfig,
    nodes: Arc<RwLock<Vec<GatewayNode>>>,
}

impl Gateway {
    /// Create a new gateway with a single node from config.
    pub fn new(config: GatewayConfig) -> Self {
        let primary = GatewayNode {
            url: config.url.clone(),
            healthy: true,
            messages_routed: 0,
            last_check: None,
        };
        Self {
            config,
            nodes: Arc::new(RwLock::new(vec![primary])),
        }
    }

    /// Add a secondary gateway node for failover.
    pub async fn add_node(&self, url: impl Into<String>) {
        let node = GatewayNode {
            url: url.into(),
            healthy: true,
            messages_routed: 0,
            last_check: None,
        };
        self.nodes.write().await.push(node);
    }

    /// Route a message to the first healthy node.
    ///
    /// This is a placeholder that serialises the message
    /// to JSON and logs it. In production, this would POST
    /// to the gateway HTTP endpoint or push over WebSocket.
    pub async fn route_message(&self, message: &ChannelMessage) -> Result<()> {
        let mut nodes = self.nodes.write().await;
        let node = nodes
            .iter_mut()
            .find(|n| n.healthy)
            .ok_or_else(|| anyhow::anyhow!("No healthy gateway nodes available"))?;

        // In a real implementation this would be an HTTP POST
        // or WebSocket send. For now we log and count.
        info!(
            target: "gateway",
            url = %node.url,
            to = %message.to,
            "Routing message to gateway node"
        );

        node.messages_routed += 1;
        Ok(())
    }

    /// Check health of all nodes.
    ///
    /// Marks nodes healthy/unhealthy based on reachability.
    /// In a real implementation this would hit a `/health`
    /// endpoint on each node.
    pub async fn health_check(&self) -> Result<Vec<GatewayNode>> {
        let mut nodes = self.nodes.write().await;
        let now = chrono::Utc::now();
        for node in nodes.iter_mut() {
            // Placeholder: assume healthy if URL is non-empty
            node.healthy = !node.url.is_empty();
            node.last_check = Some(now);
        }
        let snapshot: Vec<GatewayNode> = nodes.clone();
        Ok(snapshot)
    }

    /// Get the current config.
    pub fn config(&self) -> &GatewayConfig {
        &self.config
    }

    /// Get a snapshot of all nodes.
    pub async fn nodes(&self) -> Vec<GatewayNode> {
        self.nodes.read().await.clone()
    }

    /// Number of registered nodes.
    pub async fn node_count(&self) -> usize {
        self.nodes.read().await.len()
    }

    /// Mark a node as unhealthy.
    pub async fn mark_unhealthy(&self, url: &str) {
        let mut nodes = self.nodes.write().await;
        for node in nodes.iter_mut() {
            if node.url == url {
                node.healthy = false;
                warn!(url = url, "Gateway node marked unhealthy");
            }
        }
    }

    /// Mark a node as healthy.
    pub async fn mark_healthy(&self, url: &str) {
        let mut nodes = self.nodes.write().await;
        for node in nodes.iter_mut() {
            if node.url == url {
                node.healthy = true;
                info!(url = url, "Gateway node marked healthy");
            }
        }
    }

    /// Total messages routed across all nodes.
    pub async fn total_routed(&self) -> u64 {
        self.nodes.read().await.iter().map(|n| n.messages_routed).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> GatewayConfig {
        GatewayConfig {
            url: "http://localhost:8080".into(),
            token: "test-token".into(),
            timeout_ms: 5000,
            max_retries: 2,
        }
    }

    #[tokio::test]
    async fn test_create_gateway() {
        let gw = Gateway::new(test_config());
        assert_eq!(gw.node_count().await, 1);
    }

    #[tokio::test]
    async fn test_add_node() {
        let gw = Gateway::new(test_config());
        gw.add_node("http://localhost:8081").await;
        assert_eq!(gw.node_count().await, 2);
    }

    #[tokio::test]
    async fn test_route_message() {
        let gw = Gateway::new(test_config());
        let msg = ChannelMessage::text("user1", "Hello");
        let result = gw.route_message(&msg).await;
        assert!(result.is_ok());
        assert_eq!(gw.total_routed().await, 1);
    }

    #[tokio::test]
    async fn test_route_no_healthy_nodes() {
        let gw = Gateway::new(test_config());
        gw.mark_unhealthy("http://localhost:8080").await;

        let msg = ChannelMessage::text("user1", "Hello");
        let result = gw.route_message(&msg).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_health_check() {
        let gw = Gateway::new(test_config());
        let nodes = gw.health_check().await.expect("ok");
        assert_eq!(nodes.len(), 1);
        assert!(nodes[0].healthy);
        assert!(nodes[0].last_check.is_some());
    }

    #[tokio::test]
    async fn test_mark_healthy_unhealthy() {
        let gw = Gateway::new(test_config());

        gw.mark_unhealthy("http://localhost:8080").await;
        let nodes = gw.nodes().await;
        assert!(!nodes[0].healthy);

        gw.mark_healthy("http://localhost:8080").await;
        let nodes = gw.nodes().await;
        assert!(nodes[0].healthy);
    }

    #[test]
    fn test_default_config() {
        let cfg = GatewayConfig::default();
        assert!(!cfg.url.is_empty());
        assert_eq!(cfg.max_retries, 3);
    }
}
