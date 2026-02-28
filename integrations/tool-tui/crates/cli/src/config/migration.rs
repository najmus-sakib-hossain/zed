//! TOML to YAML Configuration Migration
//!
//! Converts legacy TOML configuration files to the new YAML format.

use std::path::Path;

/// Migrate a TOML configuration file to YAML format.
///
/// Reads the TOML file, converts it to the YAML configuration structure,
/// and returns the YAML string. The original file is not modified.
pub fn migrate_toml_to_yaml(toml_content: &str) -> Result<String, MigrationError> {
    // Parse TOML
    let toml_value: toml::Value =
        toml::from_str(toml_content).map_err(|e| MigrationError::ParseError {
            format: "TOML".to_string(),
            msg: e.to_string(),
        })?;

    // Convert to serde_json::Value as intermediate format
    let json_value = toml_to_json(&toml_value);

    // Apply migration transformations
    let migrated = apply_migrations(json_value)?;

    // Serialize to YAML
    let yaml = serde_yaml::to_string(&migrated)
        .map_err(|e| MigrationError::SerializeError(e.to_string()))?;

    Ok(yaml)
}

/// Migrate a TOML file and write the YAML output
pub fn migrate_file(toml_path: &Path, yaml_path: &Path) -> Result<MigrationReport, MigrationError> {
    let toml_content = std::fs::read_to_string(toml_path).map_err(|e| MigrationError::IoError {
        path: toml_path.display().to_string(),
        source: e,
    })?;

    let yaml_content = migrate_toml_to_yaml(&toml_content)?;

    // Create parent directory if needed
    if let Some(parent) = yaml_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| MigrationError::IoError {
            path: parent.display().to_string(),
            source: e,
        })?;
    }

    std::fs::write(yaml_path, &yaml_content).map_err(|e| MigrationError::IoError {
        path: yaml_path.display().to_string(),
        source: e,
    })?;

    Ok(MigrationReport {
        source: toml_path.display().to_string(),
        destination: yaml_path.display().to_string(),
        fields_migrated: count_fields(
            &serde_yaml::from_str::<serde_yaml::Value>(&yaml_content).unwrap_or_default(),
        ),
        warnings: Vec::new(),
    })
}

/// Convert TOML Value to JSON Value
fn toml_to_json(toml: &toml::Value) -> serde_json::Value {
    match toml {
        toml::Value::String(s) => serde_json::Value::String(s.clone()),
        toml::Value::Integer(i) => serde_json::json!(*i),
        toml::Value::Float(f) => serde_json::json!(*f),
        toml::Value::Boolean(b) => serde_json::Value::Bool(*b),
        toml::Value::Datetime(dt) => serde_json::Value::String(dt.to_string()),
        toml::Value::Array(arr) => serde_json::Value::Array(arr.iter().map(toml_to_json).collect()),
        toml::Value::Table(table) => {
            let map: serde_json::Map<String, serde_json::Value> =
                table.iter().map(|(k, v)| (k.clone(), toml_to_json(v))).collect();
            serde_json::Value::Object(map)
        }
    }
}

/// Apply migration transformations to convert old config structure to new
fn apply_migrations(mut value: serde_json::Value) -> Result<serde_json::Value, MigrationError> {
    if let Some(obj) = value.as_object_mut() {
        // Migration 1: Move top-level "port" into "gateway.port"
        if let Some(port) = obj.remove("port") {
            let gateway = obj.entry("gateway").or_insert_with(|| serde_json::json!({}));
            if let Some(gw) = gateway.as_object_mut() {
                gw.entry("port").or_insert(port);
            }
        }

        // Migration 2: Move "host" into "gateway.host"
        if let Some(host) = obj.remove("host") {
            let gateway = obj.entry("gateway").or_insert_with(|| serde_json::json!({}));
            if let Some(gw) = gateway.as_object_mut() {
                gw.entry("host").or_insert(host);
            }
        }

        // Migration 3: Move "model" into "llm.default_model"
        if let Some(model) = obj.remove("model") {
            let llm = obj.entry("llm").or_insert_with(|| serde_json::json!({}));
            if let Some(l) = llm.as_object_mut() {
                l.entry("default_model").or_insert(model);
            }
        }

        // Migration 4: Move "provider" into "llm.default_provider"
        if let Some(provider) = obj.remove("provider") {
            let llm = obj.entry("llm").or_insert_with(|| serde_json::json!({}));
            if let Some(l) = llm.as_object_mut() {
                l.entry("default_provider").or_insert(provider);
            }
        }

        // Migration 5: Rename "max_tokens" to nested path
        if let Some(tokens) = obj.remove("max_tokens") {
            let llm = obj.entry("llm").or_insert_with(|| serde_json::json!({}));
            if let Some(l) = llm.as_object_mut() {
                l.entry("max_tokens").or_insert(tokens);
            }
        }

        // Migration 6: Move "api_key" into provider config
        if let Some(api_key) = obj.remove("api_key") {
            let llm = obj.entry("llm").or_insert_with(|| serde_json::json!({}));
            if let Some(l) = llm.as_object_mut() {
                // Extract default_provider before mutable borrow
                let default_provider = l
                    .get("default_provider")
                    .and_then(|v| v.as_str())
                    .unwrap_or("openai")
                    .to_string();
                let providers = l.entry("providers").or_insert_with(|| serde_json::json!({}));
                if let Some(p) = providers.as_object_mut() {
                    let provider =
                        p.entry(&default_provider).or_insert_with(|| serde_json::json!({}));
                    if let Some(prov) = provider.as_object_mut() {
                        prov.entry("api_key").or_insert(api_key);
                    }
                }
            }
        }

        // Migration 7: Convert "log_level" to "logging.level"
        if let Some(level) = obj.remove("log_level") {
            let logging = obj.entry("logging").or_insert_with(|| serde_json::json!({}));
            if let Some(l) = logging.as_object_mut() {
                l.entry("level").or_insert(level);
            }
        }
    }

    Ok(value)
}

