//! ASGI Protocol Support
//!
//! Provides ASGI (Asynchronous Server Gateway Interface) compatibility
//! for FastAPI applications.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

/// Errors that can occur during ASGI operations
#[derive(Debug, Error)]
pub enum AsgiError {
    #[error("Invalid ASGI scope: {0}")]
    InvalidScope(String),

    #[error("Invalid ASGI message: {0}")]
    InvalidMessage(String),

    #[error("Application error: {0}")]
    ApplicationError(String),

    #[error("Connection closed")]
    ConnectionClosed,

    #[error("Timeout")]
    Timeout,
}

/// ASGI scope types
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ScopeType {
    Http,
    Websocket,
    Lifespan,
}

/// ASGI scope - connection information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AsgiScope {
    /// Scope type (http, websocket, lifespan)
    #[serde(rename = "type")]
    pub scope_type: ScopeType,
    /// ASGI version
    pub asgi: AsgiVersion,
    /// HTTP version (for HTTP scopes)
    pub http_version: Option<String>,
    /// HTTP method (for HTTP scopes)
    pub method: Option<String>,
    /// URL scheme (http or https)
    pub scheme: Option<String>,
    /// Request path
    pub path: Option<String>,
    /// Raw path bytes
    pub raw_path: Option<Vec<u8>>,
    /// Query string
    pub query_string: Option<Vec<u8>>,
    /// Root path (for mounted apps)
    pub root_path: Option<String>,
    /// Request headers
    pub headers: Vec<(Vec<u8>, Vec<u8>)>,
    /// Server address (host, port)
    pub server: Option<(String, u16)>,
    /// Client address (host, port)
    pub client: Option<(String, u16)>,
    /// State dictionary
    pub state: HashMap<String, serde_json::Value>,
}

/// ASGI version information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AsgiVersion {
    pub version: String,
    pub spec_version: Option<String>,
}

impl Default for AsgiVersion {
    fn default() -> Self {
        Self {
            version: "3.0".to_string(),
            spec_version: Some("2.3".to_string()),
        }
    }
}

impl AsgiScope {
    /// Create a new HTTP scope
    pub fn http(method: impl Into<String>, path: impl Into<String>) -> Self {
        Self {
            scope_type: ScopeType::Http,
            asgi: AsgiVersion::default(),
            http_version: Some("1.1".to_string()),
            method: Some(method.into()),
            scheme: Some("http".to_string()),
            path: Some(path.into()),
            raw_path: None,
            query_string: Some(Vec::new()),
            root_path: Some(String::new()),
            headers: Vec::new(),
            server: Some(("localhost".to_string(), 8000)),
            client: Some(("127.0.0.1".to_string(), 0)),
            state: HashMap::new(),
        }
    }

    /// Create a new WebSocket scope
    pub fn websocket(path: impl Into<String>) -> Self {
        Self {
            scope_type: ScopeType::Websocket,
            asgi: AsgiVersion::default(),
            http_version: Some("1.1".to_string()),
            method: None,
            scheme: Some("ws".to_string()),
            path: Some(path.into()),
            raw_path: None,
            query_string: Some(Vec::new()),
            root_path: Some(String::new()),
            headers: Vec::new(),
            server: Some(("localhost".to_string(), 8000)),
            client: Some(("127.0.0.1".to_string(), 0)),
            state: HashMap::new(),
        }
    }

    /// Create a lifespan scope
    pub fn lifespan() -> Self {
        Self {
            scope_type: ScopeType::Lifespan,
            asgi: AsgiVersion::default(),
            http_version: None,
            method: None,
            scheme: None,
            path: None,
            raw_path: None,
            query_string: None,
            root_path: None,
            headers: Vec::new(),
            server: None,
            client: None,
            state: HashMap::new(),
        }
    }

    /// Add a header
    pub fn with_header(mut self, name: impl Into<Vec<u8>>, value: impl Into<Vec<u8>>) -> Self {
        self.headers.push((name.into(), value.into()));
        self
    }

    /// Set query string
    pub fn with_query(mut self, query: impl Into<Vec<u8>>) -> Self {
        self.query_string = Some(query.into());
        self
    }

    /// Set server address
    pub fn with_server(mut self, host: impl Into<String>, port: u16) -> Self {
        self.server = Some((host.into(), port));
        self
    }

    /// Set client address
    pub fn with_client(mut self, host: impl Into<String>, port: u16) -> Self {
        self.client = Some((host.into(), port));
        self
    }

