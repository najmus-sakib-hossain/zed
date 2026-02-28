//! Request context logging for dx-server.
//!
//! This module provides structured request context capture and logging for:
//! - Request path, method, and headers
//! - Request timing and duration
//! - Request ID correlation
//! - Error context for debugging

use axum::http::{HeaderMap, Method, Uri};
use std::time::{Duration, Instant};
use uuid::Uuid;

/// Request context for logging and error correlation.
///
/// Captures all relevant information about an HTTP request for logging
/// and debugging purposes.
#[derive(Debug, Clone)]
pub struct RequestContext {
    /// Unique request identifier for correlation
    pub request_id: String,
    /// HTTP method (GET, POST, etc.)
    pub method: String,
    /// Request path
    pub path: String,
    /// Query string (if any)
    pub query: Option<String>,
    /// Selected request headers (sanitized)
    pub headers: RequestHeaders,
    /// Request start time
    pub started_at: Instant,
    /// Client IP address (if available)
    pub client_ip: Option<String>,
    /// User agent string (if available)
    pub user_agent: Option<String>,
}

/// Sanitized request headers for logging.
///
/// Only includes safe headers that don't contain sensitive information.
#[derive(Debug, Clone, Default)]
pub struct RequestHeaders {
    pub content_type: Option<String>,
    pub content_length: Option<u64>,
    pub accept: Option<String>,
    pub accept_language: Option<String>,
    pub accept_encoding: Option<String>,
    pub host: Option<String>,
    pub origin: Option<String>,
    pub referer: Option<String>,
}

impl RequestContext {
    /// Create a new request context from HTTP request parts.
    pub fn new(method: &Method, uri: &Uri, headers: &HeaderMap) -> Self {
        let request_id = Uuid::new_v4().to_string();

        Self {
            request_id,
            method: method.to_string(),
            path: uri.path().to_string(),
            query: uri.query().map(|s| s.to_string()),
            headers: RequestHeaders::from_header_map(headers),
            started_at: Instant::now(),
            client_ip: None,
            user_agent: headers
                .get("user-agent")
                .and_then(|v| v.to_str().ok())
                .map(|s| s.to_string()),
        }
    }

    /// Create a request context with a specific request ID.
    pub fn with_request_id(mut self, request_id: String) -> Self {
        self.request_id = request_id;
        self
    }

    /// Set the client IP address.
    pub fn with_client_ip(mut self, ip: String) -> Self {
        self.client_ip = Some(ip);
        self
    }

    /// Get the elapsed time since the request started.
    pub fn elapsed(&self) -> Duration {
        self.started_at.elapsed()
    }

    /// Get the elapsed time in milliseconds.
    pub fn elapsed_ms(&self) -> u128 {
        self.elapsed().as_millis()
    }

    /// Log the request context at info level.
    pub fn log_request(&self) {
        tracing::info!(
            request_id = %self.request_id,
            method = %self.method,
            path = %self.path,
            query = ?self.query,
            client_ip = ?self.client_ip,
            user_agent = ?self.user_agent,
            "Request received"
        );
    }

    /// Log the request completion at info level.
    pub fn log_response(&self, status_code: u16) {
        tracing::info!(
            request_id = %self.request_id,
            method = %self.method,
            path = %self.path,
            status = status_code,
            duration_ms = self.elapsed_ms(),
            "Request completed"
        );
    }

    /// Log an error with full request context.
    pub fn log_error(&self, error: &dyn std::error::Error, status_code: u16) {
        tracing::error!(
            request_id = %self.request_id,
            method = %self.method,
            path = %self.path,
            query = ?self.query,
            client_ip = ?self.client_ip,
            user_agent = ?self.user_agent,
            content_type = ?self.headers.content_type,
            host = ?self.headers.host,
            origin = ?self.headers.origin,
            referer = ?self.headers.referer,
            status = status_code,
            duration_ms = self.elapsed_ms(),
            error = %error,
            "Request error"
        );
    }

    /// Log an error message with full request context.
    pub fn log_error_message(&self, message: &str, status_code: u16) {
        tracing::error!(
            request_id = %self.request_id,
            method = %self.method,
            path = %self.path,
            query = ?self.query,
            client_ip = ?self.client_ip,
            user_agent = ?self.user_agent,
            content_type = ?self.headers.content_type,
            host = ?self.headers.host,
            origin = ?self.headers.origin,
            referer = ?self.headers.referer,
            status = status_code,
            duration_ms = self.elapsed_ms(),
            error = %message,
            "Request error"
        );
    }

    /// Log a warning with request context.
    pub fn log_warning(&self, message: &str) {
        tracing::warn!(
            request_id = %self.request_id,
            method = %self.method,
            path = %self.path,
            duration_ms = self.elapsed_ms(),
            message = %message,
            "Request warning"
        );
    }

