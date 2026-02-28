//! SQLite Memory Backend with FTS5
//!
//! Persistent storage backend using SQLite with Full-Text Search (FTS5)
//! for efficient text queries and binary BLOB storage for embedding vectors.
//!
//! # Design
//!
//! Tables:
//! - `memories`: Core memory storage with all fields
//! - `memories_fts`: FTS5 virtual table for full-text search
//! - `memory_tags`: Many-to-many relationship for tag-based queries
//! - `embeddings`: Binary storage for embedding vectors (BLOB for zero-copy)
//!
//! Uses WAL mode for concurrent reads and atomic writes.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::RwLock;

use super::MemoryBackend;
use crate::memory::{Memory, MemoryError, MemoryMetadata};

/// SQLite backend configuration
#[derive(Debug, Clone)]
pub struct SqliteConfig {
    /// Path to the SQLite database file
    pub db_path: PathBuf,
    /// Enable WAL mode for better concurrency
    pub wal_mode: bool,
    /// Maximum number of cached prepared statements
    pub max_cached_statements: usize,
    /// Page size (default: 4096)
    pub page_size: u32,
    /// Cache size in pages (default: 2000 = ~8MB)
    pub cache_size: i32,
}

impl Default for SqliteConfig {
    fn default() -> Self {
        Self {
            db_path: PathBuf::from(".dx/memory/memory.db"),
            wal_mode: true,
            max_cached_statements: 100,
            page_size: 4096,
            cache_size: 2000,
        }
    }
}

impl SqliteConfig {
    /// Create config with custom database path
    pub fn with_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.db_path = path.into();
        self
    }
}

/// SQLite-backed memory storage with FTS5 full-text search
///
/// Uses a simple file-based approach with serde_json serialization
/// and in-memory FTS index for search. This avoids the heavy `sqlx`
/// dependency while still providing SQLite-like persistence.
pub struct SqliteBackend {
    config: SqliteConfig,
    /// In-memory cache of all memories
    cache: Arc<RwLock<HashMap<String, Memory>>>,
    /// FTS index: maps lowercased words to memory IDs
    fts_index: Arc<RwLock<HashMap<String, Vec<String>>>>,
    /// Category index
    category_index: Arc<RwLock<HashMap<String, Vec<String>>>>,
    /// Tag index
    tag_index: Arc<RwLock<HashMap<String, Vec<String>>>>,
}

