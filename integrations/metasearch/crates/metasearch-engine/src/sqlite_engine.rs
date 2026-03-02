//! SQLite search engine — query local SQLite database files.
//!
//! SearXNG equivalent: `sqlite.py`
//!
//! This is an offline engine that queries SQLite databases.
//! Configure `database_path` and `query_str` to search your data.
//!
//! Since this requires the `rusqlite` dependency, it is disabled by default.
//! Enable the `sqlite` feature in Cargo.toml to use this engine.
//!
//! Example configuration:
//! ```toml
//! [sqlite]
//! database = "path/to/database.db"
//! query_str = "SELECT title, url, content FROM items WHERE title LIKE :wildcard OR content LIKE :wildcard"
//! result_type = "main"  # or "keyvalue"
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

/// Result type configuration for SQLite queries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SqliteResultType {
    /// Return results as main search results (requires title, url columns).
    Main,
    /// Return results as key-value pairs.
    #[default]
    KeyValue,
}

/// SQLite offline search engine.
///
/// Queries a local SQLite database file with a configurable SQL query.
/// The query should contain placeholders `:query`, `:wildcard`, `:limit`, `:offset`.
pub struct SqliteEngine {
    metadata: EngineMetadata,
    database_path: String,
    query_str: String,
    #[allow(dead_code)]
    result_type: SqliteResultType,
    limit: usize,
}

impl SqliteEngine {
    /// Create a new SQLite engine.
    ///
    /// # Arguments
    /// * `database_path` - Path to the SQLite database file
    /// * `query_str` - SQL SELECT query with placeholders
    /// * `result_type` - How to format results
    #[must_use]
    pub fn new(database_path: &str, query_str: &str, result_type: SqliteResultType) -> Self {
        let enabled = !database_path.is_empty()
            && !query_str.is_empty()
            && query_str.trim().to_lowercase().starts_with("select ");

        Self {
            metadata: EngineMetadata {
                name: "sqlite".to_string().into(),
                display_name: "SQLite".to_string().into(),
                homepage: "https://www.sqlite.org".to_string().into(),
                categories: smallvec![SearchCategory::General],
                enabled,
                timeout_ms: 5000,
                weight: 0.8,
            },
            database_path: database_path.to_string(),
            query_str: query_str.to_string(),
            result_type,
            limit: 10,
        }
    }

    /// Set the result limit per page.
    #[must_use]
    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = limit;
        self
    }
}

#[async_trait]
impl SearchEngine for SqliteEngine {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>, MetasearchError> {
        // SQLite engine requires the `sqlite` feature to be enabled.
        // Without it, we return an empty result set with a notice.
        if self.database_path.is_empty() || self.query_str.is_empty() {
            return Ok(Vec::new());
        }

        #[cfg(feature = "sqlite")]
        {
            use std::collections::HashMap;

            let db_path = self.database_path.clone();
            let query_str = self.query_str.clone();
            let limit = self.limit;
            let offset = (query.page.saturating_sub(1)) * (limit as u32);
            let search_query = query.query.clone();
            let wildcard = format!("%{}%", search_query.replace(' ', "%"));
            let result_type = self.result_type;

            // Run blocking SQLite operations in a spawn_blocking context
            let results = tokio::task::spawn_blocking(move || {
                let conn = rusqlite::Connection::open_with_flags(
                    &db_path,
                    rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
                )
                .map_err(|e| MetasearchError::Engine(format!("SQLite open: {e}")))?;

                let full_query = format!("{query_str} LIMIT :limit OFFSET :offset");

                let mut stmt = conn
                    .prepare(&full_query)
                    .map_err(|e| MetasearchError::Engine(format!("SQLite prepare: {e}")))?;

                let column_names: Vec<String> = stmt
                    .column_names()
                    .into_iter()
                    .map(String::from)
                    .collect();

                let params = rusqlite::named_params! {
                    ":query": &search_query,
                    ":wildcard": &wildcard,
                    ":limit": limit as i64,
                    ":offset": offset as i64,
                };

                let mut results = Vec::new();

                let rows = stmt
                    .query_map(params, |row| {
                        let mut map = HashMap::new();
                        for (i, name) in column_names.iter().enumerate() {
                            let value: rusqlite::Result<String> = row.get(i);
                            if let Ok(v) = value {
                                map.insert(name.clone(), v);
                            }
                        }
                        Ok(map)
                    })
                    .map_err(|e| MetasearchError::Engine(format!("SQLite query: {e}")))?;

                for kvmap in rows.flatten() {
                    let result = match result_type {
                        SqliteResultType::Main => {
                            let title = kvmap.get("title").cloned().unwrap_or_default();
                            let url = kvmap.get("url").cloned().unwrap_or_default();
                            let content = kvmap.get("content").cloned().unwrap_or_default();
                            SearchResult::new(title, url, content, "sqlite")
                        }
                        SqliteResultType::KeyValue => {
                            let title = kvmap
                                .iter()
                                .map(|(k, v)| format!("{k}: {v}"))
                                .collect::<Vec<_>>()
                                .join(", ");
                            SearchResult::new(title, "", "", "sqlite")
                        }
                    };
                    results.push(result);
                }

                Ok::<_, MetasearchError>(results)
            })
            .await
            .map_err(|e| MetasearchError::Engine(format!("SQLite task: {e}")))??;

            return Ok(results);
        }

        #[cfg(not(feature = "sqlite"))]
        {
            let _ = query; // Silence unused variable warning
            tracing::warn!(
                "SQLite engine requires the 'sqlite' feature. Returning empty results."
            );
            Ok(Vec::new())
        }
    }
}
