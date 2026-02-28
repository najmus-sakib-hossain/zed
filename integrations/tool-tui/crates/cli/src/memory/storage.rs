//! Memory Storage
//!
//! Persistent storage backend for memories using LanceDB-compatible format.

use super::{Memory, MemoryError, MemoryMetadata};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::sync::RwLock;

/// Memory storage backend
pub struct MemoryStorage {
    /// Storage directory
    path: PathBuf,
    /// In-memory cache
    cache: RwLock<HashMap<String, Memory>>,
    /// Index for fast lookups
    index: RwLock<StorageIndex>,
}

/// Storage index for fast lookups
#[derive(Default)]
struct StorageIndex {
    /// IDs by category
    by_category: HashMap<String, Vec<String>>,
    /// IDs by tag
    by_tag: HashMap<String, Vec<String>>,
    /// IDs by conversation
    by_conversation: HashMap<String, Vec<String>>,
}

impl MemoryStorage {
    /// Create a new storage backend
    pub async fn new(path: &Path) -> Result<Self, MemoryError> {
        // Create directory if it doesn't exist
        if !path.exists() {
            std::fs::create_dir_all(path)?;
        }

        let storage = Self {
            path: path.to_path_buf(),
            cache: RwLock::new(HashMap::new()),
            index: RwLock::new(StorageIndex::default()),
        };

        // Load existing memories
        storage.load_from_disk().await?;

        Ok(storage)
    }

    /// Load memories from disk
    async fn load_from_disk(&self) -> Result<(), MemoryError> {
        let data_file = self.path.join("memories.json");

        if data_file.exists() {
            let content = std::fs::read_to_string(&data_file)?;
            let memories: Vec<SerializedMemory> = serde_json::from_str(&content).map_err(|e| {
                MemoryError::StorageError(format!("Failed to parse memories: {}", e))
            })?;

            let mut cache = self.cache.write().await;
            let mut index = self.index.write().await;

            for mem in memories {
                let memory = mem.into_memory();

                // Update index
                index
                    .by_category
                    .entry(memory.metadata.category.clone())
                    .or_default()
                    .push(memory.id.clone());

                for tag in &memory.metadata.tags {
                    index.by_tag.entry(tag.clone()).or_default().push(memory.id.clone());
                }

                if let Some(ref conv_id) = memory.metadata.conversation_id {
                    index
                        .by_conversation
                        .entry(conv_id.clone())
                        .or_default()
                        .push(memory.id.clone());
                }

                cache.insert(memory.id.clone(), memory);
            }
        }

        Ok(())
    }

    /// Save memories to disk
    async fn save_to_disk(&self) -> Result<(), MemoryError> {
        let cache = self.cache.read().await;
        let memories: Vec<SerializedMemory> =
            cache.values().map(SerializedMemory::from_memory).collect();

        let content = serde_json::to_string_pretty(&memories)
            .map_err(|e| MemoryError::StorageError(format!("Failed to serialize: {}", e)))?;

        let data_file = self.path.join("memories.json");
        std::fs::write(&data_file, content)?;

        Ok(())
    }

    /// Insert a memory
    pub async fn insert(&mut self, memory: Memory) -> Result<(), MemoryError> {
        let id = memory.id.clone();

        // Update index
        {
            let mut index = self.index.write().await;

            index
                .by_category
                .entry(memory.metadata.category.clone())
                .or_default()
                .push(id.clone());

            for tag in &memory.metadata.tags {
                index.by_tag.entry(tag.clone()).or_default().push(id.clone());
            }

            if let Some(ref conv_id) = memory.metadata.conversation_id {
                index.by_conversation.entry(conv_id.clone()).or_default().push(id.clone());
            }
        }

        // Insert into cache
        {
            let mut cache = self.cache.write().await;
            cache.insert(id, memory);
        }

        // Persist to disk
        self.save_to_disk().await
    }

    /// Get a memory by ID
    pub async fn get(&self, id: &str) -> Result<Memory, MemoryError> {
        let cache = self.cache.read().await;
        cache.get(id).cloned().ok_or_else(|| MemoryError::NotFound(id.to_string()))
    }

    /// Update a memory
    pub async fn update(&mut self, memory: &Memory) -> Result<(), MemoryError> {
        {
            let mut cache = self.cache.write().await;
            if !cache.contains_key(&memory.id) {
                return Err(MemoryError::NotFound(memory.id.clone()));
            }
            cache.insert(memory.id.clone(), memory.clone());
        }

        self.save_to_disk().await
    }

    /// Delete a memory
    pub async fn delete(&mut self, id: &str) -> Result<(), MemoryError> {
        // Remove from cache
        let memory = {
            let mut cache = self.cache.write().await;
            cache.remove(id)
        };

        if let Some(memory) = memory {
            // Update index
            let mut index = self.index.write().await;

            if let Some(ids) = index.by_category.get_mut(&memory.metadata.category) {
                ids.retain(|x| x != id);
            }

            for tag in &memory.metadata.tags {
                if let Some(ids) = index.by_tag.get_mut(tag) {
                    ids.retain(|x| x != id);
                }
            }

            if let Some(ref conv_id) = memory.metadata.conversation_id {
                if let Some(ids) = index.by_conversation.get_mut(conv_id) {
                    ids.retain(|x| x != id);
                }
            }

            drop(index);
            self.save_to_disk().await
        } else {
            Err(MemoryError::NotFound(id.to_string()))
        }
    }

