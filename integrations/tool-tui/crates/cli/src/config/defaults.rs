//! Comprehensive Configuration Defaults
//!
//! Provides default configuration values and generates default config files.

use super::gateway_config::GatewayCliConfig;

/// Generate a default configuration as YAML string
pub fn default_config_yaml() -> String {
    let config = GatewayCliConfig::default();
    serde_yaml::to_string(&config).expect("Failed to serialize default config")
}

/// Generate a commented default configuration for documentation
pub fn default_config_yaml_commented() -> String {
    format!(
        r#"# DX CLI Configuration
# This file configures the DX gateway, LLM providers, and related systems.
# Environment variables can be used with ${{VAR}} or ${{VAR:-default}} syntax.
# Additional config files can be included with the 'include' directive.

# ─── Gateway Server ─────────────────────────────────────────────
gateway:
  # Host address to bind to (0.0.0.0 for all interfaces)
  host: "0.0.0.0"
  # WebSocket port
  port: 31337
  # HTTP API port (separate from WebSocket)
  http_port: 31338
  # Enable HTTP API
  http_enabled: true
  # Enable mDNS/Bonjour service discovery
  mdns_enabled: true
  # Service name for mDNS
  service_name: "dx-gateway"
  # Require authentication for connections
  require_auth: true
  # Session timeout in seconds
  session_timeout: 3600
  # Maximum concurrent connections
  max_connections: 10
  # Restrict commands (empty = allow all)
  allowed_commands: []
  # CORS allowed origins
  cors_origins: []

# ─── LLM Configuration ──────────────────────────────────────────
llm:
  # Default LLM provider
  default_provider: "ollama"
  # Default model
  default_model: "llama3.2"
  # Max tokens for responses
  max_tokens: 4096
  # Temperature for generation (0.0 = deterministic, 2.0 = very creative)
  temperature: 0.7
  # Enable streaming responses
  streaming: true
  # Provider-specific configuration
  providers:
    ollama:
      base_url: "http://localhost:11434"
      models:
        - llama3.2
        - codellama
    # openai:
    #   api_key: "${{OPENAI_API_KEY}}"
    #   models:
    #     - gpt-4
    #     - gpt-4-turbo
    # anthropic:
    #   api_key: "${{ANTHROPIC_API_KEY}}"
    #   models:
    #     - claude-3-opus
    #     - claude-3-sonnet

# ─── Agent Configuration ────────────────────────────────────────
agent:
  # Default agent name
  name: "dx"
  # Enable tool use
  tools_enabled: true
  # Max conversation turns before compaction
  max_turns: 50
  # System prompt (inline or file path)
  # system_prompt: "You are a helpful assistant."
  # system_prompt_file: "~/.dx/system_prompt.md"
  # Custom agents
  # custom_agents:
  #   coder:
  #     name: "Code Assistant"
  #     model: "codellama"
  #     temperature: 0.3

# ─── Memory System ──────────────────────────────────────────────
memory:
  # Enable semantic memory
  enabled: true
  # Storage backend: file, sqlite, lancedb
  backend: "file"
  # storage_path: "~/.dx/memory"
  # Embedding model for semantic search
  embedding_model: "all-MiniLM-L6-v2"
  # Embedding vector dimension
  embedding_dim: 384
  # Maximum memories to keep
  max_memories: 100000
  # Encrypt memory contents at rest
  encrypt: true
  # Relevance decay rate per day
  decay_rate: 0.01
  # Minimum relevance before pruning
  min_relevance: 0.1

# ─── Plugin System ──────────────────────────────────────────────
plugins:
  # Enable plugin system
  enabled: true
  # Auto-load plugins on startup
  auto_load: true
  # Allow unsigned native plugins (security risk)
  allow_unsigned: false
  # Additional plugin directories
  directories: []
  # Default sandbox limits
  sandbox:
    memory_limit_mb: 256
    cpu_limit_ms: 30000
    timeout_seconds: 60

# ─── Channel Integrations ───────────────────────────────────────
channels:
  # discord:
  #   enabled: true
  #   token: "${{DISCORD_BOT_TOKEN}}"
  # telegram:
  #   enabled: true
  #   token: "${{TELEGRAM_BOT_TOKEN}}"
  # slack:
  #   enabled: true
  #   token: "${{SLACK_BOT_TOKEN}}"

# ─── Security ───────────────────────────────────────────────────
security:
  # Require approval before executing commands
  exec_approval: true
  # Commands that are auto-approved
  auto_approve: []
  # Enable audit logging
  audit_logging: false
  # Ed25519 trusted keys for plugin verification
  trusted_keys: []

# ─── Logging ────────────────────────────────────────────────────
logging:
  # Log level: trace, debug, info, warn, error
  level: "info"
  # Log format: text, json
  format: "text"
  # Log to file (optional)
  # file: "~/.dx/dx.log"
  # Enable colored output
  color: true

# ─── Session Management ─────────────────────────────────────────
session:
  # storage_path: "~/.dx/sessions"
  # Auto-compact after N messages
  auto_compact_threshold: 100
  # Enable session backups
  backups: true
  # Max sessions to keep
  max_sessions: 1000
  # Compress sessions larger than this (bytes)
  compression_threshold: 1048576

# ─── Text-to-Speech ─────────────────────────────────────────────
tts:
  # Enable TTS
  enabled: false
  # TTS engine: system, openai, elevenlabs
  engine: "system"
  # Speech rate (0.5 - 2.0)
  rate: 1.0
  # voice: "default"
"#
    )
}