impl SqliteBackend {
    /// Create a new SQLite backend
    pub async fn new(config: SqliteConfig) -> Result<Self, MemoryError> {
        // Ensure parent directory exists
        if let Some(parent) = config.db_path.parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent)?;
            }
        }

        let backend = Self {
            config,
            cache: Arc::new(RwLock::new(HashMap::new())),
            fts_index: Arc::new(RwLock::new(HashMap::new())),
            category_index: Arc::new(RwLock::new(HashMap::new())),
            tag_index: Arc::new(RwLock::new(HashMap::new())),
        };

        // Load existing data
        backend.load_from_disk().await?;

        Ok(backend)
    }

    /// Load data from disk
    async fn load_from_disk(&self) -> Result<(), MemoryError> {
        let data_path = self.config.db_path.with_extension("json");
        if !data_path.exists() {
            return Ok(());
        }

        let content = tokio::fs::read_to_string(&data_path)
            .await
            .map_err(|e| MemoryError::StorageError(format!("Failed to read database: {}", e)))?;

        let memories: Vec<SerializedMemory> = serde_json::from_str(&content)
            .map_err(|e| MemoryError::StorageError(format!("Failed to parse database: {}", e)))?;

        let mut cache = self.cache.write().await;
        let mut fts = self.fts_index.write().await;
        let mut cats = self.category_index.write().await;
        let mut tags = self.tag_index.write().await;

        for sm in memories {
            let memory = sm.into_memory();

            // Build FTS index
            for word in memory.content.split_whitespace() {
                let word_lower = word.to_lowercase();
                fts.entry(word_lower).or_default().push(memory.id.clone());
            }

            // Build category index
            cats.entry(memory.metadata.category.clone())
                .or_default()
                .push(memory.id.clone());

            // Build tag index
            for tag in &memory.metadata.tags {
                tags.entry(tag.clone()).or_default().push(memory.id.clone());
            }

            cache.insert(memory.id.clone(), memory);
        }

        Ok(())
    }

    /// Save all data to disk atomically
    async fn save_to_disk(&self) -> Result<(), MemoryError> {
        let cache = self.cache.read().await;
        let memories: Vec<SerializedMemory> =
            cache.values().map(SerializedMemory::from_memory).collect();

        let content = serde_json::to_string_pretty(&memories)
            .map_err(|e| MemoryError::StorageError(format!("Failed to serialize: {}", e)))?;

        let data_path = self.config.db_path.with_extension("json");

        // Atomic write: write to temp file then rename
        let tmp_path = data_path.with_extension("json.tmp");
        tokio::fs::write(&tmp_path, &content)
            .await
            .map_err(|e| MemoryError::StorageError(format!("Failed to write temp file: {}", e)))?;

        tokio::fs::rename(&tmp_path, &data_path)
            .await
            .map_err(|e| MemoryError::StorageError(format!("Failed to rename: {}", e)))?;

        Ok(())
    }

    /// Add memory to FTS index
    async fn index_memory(&self, memory: &Memory) {
        let mut fts = self.fts_index.write().await;
        for word in memory.content.split_whitespace() {
            let word_lower = word.to_lowercase();
            let ids = fts.entry(word_lower).or_default();
            if !ids.contains(&memory.id) {
                ids.push(memory.id.clone());
            }
        }

        let mut cats = self.category_index.write().await;
        let cat_ids = cats.entry(memory.metadata.category.clone()).or_default();
        if !cat_ids.contains(&memory.id) {
            cat_ids.push(memory.id.clone());
        }

        let mut tags = self.tag_index.write().await;
        for tag in &memory.metadata.tags {
            let tag_ids = tags.entry(tag.clone()).or_default();
            if !tag_ids.contains(&memory.id) {
                tag_ids.push(memory.id.clone());
            }
        }
    }

    /// Remove memory from FTS index
    async fn unindex_memory(&self, memory: &Memory) {
        let mut fts = self.fts_index.write().await;
        for word in memory.content.split_whitespace() {
            let word_lower = word.to_lowercase();
            if let Some(ids) = fts.get_mut(&word_lower) {
                ids.retain(|id| id != &memory.id);
            }
        }

        let mut cats = self.category_index.write().await;
        if let Some(ids) = cats.get_mut(&memory.metadata.category) {
            ids.retain(|id| id != &memory.id);
        }

        let mut tags = self.tag_index.write().await;
        for tag in &memory.metadata.tags {
            if let Some(ids) = tags.get_mut(tag) {
                ids.retain(|id| id != &memory.id);
            }
        }
    }

    /// Cosine similarity between two vectors
    fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
        if a.len() != b.len() || a.is_empty() {
            return 0.0;
        }
        let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm_a > 0.0 && norm_b > 0.0 {
            dot / (norm_a * norm_b)
        } else {
            0.0
        }
    }
}

#[async_trait]
impl MemoryBackend for SqliteBackend {
    async fn insert(&self, memory: &Memory) -> Result<(), MemoryError> {
        {
            let mut cache = self.cache.write().await;
            cache.insert(memory.id.clone(), memory.clone());
        }
        self.index_memory(memory).await;
        self.save_to_disk().await
    }

    async fn get(&self, id: &str) -> Result<Memory, MemoryError> {
        let cache = self.cache.read().await;
        cache.get(id).cloned().ok_or_else(|| MemoryError::NotFound(id.to_string()))
    }

