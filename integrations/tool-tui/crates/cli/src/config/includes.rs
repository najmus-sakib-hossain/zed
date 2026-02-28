//! File Includes with Deep Merge
//!
//! Supports `include:` directives in YAML configuration files.
//! Included files are deeply merged with the base configuration,
//! with the including file taking precedence.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// Process file includes in a YAML configuration.
///
/// Reads the `include` key from the config, loads each referenced file,
/// and deep-merges them in order (later files override earlier ones).
/// The base config values override all included values.
pub fn process_includes(
    base_path: &Path,
    base_value: &mut serde_yaml::Value,
) -> Result<(), IncludeError> {
    let mut visited = HashSet::new();
    visited.insert(base_path.canonicalize().unwrap_or_else(|_| base_path.to_path_buf()));
    process_includes_recursive(base_path, base_value, &mut visited, 0)
}

/// Process includes recursively with cycle detection
fn process_includes_recursive(
    base_path: &Path,
    value: &mut serde_yaml::Value,
    visited: &mut HashSet<PathBuf>,
    depth: usize,
) -> Result<(), IncludeError> {
    const MAX_DEPTH: usize = 10;

    if depth > MAX_DEPTH {
        return Err(IncludeError::MaxDepthExceeded(MAX_DEPTH));
    }

    // Extract include paths
    let include_paths = extract_include_paths(value);

    if include_paths.is_empty() {
        return Ok(());
    }

    let base_dir = base_path.parent().unwrap_or_else(|| Path::new("."));

    // Load and merge each included file
    for include_path_str in &include_paths {
        let include_path = resolve_include_path(base_dir, include_path_str);

        let canonical = include_path.canonicalize().map_err(|e| IncludeError::IoError {
            path: include_path.display().to_string(),
            source: e,
        })?;

        // Cycle detection
        if visited.contains(&canonical) {
            return Err(IncludeError::CycleDetected(canonical.display().to_string()));
        }
        visited.insert(canonical.clone());

        // Read and parse included file
        let content =
            std::fs::read_to_string(&include_path).map_err(|e| IncludeError::IoError {
                path: include_path.display().to_string(),
                source: e,
            })?;

        let mut included_value: serde_yaml::Value =
            serde_yaml::from_str(&content).map_err(|e| IncludeError::ParseError {
                path: include_path.display().to_string(),
                msg: e.to_string(),
            })?;

        // Process nested includes in the included file
        process_includes_recursive(&include_path, &mut included_value, visited, depth + 1)?;

        // Deep merge: included values are the base, current values override
        deep_merge(&mut included_value, value);
        *value = included_value;
    }

    // Remove the 'include' key from final output
    if let serde_yaml::Value::Mapping(map) = value {
        map.remove(&serde_yaml::Value::String("include".to_string()));
    }

    Ok(())
}

/// Extract include paths from YAML value
fn extract_include_paths(value: &serde_yaml::Value) -> Vec<String> {
    if let serde_yaml::Value::Mapping(map) = value {
        let include_key = serde_yaml::Value::String("include".to_string());
        if let Some(include_val) = map.get(&include_key) {
            match include_val {
                serde_yaml::Value::String(s) => return vec![s.clone()],
                serde_yaml::Value::Sequence(seq) => {
                    return seq
                        .iter()
                        .filter_map(|v| {
                            if let serde_yaml::Value::String(s) = v {
                                Some(s.clone())
                            } else {
                                None
                            }
                        })
                        .collect();
                }
                _ => {}
            }
        }
    }
    Vec::new()
}

/// Resolve an include path relative to the base directory
fn resolve_include_path(base_dir: &Path, include_path: &str) -> PathBuf {
    let path = PathBuf::from(include_path);
    if path.is_absolute() {
        path
    } else {
        base_dir.join(path)
    }
}

/// Deep merge two YAML values. Values from `override_val` take precedence.
pub fn deep_merge(base: &mut serde_yaml::Value, override_val: &serde_yaml::Value) {
    match (base, override_val) {
        (serde_yaml::Value::Mapping(base_map), serde_yaml::Value::Mapping(override_map)) => {
            for (key, override_value) in override_map {
                if let Some(base_value) = base_map.get_mut(key) {
                    deep_merge(base_value, override_value);
                } else {
                    base_map.insert(key.clone(), override_value.clone());
                }
            }
        }
        (base, override_val) => {
            *base = override_val.clone();
        }
    }
}

