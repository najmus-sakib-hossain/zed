//! WSGI Protocol Support
//!
//! Provides WSGI (Web Server Gateway Interface) compatibility for Flask applications.

use crate::werkzeug::{Request, Response};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

/// Errors that can occur during WSGI operations
#[derive(Debug, Error)]
pub enum WsgiError {
    #[error("Invalid WSGI environ: {0}")]
    InvalidEnviron(String),

    #[error("Application error: {0}")]
    ApplicationError(String),

    #[error("Response error: {0}")]
    ResponseError(String),
}

/// WSGI environment dictionary
#[derive(Debug, Clone, Default)]
pub struct WsgiEnviron {
    /// REQUEST_METHOD
    pub request_method: String,
    /// SCRIPT_NAME
    pub script_name: String,
    /// PATH_INFO
    pub path_info: String,
    /// QUERY_STRING
    pub query_string: String,
    /// CONTENT_TYPE
    pub content_type: Option<String>,
    /// CONTENT_LENGTH
    pub content_length: Option<usize>,
    /// SERVER_NAME
    pub server_name: String,
    /// SERVER_PORT
    pub server_port: u16,
    /// SERVER_PROTOCOL
    pub server_protocol: String,
    /// HTTP headers (HTTP_*)
    pub http_headers: HashMap<String, String>,
    /// wsgi.version
    pub wsgi_version: (u8, u8),
    /// wsgi.url_scheme
    pub url_scheme: String,
    /// wsgi.input (request body)
    pub input: Vec<u8>,
    /// wsgi.errors (error stream)
    pub errors: Vec<String>,
    /// wsgi.multithread
    pub multithread: bool,
    /// wsgi.multiprocess
    pub multiprocess: bool,
    /// wsgi.run_once
    pub run_once: bool,
}

impl WsgiEnviron {
    /// Create a new WSGI environ
    pub fn new() -> Self {
        Self {
            request_method: "GET".to_string(),
            script_name: String::new(),
            path_info: "/".to_string(),
            query_string: String::new(),
            content_type: None,
            content_length: None,
            server_name: "localhost".to_string(),
            server_port: 80,
            server_protocol: "HTTP/1.1".to_string(),
            http_headers: HashMap::new(),
            wsgi_version: (1, 0),
            url_scheme: "http".to_string(),
            input: Vec::new(),
            errors: Vec::new(),
            multithread: true,
            multiprocess: false,
            run_once: false,
        }
    }

    /// Create environ from a Request
    pub fn from_request(request: &Request) -> Self {
        let mut environ = Self::new();
        environ.request_method = request.method.as_str().to_string();
        environ.path_info = request.path.clone();

        // Build query string
        if !request.query_params.is_empty() {
            environ.query_string = request
                .query_params
                .iter()
                .map(|(k, v)| format!("{}={}", k, v))
                .collect::<Vec<_>>()
                .join("&");
        }

        environ.content_type = request.content_type.clone();
        environ.content_length = Some(request.body.len());
        environ.input = request.body.clone();

        // Convert headers to HTTP_* format
        for (key, value) in &request.headers {
            let http_key = format!("HTTP_{}", key.to_uppercase().replace('-', "_"));
            environ.http_headers.insert(http_key, value.clone());
        }

        environ
    }

    /// Convert to a dictionary representation
    pub fn to_dict(&self) -> HashMap<String, String> {
        let mut dict = HashMap::new();

        dict.insert("REQUEST_METHOD".to_string(), self.request_method.clone());
        dict.insert("SCRIPT_NAME".to_string(), self.script_name.clone());
        dict.insert("PATH_INFO".to_string(), self.path_info.clone());
        dict.insert("QUERY_STRING".to_string(), self.query_string.clone());
        dict.insert("SERVER_NAME".to_string(), self.server_name.clone());
        dict.insert("SERVER_PORT".to_string(), self.server_port.to_string());
        dict.insert("SERVER_PROTOCOL".to_string(), self.server_protocol.clone());
        dict.insert("wsgi.url_scheme".to_string(), self.url_scheme.clone());

        if let Some(ref ct) = self.content_type {
            dict.insert("CONTENT_TYPE".to_string(), ct.clone());
        }
        if let Some(cl) = self.content_length {
            dict.insert("CONTENT_LENGTH".to_string(), cl.to_string());
        }

        for (key, value) in &self.http_headers {
            dict.insert(key.clone(), value.clone());
        }

        dict
    }

    /// Get a header value
    pub fn get_header(&self, name: &str) -> Option<&String> {
        let http_key = format!("HTTP_{}", name.to_uppercase().replace('-', "_"));
        self.http_headers.get(&http_key)
    }

    /// Set request method
    pub fn with_method(mut self, method: &str) -> Self {
        self.request_method = method.to_string();
        self
    }

    /// Set path
    pub fn with_path(mut self, path: impl Into<String>) -> Self {
        self.path_info = path.into();
        self
    }

    /// Set query string
    pub fn with_query(mut self, query: impl Into<String>) -> Self {
        self.query_string = query.into();
        self
    }

    /// Set body
    pub fn with_body(mut self, body: Vec<u8>) -> Self {
        self.content_length = Some(body.len());
        self.input = body;
        self
    }

    /// Add header
    pub fn with_header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        let http_key = format!("HTTP_{}", name.into().to_uppercase().replace('-', "_"));
        self.http_headers.insert(http_key, value.into());
        self
    }
}

/// WSGI response
#[derive(Debug, Clone)]
pub struct WsgiResponse {
    /// Status string (e.g., "200 OK")
    pub status: String,
    /// Response headers
    pub headers: Vec<(String, String)>,
    /// Response body chunks
    pub body: Vec<Vec<u8>>,
}

