//! Starlette Compatibility
//!
//! Provides compatibility with Starlette's routing and request/response handling
//! for FastAPI applications.

use crate::asgi::{AsgiMessage, AsgiScope, ScopeType};
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use thiserror::Error;

/// Errors that can occur in Starlette operations
#[derive(Debug, Error)]
pub enum StarletteError {
    #[error("Route not found: {0}")]
    NotFound(String),

    #[error("Method not allowed: {0}")]
    MethodNotAllowed(String),

    #[error("Invalid route pattern: {0}")]
    InvalidPattern(String),

    #[error("Request error: {0}")]
    RequestError(String),

    #[error("Response error: {0}")]
    ResponseError(String),
}

/// HTTP methods
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
    /// Route path pattern
    pub path: String,
    /// Compiled regex
    regex: Regex,
    /// Parameter names
    pub params: Vec<String>,
    /// Allowed methods
    pub methods: Vec<HttpMethod>,
    /// Endpoint name
    pub endpoint: String,
}

impl Route {
    /// Create a new route
    pub fn new(
        path: impl Into<String>,
        endpoint: impl Into<String>,
        methods: Vec<HttpMethod>,
    ) -> Result<Self, StarletteError> {
        let path = path.into();
        let (regex, params) = Self::compile_pattern(&path)?;

        Ok(Self {
            path,
            regex,
            params,
            methods,
            endpoint: endpoint.into(),
        })
    }

    /// Compile a Starlette-style path pattern to regex
    fn compile_pattern(pattern: &str) -> Result<(Regex, Vec<String>), StarletteError> {
        let mut regex_str = String::from("^");
        let mut params = Vec::new();
        let mut chars = pattern.chars().peekable();

        while let Some(c) = chars.next() {
            match c {
                '{' => {
                    // Parse parameter: {name} or {name:type}
                    let mut param_spec = String::new();
                    while let Some(&next) = chars.peek() {
                        if next == '}' {
                            chars.next();
                            break;
                        }
                        param_spec.push(chars.next().unwrap());
                    }

                    let (param_name, param_type) = if param_spec.contains(':') {
                        let parts: Vec<&str> = param_spec.splitn(2, ':').collect();
                        (parts[0], parts[1])
                    } else {
                        (param_spec.as_str(), "str")
                    };

                    params.push(param_name.to_string());

                    let type_pattern = match param_type {
                        "int" => r"(\d+)",
                        "float" => r"(\d+\.?\d*)",
                        "path" => r"(.+)",
                        "uuid" => r"([0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12})",
                        _ => r"([^/]+)",
                    };
                    regex_str.push_str(type_pattern);
                }
                '/' | '.' | '-' | '_' => {
                    regex_str.push('\\');
                    regex_str.push(c);
                }
                _ => {
                    regex_str.push(c);
                }
            }
        }

        regex_str.push('$');

        let regex =
            Regex::new(&regex_str).map_err(|e| StarletteError::InvalidPattern(e.to_string()))?;
        Ok((regex, params))
    }

    /// Check if this route matches the given path
    pub fn matches(&self, path: &str) -> Option<HashMap<String, String>> {
        self.regex.captures(path).map(|caps| {
            let mut params = HashMap::new();
            for (i, name) in self.params.iter().enumerate() {
                if let Some(value) = caps.get(i + 1) {
                    params.insert(name.clone(), value.as_str().to_string());
                }
            }
            params
        })
    }

    /// Check if this route allows the given method
    pub fn allows_method(&self, method: HttpMethod) -> bool {
        self.methods.is_empty() || self.methods.contains(&method)
    }
}

/// Starlette-compatible request
#[derive(Debug, Clone)]
pub struct Request {
    /// HTTP method
    pub method: HttpMethod,
    /// Request path
    pub path: String,
    /// Query parameters
    pub query_params: HashMap<String, String>,
    /// Path parameters
    pub path_params: HashMap<String, String>,
    /// Headers
    pub headers: HashMap<String, String>,
    /// Request body
    pub body: Vec<u8>,
    /// Client address
    pub client: Option<(String, u16)>,
    /// State
    pub state: HashMap<String, JsonValue>,
}

impl Request {
    /// Create a request from ASGI scope
    pub fn from_scope(scope: &AsgiScope, body: Vec<u8>) -> Result<Self, StarletteError> {
        if scope.scope_type != ScopeType::Http {
            return Err(StarletteError::RequestError("Not an HTTP scope".to_string()));
        }

        let method = scope
            .method
            .as_ref()
            .and_then(|m| HttpMethod::parse(m))
            .ok_or_else(|| StarletteError::RequestError("Invalid method".to_string()))?;

        let path = scope
            .path
            .clone()
            .ok_or_else(|| StarletteError::RequestError("Missing path".to_string()))?;

        // Parse query string
        let query_params = scope
            .query_string
            .as_ref()
            .map(|q| parse_query_string(&String::from_utf8_lossy(q)))
            .unwrap_or_default();

        // Convert headers
        let headers: HashMap<String, String> = scope
            .headers
            .iter()
            .map(|(k, v)| {
                (
                    String::from_utf8_lossy(k).to_lowercase(),
                    String::from_utf8_lossy(v).to_string(),
                )
            })
            .collect();

        Ok(Self {
            method,
            path,
            query_params,
            path_params: HashMap::new(),
            headers,
            body,
            client: scope.client.clone(),
            state: scope.state.clone(),
        })
    }

