//! Property-based tests for DX Configuration System
//!
//! These tests verify universal properties for configuration loading,
//! parsing, and validation across driven, generator, and dcp sections.
//!
//! Feature: dx-unified-tooling, Property 11: Configuration Loading
//! **Validates: Requirements 13.1, 13.2, 13.3, 13.4, 13.5, 13.6**
//!
//! Run with: cargo test --test config_property_tests

use proptest::prelude::*;
use tempfile::TempDir;

// ============================================================================
// Test Configuration Structures (mirrors config.rs)
// ============================================================================

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize, PartialEq)]
struct TestDxConfig {
    #[serde(default)]
    project: TestProjectConfig,
    #[serde(default)]
    driven: TestDrivenConfig,
    #[serde(default)]
    generator: TestGeneratorConfig,
    #[serde(default)]
    dcp: TestDcpConfig,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize, PartialEq)]
struct TestProjectConfig {
    #[serde(default = "default_name")]
    name: String,
    #[serde(default = "default_version")]
    version: String,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize, PartialEq)]
struct TestDrivenConfig {
    #[serde(default)]
    editors: TestEditorConfig,
    #[serde(default)]
    sync: TestSyncConfig,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
struct TestEditorConfig {
    #[serde(default = "default_true")]
    cursor: bool,
    #[serde(default = "default_true")]
    copilot: bool,
    #[serde(default)]
    windsurf: bool,
    #[serde(default = "default_true")]
    claude: bool,
    #[serde(default)]
    aider: bool,
    #[serde(default)]
    cline: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
struct TestSyncConfig {
    #[serde(default = "default_true")]
    watch: bool,
    #[serde(default = "default_source")]
    source_of_truth: String,
    #[serde(default = "default_debounce")]
    debounce_ms: u64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
struct TestGeneratorConfig {
    #[serde(default = "default_templates_dir")]
    templates_dir: String,
    #[serde(default = "default_true")]
    triggers_enabled: bool,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize, PartialEq)]
struct TestDcpConfig {
    #[serde(default)]
    server: TestServerConfig,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
struct TestServerConfig {
    #[serde(default = "default_host")]
    host: String,
    #[serde(default = "default_port")]
    port: u16,
}

// Default value functions
fn default_true() -> bool {
    true
}
fn default_name() -> String {
    "dx-project".to_string()
}
fn default_version() -> String {
    "0.1.0".to_string()
}
fn default_source() -> String {
    ".driven/rules.drv".to_string()
}
fn default_debounce() -> u64 {
    500
}
fn default_templates_dir() -> String {
    ".dx/templates".to_string()
}
fn default_host() -> String {
    "127.0.0.1".to_string()
}
fn default_port() -> u16 {
    9000
}

// ============================================================================
// Arbitrary Generators
// ============================================================================

fn arbitrary_project_name() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("my-project".to_string()),
        Just("dx-app".to_string()),
        Just("test_project".to_string()),
        "[a-z][a-z0-9-]{0,20}".prop_map(|s| s.to_string()),
    ]
}

fn arbitrary_version() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("0.1.0".to_string()),
        Just("1.0.0".to_string()),
        Just("2.3.4".to_string()),
        "[0-9]{1,2}\\.[0-9]{1,2}\\.[0-9]{1,2}".prop_map(|s| s.to_string()),
    ]
}

fn arbitrary_path() -> impl Strategy<Value = String> {
    prop_oneof![
        Just(".driven/rules.drv".to_string()),
        Just(".driven/rules.md".to_string()),
        Just("rules/main.drv".to_string()),
        "[a-z./]{1,30}".prop_map(|s| s.to_string()),
    ]
}

fn arbitrary_host() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("127.0.0.1".to_string()),
        Just("0.0.0.0".to_string()),
        Just("localhost".to_string()),
    ]
}

fn arbitrary_port() -> impl Strategy<Value = u16> {
    prop_oneof![
        Just(9000u16),
        Just(8080u16),
        Just(3000u16),
        1024u16..65535u16,
    ]
}

fn arbitrary_debounce() -> impl Strategy<Value = u64> {
    prop_oneof![Just(100u64), Just(500u64), Just(1000u64), 100u64..5000u64,]
}