    /// Get memory count
    pub async fn count(&self) -> usize {
        self.cache.read().await.len()
    }

    /// Iterate over all memories
    pub async fn iter(&self) -> Vec<Memory> {
        self.cache.read().await.values().cloned().collect()
    }

    /// Get memories by category
    pub async fn get_by_category(&self, category: &str) -> Vec<Memory> {
        let index = self.index.read().await;
        let cache = self.cache.read().await;

        index
            .by_category
            .get(category)
            .map(|ids| ids.iter().filter_map(|id| cache.get(id).cloned()).collect())
            .unwrap_or_default()
    }

    /// Get memories by tag
    pub async fn get_by_tag(&self, tag: &str) -> Vec<Memory> {
        let index = self.index.read().await;
        let cache = self.cache.read().await;

        index
            .by_tag
            .get(tag)
            .map(|ids| ids.iter().filter_map(|id| cache.get(id).cloned()).collect())
            .unwrap_or_default()
    }

    /// Get memories by conversation
    pub async fn get_by_conversation(&self, conversation_id: &str) -> Vec<Memory> {
        let index = self.index.read().await;
        let cache = self.cache.read().await;

        index
            .by_conversation
            .get(conversation_id)
            .map(|ids| ids.iter().filter_map(|id| cache.get(id).cloned()).collect())
            .unwrap_or_default()
    }

    /// Clear all memories
    pub async fn clear(&mut self) -> Result<(), MemoryError> {
        {
            let mut cache = self.cache.write().await;
            cache.clear();
        }
        {
            let mut index = self.index.write().await;
            index.by_category.clear();
            index.by_tag.clear();
            index.by_conversation.clear();
        }

        self.save_to_disk().await
    }
}

/// Serialized memory for JSON storage
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

    async fn create_test_storage() -> MemoryStorage {
        let dir = tempdir().unwrap();
        MemoryStorage::new(dir.path()).await.unwrap()
    }

    fn create_test_memory(id: &str) -> Memory {
        Memory {
            id: id.to_string(),
            content: "Test content".to_string(),
            embedding: vec![0.1, 0.2, 0.3],
            metadata: MemoryMetadata {
                source: "test".to_string(),
                category: "test_category".to_string(),
                tags: vec!["tag1".to_string(), "tag2".to_string()],
                conversation_id: Some("conv1".to_string()),
                custom: HashMap::new(),
            },
            created_at: chrono::Utc::now(),
            accessed_at: chrono::Utc::now(),
            relevance: 1.0,
        }
    }

    #[tokio::test]
    async fn test_insert_and_get() {
        let mut storage = create_test_storage().await;
        let memory = create_test_memory("test1");

        storage.insert(memory.clone()).await.unwrap();

        let retrieved = storage.get("test1").await.unwrap();
        assert_eq!(retrieved.id, "test1");
        assert_eq!(retrieved.content, "Test content");
    }

    #[tokio::test]
    async fn test_update() {
        let mut storage = create_test_storage().await;
        let mut memory = create_test_memory("test1");

        storage.insert(memory.clone()).await.unwrap();

        memory.content = "Updated content".to_string();
        storage.update(&memory).await.unwrap();

        let retrieved = storage.get("test1").await.unwrap();
        assert_eq!(retrieved.content, "Updated content");
    }

    #[tokio::test]
    async fn test_delete() {
        let mut storage = create_test_storage().await;
        let memory = create_test_memory("test1");

        storage.insert(memory).await.unwrap();
        assert_eq!(storage.count().await, 1);

        storage.delete("test1").await.unwrap();
        assert_eq!(storage.count().await, 0);
    }

    #[tokio::test]
    async fn test_get_by_category() {
        let mut storage = create_test_storage().await;

        let mem1 = create_test_memory("test1");
        let mut mem2 = create_test_memory("test2");
        mem2.metadata.category = "other_category".to_string();

        storage.insert(mem1).await.unwrap();
        storage.insert(mem2).await.unwrap();

        let results = storage.get_by_category("test_category").await;
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "test1");
    }

    #[tokio::test]
    async fn test_get_by_tag() {
        let mut storage = create_test_storage().await;
        let memory = create_test_memory("test1");

        storage.insert(memory).await.unwrap();

        let results = storage.get_by_tag("tag1").await;
        assert_eq!(results.len(), 1);

        let results = storage.get_by_tag("nonexistent").await;
        assert_eq!(results.len(), 0);
    }

    #[tokio::test]
    async fn test_persistence() {
        let dir = tempdir().unwrap();

        // Create storage and add memory
        {
            let mut storage = MemoryStorage::new(dir.path()).await.unwrap();
            let memory = create_test_memory("test1");
            storage.insert(memory).await.unwrap();
        }

        // Reload storage and verify memory exists
        {
            let storage = MemoryStorage::new(dir.path()).await.unwrap();
            let retrieved = storage.get("test1").await.unwrap();
            assert_eq!(retrieved.id, "test1");
        }
    }
}
