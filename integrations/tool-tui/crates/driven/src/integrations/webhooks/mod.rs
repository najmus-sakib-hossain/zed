//! # Webhook Integration
//!
//! Receive and process external webhooks.
//!
//! ## Usage
//!
//! ```rust,ignore
//! use driven::integrations::webhooks::{WebhookServer, WebhookConfig};
//!
//! let config = WebhookConfig::from_file("~/.dx/config/webhooks.sr")?;
//! let server = WebhookServer::new(&config)?;
//!
//! // Start webhook server
//! server.start(8792).await?;
//! ```

use crate::error::{DrivenError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Webhook configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookConfig {
    /// Whether webhooks are enabled
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Server port
    #[serde(default = "default_port")]
    pub port: u16,
    /// Authentication type
    #[serde(default)]
    pub auth: WebhookAuth,
    /// Configured endpoints
    #[serde(default)]
    pub endpoints: HashMap<String, WebhookEndpoint>,
    /// Security settings
    #[serde(default)]
    pub security: WebhookSecurity,
}

fn default_true() -> bool {
    true
}

fn default_port() -> u16 {
    8792
}

/// Webhook authentication type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum WebhookAuth {
    /// No authentication
    #[default]
    None,
    /// Bearer token
    Bearer,
    /// HMAC signature
    Hmac,
    /// Basic auth
    Basic,
}

/// Webhook endpoint configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookEndpoint {
    /// Endpoint path (e.g., "/webhook/github")
    pub path: String,
    /// Secret for signature verification
    #[serde(default)]
    pub secret: String,
    /// Handler script/function
    pub handler: Option<String>,
    /// Whether endpoint is enabled
    #[serde(default = "default_true")]
    pub enabled: bool,
}

/// Webhook security settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookSecurity {
    /// Allowed IP addresses (CIDR notation)
    #[serde(default = "default_allowed_ips")]
    pub allowed_ips: Vec<String>,
    /// Rate limit (requests per minute)
    #[serde(default = "default_rate_limit")]
    pub rate_limit: u32,
    /// Maximum body size in bytes
    #[serde(default = "default_max_body_size")]
    pub max_body_size: usize,
}

fn default_allowed_ips() -> Vec<String> {
    vec!["0.0.0.0/0".to_string()]
}

fn default_rate_limit() -> u32 {
    100
}

fn default_max_body_size() -> usize {
    1024 * 1024 // 1MB
}

impl Default for WebhookSecurity {
    fn default() -> Self {
        Self {
            allowed_ips: default_allowed_ips(),
            rate_limit: default_rate_limit(),
            max_body_size: default_max_body_size(),
        }
    }
}

impl Default for WebhookConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            port: default_port(),
            auth: WebhookAuth::default(),
            endpoints: HashMap::new(),
            security: WebhookSecurity::default(),
        }
    }
}

impl WebhookConfig {
    /// Load from .sr config file
    pub fn from_file(path: impl AsRef<std::path::Path>) -> Result<Self> {
        let content = std::fs::read_to_string(path.as_ref())
            .map_err(|e| DrivenError::Io(e))?;
        Self::parse_sr(&content)
    }

    fn parse_sr(_content: &str) -> Result<Self> {
        Ok(Self::default())
    }

    /// Resolve environment variables in secrets
    pub fn resolve_env_vars(&mut self) {
        for endpoint in self.endpoints.values_mut() {
            if endpoint.secret.starts_with('$') {
                let var_name = &endpoint.secret[1..];
                endpoint.secret = std::env::var(var_name).unwrap_or_default();
            }
        }
    }
}

/// Received webhook event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookEvent {
    /// Endpoint that received the webhook
    pub endpoint: String,
    /// HTTP method
    pub method: String,
    /// Request headers
    pub headers: HashMap<String, String>,
    /// Request body
    pub body: String,
    /// Parsed JSON body (if applicable)
    pub json: Option<serde_json::Value>,
    /// Timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Source IP
    pub source_ip: String,
}

/// Webhook handler result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookResponse {
    /// HTTP status code
    pub status: u16,
    /// Response body
    pub body: String,
    /// Response headers
    pub headers: HashMap<String, String>,
}

impl Default for WebhookResponse {
    fn default() -> Self {
        Self {
            status: 200,
            body: "OK".to_string(),
            headers: HashMap::new(),
        }
    }
}

/// Webhook handler function type
pub type WebhookHandler = Box<dyn Fn(WebhookEvent) -> WebhookResponse + Send + Sync>;

/// Webhook server
pub struct WebhookServer {
    config: WebhookConfig,
    handlers: Arc<RwLock<HashMap<String, WebhookHandler>>>,
    events: Arc<RwLock<Vec<WebhookEvent>>>,
}

impl WebhookServer {
    /// Create a new webhook server
    pub fn new(config: &WebhookConfig) -> Result<Self> {
        let mut config = config.clone();
        config.resolve_env_vars();

        Ok(Self {
            config,
            handlers: Arc::new(RwLock::new(HashMap::new())),
            events: Arc::new(RwLock::new(Vec::new())),
        })
    }