    /// Get a header value by name (case-insensitive)
    pub fn get_header(&self, name: &[u8]) -> Option<&[u8]> {
        let name_lower: Vec<u8> = name.iter().map(|b| b.to_ascii_lowercase()).collect();
        self.headers
            .iter()
            .find(|(n, _)| {
                let n_lower: Vec<u8> = n.iter().map(|b| b.to_ascii_lowercase()).collect();
                n_lower == name_lower
            })
            .map(|(_, v)| v.as_slice())
    }

    /// Get path as string
    pub fn path_str(&self) -> Option<&str> {
        self.path.as_deref()
    }

    /// Get query string as string
    pub fn query_str(&self) -> Option<String> {
        self.query_string.as_ref().map(|q| String::from_utf8_lossy(q).to_string())
    }
}

/// ASGI message types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum AsgiMessage {
    /// HTTP request body
    #[serde(rename = "http.request")]
    HttpRequest { body: Vec<u8>, more_body: bool },

    /// HTTP response start
    #[serde(rename = "http.response.start")]
    HttpResponseStart {
        status: u16,
        headers: Vec<(Vec<u8>, Vec<u8>)>,
    },

    /// HTTP response body
    #[serde(rename = "http.response.body")]
    HttpResponseBody { body: Vec<u8>, more_body: bool },

    /// HTTP disconnect
    #[serde(rename = "http.disconnect")]
    HttpDisconnect,

    /// WebSocket connect
    #[serde(rename = "websocket.connect")]
    WebsocketConnect,

    /// WebSocket accept
    #[serde(rename = "websocket.accept")]
    WebsocketAccept {
        subprotocol: Option<String>,
        headers: Vec<(Vec<u8>, Vec<u8>)>,
    },

    /// WebSocket receive
    #[serde(rename = "websocket.receive")]
    WebsocketReceive {
        bytes: Option<Vec<u8>>,
        text: Option<String>,
    },

    /// WebSocket send
    #[serde(rename = "websocket.send")]
    WebsocketSend {
        bytes: Option<Vec<u8>>,
        text: Option<String>,
    },

    /// WebSocket close
    #[serde(rename = "websocket.close")]
    WebsocketClose {
        code: Option<u16>,
        reason: Option<String>,
    },

    /// Lifespan startup
    #[serde(rename = "lifespan.startup")]
    LifespanStartup,

    /// Lifespan startup complete
    #[serde(rename = "lifespan.startup.complete")]
    LifespanStartupComplete,

    /// Lifespan startup failed
    #[serde(rename = "lifespan.startup.failed")]
    LifespanStartupFailed { message: String },

    /// Lifespan shutdown
    #[serde(rename = "lifespan.shutdown")]
    LifespanShutdown,

    /// Lifespan shutdown complete
    #[serde(rename = "lifespan.shutdown.complete")]
    LifespanShutdownComplete,

    /// Lifespan shutdown failed
    #[serde(rename = "lifespan.shutdown.failed")]
    LifespanShutdownFailed { message: String },
}

impl AsgiMessage {
    /// Create an HTTP request message
    pub fn http_request(body: Vec<u8>, more_body: bool) -> Self {
        Self::HttpRequest { body, more_body }
    }

    /// Create an HTTP response start message
    pub fn http_response_start(status: u16, headers: Vec<(Vec<u8>, Vec<u8>)>) -> Self {
        Self::HttpResponseStart { status, headers }
    }

    /// Create an HTTP response body message
    pub fn http_response_body(body: Vec<u8>, more_body: bool) -> Self {
        Self::HttpResponseBody { body, more_body }
    }

    /// Create a WebSocket text message
    pub fn websocket_text(text: impl Into<String>) -> Self {
        Self::WebsocketSend {
            bytes: None,
            text: Some(text.into()),
        }
    }

    /// Create a WebSocket binary message
    pub fn websocket_bytes(bytes: Vec<u8>) -> Self {
        Self::WebsocketSend {
            bytes: Some(bytes),
            text: None,
        }
    }
}

/// ASGI application trait (simplified for sync testing)
pub trait AsgiApp: Send + Sync {
    /// Handle an ASGI request synchronously (for testing)
    fn call_sync(
        &self,
        scope: AsgiScope,
        request_body: Vec<u8>,
    ) -> Result<Vec<AsgiMessage>, AsgiError>;
}

/// Simple in-memory ASGI receive implementation for testing
pub struct MemoryReceive {
    messages: Vec<AsgiMessage>,
    index: usize,
}

