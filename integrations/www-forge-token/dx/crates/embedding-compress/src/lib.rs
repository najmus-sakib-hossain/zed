//! # embedding-compress
//!
//! Deduplicates similar content using text similarity.
//! Uses Jaccard word-set similarity as a proxy for embeddings.
//!
//! ## Evidence (TOKEN.md ⚠️ PARTLY REAL)
//! - Jaccard word-set similarity is too crude for production
//! - Real embeddings need an API call (costs tokens/latency)
//! - But: within a single prompt, word overlap is a reasonable proxy
//! - **Honest: 10-30% savings on highly repetitive content**
//! - Best use case: removing near-duplicate tool outputs or RAG chunks
//!
//! STAGE: PrePrompt (priority 20)

use dx_core::*;
use std::collections::HashSet;
use std::sync::Mutex;

/// Configuration for embedding-based compression.
#[derive(Debug, Clone)]
pub struct EmbeddingCompressConfig {
    /// Jaccard similarity threshold to consider content "duplicate"
    pub similarity_threshold: f64,
    /// Minimum token count for a message to be a dedup candidate
    pub min_tokens_for_dedup: usize,
    /// Maximum number of messages to compare (O(n²) so keep small)
    pub max_compare_window: usize,
    /// Roles eligible for dedup (tool outputs are best candidates)
    pub eligible_roles: Vec<String>,
}

impl Default for EmbeddingCompressConfig {
    fn default() -> Self {
        Self {
            similarity_threshold: 0.85,
            min_tokens_for_dedup: 50,
            max_compare_window: 50,
            eligible_roles: vec!["tool".into()],
        }
    }
}

pub struct EmbeddingCompress {
    config: EmbeddingCompressConfig,
    report: Mutex<TokenSavingsReport>,
}

impl EmbeddingCompress {
    pub fn new() -> Self {
        Self::with_config(EmbeddingCompressConfig::default())
    }

    pub fn with_config(config: EmbeddingCompressConfig) -> Self {
        Self {
            config,
            report: Mutex::new(TokenSavingsReport::default()),
        }
    }

    /// Compute Jaccard similarity between two texts using word sets.
    /// This is a crude but zero-cost proxy for embedding similarity.
    fn jaccard_similarity(a: &str, b: &str) -> f64 {
        let words_a: HashSet<&str> = a.split_whitespace()
            .map(|w| w.trim_matches(|c: char| !c.is_alphanumeric()))
            .filter(|w| !w.is_empty())
            .collect();
        let words_b: HashSet<&str> = b.split_whitespace()
            .map(|w| w.trim_matches(|c: char| !c.is_alphanumeric()))
            .filter(|w| !w.is_empty())
            .collect();

        if words_a.is_empty() && words_b.is_empty() {
            return 1.0;
        }
        if words_a.is_empty() || words_b.is_empty() {
            return 0.0;
        }

        let intersection = words_a.intersection(&words_b).count();
        let union = words_a.union(&words_b).count();

        if union == 0 { 0.0 } else { intersection as f64 / union as f64 }
    }
}

#[async_trait::async_trait]
impl TokenSaver for EmbeddingCompress {
    fn name(&self) -> &str { "embedding-compress" }
    fn stage(&self) -> SaverStage { SaverStage::PrePrompt }
    fn priority(&self) -> u32 { 20 }