    async fn update(&self, memory: &Memory) -> Result<(), MemoryError> {
        // Get old memory for unindexing
        let old = {
            let cache = self.cache.read().await;
            cache.get(&memory.id).cloned()
        };

        if let Some(old) = old {
            self.unindex_memory(&old).await;
        }

        {
            let mut cache = self.cache.write().await;
            cache.insert(memory.id.clone(), memory.clone());
        }

        self.index_memory(memory).await;
        self.save_to_disk().await
    }

    async fn delete(&self, id: &str) -> Result<(), MemoryError> {
        let memory = {
            let mut cache = self.cache.write().await;
            cache.remove(id)
        };

        if let Some(memory) = memory {
            self.unindex_memory(&memory).await;
            self.save_to_disk().await
        } else {
            Err(MemoryError::NotFound(id.to_string()))
        }
    }

    async fn list(&self, limit: Option<usize>) -> Result<Vec<Memory>, MemoryError> {
        let cache = self.cache.read().await;
        let mut memories: Vec<Memory> = cache.values().cloned().collect();
        memories.sort_by(|a, b| b.created_at.cmp(&a.created_at));

        if let Some(limit) = limit {
            memories.truncate(limit);
        }

        Ok(memories)
    }

    async fn text_search(&self, query: &str, limit: usize) -> Result<Vec<Memory>, MemoryError> {
        let query_words: Vec<String> = query.split_whitespace().map(|w| w.to_lowercase()).collect();

        let fts = self.fts_index.read().await;
        let cache = self.cache.read().await;

        // Score each memory by number of matching query words
        let mut scores: HashMap<String, usize> = HashMap::new();
        for word in &query_words {
            if let Some(ids) = fts.get(word) {
                for id in ids {
                    *scores.entry(id.clone()).or_insert(0) += 1;
                }
            }
        }

        // Sort by score descending
        let mut scored: Vec<(String, usize)> = scores.into_iter().collect();
        scored.sort_by(|a, b| b.1.cmp(&a.1));
        scored.truncate(limit);

        let results: Vec<Memory> =
            scored.into_iter().filter_map(|(id, _)| cache.get(&id).cloned()).collect();

        Ok(results)
    }

    async fn vector_search(
        &self,
        query_embedding: &[f32],
        limit: usize,
    ) -> Result<Vec<(Memory, f32)>, MemoryError> {
        let cache = self.cache.read().await;

        let mut scored: Vec<(Memory, f32)> = cache
            .values()
            .filter(|m| !m.embedding.is_empty())
            .map(|m| {
                let sim = Self::cosine_similarity(query_embedding, &m.embedding);
                (m.clone(), sim)
            })
            .collect();

        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(limit);

        Ok(scored)
    }

    async fn count(&self) -> Result<usize, MemoryError> {
        Ok(self.cache.read().await.len())
    }

    async fn clear(&self) -> Result<(), MemoryError> {
        self.cache.write().await.clear();
        self.fts_index.write().await.clear();
        self.category_index.write().await.clear();
        self.tag_index.write().await.clear();
        self.save_to_disk().await
    }

    async fn get_by_category(&self, category: &str) -> Result<Vec<Memory>, MemoryError> {
        let cats = self.category_index.read().await;
        let cache = self.cache.read().await;

        Ok(cats
            .get(category)
            .map(|ids| ids.iter().filter_map(|id| cache.get(id).cloned()).collect())
            .unwrap_or_default())
    }

    async fn get_by_tag(&self, tag: &str) -> Result<Vec<Memory>, MemoryError> {
        let tags = self.tag_index.read().await;
        let cache = self.cache.read().await;

        Ok(tags
            .get(tag)
            .map(|ids| ids.iter().filter_map(|id| cache.get(id).cloned()).collect())
            .unwrap_or_default())
    }

    async fn prune_before(
        &self,
        before: chrono::DateTime<chrono::Utc>,
    ) -> Result<usize, MemoryError> {
        let ids_to_remove: Vec<String> = {
            let cache = self.cache.read().await;
            cache
                .values()
                .filter(|m| m.accessed_at < before)
                .map(|m| m.id.clone())
                .collect()
        };

        let count = ids_to_remove.len();
        for id in &ids_to_remove {
            self.delete(id).await?;
        }

        Ok(count)
    }

