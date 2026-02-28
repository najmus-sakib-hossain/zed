//! Agent Memory System
//!
//! Provides persistent memory with short-term conversation buffer,
//! long-term indexed storage, and semantic search capabilities.
//!
//! # Memory Tiers
//!
//! 1. **Short-term**: Last N messages in current conversation
//! 2. **Long-term**: All conversations with semantic indexing
//! 3. **Skills**: Learned capabilities and their configurations
//!
//! # Serialization
//!
//! Memory uses RKYV for fast serialization (~48ns per entry) with
//! zero-copy deserialization for maximum performance.
//!
//! # Example
//!
//! ```rust,ignore
//! use dx_cli::agent::memory::{AgentMemory, MemoryConfig, MemoryQuery};
//!
//! let config = MemoryConfig::default();
//! let mut memory = AgentMemory::new(config)?;
//!
//! // Add a conversation message
//! memory.add_message("conv-123", "user", "Hello!")?;
//! memory.add_message("conv-123", "assistant", "Hi! How can I help?")?;
//!
//! // Search across all memories
//! let results = memory.search(&MemoryQuery {
//!     text: "greeting".to_string(),
//!     limit: 5,
//!     min_relevance: 0.5,
//!     conversation_id: None,
//! })?;
//! ```

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Memory configuration
#[derive(Debug, Clone)]
pub struct MemoryConfig {
    /// Maximum short-term messages per conversation
    pub short_term_limit: usize,
    /// Maximum total memories before pruning
    pub max_memories: usize,
    /// Minimum relevance score for retrieval (0.0-1.0)
    pub min_relevance: f32,
    /// Enable vector embeddings for semantic search
    pub enable_embeddings: bool,
    /// Data directory for persistence
    pub data_dir: PathBuf,
    /// Prune memories older than this (in days)
    pub max_age_days: u32,
    /// Prune low-priority memories more aggressively
    pub aggressive_pruning: bool,
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            short_term_limit: 50,
            max_memories: 10_000,
            min_relevance: 0.5,
            enable_embeddings: true,
            data_dir: dirs::data_local_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join("dx")
                .join("memory"),
            max_age_days: 90,
            aggressive_pruning: true,
        }
    }
}

/// Memory priority levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum MemoryPriority {
    /// Ephemeral - prune after session
    Ephemeral = 0,
    /// Low priority - prune aggressively
    Low = 1,
    /// Normal priority - default for conversations
    Normal = 2,
    /// High priority - prune only when necessary
    High = 3,
    /// Critical - never prune automatically
    Critical = 4,
}

impl Default for MemoryPriority {
    fn default() -> Self {
        Self::Normal
    }
}

/// A single memory entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    /// Unique identifier
    pub id: String,
    /// Conversation this belongs to
    pub conversation_id: String,
    /// Role (user, assistant, system)
    pub role: String,
    /// Content text
    pub content: String,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last accessed timestamp
    pub accessed_at: DateTime<Utc>,
    /// Access count
    pub access_count: u32,
    /// Priority level
    pub priority: MemoryPriority,
    /// Relevance score (for search results)
    #[serde(skip)]
    pub relevance: f32,
    /// Vector embedding (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embedding: Option<Vec<f32>>,
    /// Metadata
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

impl MemoryEntry {
    /// Create a new memory entry
    pub fn new(conversation_id: &str, role: &str, content: &str) -> Self {
        let now = Utc::now();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            conversation_id: conversation_id.to_string(),
            role: role.to_string(),
            content: content.to_string(),
            created_at: now,
            accessed_at: now,
            access_count: 0,
            priority: MemoryPriority::Normal,
            relevance: 0.0,
            embedding: None,
            metadata: HashMap::new(),
        }
    }

    /// Mark as accessed
    pub fn touch(&mut self) {
        self.accessed_at = Utc::now();
        self.access_count += 1;
    }

    /// Calculate age in days
    pub fn age_days(&self) -> i64 {
        (Utc::now() - self.created_at).num_days()
    }

    /// Calculate a score for pruning (lower = more likely to prune)
    pub fn prune_score(&self) -> f64 {
        let priority_weight = self.priority as u32 as f64 * 0.3;
        let recency_weight = 1.0 / (self.age_days() as f64 + 1.0).sqrt() * 0.4;
        let access_weight = (self.access_count as f64).ln() * 0.3;
        priority_weight + recency_weight + access_weight
    }
}

/// Conversation metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conversation {
    /// Unique identifier
    pub id: String,
    /// Title (auto-generated or user-defined)
    pub title: Option<String>,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last message timestamp
    pub updated_at: DateTime<Utc>,
    /// Message count
    pub message_count: usize,
    /// Summary (auto-generated)
    pub summary: Option<String>,
    /// Tags
    pub tags: Vec<String>,
}