// ============================================================================
// Property Tests
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Property 11: Configuration Loading Round-Trip
    /// *For any* valid configuration, serializing to TOML and parsing back
    /// SHALL produce an equivalent configuration.
    ///
    /// **Validates: Requirements 13.1, 13.2, 13.3, 13.4**
    #[test]
    fn prop_config_round_trip(
        name in arbitrary_project_name(),
        version in arbitrary_version(),
        cursor in any::<bool>(),
        copilot in any::<bool>(),
        windsurf in any::<bool>(),
        watch in any::<bool>(),
        source in arbitrary_path(),
        debounce in arbitrary_debounce(),
        templates_dir in arbitrary_path(),
        triggers_enabled in any::<bool>(),
        host in arbitrary_host(),
        port in arbitrary_port(),
    ) {
        let config = TestDxConfig {
            project: TestProjectConfig {
                name: name.clone(),
                version: version.clone(),
            },
            driven: TestDrivenConfig {
                editors: TestEditorConfig {
                    cursor,
                    copilot,
                    windsurf,
                    claude: true,
                    aider: false,
                    cline: false,
                },
                sync: TestSyncConfig {
                    watch,
                    source_of_truth: source.clone(),
                    debounce_ms: debounce,
                },
            },
            generator: TestGeneratorConfig {
                templates_dir: templates_dir.clone(),
                triggers_enabled,
            },
            dcp: TestDcpConfig {
                server: TestServerConfig {
                    host: host.clone(),
                    port,
                },
            },
        };

        // Serialize to TOML
        let toml_str = toml::to_string_pretty(&config).unwrap();

        // Parse back
        let parsed: TestDxConfig = toml::from_str(&toml_str).unwrap();

        // Verify equality
        prop_assert_eq!(config, parsed);
    }

    /// Property 11b: Configuration Defaults
    /// *For any* missing configuration section, the system SHALL apply
    /// sensible defaults without errors.
    ///
    /// **Validates: Requirements 13.1, 13.2, 13.3, 13.4**
    #[test]
    fn prop_config_defaults(
        name in arbitrary_project_name(),
    ) {
        // Minimal config with only project name
        let minimal_toml = format!(r#"
[project]
name = "{}"
version = "1.0.0"
"#, name);

        let config: TestDxConfig = toml::from_str(&minimal_toml).unwrap();

        // Verify defaults are applied
        prop_assert_eq!(config.driven.editors.cursor, true);
        prop_assert_eq!(config.driven.editors.copilot, true);
        prop_assert_eq!(config.driven.sync.watch, true);
        prop_assert_eq!(config.driven.sync.debounce_ms, 500);
        prop_assert_eq!(config.generator.triggers_enabled, true);
        prop_assert_eq!(config.dcp.server.host, "127.0.0.1");
        prop_assert_eq!(config.dcp.server.port, 9000);
    }

    /// Property 11c: Partial Configuration Override
    /// *For any* partial configuration, only specified values SHALL be
    /// overridden while others retain defaults.
    ///
    /// **Validates: Requirements 13.5, 13.6**
    #[test]
    fn prop_partial_config_override(
        port in arbitrary_port(),
        debounce in arbitrary_debounce(),
    ) {
        // Config with only some values specified
        let partial_toml = format!(r#"
[dcp.server]
port = {}

[driven.sync]
debounce_ms = {}
"#, port, debounce);

        let config: TestDxConfig = toml::from_str(&partial_toml).unwrap();

        // Verify specified values
        prop_assert_eq!(config.dcp.server.port, port);
        prop_assert_eq!(config.driven.sync.debounce_ms, debounce);

        // Verify defaults for unspecified values
        prop_assert_eq!(config.dcp.server.host, "127.0.0.1");
        prop_assert_eq!(config.driven.sync.watch, true);
        prop_assert_eq!(config.driven.editors.cursor, true);
    }
}

// ============================================================================
// Unit Tests for Edge Cases
// ============================================================================

/// Test empty configuration file
#[test]
fn test_empty_config() {
    let config: TestDxConfig = toml::from_str("").unwrap();

    // When entire section is missing, Default::default() is used for the struct
    // which uses String::default() (empty string) for name, not the serde default
    assert_eq!(config.project.name, "");
    assert_eq!(config.driven.editors.cursor, true);
    assert_eq!(config.dcp.server.port, 9000);
}

/// Test configuration with all sections
#[test]
fn test_full_config() {
    let toml_str = r#"
[project]
name = "my-app"
version = "2.0.0"

[driven.editors]
cursor = true
copilot = false
windsurf = true
claude = true
aider = false
cline = false

[driven.sync]
watch = false
source_of_truth = "custom/rules.drv"
debounce_ms = 1000

[generator]
templates_dir = "custom/templates"
triggers_enabled = false

[dcp.server]
host = "0.0.0.0"
port = 8080
"#;

    let config: TestDxConfig = toml::from_str(toml_str).unwrap();

    assert_eq!(config.project.name, "my-app");
    assert_eq!(config.project.version, "2.0.0");
    assert!(config.driven.editors.cursor);
    assert!(!config.driven.editors.copilot);
    assert!(config.driven.editors.windsurf);
    assert!(!config.driven.sync.watch);
    assert_eq!(config.driven.sync.source_of_truth, "custom/rules.drv");
    assert_eq!(config.driven.sync.debounce_ms, 1000);
    assert_eq!(config.generator.templates_dir, "custom/templates");
    assert!(!config.generator.triggers_enabled);
    assert_eq!(config.dcp.server.host, "0.0.0.0");
    assert_eq!(config.dcp.server.port, 8080);
}

/// Test configuration file creation and loading
#[test]
fn test_config_file_io() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("dx.toml");

    let config = TestDxConfig {
        project: TestProjectConfig {
            name: "test-project".to_string(),
            version: "1.0.0".to_string(),
        },
        ..Default::default()
    };

    // Write config
    let toml_str = toml::to_string_pretty(&config).unwrap();
    std::fs::write(&config_path, &toml_str).unwrap();

    // Read and parse
    let content = std::fs::read_to_string(&config_path).unwrap();
    let loaded: TestDxConfig = toml::from_str(&content).unwrap();

    assert_eq!(config, loaded);
}

/// Test invalid TOML handling
#[test]
fn test_invalid_toml() {
    let invalid_toml = r#"
[project
name = "broken"
"#;

    let result: Result<TestDxConfig, _> = toml::from_str(invalid_toml);
    assert!(result.is_err());
}

/// Test type mismatch handling
#[test]
fn test_type_mismatch() {
    let wrong_type_toml = r#"
[dcp.server]
port = "not a number"
"#;

    let result: Result<TestDxConfig, _> = toml::from_str(wrong_type_toml);
    assert!(result.is_err());
}

/// Test JSON serialization compatibility
#[test]
fn test_json_serialization() {
    let config = TestDxConfig::default();

    // Serialize to JSON
    let json_str = serde_json::to_string_pretty(&config).unwrap();

    // Parse back
    let parsed: TestDxConfig = serde_json::from_str(&json_str).unwrap();

    assert_eq!(config, parsed);
}
