//! # context-pruner
//!
//! Removes stale, irrelevant context from conversation history —
//! file reads from 10 turns ago that are no longer relevant.
//!
//! ## Evidence (TOKEN.md ✅ REAL)
//! - Removing stale tool outputs is safe and effective
//! - **Honest savings: 20-40%** (honest range)
//!
//! STAGE: InterTurn (priority 10)

use dx_core::*;
use std::sync::Mutex;

/// Configuration for context pruning.
#[derive(Debug, Clone)]
pub struct ContextPrunerConfig {
    /// Tool outputs older than this many turns get pruned
    pub stale_turn_threshold: usize,
    /// Messages under this token count are kept regardless
    pub min_tokens_to_prune: usize,
    /// Keep at most N tool outputs total in history
    pub max_tool_outputs: usize,
    /// Roles that should never be pruned
    pub protected_roles: Vec<String>,
    /// Always keep the most recent N messages
    pub protect_recent: usize,
}

impl Default for ContextPrunerConfig {
    fn default() -> Self {
        Self {
            stale_turn_threshold: 5,
            min_tokens_to_prune: 100,
            max_tool_outputs: 20,
            protected_roles: vec!["system".into()],
            protect_recent: 6,
        }
    }
}

pub struct ContextPrunerSaver {
    config: ContextPrunerConfig,
    report: Mutex<TokenSavingsReport>,
}

impl ContextPrunerSaver {
    pub fn new() -> Self {
        Self::with_config(ContextPrunerConfig::default())
    }

    pub fn with_config(config: ContextPrunerConfig) -> Self {
        Self {
            config,
            report: Mutex::new(TokenSavingsReport::default()),
        }
    }

    /// Infer turn number for each message based on user message count.
    fn message_turns(messages: &[Message]) -> Vec<usize> {
        let mut turns = Vec::with_capacity(messages.len());
        let mut current_turn = 0usize;
        for msg in messages {
            if msg.role == "user" {
                current_turn += 1;
            }
            turns.push(current_turn);
        }
        turns
    }
}

#[async_trait::async_trait]
impl TokenSaver for ContextPrunerSaver {
    fn name(&self) -> &str { "context-pruner" }
    fn stage(&self) -> SaverStage { SaverStage::InterTurn }
    fn priority(&self) -> u32 { 10 }

    async fn process(
        &self,
        input: SaverInput,
        ctx: &SaverContext,
    ) -> Result<SaverOutput, SaverError> {
        let tokens_before: usize = input.messages.iter().map(|m| m.token_count).sum();
        let msg_turns = Self::message_turns(&input.messages);
        let current_turn = ctx.turn_number;
        let msg_count = input.messages.len();
        let protected_start = msg_count.saturating_sub(self.config.protect_recent);

        let mut messages = Vec::new();
        let mut pruned_count = 0usize;
        let mut tool_output_count = 0usize;

        for (i, msg) in input.messages.into_iter().enumerate() {
            let msg_turn = msg_turns[i];

            // Never prune protected roles or recent messages
            if self.config.protected_roles.contains(&msg.role) || i >= protected_start {
                if msg.role == "tool" { tool_output_count += 1; }
                messages.push(msg);
                continue;
            }

            // Prune stale tool outputs
            if msg.role == "tool"
                && current_turn.saturating_sub(msg_turn) > self.config.stale_turn_threshold
                && msg.token_count >= self.config.min_tokens_to_prune
            {
                // Replace with a slim reference
                pruned_count += 1;
                messages.push(Message {
                    role: msg.role,
                    content: format!("[Previous tool output pruned — {} tokens, turn {}]", msg.token_count, msg_turn),
                    images: vec![],
                    tool_call_id: msg.tool_call_id,
                    token_count: 15,
                });
                continue;
            }

            // Enforce max tool outputs
            if msg.role == "tool" {
                tool_output_count += 1;
                if tool_output_count > self.config.max_tool_outputs {
                    pruned_count += 1;
                    messages.push(Message {
                        role: msg.role,
                        content: format!("[Tool output pruned — exceeded {} limit]", self.config.max_tool_outputs),
                        images: vec![],
                        tool_call_id: msg.tool_call_id,
                        token_count: 10,
                    });
                    continue;
                }
            }

            messages.push(msg);
        }

        let tokens_after: usize = messages.iter().map(|m| m.token_count).sum();
        let tokens_saved = tokens_before.saturating_sub(tokens_after);
        let pct = if tokens_before > 0 { tokens_saved as f64 / tokens_before as f64 * 100.0 } else { 0.0 };

        let report = TokenSavingsReport {
            technique: "context-pruner".into(),
            tokens_before,
            tokens_after,
            tokens_saved,
            description: if pruned_count > 0 {
                format!(
                    "Pruned {} stale context entries: {} → {} tokens ({:.1}% saved). \
                     Threshold: {} turns stale. Protected {} recent messages.",
                    pruned_count, tokens_before, tokens_after, pct,
                    self.config.stale_turn_threshold, self.config.protect_recent
                )
            } else {
                "No stale context to prune.".into()
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

    fn msg(role: &str, content: &str, tokens: usize) -> Message {
        Message {
            role: role.into(),
            content: content.into(),
            images: vec![],
            tool_call_id: if role == "tool" { Some("tc_1".into()) } else { None },
            token_count: tokens,
        }
    }

    #[tokio::test]
    async fn test_prune_stale_tool_outputs() {
        let config = ContextPrunerConfig {
            stale_turn_threshold: 2,
            protect_recent: 2,
            ..Default::default()
        };
        let saver = ContextPrunerSaver::with_config(config);
        let ctx = SaverContext { turn_number: 10, ..Default::default() };
        let input = SaverInput {
            messages: vec![
                msg("system", "sys", 50),
                msg("user", "q1", 20),
                msg("tool", "old tool output with lots of content", 500),
                msg("user", "q2", 20),
                msg("tool", "another old tool output", 300),
                msg("user", "recent q", 20),
                msg("assistant", "recent answer", 50),
            ],
            tools: vec![],
            images: vec![],
            turn_number: 10,
        };
        let out = saver.process(input, &ctx).await.unwrap();
        assert!(saver.last_savings().tokens_saved > 0);
        assert!(out.messages.iter().any(|m| m.content.contains("pruned")));
    }
}
