//! Application settings.

use serde::{Deserialize, Serialize};

/// Top-level application configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub server: ServerSettings,
    pub search: SearchSettings,
    pub cache: CacheSettings,
    pub rate_limit: RateLimitSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerSettings {
    pub host: String,
    pub port: u16,
    pub base_url: String,
    pub secret_key: String,
    pub image_proxy: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchSettings {
    pub safe_search: u8,
    pub default_language: String,
    pub max_page: u32,
    pub request_timeout_ms: u64,
    pub max_concurrent_engines: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheSettings {
    pub enabled: bool,
    pub ttl_secs: u64,
    pub max_entries: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitSettings {
    pub enabled: bool,
    pub requests_per_second: u32,
    pub burst_size: u32,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            server: ServerSettings {
                host: "0.0.0.0".to_string(),
                port: 8888,
                base_url: "http://localhost:8888".to_string(),
                secret_key: "change-me-in-production".to_string(),
                image_proxy: true,
            },
            search: SearchSettings {
                safe_search: 1,
                default_language: "en".to_string(),
                max_page: 10,
                request_timeout_ms: 10000,  // 10 second timeout for slow engines
                max_concurrent_engines: 50,  // Query 50 engines in parallel
            },
            cache: CacheSettings {
                enabled: true,
                ttl_secs: 300,
                max_entries: 10000,
            },
            rate_limit: RateLimitSettings {
                enabled: true,
                requests_per_second: 10,
                burst_size: 30,
            },
        }
    }
}
