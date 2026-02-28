//! Plugin Manifest Validation Module
//!
//! Validates plugin manifests for correctness, security,
//! and compatibility before loading.

use super::PluginType;
use super::manifest::{MANIFEST_VERSION, PluginManifest};
use super::traits::Capability;

/// Validation severity levels
#[derive(Debug, Clone, PartialEq)]
pub enum Severity {
    Error,
    Warning,
    Info,
}

/// A single validation issue
#[derive(Debug, Clone)]
pub struct ValidationIssue {
    pub severity: Severity,
    pub field: String,
    pub message: String,
}

/// Validation result
#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub issues: Vec<ValidationIssue>,
}

impl ValidationResult {
    pub fn is_valid(&self) -> bool {
        !self.issues.iter().any(|i| i.severity == Severity::Error)
    }

    pub fn errors(&self) -> Vec<&ValidationIssue> {
        self.issues.iter().filter(|i| i.severity == Severity::Error).collect()
    }

    pub fn warnings(&self) -> Vec<&ValidationIssue> {
        self.issues.iter().filter(|i| i.severity == Severity::Warning).collect()
    }
}

/// Validate a plugin manifest
pub fn validate_manifest(manifest: &PluginManifest) -> ValidationResult {
    let mut issues = Vec::new();

    // Validate manifest version
    validate_version(manifest, &mut issues);

    // Validate plugin identity
    validate_identity(manifest, &mut issues);

    // Validate runtime requirements
    validate_runtime(manifest, &mut issues);

    // Validate capabilities
    validate_capabilities(manifest, &mut issues);

    // Validate config schema
    validate_config(manifest, &mut issues);

    // Validate hooks
    validate_hooks(manifest, &mut issues);

    // Validate dependencies
    validate_dependencies(manifest, &mut issues);

    // Security checks
    validate_security(manifest, &mut issues);

    ValidationResult { issues }
}

fn validate_version(manifest: &PluginManifest, issues: &mut Vec<ValidationIssue>) {
    if manifest.manifest_version != MANIFEST_VERSION {
        issues.push(ValidationIssue {
            severity: Severity::Warning,
            field: "manifest_version".to_string(),
            message: format!(
                "Manifest version '{}' differs from current '{}'",
                manifest.manifest_version, MANIFEST_VERSION
            ),
        });
    }
}

fn validate_identity(manifest: &PluginManifest, issues: &mut Vec<ValidationIssue>) {
    let plugin = &manifest.plugin;

    // Name validation
    if plugin.name.is_empty() {
        issues.push(ValidationIssue {
            severity: Severity::Error,
            field: "plugin.name".to_string(),
            message: "Plugin name is required".to_string(),
        });
    } else if !is_valid_plugin_name(&plugin.name) {
        issues.push(ValidationIssue {
            severity: Severity::Error,
            field: "plugin.name".to_string(),
            message: format!(
                "Plugin name '{}' is invalid. Must be lowercase alphanumeric with hyphens",
                plugin.name
            ),
        });
    }

    // Version validation
    if plugin.version.is_empty() {
        issues.push(ValidationIssue {
            severity: Severity::Error,
            field: "plugin.version".to_string(),
            message: "Plugin version is required".to_string(),
        });
    } else if !is_valid_semver(&plugin.version) {
        issues.push(ValidationIssue {
            severity: Severity::Warning,
            field: "plugin.version".to_string(),
            message: format!("Version '{}' does not appear to be valid semver", plugin.version),
        });
    }

    // Entry point validation
    if plugin.entry.is_empty() {
        issues.push(ValidationIssue {
            severity: Severity::Error,
            field: "plugin.entry".to_string(),
            message: "Plugin entry point is required".to_string(),
        });
    } else {
        // Validate entry matches plugin type
        match plugin.plugin_type {
            PluginType::Wasm => {
                if !plugin.entry.ends_with(".wasm") {
                    issues.push(ValidationIssue {
                        severity: Severity::Warning,
                        field: "plugin.entry".to_string(),
                        message: "WASM plugin entry should end with .wasm".to_string(),
                    });
                }
            }
            PluginType::Native => {
                let valid_ext = plugin.entry.ends_with(".so")
                    || plugin.entry.ends_with(".dll")
                    || plugin.entry.ends_with(".dylib");
                if !valid_ext {
                    issues.push(ValidationIssue {
                        severity: Severity::Warning,
                        field: "plugin.entry".to_string(),
                        message: "Native plugin entry should end with .so, .dll, or .dylib"
                            .to_string(),
                    });
                }
            }
        }
    }

    // Description recommendation
    if plugin.description.is_none() {
        issues.push(ValidationIssue {
            severity: Severity::Info,
            field: "plugin.description".to_string(),
            message: "Consider adding a description for your plugin".to_string(),
        });
    }

    // Author recommendation
    if plugin.author.is_none() {
        issues.push(ValidationIssue {
            severity: Severity::Info,
            field: "plugin.author".to_string(),
            message: "Consider adding an author field".to_string(),
        });
    }
}