    /// Create a structured log entry for the request.
    pub fn to_log_entry(&self) -> RequestLogEntry {
        RequestLogEntry {
            request_id: self.request_id.clone(),
            method: self.method.clone(),
            path: self.path.clone(),
            query: self.query.clone(),
            client_ip: self.client_ip.clone(),
            user_agent: self.user_agent.clone(),
            duration_ms: self.elapsed_ms() as u64,
            headers: self.headers.clone(),
        }
    }
}

impl RequestHeaders {
    /// Extract safe headers from a HeaderMap.
    ///
    /// Only extracts headers that are safe to log (no auth tokens, cookies, etc.)
    pub fn from_header_map(headers: &HeaderMap) -> Self {
        Self {
            content_type: headers
                .get("content-type")
                .and_then(|v| v.to_str().ok())
                .map(|s| s.to_string()),
            content_length: headers
                .get("content-length")
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.parse().ok()),
            accept: headers.get("accept").and_then(|v| v.to_str().ok()).map(|s| s.to_string()),
            accept_language: headers
                .get("accept-language")
                .and_then(|v| v.to_str().ok())
                .map(|s| s.to_string()),
            accept_encoding: headers
                .get("accept-encoding")
                .and_then(|v| v.to_str().ok())
                .map(|s| s.to_string()),
            host: headers.get("host").and_then(|v| v.to_str().ok()).map(|s| s.to_string()),
            origin: headers.get("origin").and_then(|v| v.to_str().ok()).map(|s| s.to_string()),
            referer: headers.get("referer").and_then(|v| v.to_str().ok()).map(|s| s.to_string()),
        }
    }
}

/// Structured log entry for a request.
///
/// This can be serialized to JSON for structured logging systems.
#[derive(Debug, Clone, serde::Serialize)]
pub struct RequestLogEntry {
    pub request_id: String,
    pub method: String,
    pub path: String,
    pub query: Option<String>,
    pub client_ip: Option<String>,
    pub user_agent: Option<String>,
    pub duration_ms: u64,
    #[serde(flatten)]
    pub headers: RequestHeaders,
}

impl serde::Serialize for RequestHeaders {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(None)?;

        if let Some(ref v) = self.content_type {
            map.serialize_entry("content_type", v)?;
        }
        if let Some(v) = self.content_length {
            map.serialize_entry("content_length", &v)?;
        }
        if let Some(ref v) = self.accept {
            map.serialize_entry("accept", v)?;
        }
        if let Some(ref v) = self.accept_language {
            map.serialize_entry("accept_language", v)?;
        }
        if let Some(ref v) = self.accept_encoding {
            map.serialize_entry("accept_encoding", v)?;
        }
        if let Some(ref v) = self.host {
            map.serialize_entry("host", v)?;
        }
        if let Some(ref v) = self.origin {
            map.serialize_entry("origin", v)?;
        }
        if let Some(ref v) = self.referer {
            map.serialize_entry("referer", v)?;
        }

        map.end()
    }
}

/// Headers that should never be logged (contain sensitive information).
pub const SENSITIVE_HEADERS: &[&str] = &[
    "authorization",
    "cookie",
    "set-cookie",
    "x-api-key",
    "x-auth-token",
    "x-csrf-token",
    "proxy-authorization",
];

/// Check if a header name is sensitive and should not be logged.
pub fn is_sensitive_header(name: &str) -> bool {
    let lower = name.to_lowercase();
    SENSITIVE_HEADERS.iter().any(|&h| lower == h)
}

