//! MySQL search engine — query MySQL databases.
//!
//! SearXNG equivalent: `mysql_server.py`
//!
//! This is an offline engine that queries MySQL databases.
//! Configure connection settings and a parameterized query.
//!
//! Since this requires the `mysql_async` dependency, it is disabled by default.
//! Enable the `mysql` feature in Cargo.toml to use this engine.
//!
//! Example configuration:
//! ```toml
//! [mysql]
//! host = "127.0.0.1"
//! port = 3306
//! database = "my_database"
//! username = "searxng"
//! password = "password"
//! query_str = "SELECT title, url, content FROM items WHERE title LIKE :query OR content LIKE :query"
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

/// MySQL offline search engine configuration.
pub struct MysqlEngine {
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

impl MysqlEngine {
    /// Create a new MySQL engine.
    ///
    /// # Arguments
    /// * `host` - Database host address
    /// * `port` - Database port (default: 3306)
    /// * `database` - Database name
    /// * `username` - Database username
    /// * `password` - Database password
    /// * `query_str` - SQL SELECT query with :query placeholder for the search term
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
                name: "mysql".to_string().into(),
                display_name: "MySQL".to_string().into(),
                homepage: "https://www.mysql.com".to_string().into(),
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
impl SearchEngine for MysqlEngine {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>, MetasearchError> {
        if self.host.is_empty() || self.database.is_empty() || self.query_str.is_empty() {
            return Ok(Vec::new());
        }

        #[cfg(feature = "mysql")]
        {
            use mysql_async::{prelude::*, Pool, Opts, OptsBuilder};

            let offset = (query.page.saturating_sub(1)) * (self.limit as u32);
            let wildcard = format!("%{}%", query.query.replace(' ', "%"));

            let opts: Opts = OptsBuilder::default()
                .ip_or_hostname(&self.host)
                .tcp_port(self.port)
                .db_name(Some(&self.database))
                .user(Some(&self.username))
                .pass(Some(&self.password))
                .into();

            let pool = Pool::new(opts);
            let mut conn = pool
                .get_conn()
                .await
                .map_err(|e| MetasearchError::Engine(format!("MySQL connect: {e}")))?;

            let full_query = format!(
                "{} LIMIT {} OFFSET {}",
                self.query_str, self.limit, offset
            );

            // Execute the query with the wildcard parameter
            let rows: Vec<mysql_async::Row> = conn
                .exec(&full_query, (wildcard.clone(),))
                .await
                .map_err(|e| MetasearchError::Engine(format!("MySQL query: {e}")))?;

            let mut results = Vec::new();
            for row in rows {
                let columns: Vec<String> = row
                    .columns_ref()
                    .iter()
                    .map(|c| c.name_str().to_string())
                    .collect();

                let mut title = String::new();
                let mut url = String::new();
                let mut content = String::new();

                for (i, col_name) in columns.iter().enumerate() {
                    let value: Option<String> = row.get(i);
                    match col_name.as_str() {
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
                        .enumerate()
                        .filter_map(|(i, col)| {
                            let val: Option<String> = row.get(i);
                            val.map(|v| format!("{col}: {v}"))
                        })
                        .collect();
                    title = parts.join(", ");
                }

                results.push(SearchResult::new(title, url, content, "mysql"));
            }

            pool.disconnect().await.ok();
            return Ok(results);
        }

        #[cfg(not(feature = "mysql"))]
        {
            let _ = query; // Silence unused variable warning
            tracing::warn!(
                "MySQL engine requires the 'mysql' feature. Returning empty results."
            );
            Ok(Vec::new())
        }
    }
}
