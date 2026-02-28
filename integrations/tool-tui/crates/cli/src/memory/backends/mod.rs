//! Memory Storage Backends
//!
//! Pluggable storage backends for the memory system.
//! Includes SQLite with FTS5 and LanceDB for vector search.

pub mod lancedb;
pub mod sqlite;

use async_trait::async_trait;

use super::{Memory, MemoryError};

/// Backend-agnostic memory storage trait
#[async_trait]
pub trait MemoryBackend: Send + Sync {
    /// Insert a memory into the backend
    async fn insert(&self, memory: &Memory) -> Result<(), MemoryError>;

    /// Get a memory by ID
    async fn get(&self, id: &str) -> Result<Memory, MemoryError>;

    /// Update an existing memory
    async fn update(&self, memory: &Memory) -> Result<(), MemoryError>;

    /// Delete a memory by ID
    async fn delete(&self, id: &str) -> Result<(), MemoryError>;

    /// List all memories (with optional limit)
    async fn list(&self, limit: Option<usize>) -> Result<Vec<Memory>, MemoryError>;

    /// Full-text search across memory content
    async fn text_search(&self, query: &str, limit: usize) -> Result<Vec<Memory>, MemoryError>;

    /// Vector similarity search using embeddings
    async fn vector_search(
        &self,
        query_embedding: &[f32],
        limit: usize,
    ) -> Result<Vec<(Memory, f32)>, MemoryError>;

    /// Get count of stored memories
    async fn count(&self) -> Result<usize, MemoryError>;

    /// Clear all memories
    async fn clear(&self) -> Result<(), MemoryError>;

    /// Get memories by category
    async fn get_by_category(&self, category: &str) -> Result<Vec<Memory>, MemoryError>;

    /// Get memories by tag
    async fn get_by_tag(&self, tag: &str) -> Result<Vec<Memory>, MemoryError>;

    /// Prune memories older than a given timestamp
    async fn prune_before(
        &self,
        before: chrono::DateTime<chrono::Utc>,
    ) -> Result<usize, MemoryError>;

    /// Prune memories below a relevance threshold
    async fn prune_below_relevance(&self, threshold: f32) -> Result<usize, MemoryError>;

    /// Backend name (e.g., "sqlite", "lancedb")
    fn name(&self) -> &str;
}
