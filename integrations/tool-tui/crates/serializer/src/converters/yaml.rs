/// YAML to DX ULTRA converter
use crate::converters::json::json_to_dx;

/// Convert YAML string to DX ULTRA format
///
/// Strategy: Convert YAML → JSON → DX
/// This leverages existing JSON converter with optimization
pub fn yaml_to_dx(yaml_str: &str) -> Result<String, String> {
    // Parse YAML
    let value: serde_json::Value =
        serde_yaml::from_str(yaml_str).map_err(|e| format!("YAML parse error: {}", e))?;

    // Convert to JSON string
    let json_str =
        serde_json::to_string(&value).map_err(|e| format!("JSON conversion error: {}", e))?;

    // Use JSON converter
    json_to_dx(&json_str)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_yaml_to_dx() {
        let yaml = r#"
name: test
version: 1.0.0
items:
  - a
  - b
  - c
"#;
        let dx = yaml_to_dx(yaml).unwrap();
        assert!(dx.contains("n:test"));
    }
}