/// Include processing errors
#[derive(Debug, thiserror::Error)]
pub enum IncludeError {
    #[error("Failed to read include file '{path}': {source}")]
    IoError {
        path: String,
        #[source]
        source: std::io::Error,
    },

    #[error("Failed to parse include file '{path}': {msg}")]
    ParseError { path: String, msg: String },

    #[error("Circular include detected: {0}")]
    CycleDetected(String),

    #[error("Maximum include depth ({0}) exceeded")]
    MaxDepthExceeded(usize),
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_deep_merge_mappings() {
        let mut base: serde_yaml::Value = serde_yaml::from_str(
            r#"
gateway:
  host: "0.0.0.0"
  port: 31337
llm:
  provider: ollama
"#,
        )
        .unwrap();

        let override_val: serde_yaml::Value = serde_yaml::from_str(
            r#"
gateway:
  port: 8080
  new_field: true
"#,
        )
        .unwrap();

        deep_merge(&mut base, &override_val);

        let map = base.as_mapping().unwrap();
        let gw = map
            .get(&serde_yaml::Value::String("gateway".to_string()))
            .unwrap()
            .as_mapping()
            .unwrap();

        // Overridden value
        assert_eq!(
            gw.get(&serde_yaml::Value::String("port".to_string())),
            Some(&serde_yaml::Value::Number(8080.into()))
        );
        // Preserved base value
        assert_eq!(
            gw.get(&serde_yaml::Value::String("host".to_string())),
            Some(&serde_yaml::Value::String("0.0.0.0".to_string()))
        );
        // New field added
        assert!(gw.get(&serde_yaml::Value::String("new_field".to_string())).is_some());
        // Preserved section from base
        assert!(map.get(&serde_yaml::Value::String("llm".to_string())).is_some());
    }

    #[test]
    fn test_process_includes() {
        let dir = tempdir().unwrap();

        // Create base config
        let base_path = dir.path().join("config.yaml");
        fs::write(
            &base_path,
            r#"
include:
  - secrets.yaml
gateway:
  port: 31337
"#,
        )
        .unwrap();

        // Create included file
        fs::write(
            dir.path().join("secrets.yaml"),
            r#"
llm:
  api_key: "sk-secret"
gateway:
  host: "127.0.0.1"
"#,
        )
        .unwrap();

        let content = fs::read_to_string(&base_path).unwrap();
        let mut value: serde_yaml::Value = serde_yaml::from_str(&content).unwrap();
        process_includes(&base_path, &mut value).unwrap();

        let map = value.as_mapping().unwrap();

        // Include key should be removed
        assert!(!map.contains_key(&serde_yaml::Value::String("include".to_string())));

        // Gateway port from base should override
        let gw = map
            .get(&serde_yaml::Value::String("gateway".to_string()))
            .unwrap()
            .as_mapping()
            .unwrap();
        assert_eq!(
            gw.get(&serde_yaml::Value::String("port".to_string())),
            Some(&serde_yaml::Value::Number(31337.into()))
        );

        // Host from included file should be present
        assert_eq!(
            gw.get(&serde_yaml::Value::String("host".to_string())),
            Some(&serde_yaml::Value::String("127.0.0.1".to_string()))
        );

        // LLM section from included file
        assert!(map.get(&serde_yaml::Value::String("llm".to_string())).is_some());
    }

    #[test]
    fn test_cycle_detection() {
        let dir = tempdir().unwrap();

        let a_path = dir.path().join("a.yaml");
        let b_path = dir.path().join("b.yaml");

        fs::write(&a_path, "include:\n  - b.yaml\nfoo: 1\n").unwrap();
        fs::write(&b_path, "include:\n  - a.yaml\nbar: 2\n").unwrap();

        let content = fs::read_to_string(&a_path).unwrap();
        let mut value: serde_yaml::Value = serde_yaml::from_str(&content).unwrap();
        let result = process_includes(&a_path, &mut value);
        assert!(result.is_err());
    }

    #[test]
    fn test_extract_include_paths_string() {
        let value: serde_yaml::Value =
            serde_yaml::from_str("include: secrets.yaml\nfoo: bar\n").unwrap();
        let paths = extract_include_paths(&value);
        assert_eq!(paths, vec!["secrets.yaml"]);
    }

    #[test]
    fn test_extract_include_paths_list() {
        let value: serde_yaml::Value =
            serde_yaml::from_str("include:\n  - a.yaml\n  - b.yaml\nfoo: bar\n").unwrap();
        let paths = extract_include_paths(&value);
        assert_eq!(paths, vec!["a.yaml", "b.yaml"]);
    }
}
