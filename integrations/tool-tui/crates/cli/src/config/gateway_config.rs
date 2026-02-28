//! Gateway Configuration Types
//!
//! Comprehensive YAML-based configuration for the DX CLI and gateway.
//! Supports environment variable substitution, file includes, hot-reload,
//! secret encryption, and JSON Schema generation.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// Main CLI/Gateway configuration structure
///
/// This configuration covers all aspects of the DX CLI including
/// gateway settings, LLM providers, agent configuration, memory,
/// plugins, channels, and security.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GatewayCliConfig {
    /// Gateway server configuration
    #[serde(default)]
    pub gateway: GatewaySection,

    /// LLM provider configuration
    #[serde(default)]
    pub llm: LlmSection,

    /// Agent configuration
    #[serde(default)]
    pub agent: AgentSection,

    /// Memory system configuration
    #[serde(default)]
    pub memory: MemorySection,

    /// Plugin system configuration
    #[serde(default)]
    pub plugins: PluginSection,

    /// Channel integrations
    #[serde(default)]
    pub channels: ChannelsSection,

    /// Security configuration
    #[serde(default)]
    pub security: SecuritySection,

    /// Logging configuration
    #[serde(default)]
    pub logging: LoggingSection,

    /// Session configuration
    #[serde(default)]
    pub session: SessionSection,

    /// TTS configuration
    #[serde(default)]
    pub tts: TtsSection,

    /// File includes (processed at load time)
    #[serde(default, rename = "include")]
    pub includes: Vec<String>,
}

/// Gateway server section
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewaySection {
    /// Host address to bind to
    #[serde(default = "default_gateway_host")]
    pub host: String,

    /// WebSocket port
    #[serde(default = "default_gateway_port")]
    pub port: u16,

    /// Enable mDNS discovery
    #[serde(default = "default_true")]
    pub mdns_enabled: bool,

    /// Service name for mDNS
    #[serde(default = "default_service_name")]
    pub service_name: String,

    /// Require authentication
    #[serde(default = "default_true")]
    pub require_auth: bool,

    /// Session timeout in seconds
    #[serde(default = "default_session_timeout")]
    pub session_timeout: u64,

    /// Maximum concurrent connections
    #[serde(default = "default_max_connections")]
    pub max_connections: usize,

    /// Allowed commands (empty = all allowed)
    #[serde(default)]
    pub allowed_commands: Vec<String>,

    /// CORS allowed origins
    #[serde(default)]
    pub cors_origins: Vec<String>,

    /// Enable HTTP API
    #[serde(default = "default_true")]
    pub http_enabled: bool,

    /// HTTP API port (separate from WebSocket)
    #[serde(default = "default_http_port")]
    pub http_port: u16,
}

impl Default for GatewaySection {
    fn default() -> Self {
        Self {
            host: default_gateway_host(),
            port: default_gateway_port(),
            mdns_enabled: true,
            service_name: default_service_name(),
            require_auth: true,
            session_timeout: default_session_timeout(),
            max_connections: default_max_connections(),
            allowed_commands: Vec::new(),
            cors_origins: Vec::new(),
            http_enabled: true,
            http_port: default_http_port(),
        }
    }
}

/// LLM provider section
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmSection {
    /// Default provider
    #[serde(default = "default_llm_provider")]
    pub default_provider: String,

    /// Default model
    #[serde(default = "default_llm_model")]
    pub default_model: String,

    /// Provider-specific configurations
    #[serde(default)]
    pub providers: HashMap<String, LlmProviderConfig>,

    /// Max tokens for responses
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,

    /// Temperature for generation
    #[serde(default = "default_temperature")]
    pub temperature: f32,

    /// Enable streaming responses
    #[serde(default = "default_true")]
    pub streaming: bool,
}

impl Default for LlmSection {
    fn default() -> Self {
        Self {
            default_provider: default_llm_provider(),
            default_model: default_llm_model(),
            providers: HashMap::new(),
            max_tokens: default_max_tokens(),
            temperature: default_temperature(),
            streaming: true,
        }
    }
}

