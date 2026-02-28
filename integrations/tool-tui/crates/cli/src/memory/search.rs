//! Semantic Search
//!
//! Vector-based semantic search for finding relevant memories.

use super::{Memory, embeddings::EmbeddingGenerator};

/// Semantic search engine
pub struct SemanticSearch {
    /// Embedding dimension
    dimension: usize,
}

impl SemanticSearch {
    /// Create a new semantic search engine
    pub fn new(dimension: usize) -> Self {
        Self { dimension }
    }

    /// Search for similar memories
    pub async fn search(
        &self,
        query_embedding: &[f32],
        memories: Vec<Memory>,
        limit: usize,
    ) -> Vec<Memory> {
        let mut scored: Vec<(Memory, f32)> = memories
            .into_iter()
            .map(|mem| {
                let score = EmbeddingGenerator::cosine_similarity(query_embedding, &mem.embedding);
                // Boost by relevance score
                let boosted_score = score * (0.5 + 0.5 * mem.relevance);
                (mem, boosted_score)
            })
            .collect();

        // Sort by score descending
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Take top results
        scored.into_iter().take(limit).map(|(mem, _)| mem).collect()
    }

    /// Search with filters
    pub async fn search_filtered(
        &self,
        query_embedding: &[f32],
        memories: Vec<Memory>,
        filters: SearchFilters,
        limit: usize,
    ) -> Vec<Memory> {
        let filtered: Vec<Memory> = memories
            .into_iter()
            .filter(|mem| {
                // Category filter
                if let Some(ref cat) = filters.category {
                    if mem.metadata.category != *cat {
                        return false;
                    }
                }

                // Tag filter
                if let Some(ref tags) = filters.tags {
                    let has_all_tags = tags.iter().all(|t| mem.metadata.tags.contains(t));
                    if !has_all_tags {
                        return false;
                    }
                }

                // Date filter
                if let Some(after) = filters.created_after {
                    if mem.created_at < after {
                        return false;
                    }
                }

                if let Some(before) = filters.created_before {
                    if mem.created_at > before {
                        return false;
                    }
                }

                // Relevance filter
                if let Some(min_relevance) = filters.min_relevance {
                    if mem.relevance < min_relevance {
                        return false;
                    }
                }

                true
            })
            .collect();

        self.search(query_embedding, filtered, limit).await
    }

    /// Hybrid search combining semantic and keyword matching
    pub async fn hybrid_search(
        &self,
        query: &str,
        query_embedding: &[f32],
        memories: Vec<Memory>,
        limit: usize,
    ) -> Vec<Memory> {
        let keywords = extract_keywords(query);

        let mut scored: Vec<(Memory, f32)> = memories
            .into_iter()
            .map(|mem| {
                // Semantic score (0-1)
                let semantic_score =
                    EmbeddingGenerator::cosine_similarity(query_embedding, &mem.embedding);

                // Keyword score (0-1)
                let keyword_score = keyword_match_score(&mem.content, &keywords);

                // Combined score (weighted average)
                let combined = 0.7 * semantic_score + 0.3 * keyword_score;

                // Boost by relevance
                let final_score = combined * (0.5 + 0.5 * mem.relevance);

                (mem, final_score)
            })
            .collect();

        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored.into_iter().take(limit).map(|(mem, _)| mem).collect()
    }

    /// Find duplicate or near-duplicate memories
    pub async fn find_duplicates(
        &self,
        memories: &[Memory],
        threshold: f32,
    ) -> Vec<(String, String)> {
        let mut duplicates = Vec::new();

        for i in 0..memories.len() {
            for j in (i + 1)..memories.len() {
                let similarity = EmbeddingGenerator::cosine_similarity(
                    &memories[i].embedding,
                    &memories[j].embedding,
                );

                if similarity >= threshold {
                    duplicates.push((memories[i].id.clone(), memories[j].id.clone()));
                }
            }
        }

        duplicates
    }
}

/// Search filters
#[derive(Debug, Clone, Default)]
pub struct SearchFilters {
    /// Filter by category
    pub category: Option<String>,
    /// Filter by tags (all must match)
    pub tags: Option<Vec<String>>,
    /// Filter by creation date (after)
    pub created_after: Option<chrono::DateTime<chrono::Utc>>,
    /// Filter by creation date (before)
    pub created_before: Option<chrono::DateTime<chrono::Utc>>,
    /// Minimum relevance score
    pub min_relevance: Option<f32>,
}

/// Extract keywords from query
fn extract_keywords(query: &str) -> Vec<String> {
    // Simple keyword extraction - split by whitespace and filter
    query
        .split_whitespace()
        .map(|s| s.to_lowercase())
        .filter(|s| s.len() > 2) // Skip very short words
        .filter(|s| !STOP_WORDS.contains(&s.as_str()))
        .collect()
}

/// Calculate keyword match score
fn keyword_match_score(content: &str, keywords: &[String]) -> f32 {
    if keywords.is_empty() {
        return 0.0;
    }

    let content_lower = content.to_lowercase();
    let matches = keywords.iter().filter(|kw| content_lower.contains(kw.as_str())).count();

    matches as f32 / keywords.len() as f32
}

