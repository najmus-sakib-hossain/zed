//! # token-budget
//!
//! Global token budget enforcer — prevents context window overflow.
//! Defensive engineering that avoids catastrophic retries/errors.
//!
//! ## Evidence (TOKEN.md ✅ REAL)
//! - This is defensive engineering, not a savings technique
//! - Prevents the catastrophic case of exceeding context windows
//! - Exceeding context → errors → retries → wasted tokens
//! - **Genuine: prevents overflow, saves tokens from failed retries**
//!
//! STAGE: PreCall (priority 20)

use dx_core::*;
use std::collections::BTreeMap;
use std::sync::Mutex;

/// Known model context window sizes (as of Feb 2026).
fn model_context_windows() -> BTreeMap<String, usize> {
    let mut m = BTreeMap::new();
    m.insert("gpt-5".into(), 256_000);
    m.insert("gpt-5-turbo".into(), 256_000);
    m.insert("gpt-4.1".into(), 1_048_576);
    m.insert("gpt-4.1-mini".into(), 1_048_576);
    m.insert("gpt-4o".into(), 128_000);
    m.insert("gpt-4o-mini".into(), 128_000);
    m.insert("o3".into(), 200_000);
    m.insert("o4-mini".into(), 200_000);
    m.insert("claude-4-opus".into(), 200_000);
    m.insert("claude-3-5-sonnet".into(), 200_000);
    m.insert("claude-3-5-haiku".into(), 200_000);
    m
}

/// Configuration for the token budget enforcer.
#[derive(Debug, Clone)]
pub struct TokenBudgetConfig {
    /// Safety margin: reserve this many tokens for the response
    pub response_reserve: usize,
    /// Hard limit override (0 = use model's known limit)
    pub hard_limit: usize,
    /// What to do when over budget
    pub overflow_strategy: OverflowStrategy,
    /// Model context window overrides
    pub model_limits: BTreeMap<String, usize>,
}

/// What to do when the token count exceeds the budget.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OverflowStrategy {
    /// Truncate oldest non-system messages until under budget
    TruncateOldest,
    /// Return an error (let the pipeline handle it)
    Error,
    /// Just warn (set a flag) but don't modify
    WarnOnly,
}

impl Default for TokenBudgetConfig {
    fn default() -> Self {
        Self {
            response_reserve: 4_096,
            hard_limit: 0,
            overflow_strategy: OverflowStrategy::TruncateOldest,
            model_limits: model_context_windows(),
        }
    }
}

pub struct TokenBudgetSaver {
    config: TokenBudgetConfig,
    report: Mutex<TokenSavingsReport>,
}

impl TokenBudgetSaver {
    pub fn new() -> Self {
        Self::with_config(TokenBudgetConfig::default())
    }

    pub fn with_config(config: TokenBudgetConfig) -> Self {
        Self {
            config,
            report: Mutex::new(TokenSavingsReport::default()),
        }
    }

    /// Get the effective token budget for a model.
    pub fn budget_for_model(&self, model: &str) -> usize {
        let context_window = if self.config.hard_limit > 0 {
            self.config.hard_limit
        } else {
            // Try exact match, then prefix match
            self.config.model_limits.get(model).copied()
                .or_else(|| {
                    self.config.model_limits.iter()
                        .find(|(k, _)| model.starts_with(k.as_str()))
                        .map(|(_, v)| *v)
                })
                .unwrap_or(128_000) // conservative default
        };
        context_window.saturating_sub(self.config.response_reserve)
    }

    /// Truncate by removing oldest non-system messages.
    fn truncate_to_budget(messages: &mut Vec<Message>, budget: usize) -> usize {
        let mut removed = 0usize;
        loop {
            let total: usize = messages.iter().map(|m| m.token_count).sum();
            if total <= budget || messages.len() <= 1 {
                break;
            }
            // Find oldest non-system message
            if let Some(idx) = messages.iter().position(|m| m.role != "system") {
                removed += messages[idx].token_count;
                messages.remove(idx);
            } else {
                break; // Only system messages left
            }
        }
        removed
    }
}

#[async_trait::async_trait]
impl TokenSaver for TokenBudgetSaver {
    fn name(&self) -> &str { "token-budget" }
    fn stage(&self) -> SaverStage { SaverStage::PreCall }
    fn priority(&self) -> u32 { 20 }

