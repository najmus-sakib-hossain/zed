//! Valkey search engine — query Valkey (Redis-compatible) key-value stores.
//!
//! SearXNG equivalent: `valkey_server.py`
//!
//! Valkey is an open source (BSD licensed) in-memory data structure store,
//! a fork of Redis. This engine searches keys and hash values.
//!
//! Since this requires the `redis` dependency, it is disabled by default.
//! Enable the `redis` feature in Cargo.toml to use this engine.
//!
//! Example configuration:
//! ```toml
//! [valkey]
//! host = "127.0.0.1"
//! port = 6379
//! password = ""
//! db = 0
//! exact_match_only = false
//! ```

use async_trait::async_trait;
use smallvec::smallvec;

use metasearch_core::{
    category::SearchCategory,
    engine::{EngineMetadata, SearchEngine},
    error::MetasearchError,
    query::SearchQuery,
    result::SearchResult,
};

/// Valkey (Redis-compatible) offline search engine.
pub struct ValkeyEngine {
    metadata: EngineMetadata,
    host: String,
    #[allow(dead_code)]
    port: u16,
    password: Option<String>,
    #[allow(dead_code)]
    db: u8,
    exact_match_only: bool,
}

impl ValkeyEngine {
    /// Create a new Valkey engine.
    ///
    /// # Arguments
    /// * `host` - Valkey/Redis host address
    /// * `port` - Valkey/Redis port (default: 6379)
    /// * `db` - Database index (default: 0)
    #[must_use]
    pub fn new(host: &str, port: u16, db: u8) -> Self {
        let enabled = !host.is_empty();

        Self {
            metadata: EngineMetadata {
                name: "valkey".to_string().into(),
                display_name: "Valkey".to_string().into(),
                homepage: "https://valkey.io".to_string().into(),
                categories: smallvec![SearchCategory::General],
                enabled,
                timeout_ms: 5000,
                weight: 0.8,
            },
            host: host.to_string(),
            port,
            password: None,
            db,
            exact_match_only: true,
        }
    }

    /// Set password for authentication.
    #[must_use]
    pub fn with_password(mut self, password: &str) -> Self {
        if !password.is_empty() {
            self.password = Some(password.to_string());
        }
        self
    }

    /// Set whether to use exact match only.
    #[must_use]
    pub fn with_exact_match(mut self, exact: bool) -> Self {
        self.exact_match_only = exact;
        self
    }
}

#[async_trait]
impl SearchEngine for ValkeyEngine {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>, MetasearchError> {
        if self.host.is_empty() {
            return Ok(Vec::new());
        }

        #[cfg(feature = "redis")]
        {
            use redis::{AsyncCommands, Client};

            // Build connection URL
            let url = if let Some(ref pass) = self.password {
                format!("redis://:{}@{}:{}/{}", pass, self.host, self.port, self.db)
            } else {
                format!("redis://{}:{}/{}", self.host, self.port, self.db)
            };

            let client = Client::open(url)
                .map_err(|e| MetasearchError::Engine(format!("Valkey client: {e}")))?;

            let mut conn = client
                .get_multiplexed_async_connection()
                .await
                .map_err(|e| MetasearchError::Engine(format!("Valkey connect: {e}")))?;

            let search_term = &query.query;
            let mut results = Vec::new();

            if self.exact_match_only {
                // Try to get hash with exact key match
                let hash_result: redis::RedisResult<std::collections::HashMap<String, String>> = 
                    conn.hgetall(search_term).await;

                if let Ok(kvmap) = hash_result {
                    if !kvmap.is_empty() {
                        let title = kvmap
                            .iter()
                            .map(|(k, v)| format!("{k}: {v}"))
                            .collect::<Vec<_>>()
                            .join(", ");

                        let mut result = SearchResult::new(title, "", format!("Key: {search_term}"), "valkey");
                        result.category = "general".to_string();
                        results.push(result);
                    }
                }

                // Also try HSCAN if query contains a space (hashset name + pattern)
                if search_term.contains(' ') {
                    if let Some((hash_key, pattern)) = search_term.split_once(' ') {
                        let pattern = format!("*{pattern}*");
                        let scan_result: redis::RedisResult<(u64, Vec<(String, String)>)> = 
                            redis::cmd("HSCAN")
                                .arg(hash_key)
                                .arg(0)
                                .arg("MATCH")
                                .arg(&pattern)
                                .query_async(&mut conn)
                                .await;

                        if let Ok((_, pairs)) = scan_result {
                            for (field, value) in pairs {
                                results.push(SearchResult::new(
                                    format!("{field}: {value}"),
                                    "",
                                    format!("Hash: {hash_key}"),
                                    "valkey",
                                ));
                            }
                        }
                    }
                }
            } else {
                // Scan for keys matching the pattern
                let pattern = format!("*{search_term}*");
                let scan_result: redis::RedisResult<Vec<String>> = 
                    redis::cmd("KEYS")
                        .arg(&pattern)
                        .query_async(&mut conn)
                        .await;

                if let Ok(keys) = scan_result {
                    for key in keys.into_iter().take(20) {
                        // Determine key type
                        let key_type: redis::RedisResult<String> = 
                            redis::cmd("TYPE")
                                .arg(&key)
                                .query_async(&mut conn)
                                .await;

                        let content = match key_type.as_deref() {
                            Ok("hash") => {
                                let hash: redis::RedisResult<std::collections::HashMap<String, String>> = 
                                    conn.hgetall(&key).await;
                                hash.ok().map(|h| {
                                    h.iter()
                                        .map(|(k, v)| format!("{k}: {v}"))
                                        .collect::<Vec<_>>()
                                        .join(", ")
                                })
                            }
                            Ok("list") => {
                                let list: redis::RedisResult<Vec<String>> = 
                                    conn.lrange(&key, 0, -1).await;
                                list.ok().map(|l| l.join(", "))
                            }
                            Ok("string") => {
                                let val: redis::RedisResult<String> = conn.get(&key).await;
                                val.ok()
                            }
                            _ => None,
                        };

                        results.push(SearchResult::new(
                            format!("Key: {key}"),
                            "",
                            content.unwrap_or_default(),
                            "valkey",
                        ));
                    }
                }
            }

            return Ok(results);
        }

        #[cfg(not(feature = "redis"))]
        {
            let _ = query; // Silence unused variable warning
            tracing::warn!(
                "Valkey engine requires the 'redis' feature. Returning empty results."
            );
            Ok(Vec::new())
        }
    }
}