/// Extract client IP from request headers.
///
/// Checks common proxy headers in order of preference:
/// 1. X-Forwarded-For (first IP)
/// 2. X-Real-IP
/// 3. CF-Connecting-IP (Cloudflare)
/// 4. True-Client-IP (Akamai)
pub fn extract_client_ip(headers: &HeaderMap) -> Option<String> {
    // X-Forwarded-For: client, proxy1, proxy2
    if let Some(xff) = headers.get("x-forwarded-for").and_then(|v| v.to_str().ok()) {
        if let Some(first_ip) = xff.split(',').next() {
            return Some(first_ip.trim().to_string());
        }
    }

    // X-Real-IP
    if let Some(ip) = headers.get("x-real-ip").and_then(|v| v.to_str().ok()) {
        return Some(ip.to_string());
    }

    // CF-Connecting-IP (Cloudflare)
    if let Some(ip) = headers.get("cf-connecting-ip").and_then(|v| v.to_str().ok()) {
        return Some(ip.to_string());
    }

    // True-Client-IP (Akamai)
    if let Some(ip) = headers.get("true-client-ip").and_then(|v| v.to_str().ok()) {
        return Some(ip.to_string());
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::{HeaderValue, Method, Uri};

    fn create_test_headers() -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert("content-type", HeaderValue::from_static("application/json"));
        headers.insert("content-length", HeaderValue::from_static("100"));
        headers.insert("user-agent", HeaderValue::from_static("test-agent/1.0"));
        headers.insert("host", HeaderValue::from_static("localhost:3000"));
        headers.insert("accept", HeaderValue::from_static("*/*"));
        headers
    }

    #[test]
    fn test_request_context_creation() {
        let method = Method::GET;
        let uri: Uri = "/api/users?page=1".parse().unwrap();
        let headers = create_test_headers();

        let ctx = RequestContext::new(&method, &uri, &headers);

        assert_eq!(ctx.method, "GET");
        assert_eq!(ctx.path, "/api/users");
        assert_eq!(ctx.query, Some("page=1".to_string()));
        assert_eq!(ctx.user_agent, Some("test-agent/1.0".to_string()));
        assert!(!ctx.request_id.is_empty());
    }

    #[test]
    fn test_request_context_with_request_id() {
        let method = Method::POST;
        let uri: Uri = "/api/data".parse().unwrap();
        let headers = create_test_headers();

        let ctx = RequestContext::new(&method, &uri, &headers)
            .with_request_id("custom-id-123".to_string());

        assert_eq!(ctx.request_id, "custom-id-123");
    }

    #[test]
    fn test_request_context_with_client_ip() {
        let method = Method::GET;
        let uri: Uri = "/".parse().unwrap();
        let headers = create_test_headers();

        let ctx =
            RequestContext::new(&method, &uri, &headers).with_client_ip("192.168.1.1".to_string());

        assert_eq!(ctx.client_ip, Some("192.168.1.1".to_string()));
    }

    #[test]
    fn test_request_headers_extraction() {
        let headers = create_test_headers();
        let req_headers = RequestHeaders::from_header_map(&headers);

        assert_eq!(req_headers.content_type, Some("application/json".to_string()));
        assert_eq!(req_headers.content_length, Some(100));
        assert_eq!(req_headers.host, Some("localhost:3000".to_string()));
        assert_eq!(req_headers.accept, Some("*/*".to_string()));
    }

    #[test]
    fn test_sensitive_header_detection() {
        assert!(is_sensitive_header("authorization"));
        assert!(is_sensitive_header("Authorization"));
        assert!(is_sensitive_header("AUTHORIZATION"));
        assert!(is_sensitive_header("cookie"));
        assert!(is_sensitive_header("x-api-key"));

        assert!(!is_sensitive_header("content-type"));
        assert!(!is_sensitive_header("user-agent"));
        assert!(!is_sensitive_header("host"));
    }

    #[test]
    fn test_extract_client_ip_xff() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "x-forwarded-for",
            HeaderValue::from_static("203.0.113.195, 70.41.3.18, 150.172.238.178"),
        );

        let ip = extract_client_ip(&headers);
        assert_eq!(ip, Some("203.0.113.195".to_string()));
    }

    #[test]
    fn test_extract_client_ip_real_ip() {
        let mut headers = HeaderMap::new();
        headers.insert("x-real-ip", HeaderValue::from_static("192.168.1.100"));

        let ip = extract_client_ip(&headers);
        assert_eq!(ip, Some("192.168.1.100".to_string()));
    }

    #[test]
    fn test_extract_client_ip_cloudflare() {
        let mut headers = HeaderMap::new();
        headers.insert("cf-connecting-ip", HeaderValue::from_static("198.51.100.178"));

        let ip = extract_client_ip(&headers);
        assert_eq!(ip, Some("198.51.100.178".to_string()));
    }

    #[test]
    fn test_extract_client_ip_none() {
        let headers = HeaderMap::new();
        let ip = extract_client_ip(&headers);
        assert_eq!(ip, None);
    }

    #[test]
    fn test_elapsed_time() {
        let method = Method::GET;
        let uri: Uri = "/".parse().unwrap();
        let headers = HeaderMap::new();

        let ctx = RequestContext::new(&method, &uri, &headers);

        // Sleep a tiny bit to ensure elapsed time is > 0
        std::thread::sleep(std::time::Duration::from_millis(1));

        assert!(ctx.elapsed().as_nanos() > 0);
    }

    #[test]
    fn test_log_entry_serialization() {
        let method = Method::GET;
        let uri: Uri = "/api/test".parse().unwrap();
        let headers = create_test_headers();

        let ctx =
            RequestContext::new(&method, &uri, &headers).with_client_ip("127.0.0.1".to_string());

        let entry = ctx.to_log_entry();

        // Should be serializable to JSON
        let json = serde_json::to_string(&entry).unwrap();
        assert!(json.contains("request_id"));
        assert!(json.contains("/api/test"));
        assert!(json.contains("127.0.0.1"));
    }
}