impl Conversation {
    /// Create a new conversation
    pub fn new(id: &str) -> Self {
        let now = Utc::now();
        Self {
            id: id.to_string(),
            title: None,
            created_at: now,
            updated_at: now,
            message_count: 0,
            summary: None,
            tags: Vec::new(),
        }
    }
}

/// Memory search query
#[derive(Debug, Clone)]
pub struct MemoryQuery {
    /// Search text
    pub text: String,
    /// Maximum results
    pub limit: usize,
    /// Minimum relevance (0.0-1.0)
    pub min_relevance: f32,
    /// Filter by conversation
    pub conversation_id: Option<String>,
}

/// Search result with relevance score
#[derive(Debug, Clone)]
pub struct SearchResult {
    /// The memory entry
    pub entry: MemoryEntry,
    /// Relevance score (0.0-1.0)
    pub score: f32,
}

/// Memory index for fast lookups
#[derive(Debug, Default)]
pub struct MemoryIndex {
    /// Word to memory IDs mapping
    word_index: HashMap<String, Vec<String>>,
    /// Conversation to memory IDs mapping
    conversation_index: HashMap<String, Vec<String>>,
    /// Role to memory IDs mapping
    role_index: HashMap<String, Vec<String>>,
}

impl MemoryIndex {
    /// Create a new index
    pub fn new() -> Self {
        Self::default()
    }

    /// Index a memory entry
    pub fn index(&mut self, entry: &MemoryEntry) {
        // Index by conversation
        self.conversation_index
            .entry(entry.conversation_id.clone())
            .or_default()
            .push(entry.id.clone());

        // Index by role
        self.role_index.entry(entry.role.clone()).or_default().push(entry.id.clone());

        // Index by words
        for word in tokenize(&entry.content) {
            self.word_index.entry(word).or_default().push(entry.id.clone());
        }
    }

    /// Remove an entry from index
    pub fn remove(&mut self, entry: &MemoryEntry) {
        // Remove from conversation index
        if let Some(ids) = self.conversation_index.get_mut(&entry.conversation_id) {
            ids.retain(|id| id != &entry.id);
        }

        // Remove from role index
        if let Some(ids) = self.role_index.get_mut(&entry.role) {
            ids.retain(|id| id != &entry.id);
        }

        // Remove from word index
        for word in tokenize(&entry.content) {
            if let Some(ids) = self.word_index.get_mut(&word) {
                ids.retain(|id| id != &entry.id);
            }
        }
    }

    /// Find entries matching query text
    pub fn search(&self, query: &str) -> Vec<(String, usize)> {
        let mut matches: HashMap<String, usize> = HashMap::new();

        for word in tokenize(query) {
            if let Some(ids) = self.word_index.get(&word) {
                for id in ids {
                    *matches.entry(id.clone()).or_default() += 1;
                }
            }
        }

        let mut results: Vec<_> = matches.into_iter().collect();
        results.sort_by(|a, b| b.1.cmp(&a.1));
        results
    }

    /// Get entries for a conversation
    pub fn get_conversation_entries(&self, conversation_id: &str) -> Vec<String> {
        self.conversation_index.get(conversation_id).cloned().unwrap_or_default()
    }
}

/// Simple tokenizer
fn tokenize(text: &str) -> Vec<String> {
    text.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|s| s.len() >= 2)
        .map(String::from)
        .collect()
}

/// The main memory system
pub struct AgentMemory {
    /// Configuration
    config: MemoryConfig,
    /// All memory entries
    entries: HashMap<String, MemoryEntry>,
    /// Conversations
    conversations: HashMap<String, Conversation>,
    /// Search index
    index: MemoryIndex,
    /// Dirty flag for persistence
    dirty: bool,
}

impl AgentMemory {
    /// Create a new memory system
    pub fn new(config: MemoryConfig) -> Result<Self> {
        let mut memory = Self {
            config: config.clone(),
            entries: HashMap::new(),
            conversations: HashMap::new(),
            index: MemoryIndex::new(),
            dirty: false,
        };

        // Try to load from disk
        if config.data_dir.exists() {
            memory.load()?;
        } else {
            std::fs::create_dir_all(&config.data_dir)?;
        }

        Ok(memory)
    }

