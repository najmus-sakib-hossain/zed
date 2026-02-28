//! Werkzeug C Extension Compatibility
//!
//! Provides compatibility with Werkzeug's C extensions for:
//! - URL routing (fast pattern matching)
//! - Request/response handling
//! - HTTP utilities

use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

/// Errors that can occur during routing
#[derive(Debug, Error)]
pub enum RoutingError {
    #[error("Invalid route pattern: {0}")]
    InvalidPattern(String),

    #[error("Route not found: {0}")]
    NotFound(String),

    #[error("Method not allowed: {0}")]
    MethodNotAllowed(String),

    #[error("Regex compilation error: {0}")]
    RegexError(#[from] regex::Error),
}

/// Errors that can occur during request handling
#[derive(Debug, Error)]
pub enum RequestError {
    #[error("Invalid request: {0}")]
    Invalid(String),

    #[error("Missing header: {0}")]
    MissingHeader(String),

    #[error("Invalid content type: {0}")]
    InvalidContentType(String),

    #[error("Body parsing error: {0}")]
    BodyParseError(String),
}

/// HTTP methods supported by the router
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Delete,
    Patch,
    Head,
    Options,
}

impl HttpMethod {
    /// Parse HTTP method from string
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_uppercase().as_str() {
            "GET" => Some(Self::Get),
            "POST" => Some(Self::Post),
            "PUT" => Some(Self::Put),
            "DELETE" => Some(Self::Delete),
            "PATCH" => Some(Self::Patch),
            "HEAD" => Some(Self::Head),
            "OPTIONS" => Some(Self::Options),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Get => "GET",
            Self::Post => "POST",
            Self::Put => "PUT",
            Self::Delete => "DELETE",
            Self::Patch => "PATCH",
            Self::Head => "HEAD",
            Self::Options => "OPTIONS",
        }
    }
}

/// A route definition
#[derive(Debug, Clone)]
pub struct Route {
    /// Route pattern (e.g., "/users/<int:id>")
    pub pattern: String,
    /// Compiled regex for matching
    regex: Regex,
    /// Parameter names extracted from pattern
    pub params: Vec<String>,
    /// Allowed HTTP methods
    pub methods: Vec<HttpMethod>,
    /// Endpoint name
    pub endpoint: String,
}

impl Route {
    /// Create a new route from a pattern
    pub fn new(
        pattern: impl Into<String>,
        endpoint: impl Into<String>,
        methods: Vec<HttpMethod>,
    ) -> Result<Self, RoutingError> {
        let pattern = pattern.into();
        let endpoint = endpoint.into();
        let (regex, params) = Self::compile_pattern(&pattern)?;

        Ok(Self {
            pattern,
            regex,
            params,
            methods,
            endpoint,
        })
    }

    /// Compile a Flask-style pattern to regex
    fn compile_pattern(pattern: &str) -> Result<(Regex, Vec<String>), RoutingError> {
        let mut regex_str = String::from("^");
        let mut params = Vec::new();
        let mut chars = pattern.chars().peekable();

        while let Some(c) = chars.next() {
            match c {
                '<' => {
                    // Parse parameter: <type:name> or <name>
                    let mut param_spec = String::new();
                    while let Some(&next) = chars.peek() {
                        if next == '>' {
                            chars.next();
                            break;
                        }
                        param_spec.push(chars.next().unwrap());
                    }

                    let (param_type, param_name) = if param_spec.contains(':') {
                        let parts: Vec<&str> = param_spec.splitn(2, ':').collect();
                        (parts[0], parts[1])
                    } else {
                        ("string", param_spec.as_str())
                    };

                    params.push(param_name.to_string());

                    // Add regex pattern based on type
                    let type_pattern = match param_type {
                        "int" => r"(\d+)",
                        "float" => r"(\d+\.?\d*)",
                        "path" => r"(.+)",
                        "uuid" => r"([0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12})",
                        _ => r"([^/]+)", // string (default)
                    };
                    regex_str.push_str(type_pattern);
                }
                '/' | '.' | '-' | '_' => {
                    // Escape special regex characters
                    regex_str.push('\\');
                    regex_str.push(c);
                }
                _ => {
                    regex_str.push(c);
                }
            }
        }

        regex_str.push('$');

        let regex = Regex::new(&regex_str)?;
        Ok((regex, params))
    }

