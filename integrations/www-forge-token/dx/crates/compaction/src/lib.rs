//! # compaction
//!
//! Compacts conversation history when it exceeds limits, using
//! progressive strategies that balance savings vs context quality.
//!
//! ## Evidence (TOKEN.md ⚠️ Partly Real)
//! - Aggressive compaction breaks the model's ability to recall prior decisions
//! - 84% claim is overclaimed — destroys context in complex agent tasks
//! - **Honest savings: 30-50% without quality loss**
//! - Strategy: conservative first (remove old tool outputs), then summarize
//!
//! STAGE: InterTurn (priority 30)

use dx_core::*;
use std::sync::Mutex;

/// Compaction strategy, applied in order of aggressiveness.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompactionStrategy {
    /// Remove tool outputs older than N turns (safest, ~15-25% savings)
    RemoveStaleToolOutputs,
    /// Collapse consecutive assistant messages into one (~5-10%)
    CollapseConsecutive,
    /// Truncate very long individual messages to head+tail (~10-15%)
    TruncateLongMessages,
    /// Drop the oldest non-system messages (~20-30%)
    DropOldest,
}

/// Configuration for the compaction saver.
#[derive(Debug, Clone)]
pub struct CompactionConfig {
    /// Maximum total token count before compaction triggers
    pub max_total_tokens: usize,
    /// Target token count after compaction (should be well below max)
    pub target_tokens: usize,
    /// How many recent turns to protect from any compaction
    pub protected_recent_turns: usize,
    /// Max tokens for a single message before truncation kicks in
    pub max_single_message_tokens: usize,
    /// Strategy order — applied progressively until under target
    pub strategies: Vec<CompactionStrategy>,
    /// How many tail lines to keep when truncating a long message
    pub truncation_tail_lines: usize,
}

impl Default for CompactionConfig {
    fn default() -> Self {
        Self {
            max_total_tokens: 80_000,
            target_tokens: 50_000,
            protected_recent_turns: 4,
            max_single_message_tokens: 3_000,
            strategies: vec![
                CompactionStrategy::RemoveStaleToolOutputs,
                CompactionStrategy::CollapseConsecutive,
                CompactionStrategy::TruncateLongMessages,
                CompactionStrategy::DropOldest,
            ],
            truncation_tail_lines: 20,
        }
    }
}

pub struct CompactionSaver {
    config: CompactionConfig,
    report: Mutex<TokenSavingsReport>,
}

impl CompactionSaver {
    pub fn new() -> Self {
        Self::with_config(CompactionConfig::default())
    }

    pub fn with_config(config: CompactionConfig) -> Self {
        Self {
            config,
            report: Mutex::new(TokenSavingsReport::default()),
        }
    }

    fn total_tokens(messages: &[Message]) -> usize {
        messages.iter().map(|m| m.token_count).sum()
    }

    /// Strategy 1: Remove tool outputs from old turns.
    fn remove_stale_tool_outputs(&self, messages: &mut Vec<Message>, turn: usize) {
        let cutoff = turn.saturating_sub(self.config.protected_recent_turns);
        // Tool messages from early turns get their content cleared
        let mut current_turn = 0usize;
        for msg in messages.iter_mut() {
            if msg.role == "user" {
                current_turn += 1;
            }
            if msg.role == "tool" && current_turn <= cutoff && msg.token_count > 50 {
                let summary = format!("[Tool output removed — {} tokens, turn {}]", msg.token_count, current_turn);
                msg.token_count = summary.len() / 4; // rough estimate
                msg.content = summary;
            }
        }
    }

    /// Strategy 2: Collapse consecutive same-role messages.
    fn collapse_consecutive(messages: &mut Vec<Message>) {
        let mut i = 0;
        while i + 1 < messages.len() {
            if messages[i].role == messages[i + 1].role
                && messages[i].role == "assistant"
                && messages[i].tool_call_id.is_none()
                && messages[i + 1].tool_call_id.is_none()
            {
                let merged_content = format!("{}\n{}", messages[i].content, messages[i + 1].content);
                let merged_tokens = merged_content.len() / 4;
                messages[i].content = merged_content;
                messages[i].token_count = merged_tokens;
                messages.remove(i + 1);
            } else {
                i += 1;
            }
        }
    }

    /// Strategy 3: Truncate long messages to head + tail.
    fn truncate_long_messages(&self, messages: &mut Vec<Message>) {
        for msg in messages.iter_mut() {
            if msg.token_count > self.config.max_single_message_tokens && msg.role != "system" {
                let lines: Vec<&str> = msg.content.lines().collect();
                if lines.len() > self.config.truncation_tail_lines * 2 {
                    let head_lines = self.config.truncation_tail_lines;
                    let tail_lines = self.config.truncation_tail_lines;
                    let omitted = lines.len() - head_lines - tail_lines;
                    let mut new_content = String::new();
                    for line in &lines[..head_lines] {
                        new_content.push_str(line);
                        new_content.push('\n');
                    }
                    new_content.push_str(&format!("\n[... {} lines omitted ...]\n\n", omitted));
                    for line in &lines[lines.len() - tail_lines..] {
                        new_content.push_str(line);
                        new_content.push('\n');
                    }
                    msg.token_count = new_content.len() / 4;
                    msg.content = new_content;
                }
            }
        }
    }

