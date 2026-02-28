//! # Memory System Module
//!
//! Vector-based semantic memory with LanceDB integration,
//! local embeddings, and encrypted storage.

pub mod backends;
pub mod embeddings;
pub mod encryption;
pub mod indexing;
pub mod pruning;
pub mod search;
pub mod storage;

// Re-export new submodule types
pub use backends::MemoryBackend;
pub use backends::sqlite::{SqliteBackend, SqliteConfig};

use std::path::PathBuf;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::RwLock;

/// Memory system errors
#[derive(Debug, Error)]
pub enum MemoryError {
    #[error("Storage error: {0}")]
    StorageError(String),
    #[error("Embedding error: {0}")]
    EmbeddingError(String),
    #[error("Search error: {0}")]
    SearchError(String),
    #[error("Encryption error: {0}")]
    EncryptionError(String),
    #[error("Memory not found: {0}")]
    NotFound(String),
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

/// Memory entry
#[derive(Debug, Clone)]
pub struct Memory {
    /// Unique identifier
    pub id: String,
    /// Memory content
    pub content: String,
    /// Semantic embedding vector
    pub embedding: Vec<f32>,
    /// Metadata
    pub metadata: MemoryMetadata,
    /// Creation timestamp
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Last accessed timestamp
    pub accessed_at: chrono::DateTime<chrono::Utc>,
    /// Relevance score (for decay)
    pub relevance: f32,
}

/// Memory metadata
#[derive(Debug, Clone, Default)]
pub struct MemoryMetadata {
    /// Source of the memory
    pub source: String,
    /// Category/type
    pub category: String,
    /// Tags
    pub tags: Vec<String>,
    /// Associated conversation ID
    pub conversation_id: Option<String>,
    /// Custom key-value pairs
    pub custom: std::collections::HashMap<String, String>,
}

/// Memory system configuration
#[derive(Debug, Clone)]
pub struct MemoryConfig {
    /// Storage directory
    pub storage_path: PathBuf,
    /// Embedding model name
    pub embedding_model: String,
    /// Embedding dimension
    pub embedding_dim: usize,
    /// Maximum memories to keep
    pub max_memories: usize,
    /// Enable encryption
    pub enable_encryption: bool,
    /// Encryption key (32 bytes for AES-256)
    pub encryption_key: Option<[u8; 32]>,
    /// Relevance decay rate (per day)
    pub decay_rate: f32,
    /// Minimum relevance before pruning
    pub min_relevance: f32,
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            storage_path: PathBuf::from(".dx/memory"),
            embedding_model: "all-MiniLM-L6-v2".to_string(),
            embedding_dim: 384, // MiniLM output dimension
            max_memories: 100_000,
            enable_encryption: true,
            encryption_key: None,
            decay_rate: 0.01,
            min_relevance: 0.1,
        }
    }
}

/// Memory system
pub struct MemorySystem {
    /// Configuration
    config: MemoryConfig,
    /// Storage backend
    storage: Arc<RwLock<storage::MemoryStorage>>,
    /// Embedding generator
    embedder: embeddings::EmbeddingGenerator,
    /// Search engine
    searcher: search::SemanticSearch,
    /// Encryption handler
    encryption: Option<encryption::MemoryEncryption>,
    /// Pruning handler
    pruner: pruning::MemoryPruner,
}

impl MemorySystem {
    /// Create a new memory system
    pub async fn new(config: MemoryConfig) -> Result<Self, MemoryError> {
        // Initialize storage
        let storage = storage::MemoryStorage::new(&config.storage_path).await?;

        // Initialize encryption if enabled
        let encryption = if config.enable_encryption {
            let key = config.encryption_key.unwrap_or_else(|| {
                // Generate random key if not provided
                let mut key = [0u8; 32];
                getrandom::getrandom(&mut key).ok();
                key
            });
            Some(encryption::MemoryEncryption::new(key))
        } else {
            None
        };

        Ok(Self {
            embedder: embeddings::EmbeddingGenerator::new(
                &config.embedding_model,
                config.embedding_dim,
            ),
            searcher: search::SemanticSearch::new(config.embedding_dim),
            pruner: pruning::MemoryPruner::new(config.decay_rate, config.min_relevance),
            storage: Arc::new(RwLock::new(storage)),
            encryption,
            config,
        })
    }

    /// Store a new memory
    pub async fn store(
        &self,
        content: &str,
        metadata: MemoryMetadata,
    ) -> Result<String, MemoryError> {
        // Generate embedding
        let embedding = self.embedder.generate(content).await?;

        // Create memory entry
        let now = chrono::Utc::now();
        let id = generate_id();

        // Encrypt content if enabled
        let stored_content = if let Some(ref enc) = self.encryption {
            enc.encrypt(content)?
        } else {
            content.to_string()
        };

        let memory = Memory {
            id: id.clone(),
            content: stored_content,
            embedding,
            metadata,
            created_at: now,
            accessed_at: now,
            relevance: 1.0,
        };

        // Store memory
        let mut storage = self.storage.write().await;
        storage.insert(memory).await?;

        // Check if pruning is needed
        if storage.count().await > self.config.max_memories {
            let to_prune = self.pruner.identify_prunable(storage.iter().await).await;
            for id in to_prune {
                storage.delete(&id).await?;
            }
        }

        Ok(id)
    }

