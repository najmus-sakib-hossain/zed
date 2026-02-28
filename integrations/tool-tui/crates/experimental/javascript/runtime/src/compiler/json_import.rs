//! JSON import support
//!
//! Handles importing JSON files as JavaScript modules.
//! Supports both static imports and dynamic imports.

use crate::error::{DxError, DxResult};
use crate::value::Value;
use std::path::Path;

/// JSON module loader
pub struct JsonLoader {
    /// Cached JSON modules
    cache: std::collections::HashMap<String, Value>,
}

impl JsonLoader {
    /// Create a new JSON loader
    pub fn new() -> Self {
        Self {
            cache: std::collections::HashMap::new(),
        }
    }

    /// Load a JSON file and convert to a Value
    pub fn load_file(&mut self, path: &Path) -> DxResult<Value> {
        let path_str = path.to_string_lossy().to_string();

        // Check cache
        if let Some(cached) = self.cache.get(&path_str) {
            return Ok(cached.clone());
        }

        // Read file
        let content = std::fs::read_to_string(path)
            .map_err(|e| DxError::RuntimeError(format!("Failed to read JSON file: {}", e)))?;

        // Parse JSON
        let value = self.parse_json(&content)?;

        // Cache and return
        self.cache.insert(path_str, value.clone());
        Ok(value)
    }

    /// Load JSON from a string
    pub fn load_string(&self, content: &str) -> DxResult<Value> {
        self.parse_json(content)
    }

    /// Parse JSON string into a Value
    fn parse_json(&self, content: &str) -> DxResult<Value> {
        let json: serde_json::Value = serde_json::from_str(content)
            .map_err(|e| DxError::RuntimeError(format!("JSON parse error: {}", e)))?;

        Ok(json_to_value(&json))
    }

    /// Clear the cache
    pub fn clear_cache(&mut self) {
        self.cache.clear();
    }
}

impl Default for JsonLoader {
    fn default() -> Self {
        Self::new()
    }
}

/// Convert serde_json::Value to our Value type
fn json_to_value(json: &serde_json::Value) -> Value {
    match json {
        serde_json::Value::Null => Value::Null,
        serde_json::Value::Bool(b) => Value::Boolean(*b),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Value::Number(i as f64)
            } else if let Some(f) = n.as_f64() {
                Value::Number(f)
            } else {
                Value::Number(f64::NAN)
            }
        }
        serde_json::Value::String(s) => Value::String(s.clone()),
        serde_json::Value::Array(arr) => {
            let values: Vec<Value> = arr.iter().map(json_to_value).collect();
            Value::Array(values)
        }
        serde_json::Value::Object(obj) => {
            let mut object = crate::value::object::Object::new();
            for (key, val) in obj {
                object.set(key.clone(), json_to_value(val));
            }
            Value::Object(object)
        }
    }
}

/// Convert our Value type to serde_json::Value (for serialization)
pub fn value_to_json(value: &Value) -> serde_json::Value {
    match value {
        Value::Undefined => serde_json::Value::Null,
        Value::Null => serde_json::Value::Null,
        Value::Boolean(b) => serde_json::Value::Bool(*b),
        Value::Number(n) => {
            if n.is_nan() || n.is_infinite() {
                serde_json::Value::Null
            } else {
                serde_json::json!(*n)
            }
        }
        Value::String(s) => serde_json::Value::String(s.clone()),
        Value::Array(arr) => {
            let values: Vec<serde_json::Value> = arr.iter().map(value_to_json).collect();
            serde_json::Value::Array(values)
        }
        Value::Object(obj) => {
            let mut map = serde_json::Map::new();
            for (key, val) in obj.entries() {
                map.insert(key.clone(), value_to_json(val));
            }
            serde_json::Value::Object(map)
        }
        // Functions and other types serialize to null
        _ => serde_json::Value::Null,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_null() {
        let loader = JsonLoader::new();
        let value = loader.load_string("null").unwrap();
        assert!(matches!(value, Value::Null));
    }

    #[test]
    fn test_parse_boolean() {
        let loader = JsonLoader::new();

        let value = loader.load_string("true").unwrap();
        assert!(matches!(value, Value::Boolean(true)));

        let value = loader.load_string("false").unwrap();
        assert!(matches!(value, Value::Boolean(false)));
    }

    #[test]
    fn test_parse_number() {
        let loader = JsonLoader::new();

        let value = loader.load_string("42").unwrap();
        if let Value::Number(n) = value {
            assert_eq!(n, 42.0);
        } else {
            panic!("Expected number");
        }

        let value = loader.load_string("1.234").unwrap();
        if let Value::Number(n) = value {
            assert!((n - 1.234).abs() < 0.001);
        } else {
            panic!("Expected number");
        }
    }

    #[test]
    fn test_parse_string() {
        let loader = JsonLoader::new();
        let value = loader.load_string("\"hello\"").unwrap();
        if let Value::String(s) = value {
            assert_eq!(s, "hello");
        } else {
            panic!("Expected string");
        }
    }

    #[test]
    fn test_parse_array() {
        let loader = JsonLoader::new();
        let value = loader.load_string("[1, 2, 3]").unwrap();
        if let Value::Array(arr) = value {
            assert_eq!(arr.len(), 3);
        } else {
            panic!("Expected array");
        }
    }

    #[test]
    fn test_parse_object() {
        let loader = JsonLoader::new();
        let value = loader.load_string("{\"name\": \"test\", \"value\": 42}").unwrap();
        if let Value::Object(obj) = value {
            assert_eq!(obj.len(), 2);
            assert!(obj.has_own("name"));
            assert!(obj.has_own("value"));
        } else {
            panic!("Expected object");
        }
    }

    #[test]
    fn test_round_trip() {
        let loader = JsonLoader::new();
        let original = r#"{"name":"test","numbers":[1,2,3],"nested":{"a":true}}"#;
        let value = loader.load_string(original).unwrap();
        let json = value_to_json(&value);

        // Verify structure is preserved
        assert!(json.is_object());
        assert_eq!(json["name"], "test");
        assert!(json["numbers"].is_array());
        assert!(json["nested"]["a"].as_bool().unwrap());
    }
}