    /// Create a simple request for testing
    pub fn new(method: HttpMethod, path: impl Into<String>) -> Self {
        Self {
            method,
            path: path.into(),
            query_params: HashMap::new(),
            path_params: HashMap::new(),
            headers: HashMap::new(),
            body: Vec::new(),
            client: None,
            state: HashMap::new(),
        }
    }

    /// Set path parameters
    pub fn with_path_params(mut self, params: HashMap<String, String>) -> Self {
        self.path_params = params;
        self
    }

    /// Set query parameters
    pub fn with_query_params(mut self, params: HashMap<String, String>) -> Self {
        self.query_params = params;
        self
    }

    /// Set body
    pub fn with_body(mut self, body: Vec<u8>) -> Self {
        self.body = body;
        self
    }

    /// Set JSON body
    pub fn with_json<T: Serialize>(mut self, value: &T) -> Result<Self, serde_json::Error> {
        self.body = serde_json::to_vec(value)?;
        self.headers.insert("content-type".to_string(), "application/json".to_string());
        Ok(self)
    }

    /// Get a header value
    pub fn get_header(&self, name: &str) -> Option<&String> {
        self.headers.get(&name.to_lowercase())
    }

    /// Get a query parameter
    pub fn get_query(&self, name: &str) -> Option<&String> {
        self.query_params.get(name)
    }

    /// Get a path parameter
    pub fn get_path_param(&self, name: &str) -> Option<&String> {
        self.path_params.get(name)
    }

    /// Parse body as JSON
    pub fn json<T: serde::de::DeserializeOwned>(&self) -> Result<T, serde_json::Error> {
        serde_json::from_slice(&self.body)
    }

    /// Get body as string
    pub fn text(&self) -> Result<String, std::string::FromUtf8Error> {
        String::from_utf8(self.body.clone())
    }

    /// Check if request is JSON
    pub fn is_json(&self) -> bool {
        self.get_header("content-type")
            .map(|ct| ct.contains("application/json"))
            .unwrap_or(false)
    }
}

/// Parse query string into parameters
fn parse_query_string(query: &str) -> HashMap<String, String> {
    let mut params = HashMap::new();
    for pair in query.split('&') {
        if let Some((key, value)) = pair.split_once('=') {
            params.insert(key.to_string(), value.to_string());
        }
    }
    params
}

/// Starlette-compatible response
#[derive(Debug, Clone)]
pub struct Response {
    /// Status code
    pub status_code: u16,
    /// Headers
    pub headers: HashMap<String, String>,
    /// Body
    pub body: Vec<u8>,
    /// Media type
    pub media_type: Option<String>,
}

impl Response {
    /// Create a new response
    pub fn new(status_code: u16) -> Self {
        Self {
            status_code,
            headers: HashMap::new(),
            body: Vec::new(),
            media_type: None,
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

    /// Set body
    pub fn with_body(mut self, body: impl Into<Vec<u8>>) -> Self {
        self.body = body.into();
        self
    }

    /// Set text body
    pub fn with_text(mut self, text: impl Into<String>) -> Self {
        self.body = text.into().into_bytes();
        self.media_type = Some("text/plain; charset=utf-8".to_string());
        self
    }

    /// Set JSON body
    pub fn with_json<T: Serialize>(mut self, value: &T) -> Result<Self, serde_json::Error> {
        self.body = serde_json::to_vec(value)?;
        self.media_type = Some("application/json".to_string());
        Ok(self)
    }

    /// Set HTML body
    pub fn with_html(mut self, html: impl Into<String>) -> Self {
        self.body = html.into().into_bytes();
        self.media_type = Some("text/html; charset=utf-8".to_string());
        self
    }

    /// Add a header
    pub fn with_header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(name.into().to_lowercase(), value.into());
        self
    }

    /// Convert to ASGI messages
    pub fn to_asgi_messages(&self) -> Vec<AsgiMessage> {
        let mut headers: Vec<(Vec<u8>, Vec<u8>)> = self
            .headers
            .iter()
            .map(|(k, v)| (k.as_bytes().to_vec(), v.as_bytes().to_vec()))
            .collect();

        if let Some(ref media_type) = self.media_type {
            headers.push((b"content-type".to_vec(), media_type.as_bytes().to_vec()));
        }

        headers.push((b"content-length".to_vec(), self.body.len().to_string().into_bytes()));

        vec![
            AsgiMessage::http_response_start(self.status_code, headers),
            AsgiMessage::http_response_body(self.body.clone(), false),
        ]
    }
}

/// Starlette application
pub struct StarletteApp {
    routes: Vec<Route>,
}

impl StarletteApp {
    /// Create a new application
    pub fn new() -> Self {
        Self { routes: Vec::new() }
    }

