//! Error types for dx-db-teleport.

use thiserror::Error;

/// Result type for dx-db-teleport operations.
pub type Result<T> = std::result::Result<T, DbTeleportError>;

/// Errors that can occur in dx-db-teleport.
#[derive(Debug, Error)]
pub enum DbTeleportError {
    /// Query not found in registry.
    #[error("query not found: {0}")]
    QueryNotFound(String),

    /// Cache miss - query result not in cache.
    #[error("cache miss for query: {0}")]
    CacheMiss(String),

    /// Serialization error.
    #[error("serialization error: {0}")]
    Serialization(String),

    /// Database error.
    #[cfg(feature = "postgres")]
    #[error("database error: {0}")]
    Database(#[from] tokio_postgres::Error),

    /// Pool error.
    #[cfg(feature = "postgres")]
    #[error("pool error: {0}")]
    Pool(#[from] deadpool_postgres::PoolError),

    /// Internal error.
    #[error("internal error: {0}")]
    Internal(String),
}