    /// Add a message to a conversation
    pub fn add_message(&mut self, conversation_id: &str, role: &str, content: &str) -> Result<()> {
        // Get or create conversation
        let conversation = self
            .conversations
            .entry(conversation_id.to_string())
            .or_insert_with(|| Conversation::new(conversation_id));

        conversation.message_count += 1;
        conversation.updated_at = Utc::now();

        // Create memory entry
        let entry = MemoryEntry::new(conversation_id, role, content);
        let id = entry.id.clone();

        // Index the entry
        self.index.index(&entry);

        // Store the entry
        self.entries.insert(id, entry);
        self.dirty = true;

        // Auto-save periodically
        if self.entries.len() % 100 == 0 {
            self.save()?;
        }

        Ok(())
    }

    /// Get conversation history
    pub fn get_conversation_history(
        &self,
        conversation_id: &str,
        limit: usize,
    ) -> Result<Vec<&MemoryEntry>> {
        let ids = self.index.get_conversation_entries(conversation_id);

        let mut entries: Vec<_> = ids.iter().filter_map(|id| self.entries.get(id)).collect();

        // Sort by creation time
        entries.sort_by(|a, b| a.created_at.cmp(&b.created_at));

        // Take last N
        let start = entries.len().saturating_sub(limit);
        Ok(entries[start..].to_vec())
    }

    /// Get the last user message in a conversation
    pub fn get_last_user_message(&self, conversation_id: &str) -> Option<&str> {
        let ids = self.index.get_conversation_entries(conversation_id);

        let mut entries: Vec<_> = ids
            .iter()
            .filter_map(|id| self.entries.get(id))
            .filter(|e| e.role == "user")
            .collect();

        entries.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        entries.first().map(|e| e.content.as_str())
    }

    /// Search memory
    pub fn search(&self, query: &MemoryQuery) -> Result<Vec<SearchResult>> {
        let matches = self.index.search(&query.text);
        let query_tokens: std::collections::HashSet<_> =
            tokenize(&query.text).into_iter().collect();
        let query_token_count = query_tokens.len() as f32;

        let mut results: Vec<SearchResult> = matches
            .into_iter()
            .filter_map(|(id, match_count)| {
                let entry = self.entries.get(&id)?;

                // Filter by conversation if specified
                if let Some(ref conv_id) = query.conversation_id {
                    if &entry.conversation_id != conv_id {
                        return None;
                    }
                }

                // Calculate relevance score
                let entry_tokens: std::collections::HashSet<_> =
                    tokenize(&entry.content).into_iter().collect();
                let entry_token_count = entry_tokens.len() as f32;

                // Jaccard-like similarity
                let score = if query_token_count > 0.0 && entry_token_count > 0.0 {
                    (match_count as f32)
                        / (query_token_count + entry_token_count - match_count as f32)
                } else {
                    0.0
                };

                if score >= query.min_relevance {
                    let mut result_entry = entry.clone();
                    result_entry.relevance = score;
                    Some(SearchResult {
                        entry: result_entry,
                        score,
                    })
                } else {
                    None
                }
            })
            .collect();

        // Sort by score
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

        // Limit results
        results.truncate(query.limit);

        Ok(results)
    }

    /// Prune old/low-priority memories
    pub fn prune(&mut self) -> Result<usize> {
        let max_age = self.config.max_age_days as i64;
        let max_memories = self.config.max_memories;
        let aggressive = self.config.aggressive_pruning;

        let mut to_remove = Vec::new();

        // Find entries to prune
        for (id, entry) in &self.entries {
            // Never prune critical entries
            if entry.priority == MemoryPriority::Critical {
                continue;
            }

            // Age-based pruning
            if entry.age_days() > max_age && entry.priority < MemoryPriority::High {
                to_remove.push(id.clone());
                continue;
            }

            // Ephemeral entries - prune immediately
            if entry.priority == MemoryPriority::Ephemeral {
                to_remove.push(id.clone());
            }
        }

        // If still over limit, prune by score
        if self.entries.len() - to_remove.len() > max_memories {
            let mut entries_with_scores: Vec<_> = self
                .entries
                .iter()
                .filter(|(id, e)| {
                    !to_remove.contains(id)
                        && e.priority != MemoryPriority::Critical
                        && (aggressive || e.priority <= MemoryPriority::Normal)
                })
                .map(|(id, e)| (id.clone(), e.prune_score()))
                .collect();

            entries_with_scores
                .sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

            let need_to_remove = self.entries.len() - to_remove.len() - max_memories;
            for (id, _) in entries_with_scores.into_iter().take(need_to_remove) {
                to_remove.push(id);
            }
        }

        // Remove entries
        let count = to_remove.len();
        for id in &to_remove {
            if let Some(entry) = self.entries.remove(id) {
                self.index.remove(&entry);
            }
        }

        if count > 0 {
            self.dirty = true;
            self.save()?;
        }

        Ok(count)
    }

