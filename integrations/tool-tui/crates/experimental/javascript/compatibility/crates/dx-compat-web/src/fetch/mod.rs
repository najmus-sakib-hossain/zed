//! Fetch API implementation.

use crate::error::{WebError, WebResult};
use http::{Method, StatusCode};
use std::collections::HashMap;

/// Headers with case-insensitive access.
#[derive(Debug, Clone, Default)]
pub struct Headers {
    inner: HashMap<String, Vec<String>>,
}

impl Headers {
    /// Create new headers.
    pub fn new() -> Self {
        Self::default()
    }

    /// Get header value.
    pub fn get(&self, name: &str) -> Option<&str> {
        self.inner.get(&name.to_lowercase()).and_then(|v| v.first()).map(|s| s.as_str())
    }

    /// Set header value.
    pub fn set(&mut self, name: &str, value: &str) {
        self.inner.insert(name.to_lowercase(), vec![value.to_string()]);
    }

    /// Append header value.
    pub fn append(&mut self, name: &str, value: &str) {
        self.inner.entry(name.to_lowercase()).or_default().push(value.to_string());
    }

    /// Delete header.
    pub fn delete(&mut self, name: &str) {
        self.inner.remove(&name.to_lowercase());
    }

    /// Check if header exists.
    pub fn has(&self, name: &str) -> bool {
        self.inner.contains_key(&name.to_lowercase())
    }
}

/// Request object.
#[derive(Debug)]
pub struct Request {
    /// HTTP method
    pub method: Method,
    /// Request URL
    pub url: String,
    /// Request headers
    pub headers: Headers,
    /// Request body
    pub body: Option<Vec<u8>>,
}

impl Request {
    /// Create a new request.
    pub fn new(url: &str) -> Self {
        Self {
            method: Method::GET,
            url: url.to_string(),
            headers: Headers::new(),
            body: None,
        }
    }

    /// Set method.
    pub fn method(mut self, method: Method) -> Self {
        self.method = method;
        self
    }

    /// Set body.
    pub fn body(mut self, body: Vec<u8>) -> Self {
        self.body = Some(body);
        self
    }
}

/// Response object.
#[derive(Debug)]
pub struct Response {
    /// HTTP status code
    pub status: StatusCode,
    /// Status text
    pub status_text: String,
    /// Response headers
    pub headers: Headers,
    /// Response body
    body: Vec<u8>,
}

impl Response {
    /// Get body as text.
    pub async fn text(&self) -> WebResult<String> {
        String::from_utf8(self.body.clone()).map_err(|e| WebError::Fetch(e.to_string()))
    }

    /// Parse body as JSON.
    pub async fn json<T: serde::de::DeserializeOwned>(&self) -> WebResult<T> {
        serde_json::from_slice(&self.body).map_err(|e| WebError::Fetch(e.to_string()))
    }

    /// Get body as bytes.
    pub async fn array_buffer(&self) -> WebResult<Vec<u8>> {
        Ok(self.body.clone())
    }

    /// Check if response is OK (2xx).
    pub fn ok(&self) -> bool {
        self.status.is_success()
    }
}

/// Global fetch function.
pub async fn fetch(url: &str, _init: Option<RequestInit>) -> WebResult<Response> {
    let client = reqwest::Client::new();
    let response = client.get(url).send().await.map_err(|e| WebError::Network(e.to_string()))?;

    let status = response.status();
    let mut headers = Headers::new();
    for (name, value) in response.headers() {
        if let Ok(v) = value.to_str() {
            headers.append(name.as_str(), v);
        }
    }

    let body = response.bytes().await.map_err(|e| WebError::Network(e.to_string()))?.to_vec();

    Ok(Response {
        status: StatusCode::from_u16(status.as_u16()).unwrap_or(StatusCode::OK),
        status_text: status.canonical_reason().unwrap_or("").to_string(),
        headers,
        body,
    })
}

/// Request initialization options.
#[derive(Debug, Default)]
pub struct RequestInit {
    /// HTTP method
    pub method: Option<Method>,
    /// Request headers
    pub headers: Option<Headers>,
    /// Request body
    pub body: Option<Vec<u8>>,
}