    /// Strategy 4: Drop oldest non-system, non-protected messages.
    fn drop_oldest(&self, messages: &mut Vec<Message>, target: usize) {
        // Find the first droppable message (non-system, non-recent)
        while Self::total_tokens(messages) > target && messages.len() > 2 {
            // Find oldest non-system message
            if let Some(idx) = messages.iter().position(|m| m.role != "system") {
                // Don't drop if we're in the protected recent zone
                let remaining_after = messages.len() - idx - 1;
                if remaining_after < self.config.protected_recent_turns * 2 {
                    break; // Would eat into protected recent turns
                }
                messages.remove(idx);
            } else {
                break;
            }
        }
    }
}

#[async_trait::async_trait]
impl TokenSaver for CompactionSaver {
    fn name(&self) -> &str { "compaction" }
    fn stage(&self) -> SaverStage { SaverStage::InterTurn }
    fn priority(&self) -> u32 { 30 }

    async fn process(
        &self,
        input: SaverInput,
        ctx: &SaverContext,
    ) -> Result<SaverOutput, SaverError> {
        let tokens_before = Self::total_tokens(&input.messages);
        let mut messages = input.messages;

        // Only compact if over the threshold
        if tokens_before <= self.config.max_total_tokens {
            let report = TokenSavingsReport {
                technique: "compaction".into(),
                tokens_before,
                tokens_after: tokens_before,
                tokens_saved: 0,
                description: format!(
                    "No compaction needed ({} tokens ≤ {} limit)",
                    tokens_before, self.config.max_total_tokens
                ),
            };
            *self.report.lock().unwrap() = report;
            return Ok(SaverOutput {
                messages,
                tools: input.tools,
                images: input.images,
                skipped: false,
                cached_response: None,
            });
        }

        // Apply strategies progressively until under target
        let mut strategies_applied = Vec::new();
        for strategy in &self.config.strategies {
            if Self::total_tokens(&messages) <= self.config.target_tokens {
                break;
            }
            match strategy {
                CompactionStrategy::RemoveStaleToolOutputs => {
                    self.remove_stale_tool_outputs(&mut messages, ctx.turn_number);
                    strategies_applied.push("remove-stale-tool-outputs");
                }
                CompactionStrategy::CollapseConsecutive => {
                    Self::collapse_consecutive(&mut messages);
                    strategies_applied.push("collapse-consecutive");
                }
                CompactionStrategy::TruncateLongMessages => {
                    self.truncate_long_messages(&mut messages);
                    strategies_applied.push("truncate-long-messages");
                }
                CompactionStrategy::DropOldest => {
                    self.drop_oldest(&mut messages, self.config.target_tokens);
                    strategies_applied.push("drop-oldest");
                }
            }
        }

        let tokens_after = Self::total_tokens(&messages);
        let tokens_saved = tokens_before.saturating_sub(tokens_after);

        let report = TokenSavingsReport {
            technique: "compaction".into(),
            tokens_before,
            tokens_after,
            tokens_saved,
            description: format!(
                "Applied [{}], {} → {} tokens ({:.1}% saved). Strategies: progressive, quality-preserving.",
                strategies_applied.join(", "),
                tokens_before,
                tokens_after,
                if tokens_before > 0 { tokens_saved as f64 / tokens_before as f64 * 100.0 } else { 0.0 }
            ),
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

    fn msg(role: &str, content: &str, tokens: usize) -> Message {
        Message {
            role: role.into(),
            content: content.into(),
            images: vec![],
            tool_call_id: None,
            token_count: tokens,
        }
    }

    #[tokio::test]
    async fn test_no_compaction_under_limit() {
        let saver = CompactionSaver::new();
        let ctx = SaverContext { turn_number: 5, ..Default::default() };
        let input = SaverInput {
            messages: vec![msg("system", "hello", 100), msg("user", "hi", 50)],
            tools: vec![],
            images: vec![],
            turn_number: 5,
        };
        let out = saver.process(input, &ctx).await.unwrap();
        assert_eq!(saver.last_savings().tokens_saved, 0);
        assert_eq!(out.messages.len(), 2);
    }

    #[tokio::test]
    async fn test_compaction_removes_stale_tool_outputs() {
        let config = CompactionConfig {
            max_total_tokens: 1000,
            target_tokens: 500,
            protected_recent_turns: 1,
            ..Default::default()
        };
        let saver = CompactionSaver::with_config(config);
        let ctx = SaverContext { turn_number: 10, ..Default::default() };
        let mut tool_msg = msg("tool", "very long tool output here", 800);
        tool_msg.tool_call_id = Some("tc_1".into());
        let input = SaverInput {
            messages: vec![
                msg("system", "sys", 100),
                msg("user", "q1", 50),
                tool_msg,
                msg("user", "q2", 50),
                msg("assistant", "answer", 50),
            ],
            tools: vec![],
            images: vec![],
            turn_number: 10,
        };
        let out = saver.process(input, &ctx).await.unwrap();
        assert!(saver.last_savings().tokens_saved > 0);
        assert!(out.messages.len() >= 2);
    }
}
