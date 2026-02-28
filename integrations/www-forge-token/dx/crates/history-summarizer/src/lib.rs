//! # history-summarizer
//!
//! Compresses old conversation history into summaries.
//! Net savings after summary generation cost.
//!
//! ## Evidence (TOKEN.md ⚠️ PARTLY REAL)
//! - Summarization itself costs tokens (the API call to summarize)
//! - 60-90% compression per summarized block
//! - But: net savings only positive after 2+ turns of reuse
//! - **Honest: 40-70% net savings if summary is reused across turns**
//! - Must track summary freshness and avoid re-summarizing
//!
//! STAGE: InterTurn (priority 15)

use dx_core::*;
use std::sync::Mutex;

/// Configuration for history summarization.
#[derive(Debug, Clone)]
pub struct HistorySummarizerConfig {
    /// Minimum number of messages before summarization kicks in
    pub min_messages_before_summarize: usize,
    /// Number of recent messages to always keep verbatim
    pub keep_recent: usize,
    /// Target ratio: summary should be this fraction of original
    pub target_ratio: f64,
    /// Maximum summary tokens (prevents runaway summaries)
    pub max_summary_tokens: usize,
    /// Minimum tokens in a block before it's worth summarizing
    pub min_block_tokens: usize,
}

impl Default for HistorySummarizerConfig {
    fn default() -> Self {
        Self {
            min_messages_before_summarize: 6,
            keep_recent: 4,
            target_ratio: 0.25,
            max_summary_tokens: 2_000,
            min_block_tokens: 500,
        }
    }
}

pub struct HistorySummarizer {
    config: HistorySummarizerConfig,
    report: Mutex<TokenSavingsReport>,
}

impl HistorySummarizer {
    pub fn new() -> Self {
        Self::with_config(HistorySummarizerConfig::default())
    }

    pub fn with_config(config: HistorySummarizerConfig) -> Self {
        Self {
            config,
            report: Mutex::new(TokenSavingsReport::default()),
        }
    }

    /// Simple extractive summary: take first and last sentence of each message.
    /// In production, you'd call an LLM or use a proper summarization model.
    fn extractive_summarize(messages: &[Message], target_ratio: f64, max_tokens: usize) -> Message {
        let mut summary_parts: Vec<String> = Vec::new();
        let total_tokens: usize = messages.iter().map(|m| m.token_count).sum();

        for msg in messages {
            let lines: Vec<&str> = msg.content.lines().filter(|l| !l.trim().is_empty()).collect();
            if lines.is_empty() {
                continue;
            }
            let role_prefix = match msg.role.as_str() {
                "user" => "[User]",
                "assistant" => "[Assistant]",
                "tool" => "[Tool]",
                _ => "[System]",
            };
            if lines.len() <= 2 {
                summary_parts.push(format!("{} {}", role_prefix, lines.join(" ")));
            } else {
                // First line + last line as extractive summary
                summary_parts.push(format!("{} {} ... {}", role_prefix, lines[0], lines[lines.len()-1]));
            }
        }

        let summary_text = format!(
            "[Summary of {} messages, ~{} original tokens]\n{}",
            messages.len(),
            total_tokens,
            summary_parts.join("\n")
        );

        // Estimate summary tokens (rough: 1 token per 4 chars)
        let estimated_tokens = (summary_text.len() / 4)
            .min(max_tokens)
            .min((total_tokens as f64 * target_ratio) as usize);

        Message {
            role: "system".into(),
            content: summary_text,
            images: vec![],
            tool_call_id: None,
            token_count: estimated_tokens.max(1),
        }
    }
}

#[async_trait::async_trait]
impl TokenSaver for HistorySummarizer {
    fn name(&self) -> &str { "history-summarizer" }
    fn stage(&self) -> SaverStage { SaverStage::InterTurn }
    fn priority(&self) -> u32 { 15 }

