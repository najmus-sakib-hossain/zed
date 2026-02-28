//! CSRF Protection Middleware for dx-server
//!
//! Provides Cross-Site Request Forgery protection using HMAC-based tokens.
//!
//! ## Features
//! - HMAC-SHA256 token generation bound to session
//! - Token validation for state-changing requests (POST, PUT, DELETE, PATCH)
//! - Support for tokens in form fields and headers
//! - Configurable token TTL

use axum::{
    body::Body,
    http::{Method, Request, Response, StatusCode, header::HeaderValue},
    middleware::Next,
};
use blake3::Hasher;
use chrono::{DateTime, Duration, Utc};
use rand::RngCore;
use std::sync::Arc;

/// CSRF token structure
#[derive(Debug, Clone)]
pub struct CsrfToken {
    /// The token value (base64 encoded)
    pub value: String,
    /// Session ID this token is bound to
    pub session_id: String,
    /// Token creation time
    pub created_at: DateTime<Utc>,
    /// Token expiration time
    pub expires_at: DateTime<Utc>,
}

impl CsrfToken {
    /// Check if the token is expired
    pub fn is_expired(&self) -> bool {
        Utc::now() >= self.expires_at
    }

    /// Check if the token is expired at a specific time
    pub fn is_expired_at(&self, at: DateTime<Utc>) -> bool {
        at >= self.expires_at
    }
}

/// CSRF manager for token generation and validation
#[derive(Clone)]
pub struct CsrfManager {
    /// Secret key for HMAC
    secret: [u8; 32],
    /// Token TTL
    ttl: Duration,
}

impl CsrfManager {
    /// Create a new CSRF manager with a random secret
    pub fn new() -> Self {
        let mut secret = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut secret);
        Self {
            secret,
            ttl: Duration::hours(1),
        }
    }

    /// Create a CSRF manager with a specific secret
    pub fn with_secret(secret: [u8; 32]) -> Self {
        Self {
            secret,
            ttl: Duration::hours(1),
        }
    }

    /// Create a CSRF manager with custom TTL
    pub fn with_ttl(mut self, ttl: Duration) -> Self {
        self.ttl = ttl;
        self
    }

    /// Generate a new CSRF token bound to a session
    pub fn generate(&self, session_id: &str) -> CsrfToken {
        let now = Utc::now();
        let expires_at = now + self.ttl;

        // Generate random nonce
        let mut nonce = [0u8; 16];
        rand::thread_rng().fill_bytes(&mut nonce);

        // Create token payload: nonce || session_id || timestamp
        let timestamp = now.timestamp();
        let mut payload = Vec::new();
        payload.extend_from_slice(&nonce);
        payload.extend_from_slice(session_id.as_bytes());
        payload.extend_from_slice(&timestamp.to_le_bytes());

        // Generate HMAC
        let mut hasher = Hasher::new_keyed(&self.secret);
        hasher.update(&payload);
        let hash = hasher.finalize();

        // Combine nonce, timestamp, and hash into token
        let mut token_bytes = Vec::new();
        token_bytes.extend_from_slice(&nonce);
        token_bytes.extend_from_slice(&timestamp.to_le_bytes());
        token_bytes.extend_from_slice(hash.as_bytes());

        let value = base64_encode(&token_bytes);

        CsrfToken {
            value,
            session_id: session_id.to_string(),
            created_at: now,
            expires_at,
        }
    }

    /// Validate a CSRF token
    pub fn validate(&self, token: &str, session_id: &str) -> Result<(), CsrfError> {
        self.validate_at(token, session_id, Utc::now())
    }

    /// Validate a CSRF token at a specific time
    pub fn validate_at(
        &self,
        token: &str,
        session_id: &str,
        at: DateTime<Utc>,
    ) -> Result<(), CsrfError> {
        // Decode token
        let token_bytes = base64_decode(token).map_err(|_| CsrfError::Invalid)?;

        // Token should be: nonce (16) + timestamp (8) + hash (32) = 56 bytes
        if token_bytes.len() != 56 {
            return Err(CsrfError::Invalid);
        }

        let nonce = &token_bytes[0..16];
        let timestamp_bytes: [u8; 8] =
            token_bytes[16..24].try_into().map_err(|_| CsrfError::Invalid)?;
        let provided_hash = &token_bytes[24..56];

        let timestamp = i64::from_le_bytes(timestamp_bytes);

        // Check expiration
        let created_at = DateTime::from_timestamp(timestamp, 0).ok_or(CsrfError::Invalid)?;
        let expires_at = created_at + self.ttl;
        if at >= expires_at {
            return Err(CsrfError::Expired);
        }

        // Recreate the payload and verify HMAC
        let mut payload = Vec::new();
        payload.extend_from_slice(nonce);
        payload.extend_from_slice(session_id.as_bytes());
        payload.extend_from_slice(&timestamp.to_le_bytes());

        let mut hasher = Hasher::new_keyed(&self.secret);
        hasher.update(&payload);
        let expected_hash = hasher.finalize();

        // Constant-time comparison
        if !constant_time_eq(provided_hash, expected_hash.as_bytes()) {
            return Err(CsrfError::Invalid);
        }

        Ok(())
    }

    /// Get the TTL for tokens
    pub fn ttl(&self) -> Duration {
        self.ttl
    }
}