fn validate_runtime(manifest: &PluginManifest, issues: &mut Vec<ValidationIssue>) {
    let rt = &manifest.runtime;

    // Memory limits
    if rt.max_memory == 0 {
        issues.push(ValidationIssue {
            severity: Severity::Error,
            field: "runtime.max_memory".to_string(),
            message: "Max memory cannot be zero".to_string(),
        });
    } else if rt.max_memory > 2 * 1024 * 1024 * 1024 {
        // > 2 GB
        issues.push(ValidationIssue {
            severity: Severity::Warning,
            field: "runtime.max_memory".to_string(),
            message: "Max memory exceeds 2GB, this may cause issues".to_string(),
        });
    }

    // CPU limits
    if rt.max_cpu_ms == 0 {
        issues.push(ValidationIssue {
            severity: Severity::Error,
            field: "runtime.max_cpu_ms".to_string(),
            message: "Max CPU time cannot be zero".to_string(),
        });
    }

    // Timeout
    if rt.timeout_ms < rt.max_cpu_ms {
        issues.push(ValidationIssue {
            severity: Severity::Warning,
            field: "runtime.timeout_ms".to_string(),
            message: "Timeout is less than max CPU time, plugin may be killed prematurely"
                .to_string(),
        });
    }
}

fn validate_capabilities(manifest: &PluginManifest, issues: &mut Vec<ValidationIssue>) {
    for cap_str in &manifest.capabilities {
        // Capability::from_str always returns a valid variant (defaults to Network)
        let _cap = Capability::from_str(cap_str);
    }

    // Check for redundant capabilities
    if manifest.capabilities.contains(&"system".to_string()) {
        issues.push(ValidationIssue {
            severity: Severity::Warning,
            field: "capabilities".to_string(),
            message: "'system' capability grants all permissions; other capabilities are redundant"
                .to_string(),
        });
    }
}

fn validate_config(manifest: &PluginManifest, issues: &mut Vec<ValidationIssue>) {
    let valid_types = ["string", "number", "boolean", "array", "object", "integer"];

    for (name, field) in &manifest.config {
        if !valid_types.contains(&field.field_type.as_str()) {
            issues.push(ValidationIssue {
                severity: Severity::Error,
                field: format!("config.{}", name),
                message: format!(
                    "Invalid field type '{}'. Must be one of: {}",
                    field.field_type,
                    valid_types.join(", ")
                ),
            });
        }

        // Warn about required fields without defaults
        if field.required && field.default.is_some() {
            issues.push(ValidationIssue {
                severity: Severity::Info,
                field: format!("config.{}", name),
                message: "Field is both required and has a default value".to_string(),
            });
        }

        // Secret fields should be strings
        if field.secret && field.field_type != "string" {
            issues.push(ValidationIssue {
                severity: Severity::Warning,
                field: format!("config.{}", name),
                message: "Secret fields should be of type 'string'".to_string(),
            });
        }
    }
}

