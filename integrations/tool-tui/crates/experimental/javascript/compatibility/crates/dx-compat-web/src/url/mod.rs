//! URL API.

use crate::error::{WebError, WebResult};

/// URL implementation.
#[derive(Debug, Clone)]
pub struct Url {
    inner: url::Url,
}

impl Url {
    /// Parse a URL string.
    pub fn parse(input: &str) -> WebResult<Self> {
        let inner = url::Url::parse(input).map_err(|e| WebError::Url(e.to_string()))?;
        Ok(Self { inner })
    }

    /// Get the href.
    pub fn href(&self) -> &str {
        self.inner.as_str()
    }

    /// Get the protocol.
    pub fn protocol(&self) -> &str {
        self.inner.scheme()
    }

    /// Get the host.
    pub fn host(&self) -> Option<&str> {
        self.inner.host_str()
    }

    /// Get the hostname.
    pub fn hostname(&self) -> Option<&str> {
        self.inner.host_str()
    }

    /// Get the port.
    pub fn port(&self) -> Option<u16> {
        self.inner.port()
    }

    /// Get the pathname.
    pub fn pathname(&self) -> &str {
        self.inner.path()
    }

    /// Get the search (query string).
    pub fn search(&self) -> Option<&str> {
        self.inner.query()
    }

    /// Get the hash (fragment).
    pub fn hash(&self) -> Option<&str> {
        self.inner.fragment()
    }

    /// Get the origin.
    pub fn origin(&self) -> String {
        self.inner.origin().ascii_serialization()
    }
}

impl std::fmt::Display for Url {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.inner)
    }
}

/// URLSearchParams implementation.
#[derive(Debug, Clone, Default)]
pub struct URLSearchParams {
    params: Vec<(String, String)>,
}

impl URLSearchParams {
    /// Create new search params.
    pub fn new() -> Self {
        Self::default()
    }

    /// Parse from query string.
    pub fn parse(query: &str) -> Self {
        let params = query
            .trim_start_matches('?')
            .split('&')
            .filter(|s| !s.is_empty())
            .filter_map(|pair| {
                let mut parts = pair.splitn(2, '=');
                let key = parts.next()?;
                let value = parts.next().unwrap_or("");
                Some((key.to_string(), value.to_string()))
            })
            .collect();
        Self { params }
    }

    /// Get a parameter value.
    pub fn get(&self, name: &str) -> Option<&str> {
        self.params.iter().find(|(k, _)| k == name).map(|(_, v)| v.as_str())
    }

    /// Get all values for a parameter.
    pub fn get_all(&self, name: &str) -> Vec<&str> {
        self.params.iter().filter(|(k, _)| k == name).map(|(_, v)| v.as_str()).collect()
    }

    /// Set a parameter.
    pub fn set(&mut self, name: &str, value: &str) {
        self.params.retain(|(k, _)| k != name);
        self.params.push((name.to_string(), value.to_string()));
    }

    /// Append a parameter.
    pub fn append(&mut self, name: &str, value: &str) {
        self.params.push((name.to_string(), value.to_string()));
    }

    /// Delete a parameter.
    pub fn delete(&mut self, name: &str) {
        self.params.retain(|(k, _)| k != name);
    }

    /// Check if parameter exists.
    pub fn has(&self, name: &str) -> bool {
        self.params.iter().any(|(k, _)| k == name)
    }
}

impl std::fmt::Display for URLSearchParams {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = self
            .params
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<_>>()
            .join("&");
        write!(f, "{}", s)
    }
}
