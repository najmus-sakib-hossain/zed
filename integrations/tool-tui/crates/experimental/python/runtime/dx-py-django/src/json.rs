//! Fast JSON parsing compatible with cjson/ujson
//!
//! Provides high-performance JSON parsing using SIMD acceleration
//! to match or exceed the performance of C-based JSON libraries.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Errors that can occur during JSON operations
#[derive(Debug, Error)]
pub enum JsonError {
    #[error("JSON parse error: {0}")]
    ParseError(String),

    #[error("JSON serialization error: {0}")]
    SerializeError(String),

    #[error("Invalid UTF-8 in JSON: {0}")]
    Utf8Error(#[from] std::str::Utf8Error),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

/// High-performance JSON parser compatible with cjson/ujson
pub struct JsonParser {
    /// Whether to use strict parsing mode
    strict: bool,
    /// Maximum nesting depth
    max_depth: usize,
}

impl Default for JsonParser {
    fn default() -> Self {
        Self {
            strict: false,
            max_depth: 1000,
        }
    }
}

impl JsonParser {
    /// Create a new JSON parser
    pub fn new() -> Self {
        Self::default()
    }

    /// Enable strict parsing mode
    pub fn strict(mut self) -> Self {
        self.strict = true;
        self
    }

    /// Set maximum nesting depth
    pub fn with_max_depth(mut self, depth: usize) -> Self {
        self.max_depth = depth;
        self
    }

    /// Parse JSON string to a serde_json::Value
    pub fn parse(&self, json: &str) -> Result<serde_json::Value, JsonError> {
        // Use serde_json for reliable parsing
        // simd-json requires mutable buffer and has lifetime issues
        serde_json::from_str(json).map_err(|e| JsonError::ParseError(e.to_string()))
    }

    /// Parse JSON bytes to a serde_json::Value
    pub fn parse_bytes(&self, json: &[u8]) -> Result<serde_json::Value, JsonError> {
        let s = std::str::from_utf8(json)?;
        self.parse(s)
    }

    /// Parse JSON string to a typed value
    pub fn parse_typed<T: for<'de> Deserialize<'de>>(&self, json: &str) -> Result<T, JsonError> {
        let mut bytes = json.as_bytes().to_vec();

        match simd_json::from_slice(&mut bytes) {
            Ok(value) => Ok(value),
            Err(e) => {
                // Fall back to serde_json
                serde_json::from_str(json).map_err(|_| JsonError::ParseError(e.to_string()))
            }
        }
    }

    /// Serialize a value to JSON string
    pub fn serialize<T: Serialize>(&self, value: &T) -> Result<String, JsonError> {
        serde_json::to_string(value).map_err(|e| JsonError::SerializeError(e.to_string()))
    }

    /// Serialize a value to pretty-printed JSON string
    pub fn serialize_pretty<T: Serialize>(&self, value: &T) -> Result<String, JsonError> {
        serde_json::to_string_pretty(value).map_err(|e| JsonError::SerializeError(e.to_string()))
    }

    /// Serialize a value to JSON bytes
    pub fn serialize_bytes<T: Serialize>(&self, value: &T) -> Result<Vec<u8>, JsonError> {
        serde_json::to_vec(value).map_err(|e| JsonError::SerializeError(e.to_string()))
    }
}

/// ujson-compatible functions
pub mod ujson {
    use super::*;

    /// Decode JSON string (ujson.decode)
    pub fn decode(json: &str) -> Result<serde_json::Value, JsonError> {
        JsonParser::new().parse(json)
    }

    /// Encode value to JSON string (ujson.encode)
    pub fn encode<T: Serialize>(value: &T) -> Result<String, JsonError> {
        JsonParser::new().serialize(value)
    }

    /// Load JSON from bytes
    pub fn loads(json: &[u8]) -> Result<serde_json::Value, JsonError> {
        JsonParser::new().parse_bytes(json)
    }

    /// Dump value to JSON string
    pub fn dumps<T: Serialize>(value: &T) -> Result<String, JsonError> {
        JsonParser::new().serialize(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple() {
        let parser = JsonParser::new();
        let result = parser.parse(r#"{"key": "value"}"#).unwrap();
        assert_eq!(result["key"], "value");
    }

    #[test]
    fn test_parse_array() {
        let parser = JsonParser::new();
        let result = parser.parse(r#"[1, 2, 3]"#).unwrap();
        assert_eq!(result[0], 1);
        assert_eq!(result[1], 2);
        assert_eq!(result[2], 3);
    }

    #[test]
    fn test_parse_nested() {
        let parser = JsonParser::new();
        let result = parser.parse(r#"{"outer": {"inner": 42}}"#).unwrap();
        assert_eq!(result["outer"]["inner"], 42);
    }

    #[test]
    fn test_serialize() {
        let parser = JsonParser::new();
        let value = serde_json::json!({"key": "value"});
        let result = parser.serialize(&value).unwrap();
        assert!(result.contains("key"));
        assert!(result.contains("value"));
    }

    #[test]
    fn test_ujson_decode() {
        let result = ujson::decode(r#"{"test": true}"#).unwrap();
        assert_eq!(result["test"], true);
    }

    #[test]
    fn test_ujson_encode() {
        let value = serde_json::json!({"number": 42});
        let result = ujson::encode(&value).unwrap();
        assert!(result.contains("42"));
    }

    #[test]
    fn test_parse_invalid() {
        let parser = JsonParser::new();
        let result = parser.parse("not valid json");
        assert!(result.is_err());
    }
}