impl Default for CsrfManager {
    fn default() -> Self {
        Self::new()
    }
}

/// CSRF validation errors
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum CsrfError {
    #[error("CSRF token missing")]
    Missing,
    #[error("CSRF token invalid")]
    Invalid,
    #[error("CSRF token expired")]
    Expired,
}

/// Constant-time comparison to prevent timing attacks
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }

    let mut result = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        result |= x ^ y;
    }
    result == 0
}

/// Base64 encode bytes
fn base64_encode(bytes: &[u8]) -> String {
    use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
    URL_SAFE_NO_PAD.encode(bytes)
}

/// Base64 decode string
fn base64_decode(s: &str) -> Result<Vec<u8>, base64::DecodeError> {
    use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
    URL_SAFE_NO_PAD.decode(s)
}

/// Header name for CSRF token
pub const CSRF_HEADER: &str = "x-csrf-token";

/// Form field name for CSRF token
pub const CSRF_FIELD: &str = "_csrf";

/// Extract CSRF token from request (header or form field)
pub fn extract_csrf_token<B>(req: &Request<B>) -> Option<String> {
    // Check header first
    if let Some(header) = req.headers().get(CSRF_HEADER) {
        if let Ok(value) = header.to_str() {
            return Some(value.to_string());
        }
    }

    // Form field extraction would require body parsing, which is handled separately
    None
}

/// Extract CSRF token from form body (URL-encoded)
pub fn extract_csrf_from_form_body(body: &str) -> Option<String> {
    for pair in body.split('&') {
        let mut parts = pair.splitn(2, '=');
        if let (Some(key), Some(value)) = (parts.next(), parts.next()) {
            if key == CSRF_FIELD {
                // URL decode the value
                return Some(url_decode(value));
            }
        }
    }
    None
}

/// URL decode a string (simple implementation for form data)
fn url_decode(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '%' {
            let hex: String = chars.by_ref().take(2).collect();
            if hex.len() == 2 {
                if let Ok(byte) = u8::from_str_radix(&hex, 16) {
                    result.push(byte as char);
                    continue;
                }
            }
            result.push('%');
            result.push_str(&hex);
        } else if c == '+' {
            result.push(' ');
        } else {
            result.push(c);
        }
    }
    result
}

/// Generate HTML hidden input field for CSRF token
///
/// Use this when rendering forms to automatically include the CSRF token.
///
/// # Example
/// ```
/// use dx_www_server::csrf::{CsrfManager, generate_csrf_hidden_field};
///
/// let manager = CsrfManager::new();
/// let token = manager.generate("session123");
/// let html = generate_csrf_hidden_field(&token.value);
/// // Returns: <input type="hidden" name="_csrf" value="...token...">
/// ```
pub fn generate_csrf_hidden_field(token_value: &str) -> String {
    format!(
        r#"<input type="hidden" name="{}" value="{}">"#,
        CSRF_FIELD,
        html_escape(token_value)
    )
}