impl MemoryReceive {
    pub fn new(messages: Vec<AsgiMessage>) -> Self {
        Self { messages, index: 0 }
    }

    /// Receive the next message synchronously
    pub fn receive(&mut self) -> Result<AsgiMessage, AsgiError> {
        if self.index < self.messages.len() {
            let msg = self.messages[self.index].clone();
            self.index += 1;
            Ok(msg)
        } else {
            Err(AsgiError::ConnectionClosed)
        }
    }
}

/// Simple in-memory ASGI send implementation for testing
pub struct MemorySend {
    messages: Vec<AsgiMessage>,
}

impl MemorySend {
    pub fn new() -> Self {
        Self {
            messages: Vec::new(),
        }
    }

    pub fn messages(&self) -> &[AsgiMessage] {
        &self.messages
    }

    pub fn into_messages(self) -> Vec<AsgiMessage> {
        self.messages
    }

    /// Send a message synchronously
    pub fn send(&mut self, message: AsgiMessage) -> Result<(), AsgiError> {
        self.messages.push(message);
        Ok(())
    }
}

impl Default for MemorySend {
    fn default() -> Self {
        Self::new()
    }
}

/// ASGI Lifespan handler for managing application startup/shutdown
#[derive(Debug)]
pub struct LifespanHandler {
    /// Whether startup has completed
    pub startup_complete: bool,
    /// Whether shutdown has completed
    pub shutdown_complete: bool,
    /// Startup error message (if any)
    pub startup_error: Option<String>,
    /// Shutdown error message (if any)
    pub shutdown_error: Option<String>,
    /// Application state
    pub state: HashMap<String, serde_json::Value>,
}

impl LifespanHandler {
    /// Create a new lifespan handler
    pub fn new() -> Self {
        Self {
            startup_complete: false,
            shutdown_complete: false,
            startup_error: None,
            shutdown_error: None,
            state: HashMap::new(),
        }
    }

    /// Handle a lifespan message
    pub fn handle(&mut self, message: &AsgiMessage) -> Result<Option<AsgiMessage>, AsgiError> {
        match message {
            AsgiMessage::LifespanStartup => {
                // Perform startup logic
                self.startup_complete = true;
                Ok(Some(AsgiMessage::LifespanStartupComplete))
            }
            AsgiMessage::LifespanShutdown => {
                // Perform shutdown logic
                self.shutdown_complete = true;
                Ok(Some(AsgiMessage::LifespanShutdownComplete))
            }
            _ => Err(AsgiError::InvalidMessage("Expected lifespan message".to_string())),
        }
    }

    /// Mark startup as failed
    pub fn fail_startup(&mut self, message: impl Into<String>) -> AsgiMessage {
        let msg = message.into();
        self.startup_error = Some(msg.clone());
        AsgiMessage::LifespanStartupFailed { message: msg }
    }

    /// Mark shutdown as failed
    pub fn fail_shutdown(&mut self, message: impl Into<String>) -> AsgiMessage {
        let msg = message.into();
        self.shutdown_error = Some(msg.clone());
        AsgiMessage::LifespanShutdownFailed { message: msg }
    }

    /// Check if the application is ready
    pub fn is_ready(&self) -> bool {
        self.startup_complete && self.startup_error.is_none()
    }

    /// Set state value
    pub fn set_state(&mut self, key: impl Into<String>, value: serde_json::Value) {
        self.state.insert(key.into(), value);
    }

    /// Get state value
    pub fn get_state(&self, key: &str) -> Option<&serde_json::Value> {
        self.state.get(key)
    }
}

impl Default for LifespanHandler {
    fn default() -> Self {
        Self::new()
    }
}

/// HTTP connection state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HttpConnectionState {
    /// Waiting for request
    Pending,
    /// Request received, processing
    Processing,
    /// Response started (headers sent)
    ResponseStarted,
    /// Response complete
    Complete,
    /// Connection closed/disconnected
    Disconnected,
}

/// HTTP connection handler
#[derive(Debug)]
pub struct HttpConnection {
    /// Connection state
    pub state: HttpConnectionState,
    /// Request scope
    pub scope: AsgiScope,
    /// Request body chunks
    pub request_body: Vec<u8>,
    /// Whether more body is expected
    pub more_body: bool,
    /// Response status code
    pub response_status: Option<u16>,
    /// Response headers
    pub response_headers: Vec<(Vec<u8>, Vec<u8>)>,
    /// Response body chunks
    pub response_body: Vec<u8>,
}