    async fn process(
        &self,
        input: SaverInput,
        _ctx: &SaverContext,
    ) -> Result<SaverOutput, SaverError> {
        let total_messages = input.messages.len();
        let tokens_before: usize = input.messages.iter().map(|m| m.token_count).sum();

        if total_messages < self.config.min_messages_before_summarize {
            let report = TokenSavingsReport {
                technique: "history-summarizer".into(),
                tokens_before,
                tokens_after: tokens_before,
                tokens_saved: 0,
                description: format!(
                    "Skipped: only {} messages (need {}+). Net savings only begin after 2+ turns of reuse.",
                    total_messages, self.config.min_messages_before_summarize
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

        // Split: older messages to summarize, recent to keep
        let keep_count = self.config.keep_recent.min(total_messages);
        let split_point = total_messages.saturating_sub(keep_count);

        let to_summarize = &input.messages[..split_point];
        let to_keep = &input.messages[split_point..];

        let old_tokens: usize = to_summarize.iter().map(|m| m.token_count).sum();

        if old_tokens < self.config.min_block_tokens {
            let report = TokenSavingsReport {
                technique: "history-summarizer".into(),
                tokens_before,
                tokens_after: tokens_before,
                tokens_saved: 0,
                description: format!(
                    "Skipped: old block only {} tokens (need {}+). Not worth the summary cost.",
                    old_tokens, self.config.min_block_tokens
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

        // Generate extractive summary
        let summary_msg = Self::extractive_summarize(
            to_summarize,
            self.config.target_ratio,
            self.config.max_summary_tokens,
        );
        let summary_tokens = summary_msg.token_count;

        // Build new message list
        let mut new_messages = Vec::with_capacity(1 + to_keep.len());
        new_messages.push(summary_msg);
        new_messages.extend_from_slice(to_keep);

        let keep_tokens: usize = to_keep.iter().map(|m| m.token_count).sum();
        let tokens_after = summary_tokens + keep_tokens;
        let tokens_saved = tokens_before.saturating_sub(tokens_after);

        let report = TokenSavingsReport {
            technique: "history-summarizer".into(),
            tokens_before,
            tokens_after,
            tokens_saved,
            description: format!(
                "Summarized {} old messages ({} tokens) → {} token summary. \
                 Kept {} recent messages ({} tokens). \
                 Compression: {:.0}%. \
                 NOTE: Net savings only positive if summary reused across 2+ turns.",
                split_point, old_tokens, summary_tokens,
                keep_count, keep_tokens,
                if old_tokens > 0 { (1.0 - summary_tokens as f64 / old_tokens as f64) * 100.0 } else { 0.0 }
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
    async fn test_skips_short_conversations() {
        let saver = HistorySummarizer::new();
        let ctx = SaverContext::default();
        let input = SaverInput {
            messages: vec![msg("user", "hello", 10), msg("assistant", "hi", 10)],
            tools: vec![],
            images: vec![],
            turn_number: 1,
        };
        let out = saver.process(input, &ctx).await.unwrap();
        assert!(out.skipped);
        assert_eq!(out.messages.len(), 2);
    }

    #[tokio::test]
    async fn test_summarizes_long_conversation() {
        let config = HistorySummarizerConfig {
            min_messages_before_summarize: 4,
            keep_recent: 2,
            min_block_tokens: 100,
            ..Default::default()
        };
        let saver = HistorySummarizer::with_config(config);
        let ctx = SaverContext::default();
        let input = SaverInput {
            messages: vec![
                msg("user", "First question with lots of detail about topic A", 200),
                msg("assistant", "Detailed answer about topic A with examples", 300),
                msg("user", "Follow up about topic A with more context", 200),
                msg("assistant", "More details about A", 250),
                msg("user", "New question about topic B", 100),
                msg("assistant", "Answer about topic B", 150),
            ],
            tools: vec![],
            images: vec![],
            turn_number: 6,
        };
        let out = saver.process(input, &ctx).await.unwrap();
        assert!(!out.skipped);
        // Should have: 1 summary + 2 recent
        assert_eq!(out.messages.len(), 3);
        assert_eq!(out.messages[0].role, "system"); // summary
        assert!(saver.last_savings().tokens_saved > 0);
    }
}
