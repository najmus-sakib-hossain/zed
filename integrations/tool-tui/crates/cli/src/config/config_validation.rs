//! Configuration Validation
//!
//! Validates configuration values against business rules and constraints.

use super::gateway_config::GatewayCliConfig;
use super::schema::{IssueSeverity, ValidationIssue};

/// Validate a parsed configuration for semantic correctness.
///
/// This goes beyond schema validation to check business logic,
/// such as port conflicts, path existence, and security requirements.
pub fn validate_config(config: &GatewayCliConfig) -> Vec<ValidationIssue> {
    let mut issues = Vec::new();

    validate_gateway(&config.gateway, &mut issues);
    validate_llm(&config.llm, &mut issues);
    validate_memory(&config.memory, &mut issues);
    validate_security(&config.security, &mut issues);
    validate_session(&config.session, &mut issues);
    validate_tts(&config.tts, &mut issues);

    issues
}

fn validate_gateway(gw: &super::gateway_config::GatewaySection, issues: &mut Vec<ValidationIssue>) {
    // Port range
    if gw.port == 0 {
        issues.push(ValidationIssue {
            path: "gateway.port".to_string(),
            severity: IssueSeverity::Error,
            message: "Gateway port must be non-zero".to_string(),
        });
    }

    // Port conflict check
    if gw.http_enabled && gw.port == gw.http_port {
        issues.push(ValidationIssue {
            path: "gateway.http_port".to_string(),
            severity: IssueSeverity::Error,
            message: "HTTP port must be different from WebSocket port".to_string(),
        });
    }

    // Host validation
    if gw.host.is_empty() {
        issues.push(ValidationIssue {
            path: "gateway.host".to_string(),
            severity: IssueSeverity::Error,
            message: "Gateway host must not be empty".to_string(),
        });
    }

    // Max connections
    if gw.max_connections == 0 {
        issues.push(ValidationIssue {
            path: "gateway.max_connections".to_string(),
            severity: IssueSeverity::Error,
            message: "Max connections must be at least 1".to_string(),
        });
    }

    // Security warning for no auth
    if !gw.require_auth {
        issues.push(ValidationIssue {
            path: "gateway.require_auth".to_string(),
            severity: IssueSeverity::Warning,
            message: "Authentication is disabled - gateway is open to all connections".to_string(),
        });
    }

    // Warning for binding to all interfaces
    if gw.host == "0.0.0.0" && !gw.require_auth {
        issues.push(ValidationIssue {
            path: "gateway.host".to_string(),
            severity: IssueSeverity::Warning,
            message: "Binding to all interfaces without auth is a security risk".to_string(),
        });
    }
}

fn validate_llm(llm: &super::gateway_config::LlmSection, issues: &mut Vec<ValidationIssue>) {
    // Temperature range
    if llm.temperature < 0.0 || llm.temperature > 2.0 {
        issues.push(ValidationIssue {
            path: "llm.temperature".to_string(),
            severity: IssueSeverity::Error,
            message: format!("Temperature must be between 0.0 and 2.0, got {}", llm.temperature),
        });
    }

    // Max tokens
    if llm.max_tokens == 0 {
        issues.push(ValidationIssue {
            path: "llm.max_tokens".to_string(),
            severity: IssueSeverity::Error,
            message: "Max tokens must be at least 1".to_string(),
        });
    }

    // Check for API key in provider configs
    for (name, provider) in &llm.providers {
        if provider.api_key.is_empty() && name != "ollama" && name != "local" {
            issues.push(ValidationIssue {
                path: format!("llm.providers.{}.api_key", name),
                severity: IssueSeverity::Warning,
                message: format!(
                    "Provider '{}' has no API key configured. Use ${{ENV_VAR}} syntax.",
                    name
                ),
            });
        }

        // Warn about unresolved env vars
        if provider.api_key.contains("${") {
            issues.push(ValidationIssue {
                path: format!("llm.providers.{}.api_key", name),
                severity: IssueSeverity::Info,
                message: format!(
                    "Provider '{}' API key uses environment variable substitution",
                    name
                ),
            });
        }
    }
}