/// LLM provider configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmProviderConfig {
    /// API key (can use ${ENV_VAR} syntax)
    #[serde(default)]
    pub api_key: String,

    /// API base URL
    #[serde(default)]
    pub base_url: Option<String>,

    /// Organization ID
    #[serde(default)]
    pub organization: Option<String>,

    /// Available models
    #[serde(default)]
    pub models: Vec<String>,

    /// Rate limit (requests per minute)
    #[serde(default)]
    pub rate_limit: Option<u32>,
}

/// Agent section
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSection {
    /// Default agent name
    #[serde(default = "default_agent_name")]
    pub name: String,

    /// System prompt
    #[serde(default)]
    pub system_prompt: Option<String>,

    /// System prompt file path
    #[serde(default)]
    pub system_prompt_file: Option<String>,

    /// Enable tool use
    #[serde(default = "default_true")]
    pub tools_enabled: bool,

    /// Maximum conversation turns before compaction
    #[serde(default = "default_max_turns")]
    pub max_turns: u32,

    /// Custom agent configurations
    #[serde(default)]
    pub custom_agents: HashMap<String, CustomAgentConfig>,
}

impl Default for AgentSection {
    fn default() -> Self {
        Self {
            name: default_agent_name(),
            system_prompt: None,
            system_prompt_file: None,
            tools_enabled: true,
            max_turns: default_max_turns(),
            custom_agents: HashMap::new(),
        }
    }
}

/// Custom agent configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomAgentConfig {
    /// Agent display name
    pub name: String,

    /// Provider override
    #[serde(default)]
    pub provider: Option<String>,

    /// Model override
    #[serde(default)]
    pub model: Option<String>,

    /// System prompt
    #[serde(default)]
    pub system_prompt: Option<String>,

    /// Temperature override
    #[serde(default)]
    pub temperature: Option<f32>,
}

/// Memory system section
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemorySection {
    /// Enable memory system
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Storage backend: "file", "sqlite", "lancedb"
    #[serde(default = "default_memory_backend")]
    pub backend: String,

    /// Storage path
    #[serde(default)]
    pub storage_path: Option<String>,

    /// Embedding model
    #[serde(default = "default_embedding_model")]
    pub embedding_model: String,

    /// Embedding dimension
    #[serde(default = "default_embedding_dim")]
    pub embedding_dim: usize,

    /// Maximum memories
    #[serde(default = "default_max_memories")]
    pub max_memories: usize,

    /// Enable encryption
    #[serde(default = "default_true")]
    pub encrypt: bool,

    /// Relevance decay rate per day
    #[serde(default = "default_decay_rate")]
    pub decay_rate: f32,

    /// Auto-prune threshold
    #[serde(default = "default_min_relevance")]
    pub min_relevance: f32,
}

impl Default for MemorySection {
    fn default() -> Self {
        Self {
            enabled: true,
            backend: default_memory_backend(),
            storage_path: None,
            embedding_model: default_embedding_model(),
            embedding_dim: default_embedding_dim(),
            max_memories: default_max_memories(),
            encrypt: true,
            decay_rate: default_decay_rate(),
            min_relevance: default_min_relevance(),
        }
    }
}

/// Plugin system section
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginSection {
    /// Enable plugin system
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Plugin directories
    #[serde(default)]
    pub directories: Vec<String>,

    /// Auto-load plugins on startup
    #[serde(default = "default_true")]
    pub auto_load: bool,

    /// Allow unsigned native plugins
    #[serde(default)]
    pub allow_unsigned: bool,

    /// Default sandbox configuration
    #[serde(default)]
    pub sandbox: SandboxSection,

    /// Per-plugin overrides
    #[serde(default)]
    pub overrides: HashMap<String, PluginOverride>,
}

impl Default for PluginSection {
    fn default() -> Self {
        Self {
            enabled: true,
            directories: Vec::new(),
            auto_load: true,
            allow_unsigned: false,
            sandbox: SandboxSection::default(),
            overrides: HashMap::new(),
        }
    }
}

/// Sandbox section
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxSection {
    /// Memory limit in MB
    #[serde(default = "default_sandbox_memory")]
    pub memory_limit_mb: usize,

    /// CPU time limit in ms
    #[serde(default = "default_sandbox_cpu")]
    pub cpu_limit_ms: u64,

    /// Timeout in seconds
    #[serde(default = "default_sandbox_timeout")]
    pub timeout_seconds: u64,
}

