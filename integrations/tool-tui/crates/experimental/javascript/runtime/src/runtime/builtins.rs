//! Built-in functions (console, JSON, etc.)

use crate::value::object::Object;
use crate::value::Value;

/// JavaScript error types
#[derive(Debug, Clone)]
pub enum JsErrorType {
    TypeError,
    SyntaxError,
    ReferenceError,
    RangeError,
}

/// JavaScript error with type and message
#[derive(Debug, Clone)]
pub struct JsError {
    pub error_type: JsErrorType,
    pub message: String,
    pub line: Option<u32>,
    pub column: Option<u32>,
}

impl JsError {
    pub fn type_error(message: impl Into<String>) -> Self {
        Self {
            error_type: JsErrorType::TypeError,
            message: message.into(),
            line: None,
            column: None,
        }
    }

    pub fn syntax_error(
        message: impl Into<String>,
        line: Option<u32>,
        column: Option<u32>,
    ) -> Self {
        Self {
            error_type: JsErrorType::SyntaxError,
            message: message.into(),
            line,
            column,
        }
    }

    pub fn reference_error(message: impl Into<String>) -> Self {
        Self {
            error_type: JsErrorType::ReferenceError,
            message: message.into(),
            line: None,
            column: None,
        }
    }

    pub fn range_error(message: impl Into<String>) -> Self {
        Self {
            error_type: JsErrorType::RangeError,
            message: message.into(),
            line: None,
            column: None,
        }
    }
}

impl std::fmt::Display for JsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let type_name = match self.error_type {
            JsErrorType::TypeError => "TypeError",
            JsErrorType::SyntaxError => "SyntaxError",
            JsErrorType::ReferenceError => "ReferenceError",
            JsErrorType::RangeError => "RangeError",
        };

        if let (Some(line), Some(col)) = (self.line, self.column) {
            write!(f, "{}: {} at line {}, column {}", type_name, self.message, line, col)
        } else {
            write!(f, "{}: {}", type_name, self.message)
        }
    }
}

/// Console.log implementation
pub fn console_log(args: &[Value]) {
    for (i, arg) in args.iter().enumerate() {
        if i > 0 {
            print!(" ");
        }
        print!("{}", arg);
    }
    println!();
}

/// JSON.parse implementation with proper error handling
pub fn json_parse(s: &str) -> Result<Value, JsError> {
    let s = s.trim();

    if s.is_empty() {
        return Err(JsError::syntax_error("Unexpected end of JSON input", Some(1), Some(1)));
    }

    if s == "null" {
        return Ok(Value::Null);
    }
    if s == "true" {
        return Ok(Value::Boolean(true));
    }
    if s == "false" {
        return Ok(Value::Boolean(false));
    }

    // Try parsing as number
    if let Ok(n) = s.parse::<f64>() {
        return Ok(Value::Number(n));
    }

    // Try parsing as string
    if s.starts_with('"') && s.ends_with('"') && s.len() >= 2 {
        let inner = &s[1..s.len() - 1];
        // Check for invalid escape sequences
        let mut chars = inner.chars().peekable();
        let mut result = String::new();
        let mut col = 2u32; // Start after opening quote

        while let Some(c) = chars.next() {
            col += 1;
            if c == '\\' {
                match chars.next() {
                    Some('n') => result.push('\n'),
                    Some('r') => result.push('\r'),
                    Some('t') => result.push('\t'),
                    Some('\\') => result.push('\\'),
                    Some('"') => result.push('"'),
                    Some('/') => result.push('/'),
                    Some('u') => {
                        // Unicode escape
                        let hex: String = chars.by_ref().take(4).collect();
                        if hex.len() != 4 {
                            return Err(JsError::syntax_error(
                                format!("Invalid Unicode escape sequence at position {}", col),
                                Some(1),
                                Some(col),
                            ));
                        }
                        match u32::from_str_radix(&hex, 16) {
                            Ok(code) => {
                                if let Some(ch) = char::from_u32(code) {
                                    result.push(ch);
                                } else {
                                    return Err(JsError::syntax_error(
                                        format!("Invalid Unicode code point at position {}", col),
                                        Some(1),
                                        Some(col),
                                    ));
                                }
                            }
                            Err(_) => {
                                return Err(JsError::syntax_error(
                                    format!("Invalid Unicode escape sequence at position {}", col),
                                    Some(1),
                                    Some(col),
                                ));
                            }
                        }
                    }
                    Some(c) => {
                        return Err(JsError::syntax_error(
                            format!("Invalid escape character '\\{}' at position {}", c, col),
                            Some(1),
                            Some(col),
                        ));
                    }
                    None => {
                        return Err(JsError::syntax_error(
                            "Unexpected end of string",
                            Some(1),
                            Some(col),
                        ));
                    }
                }
            } else if c.is_control() && c != '\t' {
                return Err(JsError::syntax_error(
                    format!("Invalid control character at position {}", col),
                    Some(1),
                    Some(col),
                ));
            } else {
                result.push(c);
            }
        }

        return Ok(Value::String(result));
    }

    // Try parsing as array
    if s.starts_with('[') && s.ends_with(']') {
        // Simplified array parsing
        let inner = s[1..s.len() - 1].trim();
        if inner.is_empty() {
            return Ok(Value::Array(vec![]));
        }

        // Split by comma (simplified - doesn't handle nested structures)
        let elements: Result<Vec<Value>, JsError> =
            inner.split(',').map(|elem| json_parse(elem.trim())).collect();

        return Ok(Value::Array(elements?));
    }

    // Try parsing as object
    if s.starts_with('{') && s.ends_with('}') {
        let inner = s[1..s.len() - 1].trim();
        if inner.is_empty() {
            return Ok(Value::Object(Object::new()));
        }

        // Simplified object parsing
        let mut obj = Object::new();

        // Very basic key-value parsing
        for pair in inner.split(',') {
            let pair = pair.trim();
            if let Some(colon_pos) = pair.find(':') {
                let key = pair[..colon_pos].trim();
                let value = pair[colon_pos + 1..].trim();

                // Key must be a string
                if key.starts_with('"') && key.ends_with('"') && key.len() >= 2 {
                    let key_str = key[1..key.len() - 1].to_string();
                    let parsed_value = json_parse(value)?;
                    obj.set(key_str, parsed_value);
                } else {
                    return Err(JsError::syntax_error(
                        "Expected property name in JSON at position 1".to_string(),
                        Some(1),
                        Some(1),
                    ));
                }
            } else {
                return Err(JsError::syntax_error(
                    "Expected ':' after property name",
                    Some(1),
                    Some(1),
                ));
            }
        }

        return Ok(Value::Object(obj));
    }

    // Find the position of the error
    let error_char = s.chars().next().unwrap_or(' ');
    Err(JsError::syntax_error(
        format!("Unexpected token '{}' in JSON at position 0", error_char),
        Some(1),
        Some(1),
    ))
}