fn validate_memory(mem: &super::gateway_config::MemorySection, issues: &mut Vec<ValidationIssue>) {
    if !mem.enabled {
        return;
    }

    // Backend validation
    let valid_backends = ["file", "sqlite", "lancedb"];
    if !valid_backends.contains(&mem.backend.as_str()) {
        issues.push(ValidationIssue {
            path: "memory.backend".to_string(),
            severity: IssueSeverity::Error,
            message: format!(
                "Invalid memory backend '{}'. Must be one of: {}",
                mem.backend,
                valid_backends.join(", ")
            ),
        });
    }

    // Embedding dimension
    if mem.embedding_dim == 0 {
        issues.push(ValidationIssue {
            path: "memory.embedding_dim".to_string(),
            severity: IssueSeverity::Error,
            message: "Embedding dimension must be positive".to_string(),
        });
    }

    // Decay rate
    if mem.decay_rate < 0.0 || mem.decay_rate > 1.0 {
        issues.push(ValidationIssue {
            path: "memory.decay_rate".to_string(),
            severity: IssueSeverity::Warning,
            message: format!("Decay rate {} is outside typical range 0.0-1.0", mem.decay_rate),
        });
    }

    // Min relevance
    if mem.min_relevance < 0.0 || mem.min_relevance > 1.0 {
        issues.push(ValidationIssue {
            path: "memory.min_relevance".to_string(),
            severity: IssueSeverity::Error,
            message: "Min relevance must be between 0.0 and 1.0".to_string(),
        });
    }
}

fn validate_security(
    sec: &super::gateway_config::SecuritySection,
    issues: &mut Vec<ValidationIssue>,
) {
    // Warn if no exec approval
    if !sec.exec_approval {
        issues.push(ValidationIssue {
            path: "security.exec_approval".to_string(),
            severity: IssueSeverity::Warning,
            message: "Exec approval is disabled - commands will run without confirmation"
                .to_string(),
        });
    }

    // Check trusted keys format
    for (i, key) in sec.trusted_keys.iter().enumerate() {
        if key.len() != 64 && !key.starts_with("ssh-ed25519") {
            issues.push(ValidationIssue {
                path: format!("security.trusted_keys[{}]", i),
                severity: IssueSeverity::Warning,
                message: "Trusted key doesn't appear to be a valid Ed25519 key".to_string(),
            });
        }
    }
}

fn validate_session(
    sess: &super::gateway_config::SessionSection,
    issues: &mut Vec<ValidationIssue>,
) {
    if sess.auto_compact_threshold == 0 {
        issues.push(ValidationIssue {
            path: "session.auto_compact_threshold".to_string(),
            severity: IssueSeverity::Warning,
            message: "Auto-compact threshold of 0 means compaction on every message".to_string(),
        });
    }

    if sess.max_sessions == 0 {
        issues.push(ValidationIssue {
            path: "session.max_sessions".to_string(),
            severity: IssueSeverity::Error,
            message: "Max sessions must be at least 1".to_string(),
        });
    }
}

fn validate_tts(tts: &super::gateway_config::TtsSection, issues: &mut Vec<ValidationIssue>) {
    if !tts.enabled {
        return;
    }

    let valid_engines = ["system", "openai", "elevenlabs"];
    if !valid_engines.contains(&tts.engine.as_str()) {
        issues.push(ValidationIssue {
            path: "tts.engine".to_string(),
            severity: IssueSeverity::Error,
            message: format!(
                "Invalid TTS engine '{}'. Must be one of: {}",
                tts.engine,
                valid_engines.join(", ")
            ),
        });
    }

    if tts.rate < 0.5 || tts.rate > 2.0 {
        issues.push(ValidationIssue {
            path: "tts.rate".to_string(),
            severity: IssueSeverity::Error,
            message: format!("Speech rate must be between 0.5 and 2.0, got {}", tts.rate),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_default_config() {
        let config = GatewayCliConfig::default();
        let issues = validate_config(&config);
        let errors: Vec<_> = issues.iter().filter(|i| i.severity == IssueSeverity::Error).collect();
        assert!(errors.is_empty(), "Default config should have no errors: {:?}", errors);
    }

    #[test]
    fn test_validate_port_conflict() {
        let mut config = GatewayCliConfig::default();
        config.gateway.http_port = config.gateway.port; // Same port
        let issues = validate_config(&config);
        assert!(
            issues
                .iter()
                .any(|i| i.path == "gateway.http_port" && i.severity == IssueSeverity::Error)
        );
    }

    #[test]
    fn test_validate_invalid_temperature() {
        let mut config = GatewayCliConfig::default();
        config.llm.temperature = 5.0;
        let issues = validate_config(&config);
        assert!(
            issues
                .iter()
                .any(|i| i.path == "llm.temperature" && i.severity == IssueSeverity::Error)
        );
    }

    #[test]
    fn test_validate_disabled_auth_warning() {
        let mut config = GatewayCliConfig::default();
        config.gateway.require_auth = false;
        let issues = validate_config(&config);
        assert!(
            issues
                .iter()
                .any(|i| i.path == "gateway.require_auth" && i.severity == IssueSeverity::Warning)
        );
    }

    #[test]
    fn test_validate_invalid_memory_backend() {
        let mut config = GatewayCliConfig::default();
        config.memory.backend = "redis".to_string();
        let issues = validate_config(&config);
        assert!(
            issues
                .iter()
                .any(|i| i.path == "memory.backend" && i.severity == IssueSeverity::Error)
        );
    }
}
