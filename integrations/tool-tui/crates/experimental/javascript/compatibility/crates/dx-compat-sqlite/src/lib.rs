//! # dx-compat-sqlite
//!
//! Built-in SQLite database compatibility layer.
//!
//! High-performance SQLite wrapper targeting 200k+ operations/second with:
//! - WAL mode enabled by default
//! - Statement caching for repeated queries
//! - Transaction support with automatic rollback

#![warn(missing_docs)]

mod database;
mod error;
mod statement;

pub use database::{Database, QueryRow, Transaction, Value};
pub use error::{SqliteError, SqliteResult};
pub use statement::PreparedStatement;