impl HttpConnection {
    /// Create a new HTTP connection
    pub fn new(scope: AsgiScope) -> Result<Self, AsgiError> {
        if scope.scope_type != ScopeType::Http {
            return Err(AsgiError::InvalidScope("Expected HTTP scope".to_string()));
        }

        Ok(Self {
            state: HttpConnectionState::Pending,
            scope,
            request_body: Vec::new(),
            more_body: true,
            response_status: None,
            response_headers: Vec::new(),
            response_body: Vec::new(),
        })
    }

    /// Receive request body
    pub fn receive_body(&mut self, body: Vec<u8>, more_body: bool) {
        self.request_body.extend(body);
        self.more_body = more_body;
        if !more_body {
            self.state = HttpConnectionState::Processing;
        }
    }

    /// Start the response
    pub fn start_response(
        &mut self,
        status: u16,
        headers: Vec<(Vec<u8>, Vec<u8>)>,
    ) -> Result<(), AsgiError> {
        if self.state == HttpConnectionState::ResponseStarted {
            return Err(AsgiError::InvalidMessage("Response already started".to_string()));
        }

        self.response_status = Some(status);
        self.response_headers = headers;
        self.state = HttpConnectionState::ResponseStarted;
        Ok(())
    }

    /// Send response body
    pub fn send_body(&mut self, body: Vec<u8>, more_body: bool) -> Result<(), AsgiError> {
        if self.state != HttpConnectionState::ResponseStarted {
            return Err(AsgiError::InvalidMessage("Response not started".to_string()));
        }

        self.response_body.extend(body);
        if !more_body {
            self.state = HttpConnectionState::Complete;
        }
        Ok(())
    }

    /// Handle disconnect
    pub fn disconnect(&mut self) {
        self.state = HttpConnectionState::Disconnected;
    }

    /// Check if the connection is complete
    pub fn is_complete(&self) -> bool {
        matches!(self.state, HttpConnectionState::Complete | HttpConnectionState::Disconnected)
    }

    /// Get the full request body
    pub fn get_request_body(&self) -> &[u8] {
        &self.request_body
    }

    /// Get the response status
    pub fn get_response_status(&self) -> Option<u16> {
        self.response_status
    }

    /// Get the response body
    pub fn get_response_body(&self) -> &[u8] {
        &self.response_body
    }
}

/// WebSocket connection state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WebSocketState {
    /// Waiting for connection
    Connecting,
    /// Connection accepted
    Connected,
    /// Connection closing
    Closing,
    /// Connection closed
    Closed,
}

/// WebSocket connection handler
#[derive(Debug)]
pub struct WebSocketConnection {
    /// Connection state
    pub state: WebSocketState,
    /// Connection scope
    pub scope: AsgiScope,
    /// Accepted subprotocol
    pub subprotocol: Option<String>,
    /// Received messages
    pub received: Vec<AsgiMessage>,
    /// Sent messages
    pub sent: Vec<AsgiMessage>,
    /// Close code
    pub close_code: Option<u16>,
    /// Close reason
    pub close_reason: Option<String>,
}

impl WebSocketConnection {
    /// Create a new WebSocket connection
    pub fn new(scope: AsgiScope) -> Result<Self, AsgiError> {
        if scope.scope_type != ScopeType::Websocket {
            return Err(AsgiError::InvalidScope("Expected WebSocket scope".to_string()));
        }

        Ok(Self {
            state: WebSocketState::Connecting,
            scope,
            subprotocol: None,
            received: Vec::new(),
            sent: Vec::new(),
            close_code: None,
            close_reason: None,
        })
    }

    /// Accept the connection
    pub fn accept(
        &mut self,
        subprotocol: Option<String>,
        headers: Vec<(Vec<u8>, Vec<u8>)>,
    ) -> AsgiMessage {
        self.state = WebSocketState::Connected;
        self.subprotocol = subprotocol.clone();
        AsgiMessage::WebsocketAccept {
            subprotocol,
            headers,
        }
    }

    /// Receive a message
    pub fn receive(&mut self, message: AsgiMessage) -> Result<(), AsgiError> {
        if self.state != WebSocketState::Connected {
            return Err(AsgiError::InvalidMessage("WebSocket not connected".to_string()));
        }

        match &message {
            AsgiMessage::WebsocketReceive { .. } => {
                self.received.push(message);
                Ok(())
            }
            AsgiMessage::WebsocketClose { code, reason } => {
                self.close_code = *code;
                self.close_reason = reason.clone();
                self.state = WebSocketState::Closing;
                Ok(())
            }
            _ => Err(AsgiError::InvalidMessage("Expected WebSocket receive or close".to_string())),
        }
    }

