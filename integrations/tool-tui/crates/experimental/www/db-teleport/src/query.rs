//! Query registration and identification.

use std::collections::HashSet;
use xxhash_rust::xxh3::xxh3_64;

/// Unique identifier for a registered query.
pub type QueryId = String;

/// Hash of query parameters for cache key.
pub type QueryParams = u64;

/// A registered query with its SQL and table dependencies.
#[derive(Debug, Clone)]
pub struct RegisteredQuery {
    /// Unique query identifier.
    pub id: QueryId,
    /// SQL query string.
    pub sql: String,
    /// Tables this query depends on (for invalidation).
    pub table_dependencies: HashSet<String>,
}

impl RegisteredQuery {
    /// Create a new registered query.
    pub fn new(id: impl Into<String>, sql: impl Into<String>, tables: &[&str]) -> Self {
        Self {
            id: id.into(),
            sql: sql.into(),
            table_dependencies: tables.iter().map(|s| s.to_string()).collect(),
        }
    }

    /// Check if this query depends on a given table.
    pub fn depends_on(&self, table: &str) -> bool {
        self.table_dependencies.contains(table)
    }

    /// Generate a cache key from query ID and parameters.
    pub fn cache_key(&self, params_hash: QueryParams) -> String {
        format!("{}:{}", self.id, params_hash)
    }
}

/// Hash parameters for cache lookup.
pub fn hash_params(params: &[&[u8]]) -> QueryParams {
    let mut combined = Vec::new();
    for param in params {
        combined.extend_from_slice(param);
        combined.push(0); // Separator
    }
    xxh3_64(&combined)
}

/// Hash a single value for cache lookup.
pub fn hash_value<T: AsRef<[u8]>>(value: T) -> QueryParams {
    xxh3_64(value.as_ref())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registered_query_creation() {
        let query =
            RegisteredQuery::new("get_users", "SELECT * FROM users WHERE active = $1", &["users"]);

        assert_eq!(query.id, "get_users");
        assert!(query.depends_on("users"));
        assert!(!query.depends_on("posts"));
    }

    #[test]
    fn test_cache_key_generation() {
        let query = RegisteredQuery::new("test", "SELECT 1", &[]);
        let key = query.cache_key(12345);
        assert_eq!(key, "test:12345");
    }

    #[test]
    fn test_hash_params() {
        let params1 = hash_params(&[b"hello", b"world"]);
        let params2 = hash_params(&[b"hello", b"world"]);
        let params3 = hash_params(&[b"hello", b"other"]);

        assert_eq!(params1, params2);
        assert_ne!(params1, params3);
    }
}
