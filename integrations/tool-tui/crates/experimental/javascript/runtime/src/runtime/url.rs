//! URL and URLSearchParams API

use crate::error::{DxError, DxResult};
use std::collections::HashMap;

pub struct URL {
    pub href: String,
    pub protocol: String,
    pub host: String,
    pub hostname: String,
    pub port: String,
    pub pathname: String,
    pub search: String,
    pub hash: String,
}

impl URL {
    pub fn new(url: &str) -> DxResult<Self> {
        let (protocol, rest) = if let Some(pos) = url.find("://") {
            (&url[..pos], &url[pos + 3..])
        } else {
            return Err(DxError::RuntimeError("Invalid URL".to_string()));
        };

        let (host_part, path_part) = if let Some(pos) = rest.find('/') {
            (&rest[..pos], &rest[pos..])
        } else {
            (rest, "/")
        };

        let (host, port) = if let Some(pos) = host_part.find(':') {
            (&host_part[..pos], &host_part[pos + 1..])
        } else {
            (host_part, "")
        };

        let (pathname, search, hash) = Self::parse_path(path_part);

        Ok(Self {
            href: url.to_string(),
            protocol: format!("{}:", protocol),
            host: host_part.to_string(),
            hostname: host.to_string(),
            port: port.to_string(),
            pathname: pathname.to_string(),
            search: search.to_string(),
            hash: hash.to_string(),
        })
    }

    fn parse_path(path: &str) -> (&str, &str, &str) {
        let (path_search, hash) = if let Some(pos) = path.find('#') {
            (&path[..pos], &path[pos..])
        } else {
            (path, "")
        };

        let (pathname, search) = if let Some(pos) = path_search.find('?') {
            (&path_search[..pos], &path_search[pos..])
        } else {
            (path_search, "")
        };

        (pathname, search, hash)
    }

    pub fn search_params(&self) -> URLSearchParams {
        URLSearchParams::new(&self.search)
    }
}

pub struct URLSearchParams {
    params: HashMap<String, Vec<String>>,
}

impl URLSearchParams {
    pub fn new(search: &str) -> Self {
        let mut params = HashMap::new();
        let query = search.trim_start_matches('?');

        for pair in query.split('&') {
            if let Some(pos) = pair.find('=') {
                let key = &pair[..pos];
                let value = &pair[pos + 1..];
                params.entry(key.to_string()).or_insert_with(Vec::new).push(value.to_string());
            }
        }

        Self { params }
    }

    pub fn get(&self, key: &str) -> Option<&String> {
        self.params.get(key).and_then(|v| v.first())
    }

    pub fn get_all(&self, key: &str) -> Vec<String> {
        self.params.get(key).cloned().unwrap_or_default()
    }

    pub fn has(&self, key: &str) -> bool {
        self.params.contains_key(key)
    }

    pub fn keys(&self) -> Vec<String> {
        self.params.keys().cloned().collect()
    }

    pub fn values(&self) -> Vec<String> {
        self.params.values().flatten().cloned().collect()
    }
}