    /// Check if this route matches the given path
    pub fn matches(&self, path: &str) -> Option<RouteMatch> {
        self.regex.captures(path).map(|caps| {
            let mut params = HashMap::new();
            for (i, name) in self.params.iter().enumerate() {
                if let Some(value) = caps.get(i + 1) {
                    params.insert(name.clone(), value.as_str().to_string());
                }
            }
            RouteMatch {
                endpoint: self.endpoint.clone(),
                params,
            }
        })
    }

    /// Check if this route allows the given method
    pub fn allows_method(&self, method: HttpMethod) -> bool {
        self.methods.contains(&method)
    }
}

/// Result of a successful route match
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteMatch {
    /// The endpoint name
    pub endpoint: String,
    /// Extracted URL parameters
    pub params: HashMap<String, String>,
}

/// URL router compatible with Werkzeug's routing
pub struct UrlRouter {
    routes: Vec<Route>,
}

impl UrlRouter {
    /// Create a new empty router
    pub fn new() -> Self {
        Self { routes: Vec::new() }
    }

    /// Add a route to the router
    pub fn add_route(&mut self, route: Route) {
        self.routes.push(route);
    }

    /// Add a route from pattern
    pub fn route(
        &mut self,
        pattern: impl Into<String>,
        endpoint: impl Into<String>,
        methods: Vec<HttpMethod>,
    ) -> Result<(), RoutingError> {
        let route = Route::new(pattern, endpoint, methods)?;
        self.add_route(route);
        Ok(())
    }

    /// Match a path and method against registered routes
    pub fn match_route(&self, path: &str, method: HttpMethod) -> Result<RouteMatch, RoutingError> {
        let mut method_not_allowed = false;

        for route in &self.routes {
            if let Some(route_match) = route.matches(path) {
                if route.allows_method(method) {
                    return Ok(route_match);
                } else {
                    method_not_allowed = true;
                }
            }
        }

        if method_not_allowed {
            Err(RoutingError::MethodNotAllowed(method.as_str().to_string()))
        } else {
            Err(RoutingError::NotFound(path.to_string()))
        }
    }

    /// Get all registered routes
    pub fn routes(&self) -> &[Route] {
        &self.routes
    }

    /// Get the number of registered routes
    pub fn len(&self) -> usize {
        self.routes.len()
    }

    /// Check if the router has no routes
    pub fn is_empty(&self) -> bool {
        self.routes.is_empty()
    }
}

impl Default for UrlRouter {
    fn default() -> Self {
        Self::new()
    }
}

/// HTTP Request representation compatible with Werkzeug
#[derive(Debug, Clone)]
pub struct Request {
    /// HTTP method
    pub method: HttpMethod,
    /// Request path
    pub path: String,
    /// Query string parameters
    pub query_params: HashMap<String, String>,
    /// Request headers
    pub headers: HashMap<String, String>,
    /// Request body
    pub body: Vec<u8>,
    /// Content type
    pub content_type: Option<String>,
    /// Remote address
    pub remote_addr: Option<String>,
}

impl Request {
    /// Create a new request
    pub fn new(method: HttpMethod, path: impl Into<String>) -> Self {
        Self {
            method,
            path: path.into(),
            query_params: HashMap::new(),
            headers: HashMap::new(),
            body: Vec::new(),
            content_type: None,
            remote_addr: None,
        }
    }

    /// Set query parameters
    pub fn with_query(mut self, params: HashMap<String, String>) -> Self {
        self.query_params = params;
        self
    }

    /// Set headers
    pub fn with_headers(mut self, headers: HashMap<String, String>) -> Self {
        self.headers = headers;
        self
    }

    /// Set body
    pub fn with_body(mut self, body: Vec<u8>) -> Self {
        self.body = body;
        self
    }

