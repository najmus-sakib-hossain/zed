//! Format Converter
//!
//! Converts between JSON and DX serialization format for token optimization.

use super::TokenError;

/// Format converter for JSON ↔ DX format
pub struct FormatConverter {
    /// Whether conversion is enabled
    enabled: bool,
}

impl FormatConverter {
    /// Create a new format converter
    pub fn new(enabled: bool) -> Self {
        Self { enabled }
    }

    /// Convert JSON to DX format
    pub fn convert_to_dx(&self, content: &str) -> Result<String, TokenError> {
        if !self.enabled {
            return Ok(content.to_string());
        }

        // Detect if content is JSON
        let trimmed = content.trim();
        if trimmed.starts_with('{') || trimmed.starts_with('[') {
            match self.json_to_dx(trimmed) {
                Ok(dx) => Ok(dx),
                Err(_) => Ok(content.to_string()), // Fall back to original if conversion fails
            }
        } else {
            Ok(content.to_string())
        }
    }

    /// Convert DX format to JSON
    pub fn convert_to_json(&self, content: &str) -> Result<String, TokenError> {
        // Detect if content is DX format (simplified check)
        if content.contains(" = ") || content.contains(": ") && !content.contains("\":") {
            self.dx_to_json(content)
        } else {
            Ok(content.to_string())
        }
    }

    /// Convert JSON string to DX format
    fn json_to_dx(&self, json: &str) -> Result<String, TokenError> {
        let value: serde_json::Value = serde_json::from_str(json)
            .map_err(|e| TokenError::ConversionFailed(format!("Invalid JSON: {}", e)))?;

        Ok(self.value_to_dx(&value, 0))
    }

    /// Convert serde_json::Value to DX format
    fn value_to_dx(&self, value: &serde_json::Value, indent: usize) -> String {
        let indent_str = "  ".repeat(indent);

        match value {
            serde_json::Value::Null => "null".to_string(),
            serde_json::Value::Bool(b) => b.to_string(),
            serde_json::Value::Number(n) => n.to_string(),
            serde_json::Value::String(s) => {
                // Use quotes only if necessary
                if s.contains(' ') || s.contains('\n') || s.is_empty() {
                    format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
                } else {
                    s.clone()
                }
            }
            serde_json::Value::Array(arr) => {
                if arr.is_empty() {
                    "[]".to_string()
                } else if arr.len() <= 3 && arr.iter().all(|v| v.is_string() || v.is_number()) {
                    // Inline short arrays
                    let items: Vec<String> = arr.iter().map(|v| self.value_to_dx(v, 0)).collect();
                    format!("[{}]", items.join(", "))
                } else {
                    let items: Vec<String> = arr
                        .iter()
                        .map(|v| format!("{}  {}", indent_str, self.value_to_dx(v, indent + 1)))
                        .collect();
                    format!("[\n{}\n{}]", items.join("\n"), indent_str)
                }
            }
            serde_json::Value::Object(obj) => {
                if obj.is_empty() {
                    "{}".to_string()
                } else {
                    let items: Vec<String> = obj
                        .iter()
                        .map(|(k, v)| {
                            let value_str = self.value_to_dx(v, indent + 1);
                            format!("{}{} = {}", indent_str, k, value_str)
                        })
                        .collect();
                    items.join("\n")
                }
            }
        }
    }

    /// Convert DX format to JSON
    fn dx_to_json(&self, dx: &str) -> Result<String, TokenError> {
        let value = self.parse_dx(dx)?;
        serde_json::to_string_pretty(&value)
            .map_err(|e| TokenError::ConversionFailed(format!("JSON serialization failed: {}", e)))
    }

    /// Parse DX format into serde_json::Value
    fn parse_dx(&self, dx: &str) -> Result<serde_json::Value, TokenError> {
        let mut obj = serde_json::Map::new();

        for line in dx.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with("//") {
                continue;
            }

            // Parse key = value pairs
            if let Some(eq_pos) = trimmed.find(" = ") {
                let key = trimmed[..eq_pos].trim();
                let value = trimmed[eq_pos + 3..].trim();
                obj.insert(key.to_string(), self.parse_dx_value(value)?);
            }
        }