impl Default for SandboxSection {
    fn default() -> Self {
        Self {
            memory_limit_mb: default_sandbox_memory(),
            cpu_limit_ms: default_sandbox_cpu(),
            timeout_seconds: default_sandbox_timeout(),
        }
    }
}

/// Plugin-specific override
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginOverride {
    /// Enable/disable this plugin
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Override sandbox settings
    #[serde(default)]
    pub sandbox: Option<SandboxSection>,

    /// Additional capabilities
    #[serde(default)]
    pub capabilities: Vec<String>,
}

/// Channels section
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ChannelsSection {
    /// Discord configuration
    #[serde(default)]
    pub discord: Option<ChannelConfig>,

    /// Telegram configuration
    #[serde(default)]
    pub telegram: Option<ChannelConfig>,

    /// Slack configuration
    #[serde(default)]
    pub slack: Option<ChannelConfig>,

    /// WhatsApp configuration
    #[serde(default)]
    pub whatsapp: Option<ChannelConfig>,

    /// Matrix configuration
    #[serde(default)]
    pub matrix: Option<ChannelConfig>,
}

/// Individual channel configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelConfig {
    /// Enable this channel
    #[serde(default)]
    pub enabled: bool,

    /// API token (can use ${ENV_VAR} syntax)
    #[serde(default)]
    pub token: String,

    /// Channel-specific settings
    #[serde(default)]
    pub settings: HashMap<String, serde_json::Value>,
}

/// Security section
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecuritySection {
    /// Enable exec approval system
    #[serde(default = "default_true")]
    pub exec_approval: bool,

    /// Auto-approve safe commands
    #[serde(default)]
    pub auto_approve: Vec<String>,

    /// Encryption key path
    #[serde(default)]
    pub encryption_key_path: Option<String>,

    /// Enable audit logging
    #[serde(default)]
    pub audit_logging: bool,

    /// Ed25519 trusted keys for plugins
    #[serde(default)]
    pub trusted_keys: Vec<String>,
}

impl Default for SecuritySection {
    fn default() -> Self {
        Self {
            exec_approval: true,
            auto_approve: Vec::new(),
            encryption_key_path: None,
            audit_logging: false,
            trusted_keys: Vec::new(),
        }
    }
}

/// Logging section
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingSection {
    /// Log level: trace, debug, info, warn, error
    #[serde(default = "default_log_level")]
    pub level: String,

    /// Log format: text, json
    #[serde(default = "default_log_format")]
    pub format: String,

    /// Log file path
    #[serde(default)]
    pub file: Option<String>,

    /// Enable colored output
    #[serde(default = "default_true")]
    pub color: bool,
}

impl Default for LoggingSection {
    fn default() -> Self {
        Self {
            level: default_log_level(),
            format: default_log_format(),
            file: None,
            color: true,
        }
    }
}

/// Session section
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSection {
    /// Storage directory
    #[serde(default)]
    pub storage_path: Option<String>,

    /// Auto-compact after N messages
    #[serde(default = "default_auto_compact")]
    pub auto_compact_threshold: u32,

    /// Enable session backups
    #[serde(default = "default_true")]
    pub backups: bool,

    /// Max sessions to keep
    #[serde(default = "default_max_sessions")]
    pub max_sessions: usize,

    /// Session compression threshold in bytes
    #[serde(default = "default_compression_threshold")]
    pub compression_threshold: usize,
}

impl Default for SessionSection {
    fn default() -> Self {
        Self {
            storage_path: None,
            auto_compact_threshold: default_auto_compact(),
            backups: true,
            max_sessions: default_max_sessions(),
            compression_threshold: default_compression_threshold(),
        }
    }
}

/// TTS section
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TtsSection {
    /// Enable TTS
    #[serde(default)]
    pub enabled: bool,

    /// TTS engine: "system", "openai", "elevenlabs"
    #[serde(default = "default_tts_engine")]
    pub engine: String,

    /// Voice name
    #[serde(default)]
    pub voice: Option<String>,

    /// Speech rate (0.5 - 2.0)
    #[serde(default = "default_speech_rate")]
    pub rate: f32,
}