    async fn process(
        &self,
        input: SaverInput,
        _ctx: &SaverContext,
    ) -> Result<SaverOutput, SaverError> {
        let tokens_before: usize = input.messages.iter().map(|m| m.token_count).sum();

        // Collect indices of eligible messages
        let eligible_indices: Vec<usize> = input.messages.iter().enumerate()
            .filter(|(_, m)|
                self.config.eligible_roles.contains(&m.role) &&
                m.token_count >= self.config.min_tokens_for_dedup
            )
            .map(|(i, _)| i)
            .rev() // Start from most recent
            .take(self.config.max_compare_window)
            .collect();

        // Find duplicates using O(n²) pairwise comparison
        let mut duplicate_indices: HashSet<usize> = HashSet::new();
        for (pos_a, &idx_a) in eligible_indices.iter().enumerate() {
            if duplicate_indices.contains(&idx_a) {
                continue;
            }
            for &idx_b in &eligible_indices[pos_a + 1..] {
                if duplicate_indices.contains(&idx_b) {
                    continue;
                }
                let sim = Self::jaccard_similarity(
                    &input.messages[idx_a].content,
                    &input.messages[idx_b].content,
                );
                if sim >= self.config.similarity_threshold {
                    // Mark older one (higher index in reversed list) as duplicate
                    duplicate_indices.insert(idx_b);
                }
            }
        }

        if duplicate_indices.is_empty() {
            let report = TokenSavingsReport {
                technique: "embedding-compress".into(),
                tokens_before,
                tokens_after: tokens_before,
                tokens_saved: 0,
                description: format!(
                    "No near-duplicates found among {} eligible messages (threshold: {:.0}%). \
                     NOTE: Using Jaccard word-set similarity — crude but zero-cost.",
                    eligible_indices.len(), self.config.similarity_threshold * 100.0
                ),
            };
            *self.report.lock().unwrap() = report;
            return Ok(SaverOutput {
                messages: input.messages,
                tools: input.tools,
                images: input.images,
                skipped: true,
                cached_response: None,
            });
        }

        // Remove duplicates, replace with backreference
        let mut new_messages = Vec::with_capacity(input.messages.len());
        let mut tokens_removed = 0usize;
        let dedup_count = duplicate_indices.len();

        for (i, msg) in input.messages.into_iter().enumerate() {
            if duplicate_indices.contains(&i) {
                tokens_removed += msg.token_count;
                // Replace with a short note
                new_messages.push(Message {
                    role: msg.role,
                    content: "[duplicate content removed — see similar message above]".into(),
                    images: vec![],
                    tool_call_id: msg.tool_call_id,
                    token_count: 8,
                });
            } else {
                new_messages.push(msg);
            }
        }

        let tokens_after = tokens_before.saturating_sub(tokens_removed - dedup_count * 8);
        let tokens_saved = tokens_before.saturating_sub(tokens_after);

        let report = TokenSavingsReport {
            technique: "embedding-compress".into(),
            tokens_before,
            tokens_after,
            tokens_saved,
            description: format!(
                "Removed {} near-duplicate messages ({} tokens). \
                 Jaccard threshold: {:.0}%. \
                 WARNING: Word-set similarity is a crude proxy — may false-positive on content \
                 that shares vocabulary but differs in meaning.",
                dedup_count, tokens_removed, self.config.similarity_threshold * 100.0
            ),
        };
        *self.report.lock().unwrap() = report;

        Ok(SaverOutput {
            messages: new_messages,
            tools: input.tools,
            images: input.images,
            skipped: false,
            cached_response: None,
        })
    }

    fn last_savings(&self) -> TokenSavingsReport {
        self.report.lock().unwrap().clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn msg(role: &str, content: &str, tokens: usize) -> Message {
        Message { role: role.into(), content: content.into(), images: vec![], tool_call_id: None, token_count: tokens }
    }

    #[tokio::test]
    async fn test_no_duplicates() {
        let saver = EmbeddingCompress::new();
        let ctx = SaverContext::default();
        let input = SaverInput {
            messages: vec![
                msg("tool", "The quick brown fox jumps over the lazy dog", 100),
                msg("tool", "Completely different content about quantum physics", 100),
            ],
            tools: vec![],
            images: vec![],
            turn_number: 1,
        };
        let out = saver.process(input, &ctx).await.unwrap();
        assert!(out.skipped);
    }

    #[tokio::test]
    async fn test_finds_duplicates() {
        let saver = EmbeddingCompress::new();
        let ctx = SaverContext::default();
        let input = SaverInput {
            messages: vec![
                msg("tool", "The server returned status code 200 with response body containing user data fields name email and address", 80),
                msg("tool", "The server returned status code 200 with response body containing user data fields name email and phone", 80),
            ],
            tools: vec![],
            images: vec![],
            turn_number: 1,
        };
        let out = saver.process(input, &ctx).await.unwrap();
        // High overlap should trigger dedup
        let has_removed = out.messages.iter().any(|m| m.content.contains("duplicate content removed"));
        assert!(has_removed);
    }
}
