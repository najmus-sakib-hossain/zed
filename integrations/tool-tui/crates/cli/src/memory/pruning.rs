//! Memory Pruning
//!
//! Manages memory relevance decay and pruning of old/irrelevant memories.

use super::Memory;

/// Memory pruner for managing memory lifecycle
pub struct MemoryPruner {
    /// Decay rate per day
    decay_rate: f32,
    /// Minimum relevance threshold
    min_relevance: f32,
}

impl MemoryPruner {
    /// Create a new memory pruner
    pub fn new(decay_rate: f32, min_relevance: f32) -> Self {
        Self {
            decay_rate,
            min_relevance,
        }
    }

    /// Identify memories that should be pruned
    pub async fn identify_prunable(&self, memories: Vec<Memory>) -> Vec<String> {
        memories
            .iter()
            .filter(|mem| self.should_prune(mem))
            .map(|mem| mem.id.clone())
            .collect()
    }

    /// Check if a memory should be pruned
    fn should_prune(&self, memory: &Memory) -> bool {
        // Calculate decayed relevance
        let age_days = (chrono::Utc::now() - memory.accessed_at).num_days() as f32;
        let decay_factor = (-self.decay_rate * age_days).exp();
        let current_relevance = memory.relevance * decay_factor;

        current_relevance < self.min_relevance
    }

    /// Calculate the effective relevance of a memory
    pub fn effective_relevance(&self, memory: &Memory) -> f32 {
        let age_days = (chrono::Utc::now() - memory.accessed_at).num_days() as f32;
        let decay_factor = (-self.decay_rate * age_days).exp();
        memory.relevance * decay_factor
    }

    /// Apply decay to a memory's relevance
    pub fn apply_decay(&self, memory: &mut Memory) {
        let age_days = (chrono::Utc::now() - memory.accessed_at).num_days() as f32;
        let decay_factor = (-self.decay_rate * age_days).exp();
        memory.relevance *= decay_factor;
    }

    /// Boost relevance when memory is accessed
    pub fn boost_relevance(&self, memory: &mut Memory, boost: f32) {
        memory.relevance = (memory.relevance + boost).min(1.0);
        memory.accessed_at = chrono::Utc::now();
    }

    /// Get pruning statistics
    pub fn analyze(&self, memories: &[Memory]) -> PruningAnalysis {
        let total = memories.len();
        let mut prunable = 0;
        let mut low_relevance = 0;
        let mut high_relevance = 0;
        let mut total_relevance = 0.0;

        for memory in memories {
            let effective = self.effective_relevance(memory);
            total_relevance += effective;

            if effective < self.min_relevance {
                prunable += 1;
            }

            if effective < 0.3 {
                low_relevance += 1;
            } else if effective > 0.7 {
                high_relevance += 1;
            }
        }

        PruningAnalysis {
            total_memories: total,
            prunable_count: prunable,
            low_relevance_count: low_relevance,
            high_relevance_count: high_relevance,
            average_relevance: if total > 0 {
                total_relevance / total as f32
            } else {
                0.0
            },
        }
    }

    /// Get memories sorted by relevance (lowest first, for prioritized pruning)
    pub fn prioritize_for_pruning(&self, mut memories: Vec<Memory>) -> Vec<Memory> {
        memories.sort_by(|a, b| {
            let rel_a = self.effective_relevance(a);
            let rel_b = self.effective_relevance(b);
            rel_a.partial_cmp(&rel_b).unwrap_or(std::cmp::Ordering::Equal)
        });
        memories
    }

    /// Prune to target count, keeping most relevant memories
    pub fn prune_to_count(
        &self,
        memories: Vec<Memory>,
        target: usize,
    ) -> (Vec<Memory>, Vec<String>) {
        if memories.len() <= target {
            return (memories, vec![]);
        }

        let sorted = self.prioritize_for_pruning(memories);
        let prune_count = sorted.len() - target;

        let pruned_ids: Vec<String> = sorted[..prune_count].iter().map(|m| m.id.clone()).collect();
        let kept: Vec<Memory> = sorted[prune_count..].to_vec();

        (kept, pruned_ids)
    }
}

