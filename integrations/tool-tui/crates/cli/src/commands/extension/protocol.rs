//! JSON-RPC protocol for VS Code extension communication

use std::collections::HashMap;

/// JSON-RPC version
pub const JSONRPC_VERSION: &str = "2.0";

/// Message header
pub const CONTENT_LENGTH: &str = "Content-Length";

/// JSON-RPC request
#[derive(Debug, Clone)]
pub struct Request {
    pub jsonrpc: String,
    pub id: RequestId,
    pub method: String,
    pub params: Option<Params>,
}

/// JSON-RPC response
#[derive(Debug, Clone)]
pub struct Response {
    pub jsonrpc: String,
    pub id: RequestId,
    pub result: Option<Value>,
    pub error: Option<ResponseError>,
}

/// JSON-RPC notification
#[derive(Debug, Clone)]
pub struct Notification {
    pub jsonrpc: String,
    pub method: String,
    pub params: Option<Params>,
}

/// Request ID
#[derive(Debug, Clone)]
pub enum RequestId {
    Number(i64),
    String(String),
}

/// Parameter value
#[derive(Debug, Clone)]
pub enum Params {
    Array(Vec<Value>),
    Object(HashMap<String, Value>),
}

/// JSON value
#[derive(Debug, Clone)]
pub enum Value {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<Value>),
    Object(HashMap<String, Value>),
}

/// Response error
#[derive(Debug, Clone)]
pub struct ResponseError {
    pub code: i32,
    pub message: String,
    pub data: Option<Value>,
}

/// Standard error codes
pub mod error_codes {
    pub const PARSE_ERROR: i32 = -32700;
    pub const INVALID_REQUEST: i32 = -32600;
    pub const METHOD_NOT_FOUND: i32 = -32601;
    pub const INVALID_PARAMS: i32 = -32602;
    pub const INTERNAL_ERROR: i32 = -32603;

    // Custom error codes
    pub const CHECK_FAILED: i32 = -32001;
    pub const PROJECT_NOT_FOUND: i32 = -32002;
    pub const WATCH_FAILED: i32 = -32003;
}

/// Protocol methods
pub mod methods {
    pub const CHECK_RUN: &str = "dx/check/run";
    pub const CHECK_CANCEL: &str = "dx/check/cancel";
    pub const SCORE_GET: &str = "dx/score/get";
    pub const WATCH_START: &str = "dx/watch/start";
    pub const WATCH_STOP: &str = "dx/watch/stop";

    // Notifications (server -> client)
    pub const DIAGNOSTICS_PUBLISH: &str = "dx/diagnostics/publish";
    pub const SCORE_UPDATE: &str = "dx/score/update";
    pub const STATUS_UPDATE: &str = "dx/status/update";
}

/// Protocol encoder/decoder
pub struct Protocol;

impl Protocol {
    /// Encode message with header
    pub fn encode(content: &[u8]) -> Vec<u8> {
        let header = format!("{}: {}\r\n\r\n", CONTENT_LENGTH, content.len());
        let mut result = header.into_bytes();
        result.extend_from_slice(content);
        result
    }

    /// Decode message from buffer, returns (message, consumed_bytes)
    pub fn decode(buffer: &[u8]) -> Option<(Vec<u8>, usize)> {
        // Find header end
        let header_end = find_header_end(buffer)?;

        // Parse content length
        let header = std::str::from_utf8(&buffer[..header_end]).ok()?;
        let content_length = parse_content_length(header)?;

        // Check if we have full message
        let message_start = header_end + 4; // Skip \r\n\r\n
        let message_end = message_start + content_length;

        if buffer.len() < message_end {
            return None;
        }

        let message = buffer[message_start..message_end].to_vec();
        Some((message, message_end))
    }
}

fn find_header_end(buffer: &[u8]) -> Option<usize> {
    for i in 0..buffer.len().saturating_sub(3) {
        if &buffer[i..i + 4] == b"\r\n\r\n" {
            return Some(i);
        }
    }
    None
}

fn parse_content_length(header: &str) -> Option<usize> {
    for line in header.lines() {
        if let Some(value) = line.strip_prefix("Content-Length: ") {
            return value.trim().parse().ok();
        }
    }
    None
}

/// Request builder
pub struct RequestBuilder {
    id: RequestId,
    method: String,
    params: Option<Params>,
}

impl RequestBuilder {
    pub fn new(method: impl Into<String>) -> Self {
        Self {
            id: RequestId::Number(1),
            method: method.into(),
            params: None,
        }
    }

    pub fn id_num(mut self, id: i64) -> Self {
        self.id = RequestId::Number(id);
        self
    }

    pub fn id_str(mut self, id: impl Into<String>) -> Self {
        self.id = RequestId::String(id.into());
        self
    }

    pub fn params(mut self, params: Params) -> Self {
        self.params = Some(params);
        self
    }

    pub fn build(self) -> Request {
        Request {
            jsonrpc: JSONRPC_VERSION.to_string(),
            id: self.id,
            method: self.method,
            params: self.params,
        }
    }
}

/// Response builder
pub struct ResponseBuilder {
    id: RequestId,
    result: Option<Value>,
    error: Option<ResponseError>,
}

impl ResponseBuilder {
    pub fn success(id: RequestId, result: Value) -> Response {
        Response {
            jsonrpc: JSONRPC_VERSION.to_string(),
            id,
            result: Some(result),
            error: None,
        }
    }

    pub fn error(id: RequestId, code: i32, message: impl Into<String>) -> Response {
        Response {
            jsonrpc: JSONRPC_VERSION.to_string(),
            id,
            result: None,
            error: Some(ResponseError {
                code,
                message: message.into(),
                data: None,
            }),
        }
    }
}

/// Notification builder
pub struct NotificationBuilder;

impl NotificationBuilder {
    pub fn new(method: impl Into<String>, params: Option<Params>) -> Notification {
        Notification {
            jsonrpc: JSONRPC_VERSION.to_string(),
            method: method.into(),
            params,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_decode() {
        let content = b"test message";
        let encoded = Protocol::encode(content);

        let (decoded, consumed) = Protocol::decode(&encoded).unwrap();
        assert_eq!(decoded, content);
        assert_eq!(consumed, encoded.len());
    }

    #[test]
    fn test_parse_content_length() {
        let header = "Content-Length: 123\r\n";
        assert_eq!(parse_content_length(header), Some(123));
    }
}