fn validate_hooks(manifest: &PluginManifest, issues: &mut Vec<ValidationIssue>) {
    let known_events = [
        "before_chat",
        "after_chat",
        "before_exec",
        "after_exec",
        "on_connect",
        "on_disconnect",
        "on_message",
        "on_error",
        "before_memory_store",
        "after_memory_store",
        "before_session_create",
        "after_session_create",
        "on_config_reload",
    ];

    for (i, hook) in manifest.hooks.iter().enumerate() {
        if hook.event.is_empty() {
            issues.push(ValidationIssue {
                severity: Severity::Error,
                field: format!("hooks[{}].event", i),
                message: "Hook event name is required".to_string(),
            });
        } else if !known_events.contains(&hook.event.as_str()) {
            issues.push(ValidationIssue {
                severity: Severity::Info,
                field: format!("hooks[{}].event", i),
                message: format!("Unknown hook event '{}'; custom events are allowed", hook.event),
            });
        }

        if hook.handler.is_empty() {
            issues.push(ValidationIssue {
                severity: Severity::Error,
                field: format!("hooks[{}].handler", i),
                message: "Hook handler name is required".to_string(),
            });
        }
    }

    // Check for duplicate hook registrations
    let mut seen = std::collections::HashSet::new();
    for hook in &manifest.hooks {
        let key = format!("{}:{}", hook.event, hook.handler);
        if !seen.insert(key.clone()) {
            issues.push(ValidationIssue {
                severity: Severity::Warning,
                field: "hooks".to_string(),
                message: format!("Duplicate hook registration: {}", key),
            });
        }
    }
}

fn validate_dependencies(manifest: &PluginManifest, issues: &mut Vec<ValidationIssue>) {
    for (i, dep) in manifest.dependencies.iter().enumerate() {
        if dep.name.is_empty() {
            issues.push(ValidationIssue {
                severity: Severity::Error,
                field: format!("dependencies[{}].name", i),
                message: "Dependency name is required".to_string(),
            });
        }

        if dep.version.is_empty() {
            issues.push(ValidationIssue {
                severity: Severity::Error,
                field: format!("dependencies[{}].version", i),
                message: "Dependency version is required".to_string(),
            });
        }

        // Check for self-dependency
        if dep.name == manifest.plugin.name {
            issues.push(ValidationIssue {
                severity: Severity::Error,
                field: format!("dependencies[{}]", i),
                message: "Plugin cannot depend on itself".to_string(),
            });
        }
    }
}

fn validate_security(manifest: &PluginManifest, issues: &mut Vec<ValidationIssue>) {
    // Warn about dangerous capability combinations
    let has_network = manifest.capabilities.contains(&"network".to_string());
    let has_file_write = manifest.capabilities.contains(&"file_write".to_string());
    let has_shell = manifest.capabilities.contains(&"shell".to_string());

    if has_network && has_file_write {
        issues.push(ValidationIssue {
            severity: Severity::Warning,
            field: "capabilities".to_string(),
            message:
                "Plugin has both 'network' and 'file_write' — review for data exfiltration risk"
                    .to_string(),
        });
    }

    if has_shell {
        issues.push(ValidationIssue {
            severity: Severity::Warning,
            field: "capabilities".to_string(),
            message: "Plugin requests 'shell' capability — this allows arbitrary command execution"
                .to_string(),
        });
    }

    // Native plugins are inherently less sandboxed
    if manifest.plugin.plugin_type == PluginType::Native {
        issues.push(ValidationIssue {
            severity: Severity::Warning,
            field: "plugin.plugin_type".to_string(),
            message: "Native plugins run outside the WASM sandbox; ensure binary is trusted"
                .to_string(),
        });
    }
}

