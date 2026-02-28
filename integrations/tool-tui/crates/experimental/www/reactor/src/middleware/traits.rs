//! Middleware trait definitions.

use std::collections::HashMap;
use std::time::Instant;

/// Result type for middleware operations.
pub type MiddlewareResult<T> = Result<T, MiddlewareError>;

/// Middleware errors.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum MiddlewareError {
    /// Unauthorized access.
    #[error("unauthorized: {0}")]
    Unauthorized(String),

    /// Rate limit exceeded.
    #[error("rate limited: {0}")]
    RateLimited(String),

    /// Bad request.
    #[error("bad request: {0}")]
    BadRequest(String),

    /// Internal error.
    #[error("internal error: {0}")]
    Internal(String),
}

/// Request context for middleware.
pub struct Request {
    /// Request headers.
    pub headers: HashMap<String, String>,
    /// Request extensions (for passing data between middleware).
    pub extensions: HashMap<String, String>,
    /// Request start time.
    pub start_time: Option<Instant>,
    /// Request path.
    pub path: String,
    /// Request method.
    pub method: String,
}

impl Request {
    /// Create a new request.
    pub fn new(path: String, method: String) -> Self {
        Self {
            headers: HashMap::new(),
            extensions: HashMap::new(),
            start_time: None,
            path,
            method,
        }
    }

    /// Get a header value.
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers.get(name).map(|s| s.as_str())
    }

    /// Set a header value.
    pub fn set_header(&mut self, name: impl Into<String>, value: impl Into<String>) {
        self.headers.insert(name.into(), value.into());
    }

    /// Get an extension value.
    pub fn extension(&self, name: &str) -> Option<&str> {
        self.extensions.get(name).map(|s| s.as_str())
    }

    /// Set an extension value.
    pub fn set_extension(&mut self, name: impl Into<String>, value: impl Into<String>) {
        self.extensions.insert(name.into(), value.into());
    }
}

impl Default for Request {
    fn default() -> Self {
        Self::new(String::new(), String::new())
    }
}

/// Response context for middleware.
pub struct Response {
    /// Response headers.
    pub headers: HashMap<String, String>,
    /// Response status code.
    pub status: u16,
    /// Response body.
    pub body: Vec<u8>,
}

impl Response {
    /// Create a new response.
    pub fn new() -> Self {
        Self {
            headers: HashMap::new(),
            status: 200,
            body: Vec::new(),
        }
    }

    /// Get a header value.
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers.get(name).map(|s| s.as_str())
    }

    /// Set a header value.
    pub fn set_header(&mut self, name: impl Into<String>, value: impl Into<String>) {
        self.headers.insert(name.into(), value.into());
    }

    /// Set the status code.
    pub fn set_status(&mut self, status: u16) {
        self.status = status;
    }
}

impl Default for Response {
    fn default() -> Self {
        Self::new()
    }
}

/// Middleware trait for compile-time inlining.
///
/// Implementations should be designed for inlining - avoid virtual dispatch
/// and keep logic simple.
pub trait Middleware: Sized + 'static {
    /// Called before the request handler.
    ///
    /// Return `Err` to short-circuit the request.
    fn before(req: &mut Request) -> MiddlewareResult<()>;

    /// Called after the request handler.
    fn after(req: &Request, res: &mut Response);
}