/// Get the default configuration file path
pub fn default_config_path() -> std::path::PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("dx")
        .join("config.yaml")
}

/// Get alternative configuration file paths to search
pub fn config_search_paths() -> Vec<std::path::PathBuf> {
    let mut paths = Vec::new();

    // System config directory
    if let Some(config_dir) = dirs::config_dir() {
        paths.push(config_dir.join("dx").join("config.yaml"));
        paths.push(config_dir.join("dx").join("config.yml"));
    }

    // Home directory
    if let Some(home) = dirs::home_dir() {
        paths.push(home.join(".dx").join("config.yaml"));
        paths.push(home.join(".dx").join("config.yml"));
        paths.push(home.join(".dxrc.yaml"));
    }

    // Current directory
    paths.push(std::path::PathBuf::from("dx.yaml"));
    paths.push(std::path::PathBuf::from(".dx").join("config.yaml"));

    // Legacy TOML paths
    if let Some(home) = dirs::home_dir() {
        paths.push(home.join(".dx").join("config.toml"));
    }

    paths
}

/// Find the first existing configuration file
pub fn find_config_file() -> Option<std::path::PathBuf> {
    for path in config_search_paths() {
        if path.exists() {
            return Some(path);
        }
    }
    None
}

/// Create default config file if it doesn't exist
pub fn ensure_default_config() -> Result<std::path::PathBuf, std::io::Error> {
    let path = default_config_path();

    if !path.exists() {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&path, default_config_yaml_commented())?;
    }

    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config_is_valid_yaml() {
        let yaml = default_config_yaml();
        let parsed: Result<GatewayCliConfig, _> = serde_yaml::from_str(&yaml);
        assert!(parsed.is_ok(), "Default config should be valid YAML");
    }

    #[test]
    fn test_commented_config_parseable() {
        // The commented config should be parseable after removing comment lines
        let commented = default_config_yaml_commented();
        let lines: Vec<&str> =
            commented.lines().filter(|l| !l.trim_start().starts_with('#')).collect();
        let clean = lines.join("\n");
        let _parsed: serde_yaml::Value = serde_yaml::from_str(&clean)
            .expect("Commented config should parse with comments removed");
    }

    #[test]
    fn test_search_paths_not_empty() {
        let paths = config_search_paths();
        assert!(!paths.is_empty());
    }

    #[test]
    fn test_default_config_values() {
        let yaml = default_config_yaml();
        let config: GatewayCliConfig = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(config.gateway.port, 31337);
        assert_eq!(config.llm.default_provider, "ollama");
        assert!(config.security.exec_approval);
    }
}