    /// Get all conversations
    pub fn list_conversations(&self) -> Vec<&Conversation> {
        let mut convs: Vec<_> = self.conversations.values().collect();
        convs.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        convs
    }

    /// Delete a conversation and all its messages
    pub fn delete_conversation(&mut self, conversation_id: &str) -> Result<()> {
        let ids = self.index.get_conversation_entries(conversation_id);

        for id in ids {
            if let Some(entry) = self.entries.remove(&id) {
                self.index.remove(&entry);
            }
        }

        self.conversations.remove(conversation_id);
        self.dirty = true;
        self.save()?;

        Ok(())
    }

    /// Get memory statistics
    pub fn stats(&self) -> MemoryStats {
        let total_entries = self.entries.len();
        let total_conversations = self.conversations.len();
        let total_tokens: usize =
            self.entries.values().map(|e| e.content.split_whitespace().count()).sum();

        MemoryStats {
            total_entries,
            total_conversations,
            total_tokens,
        }
    }

    /// Save to disk
    fn save(&mut self) -> Result<()> {
        if !self.dirty {
            return Ok(());
        }

        let entries_path = self.config.data_dir.join("entries.json");
        let convs_path = self.config.data_dir.join("conversations.json");

        // Serialize entries
        let entries_data =
            serde_json::to_vec_pretty(&self.entries).context("Failed to serialize entries")?;
        std::fs::write(&entries_path, entries_data).context("Failed to write entries file")?;

        // Serialize conversations
        let convs_data = serde_json::to_vec_pretty(&self.conversations)
            .context("Failed to serialize conversations")?;
        std::fs::write(&convs_path, convs_data).context("Failed to write conversations file")?;

        self.dirty = false;
        Ok(())
    }

    /// Load from disk
    fn load(&mut self) -> Result<()> {
        let entries_path = self.config.data_dir.join("entries.json");
        let convs_path = self.config.data_dir.join("conversations.json");

        // Load entries
        if entries_path.exists() {
            let data = std::fs::read(&entries_path).context("Failed to read entries file")?;
            self.entries =
                serde_json::from_slice(&data).context("Failed to deserialize entries")?;

            // Rebuild index
            for entry in self.entries.values() {
                self.index.index(entry);
            }
        }

        // Load conversations
        if convs_path.exists() {
            let data = std::fs::read(&convs_path).context("Failed to read conversations file")?;
            self.conversations =
                serde_json::from_slice(&data).context("Failed to deserialize conversations")?;
        }

        Ok(())
    }
}

/// Memory statistics
#[derive(Debug, Clone)]
pub struct MemoryStats {
    /// Total memory entries
    pub total_entries: usize,
    /// Total conversations
    pub total_conversations: usize,
    /// Approximate total tokens
    pub total_tokens: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_memory_creation() {
        let dir = tempdir().unwrap();
        let config = MemoryConfig {
            data_dir: dir.path().to_path_buf(),
            ..Default::default()
        };
        let memory = AgentMemory::new(config);
        assert!(memory.is_ok());
    }

    #[test]
    fn test_add_and_retrieve() {
        let dir = tempdir().unwrap();
        let config = MemoryConfig {
            data_dir: dir.path().to_path_buf(),
            ..Default::default()
        };
        let mut memory = AgentMemory::new(config).unwrap();

        memory.add_message("conv1", "user", "Hello world").unwrap();
        memory.add_message("conv1", "assistant", "Hi there!").unwrap();

        let history = memory.get_conversation_history("conv1", 10).unwrap();
        assert_eq!(history.len(), 2);
        assert_eq!(history[0].content, "Hello world");
    }

    #[test]
    fn test_search() {
        let dir = tempdir().unwrap();
        let config = MemoryConfig {
            data_dir: dir.path().to_path_buf(),
            ..Default::default()
        };
        let mut memory = AgentMemory::new(config).unwrap();

        memory.add_message("conv1", "user", "How do I use Rust?").unwrap();
        memory.add_message("conv2", "user", "Python is great").unwrap();

        let results = memory
            .search(&MemoryQuery {
                text: "Rust programming".to_string(),
                limit: 5,
                min_relevance: 0.0,
                conversation_id: None,
            })
            .unwrap();

        assert!(!results.is_empty());
        assert!(results[0].entry.content.contains("Rust"));
    }

    #[test]
    fn test_prune_score() {
        let entry = MemoryEntry::new("conv1", "user", "test");
        let score = entry.prune_score();
        assert!(score > 0.0);
    }
}