/// Count fields in a YAML value recursively
fn count_fields(value: &serde_yaml::Value) -> usize {
    match value {
        serde_yaml::Value::Mapping(map) => {
            map.len() + map.values().map(count_fields).sum::<usize>()
        }
        serde_yaml::Value::Sequence(seq) => seq.iter().map(count_fields).sum(),
        _ => 0,
    }
}

/// Migration report
#[derive(Debug, Clone)]
pub struct MigrationReport {
    /// Source file path
    pub source: String,
    /// Destination file path
    pub destination: String,
    /// Number of fields migrated
    pub fields_migrated: usize,
    /// Migration warnings
    pub warnings: Vec<String>,
}

impl std::fmt::Display for MigrationReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Migration Report")?;
        writeln!(f, "  Source:      {}", self.source)?;
        writeln!(f, "  Destination: {}", self.destination)?;
        writeln!(f, "  Fields:      {}", self.fields_migrated)?;
        for warning in &self.warnings {
            writeln!(f, "  Warning:     {}", warning)?;
        }
        Ok(())
    }
}

/// Migration errors
#[derive(Debug, thiserror::Error)]
pub enum MigrationError {
    #[error("Failed to parse {format}: {msg}")]
    ParseError { format: String, msg: String },

    #[error("Failed to serialize: {0}")]
    SerializeError(String),

    #[error("IO error for '{path}': {source}")]
    IoError {
        path: String,
        #[source]
        source: std::io::Error,
    },

    #[error("Migration rule failed: {0}")]
    RuleError(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_migration() {
        let toml = r#"
[gateway]
host = "0.0.0.0"
port = 31337

[llm]
provider = "ollama"
model = "llama3.2"
"#;
        let yaml = migrate_toml_to_yaml(toml).unwrap();
        assert!(yaml.contains("gateway:"));
        assert!(yaml.contains("port: 31337"));
        assert!(yaml.contains("llm:"));
    }

    #[test]
    fn test_flat_to_nested_migration() {
        let toml = r#"
port = 8080
host = "127.0.0.1"
model = "gpt-4"
provider = "openai"
"#;
        let yaml = migrate_toml_to_yaml(toml).unwrap();
        // Should be nested under gateway/llm
        let parsed: serde_yaml::Value = serde_yaml::from_str(&yaml).unwrap();
        let map = parsed.as_mapping().unwrap();

        let gw = map
            .get(&serde_yaml::Value::String("gateway".to_string()))
            .unwrap()
            .as_mapping()
            .unwrap();
        assert_eq!(
            gw.get(&serde_yaml::Value::String("port".to_string())),
            Some(&serde_yaml::Value::Number(serde_yaml::Number::from(8080)))
        );
    }

    #[test]
    fn test_log_level_migration() {
        let toml = r#"log_level = "debug""#;
        let yaml = migrate_toml_to_yaml(toml).unwrap();
        let parsed: serde_yaml::Value = serde_yaml::from_str(&yaml).unwrap();
        let map = parsed.as_mapping().unwrap();
        let logging = map
            .get(&serde_yaml::Value::String("logging".to_string()))
            .unwrap()
            .as_mapping()
            .unwrap();
        assert_eq!(
            logging.get(&serde_yaml::Value::String("level".to_string())),
            Some(&serde_yaml::Value::String("debug".to_string()))
        );
    }

    #[test]
    fn test_invalid_toml() {
        let result = migrate_toml_to_yaml("not valid toml [[[");
        assert!(result.is_err());
    }

    #[test]
    fn test_file_migration() {
        let dir = tempfile::tempdir().unwrap();
        let toml_path = dir.path().join("config.toml");
        let yaml_path = dir.path().join("config.yaml");

        std::fs::write(&toml_path, "[gateway]\nport = 9999\n").unwrap();

        let report = migrate_file(&toml_path, &yaml_path).unwrap();
        assert!(yaml_path.exists());
        assert!(report.fields_migrated > 0);

        let yaml_content = std::fs::read_to_string(&yaml_path).unwrap();
        assert!(yaml_content.contains("9999"));
    }
}