/// Generate a meta tag for CSRF token (useful for JavaScript-based submissions)
///
/// # Example
/// ```
/// use dx_www_server::csrf::{CsrfManager, generate_csrf_meta_tag};
///
/// let manager = CsrfManager::new();
/// let token = manager.generate("session123");
/// let html = generate_csrf_meta_tag(&token.value);
/// // Returns: <meta name="csrf-token" content="...token...">
/// ```
pub fn generate_csrf_meta_tag(token_value: &str) -> String {
    format!(r#"<meta name="csrf-token" content="{}">"#, html_escape(token_value))
}

/// Escape HTML special characters to prevent XSS
fn html_escape(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            '&' => "&amp;".to_string(),
            '<' => "&lt;".to_string(),
            '>' => "&gt;".to_string(),
            '"' => "&quot;".to_string(),
            '\'' => "&#x27;".to_string(),
            _ => c.to_string(),
        })
        .collect()
}

/// Extract session ID from request (from cookie or header)
pub fn extract_session_id<B>(req: &Request<B>) -> Option<String> {
    // Check session cookie
    if let Some(cookie) = req.headers().get("cookie") {
        if let Ok(value) = cookie.to_str() {
            for part in value.split(';') {
                let part = part.trim();
                if let Some(session) = part.strip_prefix("session=") {
                    return Some(session.to_string());
                }
            }
        }
    }

    // Check session header
    if let Some(header) = req.headers().get("x-session-id") {
        if let Ok(value) = header.to_str() {
            return Some(value.to_string());
        }
    }

    None
}

/// Check if the request method requires CSRF validation
pub fn requires_csrf_validation(method: &Method) -> bool {
    matches!(*method, Method::POST | Method::PUT | Method::DELETE | Method::PATCH)
}

/// CSRF protection middleware layer
#[derive(Clone)]
pub struct CsrfLayer {
    manager: Arc<CsrfManager>,
}

impl CsrfLayer {
    /// Create a new CSRF layer
    pub fn new(manager: CsrfManager) -> Self {
        Self {
            manager: Arc::new(manager),
        }
    }

    /// Create with default manager
    pub fn default_manager() -> Self {
        Self::new(CsrfManager::new())
    }
}

impl<S> tower::Layer<S> for CsrfLayer {
    type Service = CsrfService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        CsrfService {
            inner,
            manager: self.manager.clone(),
        }
    }
}

/// CSRF protection service
#[derive(Clone)]
pub struct CsrfService<S> {
    inner: S,
    manager: Arc<CsrfManager>,
}

/// Extract CSRF token from either header or form body
async fn extract_csrf_token_from_request(req: Request<Body>) -> (Request<Body>, Option<String>) {
    // Check header first - extract value before returning req
    let header_token = req
        .headers()
        .get(CSRF_HEADER)
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string());

    if header_token.is_some() {
        return (req, header_token);
    }

    // Check content type for form data
    let is_form = req
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .map(|v| v.starts_with("application/x-www-form-urlencoded"))
        .unwrap_or(false);

    if is_form {
        // We need to read the body to extract the token
        let (parts, body) = req.into_parts();

        // Collect body bytes
        use axum::body::to_bytes;
        match to_bytes(body, 1024 * 64).await {
            Ok(bytes) => {
                let body_str = String::from_utf8_lossy(&bytes);
                let token = extract_csrf_from_form_body(&body_str);

                // Reconstruct the request with the body
                let new_body = Body::from(bytes.to_vec());
                let new_req = Request::from_parts(parts, new_body);

                (new_req, token)
            }
            Err(_) => {
                let new_req = Request::from_parts(parts, Body::empty());
                (new_req, None)
            }
        }
    } else {
        (req, None)
    }
}

impl<S> tower::Service<Request<Body>> for CsrfService<S>
where
    S: tower::Service<Request<Body>, Response = Response<Body>> + Clone + Send + 'static,
    S::Future: Send,
{
    type Response = Response<Body>;
    type Error = S::Error;
    type Future = std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>> + Send>,
    >;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        let manager = self.manager.clone();
        let mut inner = self.inner.clone();

        Box::pin(async move {
            // Only validate state-changing methods
            if requires_csrf_validation(req.method()) {
                let session_id = match extract_session_id(&req) {
                    Some(id) => id,
                    None => {
                        return Ok(csrf_error_response(CsrfError::Missing));
                    }
                };

                // Extract token from header or form body
                let (req, token) = extract_csrf_token_from_request(req).await;

                let token = match token {
                    Some(t) => t,
                    None => {
                        return Ok(csrf_error_response(CsrfError::Missing));
                    }
                };

                if let Err(e) = manager.validate(&token, &session_id) {
                    return Ok(csrf_error_response(e));
                }

                return inner.call(req).await;
            }

            inner.call(req).await
        })
    }
}