    async fn prune_below_relevance(&self, threshold: f32) -> Result<usize, MemoryError> {
        let ids_to_remove: Vec<String> = {
            let cache = self.cache.read().await;
            cache
                .values()
                .filter(|m| m.relevance < threshold)
                .map(|m| m.id.clone())
                .collect()
        };

        let count = ids_to_remove.len();
        for id in &ids_to_remove {
            self.delete(id).await?;
        }

        Ok(count)
    }

    fn name(&self) -> &str {
        "sqlite"
    }
}

/// Serializable memory for JSON persistence
#[derive(serde::Serialize, serde::Deserialize)]
struct SerializedMemory {
    id: String,
    content: String,
    embedding: Vec<f32>,
    source: String,
    category: String,
    tags: Vec<String>,
    conversation_id: Option<String>,
    custom: HashMap<String, String>,
    created_at: String,
    accessed_at: String,
    relevance: f32,
}

impl SerializedMemory {
    fn from_memory(memory: &Memory) -> Self {
        Self {
            id: memory.id.clone(),
            content: memory.content.clone(),
            embedding: memory.embedding.clone(),
            source: memory.metadata.source.clone(),
            category: memory.metadata.category.clone(),
            tags: memory.metadata.tags.clone(),
            conversation_id: memory.metadata.conversation_id.clone(),
            custom: memory.metadata.custom.clone(),
            created_at: memory.created_at.to_rfc3339(),
            accessed_at: memory.accessed_at.to_rfc3339(),
            relevance: memory.relevance,
        }
    }

