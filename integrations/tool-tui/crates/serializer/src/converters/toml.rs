/// TOML to DX ULTRA converter
use crate::converters::json::json_to_dx;

/// Convert TOML string to DX ULTRA format
///
/// Strategy: Convert TOML → JSON → DX
/// This leverages existing JSON converter with optimization
pub fn toml_to_dx(toml_str: &str) -> Result<String, String> {
    // Parse TOML
    let value: toml::Value =
        toml::from_str(toml_str).map_err(|e| format!("TOML parse error: {}", e))?;

    // Convert to serde_json::Value
    let json_str =
        serde_json::to_string(&value).map_err(|e| format!("JSON conversion error: {}", e))?;

    // Use JSON converter
    json_to_dx(&json_str)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_toml_to_dx() {
        let toml = r#"
name = "test"
version = "1.0.0"
items = ["a", "b", "c"]
"#;
        let dx = toml_to_dx(toml).unwrap();
        assert!(dx.contains("n:test"));
    }
}