impl WsgiResponse {
    /// Create a new WSGI response
    pub fn new(status: impl Into<String>) -> Self {
        Self {
            status: status.into(),
            headers: Vec::new(),
            body: Vec::new(),
        }
    }

    /// Create from a Response
    pub fn from_response(response: &Response) -> Self {
        let status = format!("{} {}", response.status_code, response.status_text());
        let headers: Vec<(String, String)> =
            response.headers.iter().map(|(k, v)| (k.clone(), v.clone())).collect();

        Self {
            status,
            headers,
            body: vec![response.body.clone()],
        }
    }

    /// Add a header
    pub fn with_header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.push((name.into(), value.into()));
        self
    }

    /// Set body
    pub fn with_body(mut self, body: impl Into<Vec<u8>>) -> Self {
        self.body = vec![body.into()];
        self
    }

    /// Get full body as bytes
    pub fn get_body(&self) -> Vec<u8> {
        self.body.iter().flatten().cloned().collect()
    }

    /// Get status code
    pub fn status_code(&self) -> Option<u16> {
        self.status.split_whitespace().next().and_then(|s| s.parse().ok())
    }

    /// Convert to Response
    pub fn to_response(&self) -> Response {
        let status_code = self.status_code().unwrap_or(200);
        let mut response = Response::new(status_code);
        response.body = self.get_body();
        for (name, value) in &self.headers {
            response.headers.insert(name.clone(), value.clone());
        }
        response
    }
}

/// WSGI application trait
pub trait WsgiApp {
    /// Handle a WSGI request
    fn call(&self, environ: &WsgiEnviron) -> Result<WsgiResponse, WsgiError>;
}

/// Simple WSGI application wrapper
pub struct SimpleWsgiApp<F>
where
    F: Fn(&WsgiEnviron) -> Result<WsgiResponse, WsgiError>,
{
    handler: F,
}

impl<F> SimpleWsgiApp<F>
where
    F: Fn(&WsgiEnviron) -> Result<WsgiResponse, WsgiError>,
{
    /// Create a new simple WSGI app
    pub fn new(handler: F) -> Self {
        Self { handler }
    }
}

impl<F> WsgiApp for SimpleWsgiApp<F>
where
    F: Fn(&WsgiEnviron) -> Result<WsgiResponse, WsgiError>,
{
    fn call(&self, environ: &WsgiEnviron) -> Result<WsgiResponse, WsgiError> {
        (self.handler)(environ)
    }
}

/// WSGI middleware trait
pub trait WsgiMiddleware {
    /// Wrap an application
    fn wrap(&self, app: Box<dyn WsgiApp>) -> Box<dyn WsgiApp>;
}

/// Start response callback (for compatibility)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StartResponse {
    pub status: String,
    pub headers: Vec<(String, String)>,
}

impl StartResponse {
    pub fn new(status: impl Into<String>, headers: Vec<(String, String)>) -> Self {
        Self {
            status: status.into(),
            headers,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::werkzeug::HttpMethod;

    #[test]
    fn test_wsgi_environ_new() {
        let environ = WsgiEnviron::new();
        assert_eq!(environ.request_method, "GET");
        assert_eq!(environ.path_info, "/");
        assert_eq!(environ.wsgi_version, (1, 0));
    }

    #[test]
    fn test_wsgi_environ_from_request() {
        let request = Request::new(HttpMethod::Post, "/api/users")
            .with_content_type("application/json")
            .with_body(b"{\"name\": \"test\"}".to_vec());

        let environ = WsgiEnviron::from_request(&request);
        assert_eq!(environ.request_method, "POST");
        assert_eq!(environ.path_info, "/api/users");
        assert_eq!(environ.content_type, Some("application/json".to_string()));
        assert_eq!(environ.content_length, Some(16));
    }

    #[test]
    fn test_wsgi_environ_builder() {
        let environ = WsgiEnviron::new()
            .with_method("POST")
            .with_path("/test")
            .with_query("foo=bar")
            .with_header("Content-Type", "application/json");

        assert_eq!(environ.request_method, "POST");
        assert_eq!(environ.path_info, "/test");
        assert_eq!(environ.query_string, "foo=bar");
        assert!(environ.http_headers.contains_key("HTTP_CONTENT_TYPE"));
    }

    #[test]
    fn test_wsgi_response() {
        let response = WsgiResponse::new("200 OK")
            .with_header("Content-Type", "text/plain")
            .with_body(b"Hello, World!".to_vec());

        assert_eq!(response.status, "200 OK");
        assert_eq!(response.status_code(), Some(200));
        assert_eq!(response.get_body(), b"Hello, World!");
    }

    #[test]
    fn test_wsgi_response_from_response() {
        let response = Response::ok().with_text("Hello");
        let wsgi_response = WsgiResponse::from_response(&response);

        assert_eq!(wsgi_response.status, "200 OK");
        assert_eq!(wsgi_response.get_body(), b"Hello");
    }

    #[test]
    fn test_simple_wsgi_app() {
        let app = SimpleWsgiApp::new(|_environ| {
            Ok(WsgiResponse::new("200 OK").with_body(b"Hello".to_vec()))
        });

        let environ = WsgiEnviron::new();
        let response = app.call(&environ).unwrap();
        assert_eq!(response.status, "200 OK");
    }

    #[test]
    fn test_environ_to_dict() {
        let environ = WsgiEnviron::new().with_method("GET").with_path("/test");

        let dict = environ.to_dict();
        assert_eq!(dict.get("REQUEST_METHOD"), Some(&"GET".to_string()));
        assert_eq!(dict.get("PATH_INFO"), Some(&"/test".to_string()));
    }
}