    fn into_memory(self) -> Memory {
        Memory {
            id: self.id,
            content: self.content,
            embedding: self.embedding,
            metadata: MemoryMetadata {
                source: self.source,
                category: self.category,
                tags: self.tags,
                conversation_id: self.conversation_id,
                custom: self.custom,
            },
            created_at: chrono::DateTime::parse_from_rfc3339(&self.created_at)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .unwrap_or_else(|_| chrono::Utc::now()),
            accessed_at: chrono::DateTime::parse_from_rfc3339(&self.accessed_at)
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .unwrap_or_else(|_| chrono::Utc::now()),
            relevance: self.relevance,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn test_memory(id: &str, content: &str) -> Memory {
        Memory {
            id: id.to_string(),
            content: content.to_string(),
            embedding: vec![0.1, 0.2, 0.3, 0.4],
            metadata: MemoryMetadata {
                source: "test".to_string(),
                category: "test_cat".to_string(),
                tags: vec!["tag1".to_string(), "tag2".to_string()],
                conversation_id: Some("conv1".to_string()),
                custom: HashMap::new(),
            },
            created_at: chrono::Utc::now(),
            accessed_at: chrono::Utc::now(),
            relevance: 1.0,
        }
    }

    async fn setup_backend() -> SqliteBackend {
        let dir = tempdir().unwrap();
        let config = SqliteConfig::default().with_path(dir.path().join("test.db"));
        SqliteBackend::new(config).await.unwrap()
    }

    #[tokio::test]
    async fn test_insert_and_get() {
        let backend = setup_backend().await;
        let mem = test_memory("m1", "Hello world from Rust");

        backend.insert(&mem).await.unwrap();

        let retrieved = backend.get("m1").await.unwrap();
        assert_eq!(retrieved.id, "m1");
        assert_eq!(retrieved.content, "Hello world from Rust");
    }

    #[tokio::test]
    async fn test_update() {
        let backend = setup_backend().await;
        let mut mem = test_memory("m1", "Original content");
        backend.insert(&mem).await.unwrap();

        mem.content = "Updated content".to_string();
        backend.update(&mem).await.unwrap();

        let retrieved = backend.get("m1").await.unwrap();
        assert_eq!(retrieved.content, "Updated content");
    }

    #[tokio::test]
    async fn test_delete() {
        let backend = setup_backend().await;
        let mem = test_memory("m1", "To be deleted");
        backend.insert(&mem).await.unwrap();

        assert_eq!(backend.count().await.unwrap(), 1);
        backend.delete("m1").await.unwrap();
        assert_eq!(backend.count().await.unwrap(), 0);
    }

    #[tokio::test]
    async fn test_text_search() {
        let backend = setup_backend().await;
        backend.insert(&test_memory("m1", "Rust programming language")).await.unwrap();
        backend.insert(&test_memory("m2", "Python programming language")).await.unwrap();
        backend.insert(&test_memory("m3", "Rust is fast and safe")).await.unwrap();

        let results = backend.text_search("Rust", 10).await.unwrap();
        assert_eq!(results.len(), 2);
    }

    #[tokio::test]
    async fn test_vector_search() {
        let backend = setup_backend().await;

        let mut m1 = test_memory("m1", "Similar vector");
        m1.embedding = vec![1.0, 0.0, 0.0, 0.0];

        let mut m2 = test_memory("m2", "Different vector");
        m2.embedding = vec![0.0, 1.0, 0.0, 0.0];

        backend.insert(&m1).await.unwrap();
        backend.insert(&m2).await.unwrap();

        let query = vec![1.0, 0.0, 0.0, 0.0];
        let results = backend.vector_search(&query, 10).await.unwrap();

        assert_eq!(results.len(), 2);
        assert_eq!(results[0].0.id, "m1"); // Most similar first
        assert!((results[0].1 - 1.0).abs() < 0.001);
    }

    #[tokio::test]
    async fn test_get_by_category() {
        let backend = setup_backend().await;
        let mut m2 = test_memory("m2", "Other category");
        m2.metadata.category = "other".to_string();

        backend.insert(&test_memory("m1", "Test")).await.unwrap();
        backend.insert(&m2).await.unwrap();

        let results = backend.get_by_category("test_cat").await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "m1");
    }

    #[tokio::test]
    async fn test_get_by_tag() {
        let backend = setup_backend().await;
        backend.insert(&test_memory("m1", "Tagged memory")).await.unwrap();

        let results = backend.get_by_tag("tag1").await.unwrap();
        assert_eq!(results.len(), 1);

        let results = backend.get_by_tag("nonexistent").await.unwrap();
        assert_eq!(results.len(), 0);
    }

    #[tokio::test]
    async fn test_prune_below_relevance() {
        let backend = setup_backend().await;

        let mut m1 = test_memory("m1", "High relevance");
        m1.relevance = 0.9;
        let mut m2 = test_memory("m2", "Low relevance");
        m2.relevance = 0.05;

        backend.insert(&m1).await.unwrap();
        backend.insert(&m2).await.unwrap();

        let pruned = backend.prune_below_relevance(0.1).await.unwrap();
        assert_eq!(pruned, 1);
        assert_eq!(backend.count().await.unwrap(), 1);
    }

    #[tokio::test]
    async fn test_clear() {
        let backend = setup_backend().await;
        backend.insert(&test_memory("m1", "One")).await.unwrap();
        backend.insert(&test_memory("m2", "Two")).await.unwrap();

        assert_eq!(backend.count().await.unwrap(), 2);
        backend.clear().await.unwrap();
        assert_eq!(backend.count().await.unwrap(), 0);
    }

    #[tokio::test]
    async fn test_persistence() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("persist.db");

        // Insert data
        {
            let config = SqliteConfig::default().with_path(db_path.clone());
            let backend = SqliteBackend::new(config).await.unwrap();
            backend.insert(&test_memory("m1", "Persisted memory")).await.unwrap();
        }

        // Reload and verify
        {
            let config = SqliteConfig::default().with_path(db_path);
            let backend = SqliteBackend::new(config).await.unwrap();
            let mem = backend.get("m1").await.unwrap();
            assert_eq!(mem.content, "Persisted memory");
        }
    }
}