    /// Add a route
    pub fn add_route(&mut self, route: Route) {
        self.routes.push(route);
    }

    /// Add a route with builder pattern
    pub fn route(
        mut self,
        path: impl Into<String>,
        endpoint: impl Into<String>,
        methods: Vec<HttpMethod>,
    ) -> Result<Self, StarletteError> {
        let route = Route::new(path, endpoint, methods)?;
        self.routes.push(route);
        Ok(self)
    }

    /// Match a request to a route
    pub fn match_route(
        &self,
        path: &str,
        method: HttpMethod,
    ) -> Result<(&Route, HashMap<String, String>), StarletteError> {
        let mut method_not_allowed = false;

        for route in &self.routes {
            if let Some(params) = route.matches(path) {
                if route.allows_method(method) {
                    return Ok((route, params));
                } else {
                    method_not_allowed = true;
                }
            }
        }

        if method_not_allowed {
            Err(StarletteError::MethodNotAllowed(method.as_str().to_string()))
        } else {
            Err(StarletteError::NotFound(path.to_string()))
        }
    }

    /// Get all routes
    pub fn routes(&self) -> &[Route] {
        &self.routes
    }
}

impl Default for StarletteApp {
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
        let route = Route::new("/users/{id}", "user_detail", vec![HttpMethod::Get]).unwrap();
        let params = route.matches("/users/123").unwrap();
        assert_eq!(params.get("id"), Some(&"123".to_string()));
    }

    #[test]
    fn test_route_with_typed_param() {
        let route = Route::new("/users/{id:int}", "user_detail", vec![HttpMethod::Get]).unwrap();
        assert!(route.matches("/users/123").is_some());
        assert!(route.matches("/users/abc").is_none());
    }

    #[test]
    fn test_starlette_app_routing() {
        let app = StarletteApp::new()
            .route("/", "index", vec![HttpMethod::Get])
            .unwrap()
            .route("/users", "users", vec![HttpMethod::Get, HttpMethod::Post])
            .unwrap()
            .route("/users/{id:int}", "user", vec![HttpMethod::Get])
            .unwrap();

        let (route, _) = app.match_route("/", HttpMethod::Get).unwrap();
        assert_eq!(route.endpoint, "index");

        let (route, params) = app.match_route("/users/42", HttpMethod::Get).unwrap();
        assert_eq!(route.endpoint, "user");
        assert_eq!(params.get("id"), Some(&"42".to_string()));
    }

    #[test]
    fn test_request_from_scope() {
        let scope = AsgiScope::http("GET", "/api/users")
            .with_header(b"content-type".to_vec(), b"application/json".to_vec())
            .with_query(b"page=1&limit=10".to_vec());

        let request = Request::from_scope(&scope, Vec::new()).unwrap();

        assert_eq!(request.method, HttpMethod::Get);
        assert_eq!(request.path, "/api/users");
        assert_eq!(request.get_query("page"), Some(&"1".to_string()));
        assert_eq!(request.get_query("limit"), Some(&"10".to_string()));
    }

    #[test]
    fn test_response_json() {
        let data = serde_json::json!({"message": "hello"});
        let response = Response::ok().with_json(&data).unwrap();

        assert_eq!(response.status_code, 200);
        assert_eq!(response.media_type, Some("application/json".to_string()));
        assert!(!response.body.is_empty());
    }

    #[test]
    fn test_response_to_asgi() {
        let response = Response::ok().with_text("Hello, World!");
        let messages = response.to_asgi_messages();

        assert_eq!(messages.len(), 2);

        if let AsgiMessage::HttpResponseStart { status, .. } = &messages[0] {
            assert_eq!(*status, 200);
        } else {
            panic!("Expected HttpResponseStart");
        }

        if let AsgiMessage::HttpResponseBody { body, more_body } = &messages[1] {
            assert_eq!(body, b"Hello, World!");
            assert!(!more_body);
        } else {
            panic!("Expected HttpResponseBody");
        }
    }

    #[test]
    fn test_parse_query_string() {
        let params = parse_query_string("foo=bar&baz=qux");
        assert_eq!(params.get("foo"), Some(&"bar".to_string()));
        assert_eq!(params.get("baz"), Some(&"qux".to_string()));
    }
}