    async fn process(
        &self,
        input: SaverInput,
        ctx: &SaverContext,
    ) -> Result<SaverOutput, SaverError> {
        let total_msg_tokens: usize = input.messages.iter().map(|m| m.token_count).sum();
        let total_tool_tokens: usize = input.tools.iter().map(|t| t.token_count).sum();
        let total_image_tokens: usize = input.images.iter().map(|i| i.processed_tokens).sum();
        let tokens_before = total_msg_tokens + total_tool_tokens + total_image_tokens;

        let budget = ctx.token_budget.unwrap_or_else(|| self.budget_for_model(&ctx.model));

        if tokens_before <= budget {
            let report = TokenSavingsReport {
                technique: "token-budget".into(),
                tokens_before,
                tokens_after: tokens_before,
                tokens_saved: 0,
                description: format!(
                    "Within budget: {} tokens / {} limit ({:.1}% utilization). \
                     Model: {}, reserve: {} for response.",
                    tokens_before, budget,
                    tokens_before as f64 / budget as f64 * 100.0,
                    ctx.model, self.config.response_reserve
                ),
            };
            *self.report.lock().unwrap() = report;
            return Ok(SaverOutput {
                messages: input.messages,
                tools: input.tools,
                images: input.images,
                skipped: false,
                cached_response: None,
            });
        }

        // Over budget!
        let mut messages = input.messages;
        let description;
        let tokens_saved;

        match self.config.overflow_strategy {
            OverflowStrategy::Error => {
                return Err(SaverError::Failed(format!(
                    "Token budget exceeded: {} tokens > {} limit for model '{}'",
                    tokens_before, budget, ctx.model
                )));
            }
            OverflowStrategy::WarnOnly => {
                description = format!(
                    "WARNING: Over budget by {} tokens ({} > {}). No truncation (warn-only mode).",
                    tokens_before - budget, tokens_before, budget
                );
                tokens_saved = 0;
            }
            OverflowStrategy::TruncateOldest => {
                let msg_budget = budget.saturating_sub(total_tool_tokens + total_image_tokens);
                let removed = Self::truncate_to_budget(&mut messages, msg_budget);
                tokens_saved = removed;
                let tokens_after: usize = messages.iter().map(|m| m.token_count).sum::<usize>()
                    + total_tool_tokens + total_image_tokens;
                description = format!(
                    "BUDGET ENFORCED: {} → {} tokens. Removed {} tokens of oldest messages. \
                     Model: {}, budget: {}.",
                    tokens_before, tokens_after, removed, ctx.model, budget
                );
            }
        }

        let tokens_after = tokens_before.saturating_sub(tokens_saved);
        let report = TokenSavingsReport {
            technique: "token-budget".into(),
            tokens_before,
            tokens_after,
            tokens_saved,
            description,
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

    fn msg(role: &str, tokens: usize) -> Message {
        Message { role: role.into(), content: "x".repeat(tokens * 4), images: vec![], tool_call_id: None, token_count: tokens }
    }

    #[tokio::test]
    async fn test_within_budget() {
        let saver = TokenBudgetSaver::new();
        let ctx = SaverContext { model: "gpt-4o".into(), ..Default::default() };
        let input = SaverInput {
            messages: vec![msg("system", 100), msg("user", 100)],
            tools: vec![],
            images: vec![],
            turn_number: 1,
        };
        let _ = saver.process(input, &ctx).await.unwrap();
        assert_eq!(saver.last_savings().tokens_saved, 0);
    }

    #[tokio::test]
    async fn test_over_budget_truncates() {
        let config = TokenBudgetConfig {
            hard_limit: 500,
            response_reserve: 100,
            ..Default::default()
        };
        let saver = TokenBudgetSaver::with_config(config);
        let ctx = SaverContext { model: "test".into(), ..Default::default() };
        let input = SaverInput {
            messages: vec![
                msg("system", 100),
                msg("user", 200),
                msg("assistant", 200),
                msg("user", 200),
            ],
            tools: vec![],
            images: vec![],
            turn_number: 1,
        };
        let out = saver.process(input, &ctx).await.unwrap();
        assert!(saver.last_savings().tokens_saved > 0);
        let total: usize = out.messages.iter().map(|m| m.token_count).sum();
        assert!(total <= 400); // 500 - 100 reserve
    }
}
