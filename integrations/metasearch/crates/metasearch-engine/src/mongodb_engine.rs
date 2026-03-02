//! MongoDB search engine — query MongoDB databases.
//!
//! SearXNG equivalent: `mongodb.py`
//!
//! This is an offline engine that queries MongoDB document databases.
//! Configure connection settings, database, collection, and search key.
//!
//! Since this requires the `mongodb` dependency, it is disabled by default.
//! Enable the `mongodb` feature in Cargo.toml to use this engine.
//!
//! Example configuration:
//! ```toml
//! [mongodb]
//! host = "127.0.0.1"
//! port = 27017
//! database = "business"
//! collection = "reviews"
//! key = "name"
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

/// MongoDB offline search engine configuration.
pub struct MongoDbEngine {
    metadata: EngineMetadata,
    host: String,
    #[allow(dead_code)]
    port: u16,
    username: Option<String>,
    password: Option<String>,
    database: String,
    collection: String,
    key: String,
    exact_match_only: bool,
    results_per_page: usize,
}

impl MongoDbEngine {
    /// Create a new MongoDB engine.
    ///
    /// # Arguments
    /// * `host` - MongoDB host address
    /// * `port` - MongoDB port (default: 27017)
    /// * `database` - Database name
    /// * `collection` - Collection name to search
    /// * `key` - Document key to search in
    #[must_use]
    pub fn new(
        host: &str,
        port: u16,
        database: &str,
        collection: &str,
        key: &str,
    ) -> Self {
        let enabled = !host.is_empty()
            && !database.is_empty()
            && !collection.is_empty()
            && !key.is_empty();

        Self {
            metadata: EngineMetadata {
                name: "mongodb".to_string().into(),
                display_name: "MongoDB".to_string().into(),
                homepage: "https://www.mongodb.com".to_string().into(),
                categories: smallvec![SearchCategory::General],
                enabled,
                timeout_ms: 5000,
                weight: 0.8,
            },
            host: host.to_string(),
            port,
            username: None,
            password: None,
            database: database.to_string(),
            collection: collection.to_string(),
            key: key.to_string(),
            exact_match_only: false,
            results_per_page: 20,
        }
    }

    /// Set authentication credentials.
    #[must_use]
    pub fn with_auth(mut self, username: &str, password: &str) -> Self {
        self.username = Some(username.to_string());
        self.password = Some(password.to_string());
        self
    }

    /// Set whether to use exact match only.
    #[must_use]
    pub fn with_exact_match(mut self, exact: bool) -> Self {
        self.exact_match_only = exact;
        self
    }

    /// Set the results per page.
    #[must_use]
    pub fn with_results_per_page(mut self, limit: usize) -> Self {
        self.results_per_page = limit;
        self
    }
}

#[async_trait]
impl SearchEngine for MongoDbEngine {
    fn metadata(&self) -> EngineMetadata {
        self.metadata.clone()
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>, MetasearchError> {
        if self.host.is_empty()
            || self.database.is_empty()
            || self.collection.is_empty()
            || self.key.is_empty()
        {
            return Ok(Vec::new());
        }

        #[cfg(feature = "mongodb")]
        {
            use mongodb::{
                bson::{doc, Bson, Document, Regex as BsonRegex},
                options::ClientOptions,
                Client,
            };
            use futures::stream::TryStreamExt;

            // Build connection string
            let conn_str = if let (Some(user), Some(pass)) = (&self.username, &self.password) {
                format!(
                    "mongodb://{}:{}@{}:{}",
                    user, pass, self.host, self.port
                )
            } else {
                format!("mongodb://{}:{}", self.host, self.port)
            };

            let client_options = ClientOptions::parse(&conn_str)
                .await
                .map_err(|e| MetasearchError::Engine(format!("MongoDB options: {e}")))?;

            let client = Client::with_options(client_options)
                .map_err(|e| MetasearchError::Engine(format!("MongoDB client: {e}")))?;

            let db = client.database(&self.database);
            let collection = db.collection::<Document>(&self.collection);

            // Build query filter
            let filter = if self.exact_match_only {
                doc! { &self.key: &query.query }
            } else {
                let pattern = format!(".*{}.*", regex::escape(&query.query));
                doc! {
                    &self.key: {
                        "$regex": Bson::RegularExpression(BsonRegex {
                            pattern,
                            options: "i".to_string(),
                        })
                    }
                }
            };

            let skip = ((query.page.saturating_sub(1)) * (self.results_per_page as u32)) as u64;
            let limit = self.results_per_page as i64;

            let mut cursor = collection
                .find(filter)
                .skip(skip)
                .limit(limit)
                .await
                .map_err(|e| MetasearchError::Engine(format!("MongoDB find: {e}")))?;

            let mut results = Vec::new();

            while let Some(doc) = cursor
                .try_next()
                .await
                .map_err(|e| MetasearchError::Engine(format!("MongoDB cursor: {e}")))?
            {
                // Build key-value representation, excluding _id
                let kvmap: Vec<String> = doc
                    .iter()
                    .filter(|(k, _)| *k != "_id")
                    .map(|(k, v)| format!("{k}: {v}"))
                    .collect();

                let title = kvmap.join(", ");

                results.push(SearchResult::new(title, "", "", "mongodb"));
            }

            return Ok(results);
        }

        #[cfg(not(feature = "mongodb"))]
        {
            let _ = query; // Silence unused variable warning
            tracing::warn!(
                "MongoDB engine requires the 'mongodb' feature. Returning empty results."
            );
            Ok(Vec::new())
        }
    }
}
