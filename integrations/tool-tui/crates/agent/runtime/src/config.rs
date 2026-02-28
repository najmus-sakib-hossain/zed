//! Runtime configuration for LLM providers

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Runtime configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeConfig {
    /// Default provider to use
    #[serde(default = "default_provider")]
    pub default_provider: String,

    /// Default model to use
    #[serde(default = "default_model")]
    pub default_model: String,

    /// Provider configurations
    #[serde(default)]
    pub providers: HashMap<String, ProviderConfig>,

    /// Failover configuration
    #[serde(default)]
    pub failover: FailoverConfig,

    /// Cost tracking settings
    #[serde(default)]
    pub cost_tracking: CostTrackingConfig,

    /// Request timeout in seconds
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,

    /// Maximum retries on failure
    #[serde(default = "default_retries")]
    pub max_retries: u32,
}

/// Per-provider configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    /// Provider type (openai, anthropic, google, ollama)
    pub provider_type: String,

    /// API key (loaded from env or config)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,

    /// API base URL override
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,

    /// Organization ID (for OpenAI)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub organization: Option<String>,

    /// Default model for this provider
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_model: Option<String>,

    /// Maximum requests per minute
    #[serde(default)]
    pub rate_limit_rpm: Option<u32>,

    /// Additional headers
    #[serde(default)]
    pub headers: HashMap<String, String>,

    /// Whether this provider is enabled
    #[serde(default = "default_true")]
    pub enabled: bool,
}

/// Failover configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailoverConfig {
    /// Enable automatic failover
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Failover chain (provider names in order)
    #[serde(default)]
    pub chain: Vec<String>,

    /// Retry on rate limit
    #[serde(default = "default_true")]
    pub retry_on_rate_limit: bool,

    /// Max failover attempts
    #[serde(default = "default_failover_attempts")]
    pub max_attempts: u32,
}

/// Cost tracking configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostTrackingConfig {
    /// Enable cost tracking
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Monthly budget limit (USD)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub monthly_budget: Option<f64>,

    /// Daily budget limit (USD)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub daily_budget: Option<f64>,

    /// Alert threshold (percentage of budget)
    #[serde(default = "default_alert_threshold")]
    pub alert_threshold: f64,
}

fn default_provider() -> String {
    "anthropic".into()
}
fn default_model() -> String {
    "claude-sonnet-4-20250514".into()
}
fn default_timeout() -> u64 {
    120
}
fn default_retries() -> u32 {
    3
}
fn default_true() -> bool {
    true
}
fn default_failover_attempts() -> u32 {
    3
}
fn default_alert_threshold() -> f64 {
    0.8
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            default_provider: default_provider(),
            default_model: default_model(),
            providers: HashMap::new(),
            failover: FailoverConfig::default(),
            cost_tracking: CostTrackingConfig::default(),
            timeout_secs: default_timeout(),
            max_retries: default_retries(),
        }
    }
}

impl Default for FailoverConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            chain: vec!["anthropic".into(), "openai".into(), "google".into()],
            retry_on_rate_limit: true,
            max_attempts: default_failover_attempts(),
        }
    }
}

impl Default for CostTrackingConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            monthly_budget: None,
            daily_budget: None,
            alert_threshold: default_alert_threshold(),
        }
    }
}

impl RuntimeConfig {
    /// Load config from TOML file
    pub fn load(path: &std::path::Path) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: RuntimeConfig = toml::from_str(&content)?;
        Ok(config)
    }

    /// Load config, resolving API keys from environment
    pub fn load_with_env(path: &std::path::Path) -> anyhow::Result<Self> {
        let mut config = Self::load(path)?;
        config.resolve_env_keys();
        Ok(config)
    }

    /// Resolve API keys from environment variables
    pub fn resolve_env_keys(&mut self) {
        for (name, provider) in &mut self.providers {
            if provider.api_key.is_none() {
                let env_key = match name.as_str() {
                    "openai" => "OPENAI_API_KEY",
                    "anthropic" => "ANTHROPIC_API_KEY",
                    "google" => "GOOGLE_API_KEY",
                    _ => continue,
                };
                provider.api_key = std::env::var(env_key).ok();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = RuntimeConfig::default();
        assert_eq!(config.default_provider, "anthropic");
        assert_eq!(config.timeout_secs, 120);
        assert_eq!(config.max_retries, 3);
        assert!(config.failover.enabled);
    }

    #[test]
    fn test_toml_roundtrip() {
        let config = RuntimeConfig::default();
        let toml_str = toml::to_string_pretty(&config).unwrap();
        let parsed: RuntimeConfig = toml::from_str(&toml_str).unwrap();
        assert_eq!(parsed.default_provider, config.default_provider);
    }
}
