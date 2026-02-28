//! # dx-db-teleport
//!
//! Reactive database caching with zero-copy binary responses.
//!
//! This crate provides a high-performance caching layer for database queries
//! that pre-serializes results into binary format for sub-millisecond access.
//!
//! ## Features
//!
//! - Pre-computed binary responses for frequently-read queries
//! - Automatic cache invalidation via Postgres NOTIFY
//! - Thread-safe concurrent access with DashMap
//! - Zero-copy response delivery
//!
//! ## Example
//!
//! ```ignore
//! use dx_db_teleport::DbTeleport;
//!
//! let db = DbTeleport::new(pool).await?;
//!
//! // Register a query with table dependencies
//! db.register_query("get_users", "SELECT * FROM users", &["users"]);
//!
//! // Get cached binary response (< 0.1ms)
//! if let Some(data) = db.get_cached("get_users", params_hash) {
//!     // Return pre-serialized binary data
//! }
//! ```

#![forbid(unsafe_code)]

mod cache;
mod error;
mod query;

#[cfg(feature = "postgres")]
mod postgres;

pub use cache::{CacheEntry, QueryCache};
pub use error::{DbTeleportError, Result};
pub use query::{QueryId, QueryParams, RegisteredQuery, hash_params, hash_value};

#[cfg(feature = "postgres")]
pub use postgres::DbTeleport;

/// Re-export commonly used types
pub mod prelude {
    pub use crate::{CacheEntry, QueryCache, QueryId, QueryParams, RegisteredQuery};
    pub use crate::{DbTeleportError, Result};

    #[cfg(feature = "postgres")]
    pub use crate::DbTeleport;
}
