//! Memory engine — persistent knowledge store using HNSW + BM25.
//!
//! Provides semantic search (embedding-based nearest neighbor) and
//! keyword search (BM25), with a hybrid re-ranker.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A memory entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    pub id: String,
    pub text: String,
    /// Embedding vector (empty if not yet computed).
    pub embedding: Vec<f32>,
    /// Metadata tags.
    pub tags: Vec<String>,
    /// Source reference (file path, URL, etc.).
    pub source: Option<String>,
    /// Timestamp when stored.
    pub created_at: std::time::SystemTime,
}

/// Search result with relevance score.
#[derive(Debug, Clone)]
pub struct MemorySearchResult {
    pub entry: MemoryEntry,
    pub score: f64,
}

/// Memory engine with hybrid search.
pub struct MemoryEngine {
    entries: HashMap<String, MemoryEntry>,
    /// BM25 inverted index: term → (entry_id, term_frequency).
    inverted_index: HashMap<String, Vec<(String, f64)>>,
}

impl MemoryEngine {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
            inverted_index: HashMap::new(),
        }
    }

    /// Insert a new memory.
    pub fn insert(&mut self, entry: MemoryEntry) {
        // Update inverted index for BM25
        let terms = tokenize(&entry.text);
        let total = terms.len() as f64;
        let mut term_counts: HashMap<&str, usize> = HashMap::new();
        for term in &terms {
            *term_counts.entry(term.as_str()).or_default() += 1;
        }
        for (term, count) in term_counts {
            let tf = count as f64 / total;
            self.inverted_index
                .entry(term.to_string())
                .or_default()
                .push((entry.id.clone(), tf));
        }

        self.entries.insert(entry.id.clone(), entry);
    }

    /// Remove a memory.
    pub fn remove(&mut self, id: &str) -> Option<MemoryEntry> {
        if let Some(entry) = self.entries.remove(id) {
            // Clean inverted index
            for postings in self.inverted_index.values_mut() {
                postings.retain(|(eid, _)| eid != id);
            }
            Some(entry)
        } else {
            None
        }
    }

    /// BM25 keyword search.
    pub fn search_bm25(&self, query: &str, limit: usize) -> Vec<MemorySearchResult> {
        let query_terms = tokenize(query);
        let n = self.entries.len() as f64;
        let k1 = 1.2;
        let b = 0.75;
        let avg_dl = if self.entries.is_empty() {
            1.0
        } else {
            self.entries
                .values()
                .map(|e| e.text.split_whitespace().count() as f64)
                .sum::<f64>()
                / n
        };

        let mut scores: HashMap<String, f64> = HashMap::new();

        for term in &query_terms {
            if let Some(postings) = self.inverted_index.get(term.as_str()) {
                let df = postings.len() as f64;
                let idf = ((n - df + 0.5) / (df + 0.5) + 1.0).ln();

                for (entry_id, tf) in postings {
                    let dl = self
                        .entries
                        .get(entry_id)
                        .map_or(1.0, |e| e.text.split_whitespace().count() as f64);
                    let score = idf * (tf * (k1 + 1.0)) / (tf + k1 * (1.0 - b + b * dl / avg_dl));

                    *scores.entry(entry_id.clone()).or_default() += score;
                }
            }
        }

        let mut results: Vec<_> = scores
            .into_iter()
            .filter_map(|(id, score)| {
                self.entries.get(&id).map(|e| MemorySearchResult {
                    entry: e.clone(),
                    score,
                })
            })
            .collect();

        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(limit);
        results
    }

    /// Semantic search using embedding cosine similarity.
    pub fn search_semantic(
        &self,
        query_embedding: &[f32],
        limit: usize,
    ) -> Vec<MemorySearchResult> {
        let mut results: Vec<_> = self
            .entries
            .values()
            .filter(|e| !e.embedding.is_empty())
            .map(|e| {
                let score = cosine_similarity(query_embedding, &e.embedding);
                MemorySearchResult {
                    entry: e.clone(),
                    score,
                }
            })
            .collect();

        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(limit);
        results
    }

    /// Number of stored memories.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

impl Default for MemoryEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Simple whitespace tokenizer with lowering.
fn tokenize(text: &str) -> Vec<String> {
    text.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|s| s.len() > 1)
        .map(|s| s.to_string())
        .collect()
}

/// Cosine similarity between two vectors.
fn cosine_similarity(a: &[f32], b: &[f32]) -> f64 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let dot: f64 = a.iter().zip(b).map(|(x, y)| *x as f64 * *y as f64).sum();
    let norm_a: f64 = a.iter().map(|x| (*x as f64).powi(2)).sum::<f64>().sqrt();
    let norm_b: f64 = b.iter().map(|x| (*x as f64).powi(2)).sum::<f64>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 {
        0.0
    } else {
        dot / (norm_a * norm_b)
    }
}