/// Common stop words to filter
const STOP_WORDS: &[&str] = &[
    "the",
    "a",
    "an",
    "and",
    "or",
    "but",
    "in",
    "on",
    "at",
    "to",
    "for",
    "of",
    "with",
    "by",
    "from",
    "up",
    "about",
    "into",
    "through",
    "during",
    "before",
    "after",
    "above",
    "below",
    "between",
    "under",
    "again",
    "further",
    "then",
    "once",
    "here",
    "there",
    "when",
    "where",
    "why",
    "how",
    "all",
    "each",
    "few",
    "more",
    "most",
    "other",
    "some",
    "such",
    "no",
    "nor",
    "not",
    "only",
    "own",
    "same",
    "so",
    "than",
    "too",
    "very",
    "can",
    "will",
    "just",
    "should",
    "now",
    "is",
    "are",
    "was",
    "were",
    "be",
    "been",
    "being",
    "have",
    "has",
    "had",
    "having",
    "do",
    "does",
    "did",
    "doing",
    "would",
    "could",
    "ought",
    "i",
    "me",
    "my",
    "myself",
    "we",
    "our",
    "ours",
    "ourselves",
    "you",
    "your",
    "yours",
    "yourself",
    "yourselves",
    "he",
    "him",
    "his",
    "himself",
    "she",
    "her",
    "hers",
    "herself",
    "it",
    "its",
    "itself",
    "they",
    "them",
    "their",
    "theirs",
    "themselves",
    "what",
    "which",
    "who",
    "whom",
    "this",
    "that",
    "these",
    "those",
    "am",
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::MemoryMetadata;

    fn create_test_memory(id: &str, content: &str, embedding: Vec<f32>) -> Memory {
        Memory {
            id: id.to_string(),
            content: content.to_string(),
            embedding,
            metadata: MemoryMetadata::default(),
            created_at: chrono::Utc::now(),
            accessed_at: chrono::Utc::now(),
            relevance: 1.0,
        }
    }

    #[tokio::test]
    async fn test_search() {
        let search = SemanticSearch::new(3);

        let memories = vec![
            create_test_memory("1", "Hello world", vec![1.0, 0.0, 0.0]),
            create_test_memory("2", "Goodbye moon", vec![0.0, 1.0, 0.0]),
            create_test_memory("3", "Hello again", vec![0.9, 0.1, 0.0]),
        ];

        let query = vec![1.0, 0.0, 0.0]; // Similar to "Hello world"
        let results = search.search(&query, memories, 2).await;

        assert_eq!(results.len(), 2);
        assert_eq!(results[0].id, "1"); // Most similar
        assert_eq!(results[1].id, "3"); // Second most similar
    }

    #[tokio::test]
    async fn test_search_filtered() {
        let search = SemanticSearch::new(3);

        let mut mem1 = create_test_memory("1", "Cat", vec![1.0, 0.0, 0.0]);
        mem1.metadata.category = "animals".to_string();

        let mut mem2 = create_test_memory("2", "Dog", vec![0.9, 0.1, 0.0]);
        mem2.metadata.category = "animals".to_string();

        let mut mem3 = create_test_memory("3", "Car", vec![0.8, 0.2, 0.0]);
        mem3.metadata.category = "vehicles".to_string();

        let memories = vec![mem1, mem2, mem3];

        let filters = SearchFilters {
            category: Some("animals".to_string()),
            ..Default::default()
        };

        let query = vec![1.0, 0.0, 0.0];
        let results = search.search_filtered(&query, memories, filters, 10).await;

        assert_eq!(results.len(), 2);
        assert!(results.iter().all(|m| m.metadata.category == "animals"));
    }

    #[test]
    fn test_extract_keywords() {
        let keywords = extract_keywords("What is the meaning of life?");

        assert!(keywords.contains(&"meaning".to_string()));
        assert!(keywords.contains(&"life".to_string()));
        assert!(!keywords.contains(&"the".to_string())); // Stop word
        assert!(!keywords.contains(&"is".to_string())); // Stop word
    }

    #[test]
    fn test_keyword_match_score() {
        let content = "The quick brown fox jumps over the lazy dog";

        let keywords = vec!["quick".to_string(), "fox".to_string()];
        let score = keyword_match_score(content, &keywords);
        assert!((score - 1.0).abs() < 0.001); // All keywords match

        let keywords = vec!["quick".to_string(), "cat".to_string()];
        let score = keyword_match_score(content, &keywords);
        assert!((score - 0.5).abs() < 0.001); // Half match
    }

    #[tokio::test]
    async fn test_find_duplicates() {
        let search = SemanticSearch::new(3);

        let memories = vec![
            create_test_memory("1", "Hello", vec![1.0, 0.0, 0.0]),
            create_test_memory("2", "Hello", vec![0.99, 0.01, 0.0]), // Near duplicate
            create_test_memory("3", "Goodbye", vec![0.0, 1.0, 0.0]), // Different
        ];

        let duplicates = search.find_duplicates(&memories, 0.95).await;

        assert_eq!(duplicates.len(), 1);
        assert!(duplicates.contains(&("1".to_string(), "2".to_string())));
    }
}
