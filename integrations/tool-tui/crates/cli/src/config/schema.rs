//! JSON Schema Generation
//!
//! Generates JSON Schema from the configuration types for validation
//! and editor auto-completion support.

use serde_json::Value;

/// Generate JSON Schema for the GatewayCliConfig.
///
/// Returns a JSON Schema document that describes the configuration structure,
/// including all sections, types, defaults, and descriptions.
pub fn generate_config_schema() -> Value {
    // Build schema manually since we control the exact output format.
    // This approach avoids the schemars compile-time dependency being required
    // and gives us full control over descriptions and examples.
    serde_json::json!({
        "$schema": "http://json-schema.org/draft-07/schema#",
        "title": "DX CLI Configuration",
        "description": "Configuration schema for the DX CLI and gateway system",
        "type": "object",
        "properties": {
            "include": {
                "description": "Files to include and merge into this configuration",
                "oneOf": [
                    { "type": "string" },
                    { "type": "array", "items": { "type": "string" } }
                ]
            },
            "gateway": gateway_schema(),
            "llm": llm_schema(),
            "agent": agent_schema(),
            "memory": memory_schema(),
            "plugins": plugins_schema(),
            "channels": channels_schema(),
            "security": security_schema(),
            "logging": logging_schema(),
            "session": session_schema(),
            "tts": tts_schema()
        },
        "additionalProperties": false
    })
}

fn gateway_schema() -> Value {
    serde_json::json!({
        "description": "Gateway server configuration",
        "type": "object",
        "properties": {
            "host": {
                "description": "Host address to bind to",
                "type": "string",
                "default": "0.0.0.0"
            },
            "port": {
                "description": "WebSocket port",
                "type": "integer",
                "minimum": 1,
                "maximum": 65535,
                "default": 31337
            },
            "mdns_enabled": {
                "description": "Enable mDNS/Bonjour service discovery",
                "type": "boolean",
                "default": true
            },
            "service_name": {
                "description": "mDNS service name",
                "type": "string",
                "default": "dx-gateway"
            },
            "require_auth": {
                "description": "Require authentication for connections",
                "type": "boolean",
                "default": true
            },
            "session_timeout": {
                "description": "Session timeout in seconds",
                "type": "integer",
                "minimum": 0,
                "default": 3600
            },
            "max_connections": {
                "description": "Maximum concurrent connections",
                "type": "integer",
                "minimum": 1,
                "default": 10
            },
            "allowed_commands": {
                "description": "Allowed commands (empty = all allowed)",
                "type": "array",
                "items": { "type": "string" }
            },
            "cors_origins": {
                "description": "CORS allowed origins",
                "type": "array",
                "items": { "type": "string" }
            },
            "http_enabled": {
                "description": "Enable HTTP API",
                "type": "boolean",
                "default": true
            },
            "http_port": {
                "description": "HTTP API port",
                "type": "integer",
                "minimum": 1,
                "maximum": 65535,
                "default": 31338
            }
        },
        "additionalProperties": false
    })
}

fn llm_schema() -> Value {
    serde_json::json!({
        "description": "LLM provider configuration",
        "type": "object",
        "properties": {
            "default_provider": {
                "description": "Default LLM provider",
                "type": "string",
                "default": "ollama",
                "enum": ["ollama", "openai", "anthropic", "google", "local"]
            },
            "default_model": {
                "description": "Default model name",
                "type": "string",
                "default": "llama3.2"
            },
            "providers": {
                "description": "Provider-specific configurations",
                "type": "object",
                "additionalProperties": {
                    "type": "object",
                    "properties": {
                        "api_key": { "type": "string", "description": "API key (supports ${ENV_VAR} syntax)" },
                        "base_url": { "type": "string", "description": "API base URL override" },
                        "organization": { "type": "string" },
                        "models": { "type": "array", "items": { "type": "string" } },
                        "rate_limit": { "type": "integer", "minimum": 0 }
                    }
                }
            },
            "max_tokens": {
                "description": "Maximum tokens for responses",
                "type": "integer",
                "minimum": 1,
                "default": 4096
            },
            "temperature": {
                "description": "Temperature for generation",
                "type": "number",
                "minimum": 0.0,
                "maximum": 2.0,
                "default": 0.7
            },
            "streaming": {
                "description": "Enable streaming responses",
                "type": "boolean",
                "default": true
            }
        },
        "additionalProperties": false
    })
}

fn agent_schema() -> Value {
    serde_json::json!({
        "description": "Agent configuration",
        "type": "object",
        "properties": {
            "name": { "type": "string", "default": "dx" },
            "system_prompt": { "type": "string" },
            "system_prompt_file": { "type": "string" },
            "tools_enabled": { "type": "boolean", "default": true },
            "max_turns": { "type": "integer", "minimum": 1, "default": 50 },
            "custom_agents": {
                "type": "object",
                "additionalProperties": {
                    "type": "object",
                    "properties": {
                        "name": { "type": "string" },
                        "provider": { "type": "string" },
                        "model": { "type": "string" },
                        "system_prompt": { "type": "string" },
                        "temperature": { "type": "number", "minimum": 0.0, "maximum": 2.0 }
                    },
                    "required": ["name"]
                }
            }
        },
        "additionalProperties": false
    })
}