    /// Send a message
    pub fn send(&mut self, message: AsgiMessage) -> Result<(), AsgiError> {
        if self.state != WebSocketState::Connected {
            return Err(AsgiError::InvalidMessage("WebSocket not connected".to_string()));
        }

        match &message {
            AsgiMessage::WebsocketSend { .. } => {
                self.sent.push(message);
                Ok(())
            }
            _ => Err(AsgiError::InvalidMessage("Expected WebSocket send".to_string())),
        }
    }

    /// Close the connection
    pub fn close(&mut self, code: Option<u16>, reason: Option<String>) -> AsgiMessage {
        self.close_code = code;
        self.close_reason = reason.clone();
        self.state = WebSocketState::Closed;
        AsgiMessage::WebsocketClose { code, reason }
    }

    /// Check if the connection is open
    pub fn is_open(&self) -> bool {
        self.state == WebSocketState::Connected
    }

    /// Get received text messages
    pub fn get_text_messages(&self) -> Vec<&str> {
        self.received
            .iter()
            .filter_map(|m| {
                if let AsgiMessage::WebsocketReceive { text: Some(t), .. } = m {
                    Some(t.as_str())
                } else {
                    None
                }
            })
            .collect()
    }

    /// Get received binary messages
    pub fn get_binary_messages(&self) -> Vec<&[u8]> {
        self.received
            .iter()
            .filter_map(|m| {
                if let AsgiMessage::WebsocketReceive { bytes: Some(b), .. } = m {
                    Some(b.as_slice())
                } else {
                    None
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_asgi_scope_http() {
        let scope = AsgiScope::http("GET", "/api/users")
            .with_header(b"content-type".to_vec(), b"application/json".to_vec())
            .with_query(b"page=1".to_vec());

        assert_eq!(scope.scope_type, ScopeType::Http);
        assert_eq!(scope.method, Some("GET".to_string()));
        assert_eq!(scope.path, Some("/api/users".to_string()));
        assert_eq!(scope.query_str(), Some("page=1".to_string()));
    }

    #[test]
    fn test_asgi_scope_websocket() {
        let scope = AsgiScope::websocket("/ws/chat");

        assert_eq!(scope.scope_type, ScopeType::Websocket);
        assert_eq!(scope.scheme, Some("ws".to_string()));
        assert_eq!(scope.path, Some("/ws/chat".to_string()));
    }

    #[test]
    fn test_asgi_scope_lifespan() {
        let scope = AsgiScope::lifespan();

        assert_eq!(scope.scope_type, ScopeType::Lifespan);
        assert!(scope.path.is_none());
    }

    #[test]
    fn test_asgi_message_http_request() {
        let msg = AsgiMessage::http_request(b"hello".to_vec(), false);

        if let AsgiMessage::HttpRequest { body, more_body } = msg {
            assert_eq!(body, b"hello");
            assert!(!more_body);
        } else {
            panic!("Expected HttpRequest");
        }
    }

    #[test]
    fn test_asgi_message_http_response() {
        let headers = vec![(b"content-type".to_vec(), b"text/plain".to_vec())];
        let msg = AsgiMessage::http_response_start(200, headers);

        if let AsgiMessage::HttpResponseStart { status, headers } = msg {
            assert_eq!(status, 200);
            assert_eq!(headers.len(), 1);
        } else {
            panic!("Expected HttpResponseStart");
        }
    }

    #[test]
    fn test_asgi_message_websocket() {
        let msg = AsgiMessage::websocket_text("Hello, WebSocket!");

        if let AsgiMessage::WebsocketSend { bytes, text } = msg {
            assert!(bytes.is_none());
            assert_eq!(text, Some("Hello, WebSocket!".to_string()));
        } else {
            panic!("Expected WebsocketSend");
        }
    }

    #[test]
    fn test_asgi_scope_get_header() {
        let scope = AsgiScope::http("GET", "/")
            .with_header(b"Content-Type".to_vec(), b"application/json".to_vec())
            .with_header(b"X-Custom".to_vec(), b"value".to_vec());

        // Case-insensitive lookup
        assert_eq!(scope.get_header(b"content-type"), Some(b"application/json".as_slice()));
        assert_eq!(scope.get_header(b"CONTENT-TYPE"), Some(b"application/json".as_slice()));
        assert_eq!(scope.get_header(b"x-custom"), Some(b"value".as_slice()));
        assert!(scope.get_header(b"nonexistent").is_none());
    }

    #[test]
    fn test_lifespan_handler() {
        let mut handler = LifespanHandler::new();

        assert!(!handler.is_ready());

        // Handle startup
        let response = handler.handle(&AsgiMessage::LifespanStartup).unwrap();
        assert!(matches!(response, Some(AsgiMessage::LifespanStartupComplete)));
        assert!(handler.is_ready());

        // Handle shutdown
        let response = handler.handle(&AsgiMessage::LifespanShutdown).unwrap();
        assert!(matches!(response, Some(AsgiMessage::LifespanShutdownComplete)));
        assert!(handler.shutdown_complete);
    }

    #[test]
    fn test_lifespan_handler_failure() {
        let mut handler = LifespanHandler::new();

        let msg = handler.fail_startup("Database connection failed");
        assert!(matches!(msg, AsgiMessage::LifespanStartupFailed { .. }));
        assert!(!handler.is_ready());
        assert_eq!(handler.startup_error, Some("Database connection failed".to_string()));
    }

    #[test]
    fn test_lifespan_handler_state() {
        let mut handler = LifespanHandler::new();

        handler.set_state("db_pool", serde_json::json!({"size": 10}));

        let state = handler.get_state("db_pool").unwrap();
        assert_eq!(state["size"], 10);
    }

    #[test]
    fn test_http_connection() {
        let scope = AsgiScope::http("POST", "/api/users");
        let mut conn = HttpConnection::new(scope).unwrap();

        assert_eq!(conn.state, HttpConnectionState::Pending);

        // Receive body
        conn.receive_body(b"hello".to_vec(), false);
        assert_eq!(conn.state, HttpConnectionState::Processing);
        assert_eq!(conn.get_request_body(), b"hello");

        // Start response
        let headers = vec![(b"content-type".to_vec(), b"text/plain".to_vec())];
        conn.start_response(200, headers).unwrap();
        assert_eq!(conn.state, HttpConnectionState::ResponseStarted);

        // Send body
        conn.send_body(b"world".to_vec(), false).unwrap();
        assert_eq!(conn.state, HttpConnectionState::Complete);
        assert!(conn.is_complete());
    }

    #[test]
    fn test_http_connection_chunked() {
        let scope = AsgiScope::http("POST", "/upload");
        let mut conn = HttpConnection::new(scope).unwrap();

        // Receive body in chunks
        conn.receive_body(b"chunk1".to_vec(), true);
        assert_eq!(conn.state, HttpConnectionState::Pending);

        conn.receive_body(b"chunk2".to_vec(), false);
        assert_eq!(conn.state, HttpConnectionState::Processing);
        assert_eq!(conn.get_request_body(), b"chunk1chunk2");
    }

    #[test]
    fn test_websocket_connection() {
        let scope = AsgiScope::websocket("/ws/chat");
        let mut conn = WebSocketConnection::new(scope).unwrap();

        assert_eq!(conn.state, WebSocketState::Connecting);

        // Accept connection
        let msg = conn.accept(Some("graphql-ws".to_string()), vec![]);
        assert!(matches!(msg, AsgiMessage::WebsocketAccept { .. }));
        assert_eq!(conn.state, WebSocketState::Connected);
        assert!(conn.is_open());

        // Send message
        conn.send(AsgiMessage::websocket_text("Hello")).unwrap();
        assert_eq!(conn.sent.len(), 1);

        // Receive message
        conn.receive(AsgiMessage::WebsocketReceive {
            bytes: None,
            text: Some("Hi there".to_string()),
        })
        .unwrap();
        assert_eq!(conn.get_text_messages(), vec!["Hi there"]);

        // Close connection
        let close_msg = conn.close(Some(1000), Some("Normal closure".to_string()));
        assert!(matches!(close_msg, AsgiMessage::WebsocketClose { .. }));
        assert_eq!(conn.state, WebSocketState::Closed);
        assert!(!conn.is_open());
    }

    #[test]
    fn test_websocket_binary_messages() {
        let scope = AsgiScope::websocket("/ws/binary");
        let mut conn = WebSocketConnection::new(scope).unwrap();

        conn.accept(None, vec![]);

        conn.receive(AsgiMessage::WebsocketReceive {
            bytes: Some(vec![1, 2, 3, 4]),
            text: None,
        })
        .unwrap();

        let binary = conn.get_binary_messages();
        assert_eq!(binary.len(), 1);
        assert_eq!(binary[0], &[1, 2, 3, 4]);
    }
}
