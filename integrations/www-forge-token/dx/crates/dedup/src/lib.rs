//! # dedup
//!
//! Eliminates duplicate tool calls and outputs — agents frequently
//! re-read the same file or re-run the same command.
//!
//! ## Evidence (TOKEN.md ✅ REAL)
//! - Agents frequently re-read the same file or re-run the same command
//! - Deduplicating identical tool outputs is pure win, zero quality loss
//! - **Honest savings: 20-50% in agentic workflows**
//!
//! STAGE: PostResponse (priority 25)

use dx_core::*;
use std::collections::HashMap;
use std::sync::Mutex;

/// Configuration for deduplication.
#[derive(Debug, Clone)]
pub struct DedupConfig {
    /// How many turns back to check for duplicates
    pub lookback_turns: usize,
    /// Minimum token count for a message to be worth dedup checking
    pub min_tokens: usize,
    /// Whether to keep a short reference when deduplicating
    pub keep_reference: bool,
    /// Maximum number of tracked hashes
    pub max_tracked: usize,
}

impl Default for DedupConfig {
    fn default() -> Self {
        Self {
            lookback_turns: 10,
            min_tokens: 50,
            keep_reference: true,
            max_tracked: 500,
        }
    }
}

/// Tracks seen content hashes for dedup.
#[derive(Debug, Default)]
struct DedupTracker {
    /// content_hash → (turn_number, tool_call_id, original_tokens)
    seen: HashMap<blake3::Hash, (usize, Option<String>, usize)>,
}

pub struct DedupSaver {
    config: DedupConfig,
    tracker: Mutex<DedupTracker>,
    report: Mutex<TokenSavingsReport>,
}

impl DedupSaver {
    pub fn new() -> Self {
        Self::with_config(DedupConfig::default())
    }

    pub fn with_config(config: DedupConfig) -> Self {
        Self {
            config,
            tracker: Mutex::new(DedupTracker::default()),
            report: Mutex::new(TokenSavingsReport::default()),
        }
    }

    /// Reset tracker (e.g., on new conversation).
    pub fn reset(&self) {
        self.tracker.lock().unwrap().seen.clear();
    }

    /// Hash the content of a message for dedup detection.
    fn hash_content(content: &str) -> blake3::Hash {
        blake3::hash(content.as_bytes())
    }
}

#[async_trait::async_trait]
impl TokenSaver for DedupSaver {
    fn name(&self) -> &str { "dedup" }
    fn stage(&self) -> SaverStage { SaverStage::PostResponse }
    fn priority(&self) -> u32 { 25 }

    async fn process(
        &self,
        input: SaverInput,
        ctx: &SaverContext,
    ) -> Result<SaverOutput, SaverError> {
        let tokens_before: usize = input.messages.iter().map(|m| m.token_count).sum();
        let mut messages = input.messages;
        let mut deduped_count = 0usize;
        let mut tokens_saved_total = 0usize;

        let turn = ctx.turn_number;
        let mut tracker = self.tracker.lock().unwrap();

        // Evict old entries beyond lookback window
        let cutoff = turn.saturating_sub(self.config.lookback_turns);
        tracker.seen.retain(|_, (t, _, _)| *t >= cutoff);

        // Limit tracker size
        while tracker.seen.len() > self.config.max_tracked {
            // Remove the oldest entries
            if let Some(oldest_hash) = tracker.seen.iter()
                .min_by_key(|(_, (t, _, _))| *t)
                .map(|(h, _)| *h)
            {
                tracker.seen.remove(&oldest_hash);
            } else {
                break;
            }
        }

        for msg in &mut messages {
            // Only dedup tool outputs and long assistant messages
            if msg.token_count < self.config.min_tokens {
                continue;
            }
            if msg.role != "tool" && msg.role != "assistant" {
                continue;
            }

            let hash = Self::hash_content(&msg.content);

            if let Some((orig_turn, orig_id, _orig_tokens)) = tracker.seen.get(&hash) {
                // Duplicate found
                let ref_text = if self.config.keep_reference {
                    format!(
                        "[Duplicate output — same as turn {}{}, {} tokens. Content deduplicated.]",
                        orig_turn,
                        orig_id.as_ref().map_or(String::new(), |id| format!(" ({})", id)),
                        msg.token_count
                    )
                } else {
                    "[Duplicate output removed.]".into()
                };

                let old_tokens = msg.token_count;
                let new_tokens = ref_text.len() / 4 + 5;
                tokens_saved_total += old_tokens.saturating_sub(new_tokens);
                msg.content = ref_text;
                msg.token_count = new_tokens;
                deduped_count += 1;
            } else {
                // Register this content
                tracker.seen.insert(
                    hash,
                    (turn, msg.tool_call_id.clone(), msg.token_count),
                );
            }
        }

        drop(tracker);

        let tokens_after = tokens_before.saturating_sub(tokens_saved_total);
        let pct = if tokens_before > 0 {
            tokens_saved_total as f64 / tokens_before as f64 * 100.0
        } else { 0.0 };

        let report = TokenSavingsReport {
            technique: "dedup".into(),
            tokens_before,
            tokens_after,
            tokens_saved: tokens_saved_total,
            description: if deduped_count > 0 {
                format!(
                    "Deduplicated {} identical outputs: {} → {} tokens ({:.1}% saved). \
                     Zero quality loss — content is byte-for-byte identical.",
                    deduped_count, tokens_before, tokens_after, pct
                )
            } else {
                "No duplicate outputs detected.".into()
            },
        };
        *self.report.lock().unwrap() = report;

        Ok(SaverOutput {
            messages,
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

    fn tool_msg(content: &str, tokens: usize) -> Message {
        Message {
            role: "tool".into(),
            content: content.into(),
            images: vec![],
            tool_call_id: Some("tc_1".into()),
            token_count: tokens,
        }
    }

    #[tokio::test]
    async fn test_no_dedup_different_content() {
        let saver = DedupSaver::new();
        let ctx = SaverContext { turn_number: 1, ..Default::default() };
        let input = SaverInput {
            messages: vec![
                tool_msg("output A", 100),
                tool_msg("output B", 100),
            ],
            tools: vec![],
            images: vec![],
            turn_number: 1,
        };
        let _ = saver.process(input, &ctx).await.unwrap();
        assert_eq!(saver.last_savings().tokens_saved, 0);
    }

    #[tokio::test]
    async fn test_dedup_identical_content() {
        let saver = DedupSaver::new();
        let ctx = SaverContext { turn_number: 1, ..Default::default() };
        let input = SaverInput {
            messages: vec![
                tool_msg("identical output content here", 200),
                tool_msg("identical output content here", 200),
            ],
            tools: vec![],
            images: vec![],
            turn_number: 1,
        };
        let out = saver.process(input, &ctx).await.unwrap();
        assert!(saver.last_savings().tokens_saved > 0);
        assert!(out.messages[1].content.contains("Duplicate"));
    }
}