/// Validate a plugin name (lowercase, alphanumeric, hyphens, 1-64 chars)
fn is_valid_plugin_name(name: &str) -> bool {
    if name.is_empty() || name.len() > 64 {
        return false;
    }
    name.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        && !name.starts_with('-')
        && !name.ends_with('-')
}

/// Basic semver validation (major.minor.patch)
fn is_valid_semver(version: &str) -> bool {
    let parts: Vec<&str> = version.split('.').collect();
    if parts.len() < 2 || parts.len() > 3 {
        return false;
    }
    parts.iter().all(|p| {
        // Allow pre-release suffixes on last part
        let base = p.split('-').next().unwrap_or(p);
        base.parse::<u64>().is_ok()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugin::manifest::example_manifest;

    #[test]
    fn test_valid_manifest() {
        let manifest = example_manifest();
        let result = validate_manifest(&manifest);
        assert!(result.is_valid(), "Errors: {:?}", result.errors());
    }

    #[test]
    fn test_empty_name() {
        let mut manifest = example_manifest();
        manifest.plugin.name = String::new();
        let result = validate_manifest(&manifest);
        assert!(!result.is_valid());
        assert!(result.errors().iter().any(|e| e.field == "plugin.name"));
    }

    #[test]
    fn test_invalid_name() {
        let mut manifest = example_manifest();
        manifest.plugin.name = "Invalid Name!".to_string();
        let result = validate_manifest(&manifest);
        assert!(!result.is_valid());
    }

    #[test]
    fn test_valid_names() {
        assert!(is_valid_plugin_name("my-plugin"));
        assert!(is_valid_plugin_name("plugin123"));
        assert!(is_valid_plugin_name("a"));
        assert!(!is_valid_plugin_name(""));
        assert!(!is_valid_plugin_name("-plugin"));
        assert!(!is_valid_plugin_name("plugin-"));
        assert!(!is_valid_plugin_name("My Plugin"));
    }

    #[test]
    fn test_dangerous_capabilities_warning() {
        let mut manifest = example_manifest();
        manifest.capabilities = vec!["shell".to_string()];
        let result = validate_manifest(&manifest);
        assert!(result.warnings().iter().any(|w| w.message.contains("shell")));
    }

    #[test]
    fn test_zero_memory_error() {
        let mut manifest = example_manifest();
        manifest.runtime.max_memory = 0;
        let result = validate_manifest(&manifest);
        assert!(!result.is_valid());
    }

    #[test]
    fn test_self_dependency() {
        let mut manifest = example_manifest();
        manifest.dependencies.push(super::super::manifest::PluginDependency {
            name: "example-plugin".to_string(),
            version: "1.0.0".to_string(),
            optional: false,
        });
        let result = validate_manifest(&manifest);
        assert!(!result.is_valid());
    }

    #[test]
    fn test_semver_validation() {
        assert!(is_valid_semver("1.0.0"));
        assert!(is_valid_semver("0.1.0"));
        assert!(is_valid_semver("1.0"));
        assert!(is_valid_semver("1.0.0-beta"));
        assert!(!is_valid_semver("abc"));
        assert!(!is_valid_semver("1"));
    }

    #[test]
    fn test_wasm_entry_warning() {
        let mut manifest = example_manifest();
        manifest.plugin.plugin_type = PluginType::Wasm;
        manifest.plugin.entry = "plugin.js".to_string();
        let result = validate_manifest(&manifest);
        assert!(result.warnings().iter().any(|w| w.field == "plugin.entry"));
    }

    #[test]
    fn test_native_plugin_warning() {
        let mut manifest = example_manifest();
        manifest.plugin.plugin_type = PluginType::Native;
        manifest.plugin.entry = "plugin.so".to_string();
        let result = validate_manifest(&manifest);
        assert!(result.warnings().iter().any(|w| w.message.contains("sandbox")));
    }
}
