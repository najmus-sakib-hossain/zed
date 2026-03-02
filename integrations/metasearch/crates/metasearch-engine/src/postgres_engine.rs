//! PostgreSQL search engine — query PostgreSQL databases.
//!
//! SearXNG equivalent: `postgresql.py`
//!
//! This is an offline engine that queries PostgreSQL databases.
//! Configure connection settings and a parameterized query.
//!
//! Since this requires the `tokio-postgres` dependency, it is disabled by default.
//! Enable the `postgres` feature in Cargo.toml to use this engine.
//!
//! Example configuration:
//! ```toml
//! [postgresql]
//! host = "127.0.0.1"
//! port = 5432
//! database = "my_database"
//! username = "searxng"
//! password = "password"
//! query_str = "SELECT title, url, content FROM items WHERE title ILIKE $1 OR content ILIKE $1"
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

/// PostgreSQL offline search engine configuration.
pub struct PostgresEngine {
    metadata: EngineMetadata,
    host: String,
    #[allow(dead_code)]
    port: u16,
    database: String,
    #[allow(dead_code)]
    username: String,
    #[allow(dead_code)]
    password: String,
    query_str: String,
    limit: usize,
}

impl PostgresEngine {
    /// Create a new PostgreSQL engine.
    ///
    /// # Arguments
    /// * `host` - Database host address
    /// * `port` - Database port (default: 5432)
    /// * `database` - Database name
    /// * `username` - Database username
    /// * `password` - Database password
    /// * `query_str` - SQL SELECT query with $1 placeholder for the search term
    #[must_use]
    pub fn new(
        host: &str,
        port: u16,
        database: &str,
        username: &str,
        password: &str,
        query_str: &str,
    ) -> Self {
        let enabled = !host.is_empty()
            && !database.is_empty()
            && !query_str.is_empty()
            && query_str.trim().to_lowercase().starts_with("select ");

        Self {
            metadata: EngineMetadata {
                name: "postgresql".to_string().into(),
                display_name: "PostgreSQL".to_string().into(),
                homepage: "https://www.postgresql.org".to_string().into(),
                categories: smallvec![SearchCategory::General],
                enabled,
                timeout_ms: 5000,
                weight: 0.8,
            },
            host: host.to_string(),
            port,
            database: database.to_string(),
            username: username.to_string(),
            password: password.to_string(),
            query_str: query_str.to_string(),
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
impl SearchEngine for PostgresEngine {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>, MetasearchError> {
        if self.host.is_empty() || self.database.is_empty() || self.query_str.is_empty() {
            return Ok(Vec::new());
        }

        #[cfg(feature = "postgres")]
        {
            use tokio_postgres::{NoTls, Row};

            let offset = (query.page.saturating_sub(1)) * (self.limit as u32);
            let wildcard = format!("%{}%", query.query.replace(' ', "%"));

            let conn_str = format!(
                "host={} port={} dbname={} user={} password={}",
                self.host, self.port, self.database, self.username, self.password
            );

            let (client, connection) = tokio_postgres::connect(&conn_str, NoTls)
                .await
                .map_err(|e| MetasearchError::Engine(format!("PostgreSQL connect: {e}")))?;

            // Spawn connection task
            tokio::spawn(async move {
                if let Err(e) = connection.await {
                    tracing::error!("PostgreSQL connection error: {e}");
                }
            });

            let full_query = format!(
                "{} LIMIT {} OFFSET {}",
                self.query_str, self.limit, offset
            );

            let rows: Vec<Row> = client
                .query(&full_query, &[&wildcard])
                .await
                .map_err(|e| MetasearchError::Engine(format!("PostgreSQL query: {e}")))?;

            let mut results = Vec::new();
            for row in rows {
                let columns = row.columns();
                let mut title = String::new();
                let mut url = String::new();
                let mut content = String::new();

                for col in columns {
                    let name = col.name();
                    let value: Option<String> = row.try_get(name).ok();
                    match name {
                        "title" | "name" => {
                            if let Some(v) = value {
                                title = v;
                            }
                        }
                        "url" | "link" => {
                            if let Some(v) = value {
                                url = v;
                            }
                        }
                        "content" | "description" | "body" => {
                            if let Some(v) = value {
                                content = v;
                            }
                        }
                        _ => {}
                    }
                }

                if title.is_empty() {
                    // Build key-value representation
                    let parts: Vec<String> = columns
                        .iter()
                        .filter_map(|col| {
                            let val: Option<String> = row.try_get(col.name()).ok();
                            val.map(|v| format!("{}: {v}", col.name()))
                        })
                        .collect();
                    title = parts.join(", ");
                }

                results.push(SearchResult::new(title, url, content, "postgresql"));
            }

            return Ok(results);
        }

        #[cfg(not(feature = "postgres"))]
        {
            let _ = query; // Silence unused variable warning
            tracing::warn!(
                "PostgreSQL engine requires the 'postgres' feature. Returning empty results."
            );
            Ok(Vec::new())
        }
    }
}