    /// Set content type
    pub fn with_content_type(mut self, content_type: impl Into<String>) -> Self {
        self.content_type = Some(content_type.into());
        self
    }

    /// Get a header value
    pub fn get_header(&self, name: &str) -> Option<&String> {
        // Case-insensitive header lookup
        let lower = name.to_lowercase();
        self.headers.iter().find(|(k, _)| k.to_lowercase() == lower).map(|(_, v)| v)
    }

    /// Get a query parameter
    pub fn get_query(&self, name: &str) -> Option<&String> {
        self.query_params.get(name)
    }

    /// Parse body as JSON
    pub fn json<T: serde::de::DeserializeOwned>(&self) -> Result<T, RequestError> {
        serde_json::from_slice(&self.body).map_err(|e| RequestError::BodyParseError(e.to_string()))
    }

    /// Get body as string
    pub fn text(&self) -> Result<String, RequestError> {
        String::from_utf8(self.body.clone())
            .map_err(|e| RequestError::BodyParseError(e.to_string()))
    }

    /// Check if request is JSON
    pub fn is_json(&self) -> bool {
        self.content_type
            .as_ref()
            .map(|ct| ct.contains("application/json"))
            .unwrap_or(false)
    }
}

/// HTTP Response builder compatible with Werkzeug
#[derive(Debug, Clone)]
pub struct Response {
    /// HTTP status code
    pub status_code: u16,
    /// Response headers
    pub headers: HashMap<String, String>,
    /// Response body
    pub body: Vec<u8>,
}

impl Response {
    /// Create a new response with status code
    pub fn new(status_code: u16) -> Self {
        Self {
            status_code,
            headers: HashMap::new(),
            body: Vec::new(),
        }
    }

    /// Create a 200 OK response
    pub fn ok() -> Self {
        Self::new(200)
    }

    /// Create a 404 Not Found response
    pub fn not_found() -> Self {
        Self::new(404)
    }

    /// Create a 500 Internal Server Error response
    pub fn internal_error() -> Self {
        Self::new(500)
    }

    /// Set response body
    pub fn with_body(mut self, body: impl Into<Vec<u8>>) -> Self {
        self.body = body.into();
        self
    }

    /// Set response body as string
    pub fn with_text(mut self, text: impl Into<String>) -> Self {
        self.body = text.into().into_bytes();
        self.headers
            .insert("Content-Type".to_string(), "text/plain; charset=utf-8".to_string());
        self
    }

    /// Set response body as JSON
    pub fn with_json<T: serde::Serialize>(mut self, value: &T) -> Result<Self, serde_json::Error> {
        self.body = serde_json::to_vec(value)?;
        self.headers.insert("Content-Type".to_string(), "application/json".to_string());
        Ok(self)
    }

    /// Set response body as HTML
    pub fn with_html(mut self, html: impl Into<String>) -> Self {
        self.body = html.into().into_bytes();
        self.headers
            .insert("Content-Type".to_string(), "text/html; charset=utf-8".to_string());
        self
    }

    /// Add a header
    pub fn with_header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(name.into(), value.into());
        self
    }

    /// Get status text for status code
    pub fn status_text(&self) -> &'static str {
        match self.status_code {
            200 => "OK",
            201 => "Created",
            204 => "No Content",
            301 => "Moved Permanently",
            302 => "Found",
            304 => "Not Modified",
            400 => "Bad Request",
            401 => "Unauthorized",
            403 => "Forbidden",
            404 => "Not Found",
            405 => "Method Not Allowed",
            500 => "Internal Server Error",
            502 => "Bad Gateway",
            503 => "Service Unavailable",
            _ => "Unknown",
        }
    }
}

/// Response builder for fluent API
pub struct ResponseBuilder {
    response: Response,
}

impl ResponseBuilder {
    /// Create a new response builder
    pub fn new() -> Self {
        Self {
            response: Response::ok(),
        }
    }

    /// Set status code
    pub fn status(mut self, code: u16) -> Self {
        self.response.status_code = code;
        self
    }

