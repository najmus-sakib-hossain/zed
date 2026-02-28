//! Error types for SQLite compatibility.

use thiserror::Error;

/// SQLite error type.
#[derive(Debug, Error)]
pub enum SqliteError {
    /// Database error
    #[error("Database error: {0}")]
    Database(String),

    /// Query error
    #[error("Query error: {0}")]
    Query(String),

    /// Constraint violation
    #[error("Constraint violation: {0}")]
    Constraint(String),

    /// Database busy
    #[error("Database busy: {0}")]
    Busy(String),

    /// Internal rusqlite error
    #[error("SQLite error: {0}")]
    Rusqlite(#[from] rusqlite::Error),
}

/// Result type for SQLite operations.
pub type SqliteResult<T> = Result<T, SqliteError>;