/// Create a CSRF error response
fn csrf_error_response(error: CsrfError) -> Response<Body> {
    let (status, message) = match error {
        CsrfError::Missing => (StatusCode::FORBIDDEN, "CSRF token missing"),
        CsrfError::Invalid => (StatusCode::FORBIDDEN, "CSRF token invalid"),
        CsrfError::Expired => (StatusCode::FORBIDDEN, "CSRF token expired"),
    };

    let mut response = Response::new(Body::from(message));
    *response.status_mut() = status;
    response
        .headers_mut()
        .insert("content-type", HeaderValue::from_static("text/plain"));
    response
}

/// Middleware function for CSRF protection
///
/// Note: This middleware creates a new CsrfManager for each request.
/// For production use, prefer `CsrfLayer` which shares a single manager.
pub async fn csrf_middleware(req: Request<Body>, next: Next) -> Response<Body> {
    let manager = CsrfManager::new();

    if requires_csrf_validation(req.method()) {
        let session_id = match extract_session_id(&req) {
            Some(id) => id,
            None => return csrf_error_response(CsrfError::Missing),
        };

        // Extract token from header or form body
        let (req, token) = extract_csrf_token_from_request(req).await;

        let token = match token {
            Some(t) => t,
            None => return csrf_error_response(CsrfError::Missing),
        };

        if let Err(e) = manager.validate(&token, &session_id) {
            return csrf_error_response(e);
        }

        return next.run(req).await;
    }

    next.run(req).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_csrf_token_generation() {
        let manager = CsrfManager::new();
        let token = manager.generate("session123");

        assert!(!token.value.is_empty());
        assert_eq!(token.session_id, "session123");
        assert!(!token.is_expired());
    }

    #[test]
    fn test_csrf_token_validation() {
        let manager = CsrfManager::new();
        let token = manager.generate("session123");

        // Should validate successfully
        assert!(manager.validate(&token.value, "session123").is_ok());
    }

    #[test]
    fn test_csrf_token_wrong_session() {
        let manager = CsrfManager::new();
        let token = manager.generate("session123");

        // Should fail with different session
        assert_eq!(manager.validate(&token.value, "session456"), Err(CsrfError::Invalid));
    }

    #[test]
    fn test_csrf_token_expired() {
        let manager = CsrfManager::with_secret([0u8; 32]).with_ttl(Duration::seconds(-1));
        let token = manager.generate("session123");

        // Should be expired
        assert_eq!(manager.validate(&token.value, "session123"), Err(CsrfError::Expired));
    }

    #[test]
    fn test_csrf_token_invalid() {
        let manager = CsrfManager::new();

        // Invalid token
        assert_eq!(manager.validate("invalid_token", "session123"), Err(CsrfError::Invalid));
    }

    #[test]
    fn test_csrf_token_uniqueness() {
        let manager = CsrfManager::new();

        let token1 = manager.generate("session123");
        let token2 = manager.generate("session123");

        // Tokens should be different (different nonces)
        assert_ne!(token1.value, token2.value);

        // Both should validate
        assert!(manager.validate(&token1.value, "session123").is_ok());
        assert!(manager.validate(&token2.value, "session123").is_ok());
    }

    #[test]
    fn test_requires_csrf_validation() {
        assert!(requires_csrf_validation(&Method::POST));
        assert!(requires_csrf_validation(&Method::PUT));
        assert!(requires_csrf_validation(&Method::DELETE));
        assert!(requires_csrf_validation(&Method::PATCH));
        assert!(!requires_csrf_validation(&Method::GET));
        assert!(!requires_csrf_validation(&Method::HEAD));
        assert!(!requires_csrf_validation(&Method::OPTIONS));
    }

    #[test]
    fn test_constant_time_eq() {
        assert!(constant_time_eq(b"hello", b"hello"));
        assert!(!constant_time_eq(b"hello", b"world"));
        assert!(!constant_time_eq(b"hello", b"hell"));
    }

    #[test]
    fn test_extract_csrf_from_form_body() {
        // Simple form body
        let body = "_csrf=abc123&name=test";
        assert_eq!(extract_csrf_from_form_body(body), Some("abc123".to_string()));

        // CSRF at end
        let body = "name=test&_csrf=xyz789";
        assert_eq!(extract_csrf_from_form_body(body), Some("xyz789".to_string()));

        // No CSRF token
        let body = "name=test&value=123";
        assert_eq!(extract_csrf_from_form_body(body), None);

        // URL encoded token
        let body = "_csrf=abc%2B123&name=test";
        assert_eq!(extract_csrf_from_form_body(body), Some("abc+123".to_string()));

        // Empty body
        assert_eq!(extract_csrf_from_form_body(""), None);
    }

    #[test]
    fn test_url_decode() {
        assert_eq!(url_decode("hello%20world"), "hello world");
        assert_eq!(url_decode("hello+world"), "hello world");
        assert_eq!(url_decode("abc%2B123"), "abc+123");
        assert_eq!(url_decode("no%encoding"), "no%encoding"); // Invalid encoding preserved
        assert_eq!(url_decode("plain"), "plain");
    }

    #[test]
    fn test_generate_csrf_hidden_field() {
        let html = generate_csrf_hidden_field("token123");
        assert_eq!(html, r#"<input type="hidden" name="_csrf" value="token123">"#);

        // Test HTML escaping
        let html = generate_csrf_hidden_field("token<script>");
        assert!(html.contains("&lt;script&gt;"));
    }

    #[test]
    fn test_generate_csrf_meta_tag() {
        let html = generate_csrf_meta_tag("token123");
        assert_eq!(html, r#"<meta name="csrf-token" content="token123">"#);

        // Test HTML escaping
        let html = generate_csrf_meta_tag("token\"test");
        assert!(html.contains("&quot;"));
    }

    #[test]
    fn test_html_escape() {
        assert_eq!(html_escape("hello"), "hello");
        assert_eq!(html_escape("<script>"), "&lt;script&gt;");
        assert_eq!(html_escape("a&b"), "a&amp;b");
        assert_eq!(html_escape("\"quoted\""), "&quot;quoted&quot;");
        assert_eq!(html_escape("it's"), "it&#x27;s");
    }

    #[test]
    fn test_csrf_manager_with_custom_ttl() {
        let manager = CsrfManager::new().with_ttl(Duration::minutes(30));
        assert_eq!(manager.ttl(), Duration::minutes(30));
    }

    #[test]
    fn test_csrf_token_is_expired_at() {
        let manager = CsrfManager::new().with_ttl(Duration::hours(1));
        let token = manager.generate("session123");

        // Not expired at creation time
        assert!(!token.is_expired_at(token.created_at));

        // Expired after TTL
        let future_time = token.created_at + Duration::hours(2);
        assert!(token.is_expired_at(future_time));
    }

    #[test]
    fn test_csrf_validate_at_specific_time() {
        let manager = CsrfManager::new().with_ttl(Duration::hours(1));
        let token = manager.generate("session123");

        // Valid at creation time
        assert!(manager.validate_at(&token.value, "session123", token.created_at).is_ok());

        // Valid just before expiration
        let just_before = token.expires_at - Duration::seconds(1);
        assert!(manager.validate_at(&token.value, "session123", just_before).is_ok());

        // Invalid at expiration time
        assert_eq!(
            manager.validate_at(&token.value, "session123", token.expires_at),
            Err(CsrfError::Expired)
        );

        // Invalid after expiration
        let after = token.expires_at + Duration::hours(1);
        assert_eq!(manager.validate_at(&token.value, "session123", after), Err(CsrfError::Expired));
    }

    #[test]
    fn test_csrf_layer_creation() {
        let manager = CsrfManager::new();
        let _layer = CsrfLayer::new(manager);

        let _default_layer = CsrfLayer::default_manager();
    }
}