fn memory_schema() -> Value {
    serde_json::json!({
        "description": "Memory system configuration",
        "type": "object",
        "properties": {
            "enabled": { "type": "boolean", "default": true },
            "backend": {
                "type": "string",
                "enum": ["file", "sqlite", "lancedb"],
                "default": "file"
            },
            "storage_path": { "type": "string" },
            "embedding_model": { "type": "string", "default": "all-MiniLM-L6-v2" },
            "embedding_dim": { "type": "integer", "default": 384 },
            "max_memories": { "type": "integer", "minimum": 1, "default": 100000 },
            "encrypt": { "type": "boolean", "default": true },
            "decay_rate": { "type": "number", "minimum": 0.0, "default": 0.01 },
            "min_relevance": { "type": "number", "minimum": 0.0, "maximum": 1.0, "default": 0.1 }
        },
        "additionalProperties": false
    })
}

fn plugins_schema() -> Value {
    serde_json::json!({
        "description": "Plugin system configuration",
        "type": "object",
        "properties": {
            "enabled": { "type": "boolean", "default": true },
            "directories": { "type": "array", "items": { "type": "string" } },
            "auto_load": { "type": "boolean", "default": true },
            "allow_unsigned": { "type": "boolean", "default": false },
            "sandbox": {
                "type": "object",
                "properties": {
                    "memory_limit_mb": { "type": "integer", "default": 256 },
                    "cpu_limit_ms": { "type": "integer", "default": 30000 },
                    "timeout_seconds": { "type": "integer", "default": 60 }
                }
            },
            "overrides": {
                "type": "object",
                "additionalProperties": {
                    "type": "object",
                    "properties": {
                        "enabled": { "type": "boolean" },
                        "sandbox": { "$ref": "#/properties/plugins/properties/sandbox" },
                        "capabilities": { "type": "array", "items": { "type": "string" } }
                    }
                }
            }
        },
        "additionalProperties": false
    })
}

fn channels_schema() -> Value {
    let channel_schema = serde_json::json!({
        "type": "object",
        "properties": {
            "enabled": { "type": "boolean" },
            "token": { "type": "string", "description": "API token (supports ${ENV_VAR} syntax)" },
            "settings": { "type": "object" }
        }
    });

    serde_json::json!({
        "description": "Channel integration configuration",
        "type": "object",
        "properties": {
            "discord": channel_schema,
            "telegram": channel_schema,
            "slack": channel_schema,
            "whatsapp": channel_schema,
            "matrix": channel_schema
        },
        "additionalProperties": false
    })
}

fn security_schema() -> Value {
    serde_json::json!({
        "description": "Security configuration",
        "type": "object",
        "properties": {
            "exec_approval": { "type": "boolean", "default": true },
            "auto_approve": { "type": "array", "items": { "type": "string" } },
            "encryption_key_path": { "type": "string" },
            "audit_logging": { "type": "boolean", "default": false },
            "trusted_keys": { "type": "array", "items": { "type": "string" } }
        },
        "additionalProperties": false
    })
}

fn logging_schema() -> Value {
    serde_json::json!({
        "description": "Logging configuration",
        "type": "object",
        "properties": {
            "level": {
                "type": "string",
                "enum": ["trace", "debug", "info", "warn", "error"],
                "default": "info"
            },
            "format": {
                "type": "string",
                "enum": ["text", "json"],
                "default": "text"
            },
            "file": { "type": "string" },
            "color": { "type": "boolean", "default": true }
        },
        "additionalProperties": false
    })
}

fn session_schema() -> Value {
    serde_json::json!({
        "description": "Session management configuration",
        "type": "object",
        "properties": {
            "storage_path": { "type": "string" },
            "auto_compact_threshold": { "type": "integer", "default": 100 },
            "backups": { "type": "boolean", "default": true },
            "max_sessions": { "type": "integer", "default": 1000 },
            "compression_threshold": { "type": "integer", "default": 1048576 }
        },
        "additionalProperties": false
    })
}

fn tts_schema() -> Value {
    serde_json::json!({
        "description": "Text-to-speech configuration",
        "type": "object",
        "properties": {
            "enabled": { "type": "boolean", "default": false },
            "engine": {
                "type": "string",
                "enum": ["system", "openai", "elevenlabs"],
                "default": "system"
            },
            "voice": { "type": "string" },
            "rate": { "type": "number", "minimum": 0.5, "maximum": 2.0, "default": 1.0 }
        },
        "additionalProperties": false
    })
}