/// JSON.stringify implementation
pub fn json_stringify(value: &Value) -> String {
    match value {
        Value::Null => "null".to_string(),
        Value::Undefined => "undefined".to_string(),
        Value::Boolean(b) => b.to_string(),
        Value::Number(n) => {
            if n.is_nan() || n.is_infinite() {
                "null".to_string()
            } else {
                n.to_string()
            }
        }
        Value::String(s) => {
            let escaped = s
                .replace('\\', "\\\\")
                .replace('"', "\\\"")
                .replace('\n', "\\n")
                .replace('\r', "\\r")
                .replace('\t', "\\t");
            format!("\"{}\"", escaped)
        }
        Value::Object(obj) => {
            let pairs: Vec<String> = obj
                .entries()
                .iter()
                .map(|(k, v)| format!("\"{}\":{}", k, json_stringify(v)))
                .collect();
            format!("{{{}}}", pairs.join(","))
        }
        Value::Array(arr) => {
            let elements: Vec<String> = arr.iter().map(json_stringify).collect();
            format!("[{}]", elements.join(","))
        }
        Value::Function(_) => "null".to_string(), // Functions are omitted in JSON
        Value::Promise(_) => "[object Promise]".to_string(),
        Value::Integer(i) => i.to_string(),
        Value::Symbol(_) => "null".to_string(), // Symbols are omitted in JSON
        Value::BigInt(b) => format!("\"{}\"", b), // BigInt as string
    }
}

/// Object.keys implementation with proper error handling
pub fn object_keys(value: &Value) -> Result<Vec<String>, JsError> {
    match value {
        Value::Object(obj) => Ok(obj.keys_owned()),
        Value::Array(arr) => Ok((0..arr.len()).map(|i| i.to_string()).collect()),
        Value::Null => Err(JsError::type_error("Cannot convert null to object")),
        Value::Undefined => Err(JsError::type_error("Cannot convert undefined to object")),
        other => Err(JsError::type_error(format!(
            "Object.keys called on non-object (received {})",
            other.type_name()
        ))),
    }
}

/// Object.values implementation with proper error handling
pub fn object_values(value: &Value) -> Result<Vec<Value>, JsError> {
    match value {
        Value::Object(obj) => Ok(obj.values_cloned()),
        Value::Array(arr) => Ok(arr.clone()),
        Value::Null => Err(JsError::type_error("Cannot convert null to object")),
        Value::Undefined => Err(JsError::type_error("Cannot convert undefined to object")),
        other => Err(JsError::type_error(format!(
            "Object.values called on non-object (received {})",
            other.type_name()
        ))),
    }
}

/// Object.entries implementation with proper error handling
pub fn object_entries(value: &Value) -> Result<Vec<(String, Value)>, JsError> {
    match value {
        Value::Object(obj) => Ok(obj.entries_cloned()),
        Value::Array(arr) => {
            Ok(arr.iter().enumerate().map(|(i, v)| (i.to_string(), v.clone())).collect())
        }
        Value::Null => Err(JsError::type_error("Cannot convert null to object")),
        Value::Undefined => Err(JsError::type_error("Cannot convert undefined to object")),
        other => Err(JsError::type_error(format!(
            "Object.entries called on non-object (received {})",
            other.type_name()
        ))),
    }
}

/// Array.isArray implementation
pub fn array_is_array(value: &Value) -> bool {
    matches!(value, Value::Array(_))
}