        Ok(serde_json::Value::Object(obj))
    }

    /// Parse a DX value
    fn parse_dx_value(&self, value: &str) -> Result<serde_json::Value, TokenError> {
        let trimmed = value.trim();

        // Null
        if trimmed == "null" {
            return Ok(serde_json::Value::Null);
        }

        // Boolean
        if trimmed == "true" {
            return Ok(serde_json::Value::Bool(true));
        }
        if trimmed == "false" {
            return Ok(serde_json::Value::Bool(false));
        }

        // Number
        if let Ok(n) = trimmed.parse::<i64>() {
            return Ok(serde_json::Value::Number(n.into()));
        }
        if let Ok(n) = trimmed.parse::<f64>() {
            if let Some(num) = serde_json::Number::from_f64(n) {
                return Ok(serde_json::Value::Number(num));
            }
        }

        // Array
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            let inner = &trimmed[1..trimmed.len() - 1];
            if inner.is_empty() {
                return Ok(serde_json::Value::Array(vec![]));
            }
            let items: Result<Vec<serde_json::Value>, _> =
                inner.split(',').map(|s| self.parse_dx_value(s.trim())).collect();
            return Ok(serde_json::Value::Array(items?));
        }

        // String (quoted or unquoted)
        if trimmed.starts_with('"') && trimmed.ends_with('"') {
            let inner = &trimmed[1..trimmed.len() - 1];
            return Ok(serde_json::Value::String(
                inner.replace("\\\"", "\"").replace("\\\\", "\\"),
            ));
        }

        // Unquoted string
        Ok(serde_json::Value::String(trimmed.to_string()))
    }

    /// Calculate token savings
    pub fn calculate_savings(&self, original_json: &str, dx: &str) -> TokenSavings {
        let original_tokens = estimate_tokens(original_json);
        let dx_tokens = estimate_tokens(dx);
        let saved = original_tokens.saturating_sub(dx_tokens);
        let percentage = if original_tokens > 0 {
            (saved as f32 / original_tokens as f32) * 100.0
        } else {
            0.0
        };

        TokenSavings {
            original_tokens,
            converted_tokens: dx_tokens,
            tokens_saved: saved,
            savings_percentage: percentage,
        }
    }
}

/// Token savings information
#[derive(Debug, Clone)]
pub struct TokenSavings {
    /// Original token count
    pub original_tokens: usize,
    /// Converted token count
    pub converted_tokens: usize,
    /// Tokens saved
    pub tokens_saved: usize,
    /// Savings percentage
    pub savings_percentage: f32,
}

/// Estimate token count (simplified)
fn estimate_tokens(content: &str) -> usize {
    // Rough estimation: ~4 characters per token
    content.len() / 4
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_to_dx_simple() {
        let converter = FormatConverter::new(true);

        let json = r#"{"name": "test", "count": 42, "active": true}"#;
        let dx = converter.convert_to_dx(json).unwrap();

        assert!(dx.contains("name = "));
        assert!(dx.contains("count = 42"));
        assert!(dx.contains("active = true"));
    }

    #[test]
    fn test_json_to_dx_nested() {
        let converter = FormatConverter::new(true);

        let json = r#"{"user": {"name": "Alice", "age": 30}, "items": [1, 2, 3]}"#;
        let dx = converter.convert_to_dx(json).unwrap();

        assert!(dx.contains("items = [1, 2, 3]"));
    }

    #[test]
    fn test_dx_to_json() {
        let converter = FormatConverter::new(true);

        let dx = "name = test\ncount = 42\nactive = true";
        let json = converter.convert_to_json(dx).unwrap();

        assert!(json.contains("\"name\""));
        assert!(json.contains("\"test\""));
        assert!(json.contains("42"));
    }

    #[test]
    fn test_roundtrip() {
        let converter = FormatConverter::new(true);

        let original = r#"{"name": "test", "value": 123}"#;
        let dx = converter.convert_to_dx(original).unwrap();
        let json = converter.convert_to_json(&dx).unwrap();

        // Parse both as JSON and compare
        let orig_val: serde_json::Value = serde_json::from_str(original).unwrap();
        let round_val: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(orig_val, round_val);
    }

    #[test]
    fn test_disabled() {
        let converter = FormatConverter::new(false);

        let json = r#"{"name": "test"}"#;
        let result = converter.convert_to_dx(json).unwrap();

        assert_eq!(result, json);
    }

    #[test]
    fn test_token_savings() {
        let converter = FormatConverter::new(true);

        let json = r#"{
    "name": "test",
    "description": "A longer description here",
    "count": 42,
    "enabled": true
}"#;
        let dx = converter.convert_to_dx(json).unwrap();
        let savings = converter.calculate_savings(json, &dx);

        // DX format should be more compact
        assert!(savings.tokens_saved > 0);
        assert!(savings.savings_percentage > 0.0);
    }
}