/// Validate a YAML configuration value against the schema
pub fn validate_config_value(value: &Value) -> Vec<ValidationIssue> {
    let schema = generate_config_schema();
    let mut issues = Vec::new();

    if let Some(obj) = value.as_object() {
        let schema_props = schema["properties"].as_object().unwrap();

        // Check for unknown top-level keys
        for key in obj.keys() {
            if !schema_props.contains_key(key) {
                issues.push(ValidationIssue {
                    path: key.clone(),
                    severity: IssueSeverity::Warning,
                    message: format!("Unknown configuration key: '{}'", key),
                });
            }
        }

        // Validate gateway section
        if let Some(gw) = obj.get("gateway") {
            validate_section(gw, &schema["properties"]["gateway"], "gateway", &mut issues);
        }

        // Validate port ranges
        if let Some(gw) = obj.get("gateway").and_then(|v| v.as_object()) {
            if let Some(port) = gw.get("port").and_then(|v| v.as_u64()) {
                if port == 0 || port > 65535 {
                    issues.push(ValidationIssue {
                        path: "gateway.port".to_string(),
                        severity: IssueSeverity::Error,
                        message: format!("Port must be between 1 and 65535, got {}", port),
                    });
                }
            }
        }

        // Validate temperature range
        if let Some(llm) = obj.get("llm").and_then(|v| v.as_object()) {
            if let Some(temp) = llm.get("temperature").and_then(|v| v.as_f64()) {
                if !(0.0..=2.0).contains(&temp) {
                    issues.push(ValidationIssue {
                        path: "llm.temperature".to_string(),
                        severity: IssueSeverity::Error,
                        message: format!("Temperature must be between 0.0 and 2.0, got {}", temp),
                    });
                }
            }
        }
    }

    issues
}

fn validate_section(value: &Value, schema: &Value, path: &str, issues: &mut Vec<ValidationIssue>) {
    if let (Some(obj), Some(schema_props)) =
        (value.as_object(), schema.get("properties").and_then(|p| p.as_object()))
    {
        for key in obj.keys() {
            if !schema_props.contains_key(key) {
                issues.push(ValidationIssue {
                    path: format!("{}.{}", path, key),
                    severity: IssueSeverity::Warning,
                    message: format!("Unknown key in {}: '{}'", path, key),
                });
            }
        }
    }
}

/// A validation issue found in the configuration
#[derive(Debug, Clone)]
pub struct ValidationIssue {
    /// JSON path to the problematic value
    pub path: String,
    /// Severity level
    pub severity: IssueSeverity,
    /// Human-readable description
    pub message: String,
}

/// Severity of a validation issue
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IssueSeverity {
    /// Informational note
    Info,
    /// Potential problem
    Warning,
    /// Invalid configuration
    Error,
}

impl std::fmt::Display for ValidationIssue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let prefix = match self.severity {
            IssueSeverity::Info => "info",
            IssueSeverity::Warning => "warning",
            IssueSeverity::Error => "error",
        };
        write!(f, "[{}] {}: {}", prefix, self.path, self.message)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_schema() {
        let schema = generate_config_schema();
        assert_eq!(schema["type"], "object");
        assert!(schema["properties"]["gateway"].is_object());
        assert!(schema["properties"]["llm"].is_object());
        assert!(schema["properties"]["memory"].is_object());
    }

    #[test]
    fn test_schema_has_all_sections() {
        let schema = generate_config_schema();
        let props = schema["properties"].as_object().unwrap();
        let expected_sections = [
            "gateway", "llm", "agent", "memory", "plugins", "channels", "security", "logging",
            "session", "tts", "include",
        ];
        for section in &expected_sections {
            assert!(props.contains_key(*section), "Missing schema section: {}", section);
        }
    }

    #[test]
    fn test_validate_valid_config() {
        let config: Value =
            serde_json::from_str(r#"{"gateway": {"port": 8080}, "llm": {"temperature": 0.7}}"#)
                .unwrap();
        let issues = validate_config_value(&config);
        let errors: Vec<_> = issues.iter().filter(|i| i.severity == IssueSeverity::Error).collect();
        assert!(errors.is_empty());
    }

    #[test]
    fn test_validate_invalid_port() {
        let config: Value = serde_json::from_str(r#"{"gateway": {"port": 0}}"#).unwrap();
        let issues = validate_config_value(&config);
        assert!(issues.iter().any(|i| i.path == "gateway.port"));
    }

    #[test]
    fn test_validate_invalid_temperature() {
        let config: Value = serde_json::from_str(r#"{"llm": {"temperature": 5.0}}"#).unwrap();
        let issues = validate_config_value(&config);
        assert!(issues.iter().any(|i| i.path == "llm.temperature"));
    }

    #[test]
    fn test_validate_unknown_key() {
        let config: Value = serde_json::from_str(r#"{"unknown_section": true}"#).unwrap();
        let issues = validate_config_value(&config);
        assert!(issues.iter().any(|i| i.message.contains("Unknown configuration key")));
    }
}