    /// Add header
    pub fn header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.response.headers.insert(name.into(), value.into());
        self
    }

    /// Set body
    pub fn body(mut self, body: impl Into<Vec<u8>>) -> Self {
        self.response.body = body.into();
        self
    }

    /// Build the response
    pub fn build(self) -> Response {
        self.response
    }
}

impl Default for ResponseBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_route_simple() {
        let route = Route::new("/users", "users_list", vec![HttpMethod::Get]).unwrap();
        assert!(route.matches("/users").is_some());
        assert!(route.matches("/posts").is_none());
    }

    #[test]
    fn test_route_with_param() {
        let route = Route::new("/users/<id>", "user_detail", vec![HttpMethod::Get]).unwrap();
        let m = route.matches("/users/123").unwrap();
        assert_eq!(m.params.get("id"), Some(&"123".to_string()));
    }

    #[test]
    fn test_route_with_typed_param() {
        let route = Route::new("/users/<int:id>", "user_detail", vec![HttpMethod::Get]).unwrap();
        assert!(route.matches("/users/123").is_some());
        assert!(route.matches("/users/abc").is_none());
    }

    #[test]
    fn test_route_with_multiple_params() {
        let route = Route::new(
            "/users/<int:user_id>/posts/<int:post_id>",
            "user_post",
            vec![HttpMethod::Get],
        )
        .unwrap();
        let m = route.matches("/users/1/posts/42").unwrap();
        assert_eq!(m.params.get("user_id"), Some(&"1".to_string()));
        assert_eq!(m.params.get("post_id"), Some(&"42".to_string()));
    }

    #[test]
    fn test_router_match() {
        let mut router = UrlRouter::new();
        router.route("/", "index", vec![HttpMethod::Get]).unwrap();
        router
            .route("/users", "users", vec![HttpMethod::Get, HttpMethod::Post])
            .unwrap();
        router.route("/users/<int:id>", "user", vec![HttpMethod::Get]).unwrap();

        let m = router.match_route("/", HttpMethod::Get).unwrap();
        assert_eq!(m.endpoint, "index");

        let m = router.match_route("/users", HttpMethod::Get).unwrap();
        assert_eq!(m.endpoint, "users");

        let m = router.match_route("/users/123", HttpMethod::Get).unwrap();
        assert_eq!(m.endpoint, "user");
        assert_eq!(m.params.get("id"), Some(&"123".to_string()));
    }

    #[test]
    fn test_router_method_not_allowed() {
        let mut router = UrlRouter::new();
        router.route("/users", "users", vec![HttpMethod::Get]).unwrap();

        let result = router.match_route("/users", HttpMethod::Post);
        assert!(matches!(result, Err(RoutingError::MethodNotAllowed(_))));
    }

    #[test]
    fn test_router_not_found() {
        let router = UrlRouter::new();
        let result = router.match_route("/nonexistent", HttpMethod::Get);
        assert!(matches!(result, Err(RoutingError::NotFound(_))));
    }

    #[test]
    fn test_request_builder() {
        let req = Request::new(HttpMethod::Post, "/api/users")
            .with_content_type("application/json")
            .with_body(b"{\"name\": \"test\"}".to_vec());

        assert_eq!(req.method, HttpMethod::Post);
        assert_eq!(req.path, "/api/users");
        assert!(req.is_json());
    }

    #[test]
    fn test_response_builder() {
        let resp = Response::ok().with_text("Hello, World!");

        assert_eq!(resp.status_code, 200);
        assert_eq!(resp.body, b"Hello, World!");
        assert_eq!(
            resp.headers.get("Content-Type"),
            Some(&"text/plain; charset=utf-8".to_string())
        );
    }

    #[test]
    fn test_http_method_from_str() {
        assert_eq!(HttpMethod::parse("GET"), Some(HttpMethod::Get));
        assert_eq!(HttpMethod::parse("post"), Some(HttpMethod::Post));
        assert_eq!(HttpMethod::parse("INVALID"), None);
    }
}