/// Pruning analysis results
#[derive(Debug, Clone)]
pub struct PruningAnalysis {
    /// Total memories analyzed
    pub total_memories: usize,
    /// Memories below threshold (prunable)
    pub prunable_count: usize,
    /// Memories with low relevance (< 0.3)
    pub low_relevance_count: usize,
    /// Memories with high relevance (> 0.7)
    pub high_relevance_count: usize,
    /// Average effective relevance
    pub average_relevance: f32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::MemoryMetadata;
    use chrono::{Duration, Utc};

    fn create_test_memory(id: &str, relevance: f32, days_ago: i64) -> Memory {
        Memory {
            id: id.to_string(),
            content: "Test".to_string(),
            embedding: vec![],
            metadata: MemoryMetadata::default(),
            created_at: Utc::now() - Duration::days(days_ago),
            accessed_at: Utc::now() - Duration::days(days_ago),
            relevance,
        }
    }

    #[test]
    fn test_should_prune_old_memory() {
        let pruner = MemoryPruner::new(0.1, 0.1);

        // Very old memory with low relevance
        let old_memory = create_test_memory("old", 0.5, 100);
        assert!(pruner.should_prune(&old_memory));
    }

    #[test]
    fn test_should_not_prune_fresh_memory() {
        let pruner = MemoryPruner::new(0.1, 0.1);

        // Fresh memory
        let fresh_memory = create_test_memory("fresh", 1.0, 0);
        assert!(!pruner.should_prune(&fresh_memory));
    }

    #[test]
    fn test_effective_relevance() {
        let pruner = MemoryPruner::new(0.1, 0.1);

        let fresh = create_test_memory("fresh", 1.0, 0);
        let old = create_test_memory("old", 1.0, 30);

        let fresh_rel = pruner.effective_relevance(&fresh);
        let old_rel = pruner.effective_relevance(&old);

        assert!(fresh_rel > old_rel);
        assert!((fresh_rel - 1.0).abs() < 0.1); // Fresh should be close to 1.0
    }

    #[test]
    fn test_boost_relevance() {
        let pruner = MemoryPruner::new(0.1, 0.1);

        let mut memory = create_test_memory("test", 0.5, 10);
        pruner.boost_relevance(&mut memory, 0.2);

        assert!((memory.relevance - 0.7).abs() < 0.001);
        // Access time should be updated to now
        assert!((Utc::now() - memory.accessed_at).num_seconds() < 1);
    }

    #[test]
    fn test_boost_relevance_cap() {
        let pruner = MemoryPruner::new(0.1, 0.1);

        let mut memory = create_test_memory("test", 0.9, 0);
        pruner.boost_relevance(&mut memory, 0.5);

        assert!((memory.relevance - 1.0).abs() < 0.001); // Capped at 1.0
    }

    #[tokio::test]
    async fn test_identify_prunable() {
        let pruner = MemoryPruner::new(0.1, 0.1);

        let memories = vec![
            create_test_memory("fresh", 1.0, 0),
            create_test_memory("old1", 0.5, 100),
            create_test_memory("old2", 0.3, 50),
        ];

        let prunable = pruner.identify_prunable(memories).await;

        assert!(!prunable.contains(&"fresh".to_string()));
    }

    #[test]
    fn test_analyze() {
        let pruner = MemoryPruner::new(0.1, 0.1);

        let memories = vec![
            create_test_memory("high", 1.0, 0),
            create_test_memory("medium", 0.5, 5),
            create_test_memory("low", 0.2, 30),
        ];

        let analysis = pruner.analyze(&memories);

        assert_eq!(analysis.total_memories, 3);
        assert!(analysis.high_relevance_count >= 1);
        assert!(analysis.average_relevance > 0.0);
    }

    #[test]
    fn test_prune_to_count() {
        let pruner = MemoryPruner::new(0.01, 0.1); // Low decay rate

        let memories = vec![
            create_test_memory("high", 1.0, 0),
            create_test_memory("medium", 0.5, 0),
            create_test_memory("low", 0.2, 0),
        ];

        let (kept, pruned) = pruner.prune_to_count(memories, 2);

        assert_eq!(kept.len(), 2);
        assert_eq!(pruned.len(), 1);
        assert!(pruned.contains(&"low".to_string()));
    }
}