impl Default for TtsSection {
    fn default() -> Self {
        Self {
            enabled: false,
            engine: default_tts_engine(),
            voice: None,
            rate: default_speech_rate(),
        }
    }
}

// ─────────────────── Default value functions ───────────────────

fn default_true() -> bool {
    true
}
fn default_gateway_host() -> String {
    "0.0.0.0".to_string()
}
fn default_gateway_port() -> u16 {
    31337
}
fn default_http_port() -> u16 {
    31338
}
fn default_service_name() -> String {
    "dx-gateway".to_string()
}
fn default_session_timeout() -> u64 {
    3600
}
fn default_max_connections() -> usize {
    10
}
fn default_llm_provider() -> String {
    "ollama".to_string()
}
fn default_llm_model() -> String {
    "llama3.2".to_string()
}
fn default_max_tokens() -> u32 {
    4096
}
fn default_temperature() -> f32 {
    0.7
}
fn default_agent_name() -> String {
    "dx".to_string()
}
fn default_max_turns() -> u32 {
    50
}
fn default_memory_backend() -> String {
    "file".to_string()
}
fn default_embedding_model() -> String {
    "all-MiniLM-L6-v2".to_string()
}
fn default_embedding_dim() -> usize {
    384
}
fn default_max_memories() -> usize {
    100_000
}
fn default_decay_rate() -> f32 {
    0.01
}
fn default_min_relevance() -> f32 {
    0.1
}
fn default_sandbox_memory() -> usize {
    256
}
fn default_sandbox_cpu() -> u64 {
    30_000
}
fn default_sandbox_timeout() -> u64 {
    60
}
fn default_log_level() -> String {
    "info".to_string()
}
fn default_log_format() -> String {
    "text".to_string()
}
fn default_auto_compact() -> u32 {
    100
}
fn default_max_sessions() -> usize {
    1000
}
fn default_compression_threshold() -> usize {
    1_048_576 // 1 MB
}
fn default_tts_engine() -> String {
    "system".to_string()
}
fn default_speech_rate() -> f32 {
    1.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = GatewayCliConfig::default();
        assert_eq!(config.gateway.port, 31337);
        assert_eq!(config.llm.default_provider, "ollama");
        assert!(config.memory.enabled);
        assert!(config.plugins.enabled);
    }

    #[test]
    fn test_serialize_yaml() {
        let config = GatewayCliConfig::default();
        let yaml = serde_yaml::to_string(&config).expect("Failed to serialize");
        assert!(yaml.contains("gateway:"));
        assert!(yaml.contains("port: 31337"));
    }

    #[test]
    fn test_deserialize_yaml() {
        let yaml = r#"
gateway:
  host: "127.0.0.1"
  port: 8080
llm:
  default_provider: openai
  default_model: gpt-4
  temperature: 0.5
"#;
        let config: GatewayCliConfig = serde_yaml::from_str(yaml).expect("Failed to deserialize");
        assert_eq!(config.gateway.host, "127.0.0.1");
        assert_eq!(config.gateway.port, 8080);
        assert_eq!(config.llm.default_provider, "openai");
        assert_eq!(config.llm.temperature, 0.5);
    }

    #[test]
    fn test_deserialize_partial_yaml() {
        let yaml = "gateway:\n  port: 9999\n";
        let config: GatewayCliConfig = serde_yaml::from_str(yaml).expect("Failed to deserialize");
        assert_eq!(config.gateway.port, 9999);
        // Defaults should be applied for missing sections
        assert_eq!(config.llm.default_provider, "ollama");
        assert!(config.memory.enabled);
    }

    #[test]
    fn test_provider_config() {
        let yaml = r#"
llm:
  default_provider: openai
  providers:
    openai:
      api_key: "${OPENAI_API_KEY}"
      models:
        - gpt-4
        - gpt-4-turbo
      rate_limit: 60
    ollama:
      base_url: "http://localhost:11434"
      models:
        - llama3.2
        - codellama
"#;
        let config: GatewayCliConfig = serde_yaml::from_str(yaml).expect("Failed to deserialize");
        assert_eq!(config.llm.providers.len(), 2);
        assert!(config.llm.providers.contains_key("openai"));
        assert_eq!(config.llm.providers["openai"].api_key, "${OPENAI_API_KEY}");
    }
}