    /// Retrieve a memory by ID
    pub async fn get(&self, id: &str) -> Result<Memory, MemoryError> {
        let mut storage = self.storage.write().await;
        let mut memory = storage.get(id).await?;

        // Update access time
        memory.accessed_at = chrono::Utc::now();
        storage.update(&memory).await?;

        // Decrypt content if encrypted
        if let Some(ref enc) = self.encryption {
            memory.content = enc.decrypt(&memory.content)?;
        }

        Ok(memory)
    }

    /// Semantic search for relevant memories
    pub async fn search(&self, query: &str, limit: usize) -> Result<Vec<Memory>, MemoryError> {
        // Generate query embedding
        let query_embedding = self.embedder.generate(query).await?;

        // Get all memories for search
        let storage = self.storage.read().await;
        let all_memories = storage.iter().await;

        // Perform semantic search
        let mut results = self.searcher.search(&query_embedding, all_memories, limit).await;

        // Decrypt results if needed
        if let Some(ref enc) = self.encryption {
            for memory in &mut results {
                memory.content = enc.decrypt(&memory.content)?;
            }
        }

        // Update relevance scores
        drop(storage);
        let mut storage = self.storage.write().await;
        for memory in &results {
            let mut updated = memory.clone();
            updated.relevance = (updated.relevance + 0.1).min(1.0);
            updated.accessed_at = chrono::Utc::now();
            storage.update(&updated).await?;
        }

        Ok(results)
    }

    /// Delete a memory
    pub async fn delete(&self, id: &str) -> Result<(), MemoryError> {
        let mut storage = self.storage.write().await;
        storage.delete(id).await
    }

    /// Update memory metadata
    pub async fn update_metadata(
        &self,
        id: &str,
        metadata: MemoryMetadata,
    ) -> Result<(), MemoryError> {
        let mut storage = self.storage.write().await;
        let mut memory = storage.get(id).await?;
        memory.metadata = metadata;
        storage.update(&memory).await
    }

    /// Decay relevance scores and prune old memories
    pub async fn maintenance(&self) -> Result<PruneStats, MemoryError> {
        let mut storage = self.storage.write().await;
        let memories = storage.iter().await;

        // Apply decay to all memories
        let mut decayed_count = 0;
        let mut pruned_count = 0;

        for mut memory in memories {
            let age_days = (chrono::Utc::now() - memory.accessed_at).num_days() as f32;
            let decay_factor = (-self.config.decay_rate * age_days).exp();
            memory.relevance *= decay_factor;

            if memory.relevance < self.config.min_relevance {
                storage.delete(&memory.id).await?;
                pruned_count += 1;
            } else {
                storage.update(&memory).await?;
                decayed_count += 1;
            }
        }

        Ok(PruneStats {
            decayed: decayed_count,
            pruned: pruned_count,
            remaining: storage.count().await,
        })
    }

    /// Get memory statistics
    pub async fn stats(&self) -> MemoryStats {
        let storage = self.storage.read().await;
        let count = storage.count().await;
        let memories = storage.iter().await;

        let mut total_relevance = 0.0;
        let mut by_category: std::collections::HashMap<String, usize> =
            std::collections::HashMap::new();

        for memory in memories {
            total_relevance += memory.relevance;
            *by_category.entry(memory.metadata.category.clone()).or_insert(0) += 1;
        }

        MemoryStats {
            total_memories: count,
            avg_relevance: if count > 0 {
                total_relevance / count as f32
            } else {
                0.0
            },
            by_category,
            storage_path: self.config.storage_path.clone(),
            encrypted: self.config.enable_encryption,
        }
    }
}

/// Memory pruning statistics
#[derive(Debug, Clone)]
pub struct PruneStats {
    /// Memories with decayed relevance
    pub decayed: usize,
    /// Memories pruned
    pub pruned: usize,
    /// Remaining memories
    pub remaining: usize,
}

/// Memory system statistics
#[derive(Debug, Clone)]
pub struct MemoryStats {
    /// Total memory count
    pub total_memories: usize,
    /// Average relevance score
    pub avg_relevance: f32,
    /// Memories by category
    pub by_category: std::collections::HashMap<String, usize>,
    /// Storage path
    pub storage_path: PathBuf,
    /// Whether encryption is enabled
    pub encrypted: bool,
}

/// Generate unique ID
fn generate_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
    format!("mem_{:x}", timestamp)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_id() {
        let id1 = generate_id();
        let id2 = generate_id();

        assert!(id1.starts_with("mem_"));
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_default_config() {
        let config = MemoryConfig::default();

        assert_eq!(config.embedding_dim, 384);
        assert!(config.enable_encryption);
        assert_eq!(config.max_memories, 100_000);
    }
}