    /// Register a handler for an endpoint
    pub async fn register_handler(&self, endpoint: &str, handler: WebhookHandler) {
        let mut handlers = self.handlers.write().await;
        handlers.insert(endpoint.to_string(), handler);
    }

    /// Start the webhook server
    pub async fn start(&self) -> Result<()> {
        if !self.config.enabled {
            return Err(DrivenError::Config("Webhooks are disabled".into()));
        }

        let port = self.config.port;
        tracing::info!("Starting webhook server on port {}", port);

        // In production, would use axum/actix-web to create HTTP server
        // This is a placeholder for the server implementation
        self.run_server(port).await
    }

    /// Stop the webhook server
    pub async fn stop(&self) -> Result<()> {
        tracing::info!("Stopping webhook server");
        Ok(())
    }

    /// Get recent events
    pub async fn get_events(&self, limit: usize) -> Vec<WebhookEvent> {
        let events = self.events.read().await;
        events.iter().rev().take(limit).cloned().collect()
    }

    /// Process incoming webhook
    pub async fn process_webhook(&self, endpoint: &str, event: WebhookEvent) -> Result<WebhookResponse> {
        // Validate endpoint exists
        let endpoint_config = self.config.endpoints.get(endpoint)
            .ok_or_else(|| DrivenError::NotFound(format!("Endpoint '{}' not found", endpoint)))?;

        if !endpoint_config.enabled {
            return Err(DrivenError::Config("Endpoint is disabled".into()));
        }

        // Validate signature if secret is configured
        if !endpoint_config.secret.is_empty() {
            self.verify_signature(&event, &endpoint_config.secret)?;
        }

        // Store event
        {
            let mut events = self.events.write().await;
            events.push(event.clone());
            // Keep only last 1000 events
            if events.len() > 1000 {
                events.drain(0..events.len() - 1000);
            }
        }

        // Call handler if registered
        let handlers = self.handlers.read().await;
        if let Some(handler) = handlers.get(endpoint) {
            Ok(handler(event))
        } else {
            Ok(WebhookResponse::default())
        }
    }

    /// Verify webhook signature
    fn verify_signature(&self, event: &WebhookEvent, secret: &str) -> Result<()> {
        // GitHub: X-Hub-Signature-256
        // Stripe: Stripe-Signature
        // Generic: X-Signature

        if let Some(signature) = event.headers.get("x-hub-signature-256") {
            return self.verify_github_signature(&event.body, signature, secret);
        }

        if let Some(signature) = event.headers.get("stripe-signature") {
            return self.verify_stripe_signature(&event.body, signature, secret);
        }

        if let Some(signature) = event.headers.get("x-signature") {
            return self.verify_hmac_signature(&event.body, signature, secret);
        }

        // No signature header found - skip verification
        Ok(())
    }

    /// Verify GitHub webhook signature
    fn verify_github_signature(&self, body: &str, signature: &str, secret: &str) -> Result<()> {
        use hmac::{Hmac, Mac};
        use sha2::Sha256;

        let signature = signature.strip_prefix("sha256=").unwrap_or(signature);
        
        let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes())
            .map_err(|_| DrivenError::Security("Invalid secret".into()))?;
        mac.update(body.as_bytes());
        
        let expected = hex::encode(mac.finalize().into_bytes());
        
        if expected != signature {
            return Err(DrivenError::Security("Invalid signature".into()));
        }

        Ok(())
    }

    /// Verify Stripe webhook signature
    fn verify_stripe_signature(&self, _body: &str, _signature: &str, _secret: &str) -> Result<()> {
        // Stripe uses timestamp-based signature format
        // t=timestamp,v1=signature
        // TODO: Implement Stripe signature verification
        Ok(())
    }

    /// Verify generic HMAC signature
    fn verify_hmac_signature(&self, body: &str, signature: &str, secret: &str) -> Result<()> {
        self.verify_github_signature(body, signature, secret)
    }

    /// Run HTTP server
    async fn run_server(&self, port: u16) -> Result<()> {
        // Placeholder for actual HTTP server implementation
        // Would use axum or actix-web
        tracing::info!("Webhook server listening on 0.0.0.0:{}", port);
        
        // Keep server running
        tokio::signal::ctrl_c()
            .await
            .map_err(|e| DrivenError::Io(std::io::Error::new(std::io::ErrorKind::Other, e)))?;
        
        Ok(())
    }
}

/// Test a webhook endpoint
pub async fn test_webhook(config: &WebhookConfig, endpoint: &str, payload: &str) -> Result<WebhookResponse> {
    let server = WebhookServer::new(config)?;
    
    let event = WebhookEvent {
        endpoint: endpoint.to_string(),
        method: "POST".to_string(),
        headers: HashMap::new(),
        body: payload.to_string(),
        json: serde_json::from_str(payload).ok(),
        timestamp: chrono::Utc::now(),
        source_ip: "127.0.0.1".to_string(),
    };

    server.process_webhook(endpoint, event).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = WebhookConfig::default();
        assert!(config.enabled);
        assert_eq!(config.port, 8792);
    }

    #[test]
    fn test_default_security() {
        let security = WebhookSecurity::default();
        assert_eq!(security.rate_limit, 100);
        assert_eq!(security.max_body_size, 1024 * 1024);
    }
}
