//! # prefix-cache
//!
//! Guarantees byte-for-byte stable prompt prefixes so provider-side
//! prompt caching activates reliably.
//!
//! ## Evidence (TOKEN.md verified ✅)
//! - OpenAI caches prefixes ≥1024 tokens automatically
//! - Discount varies by model: GPT-5 (90%), GPT-4.1 (75%), GPT-4o/O-series (50%)
//! - Anthropic cache_control marks static blocks
//! - Cache TTL: 5-10 minutes of inactivity, up to 1 hour
//! - This crate's value: **guaranteeing byte-for-byte prefix stability**
//!
//! ## Honest savings: 50-90% on cached input tokens (varies by model)
//! STAGE: PromptAssembly (priority 10)

use dx_core::*;
use std::collections::BTreeMap;
use std::sync::Mutex;

/// Configuration for prefix cache optimization.
#[derive(Debug, Clone)]
pub struct PrefixCacheConfig {
    /// Minimum prefix length in tokens to bother stabilizing (OpenAI requires ≥1024)
    pub min_prefix_tokens: usize,
    /// Roles considered "stable" (system prompts, tool defs). They go first.
    pub stable_roles: Vec<String>,
    /// Whether to sort tool schemas deterministically for cache stability
    pub sort_tools: bool,
    /// Model-specific cache discount rates
    pub model_discounts: BTreeMap<String, f64>,
}

impl Default for PrefixCacheConfig {
    fn default() -> Self {
        let mut discounts = BTreeMap::new();
        discounts.insert("gpt-5".into(), 0.90);
        discounts.insert("gpt-5-turbo".into(), 0.90);
        discounts.insert("gpt-4.1".into(), 0.75);
        discounts.insert("gpt-4.1-mini".into(), 0.75);
        discounts.insert("gpt-4o".into(), 0.50);
        discounts.insert("gpt-4o-mini".into(), 0.50);
        discounts.insert("o3".into(), 0.50);
        discounts.insert("o4-mini".into(), 0.50);
        discounts.insert("claude-3-5-sonnet".into(), 0.90);
        discounts.insert("claude-4-opus".into(), 0.90);

        Self {
            min_prefix_tokens: 1024,
            stable_roles: vec!["system".into()],
            sort_tools: true,
            model_discounts: discounts,
        }
    }
}

pub struct PrefixCacheSaver {
    config: PrefixCacheConfig,
    last_prefix_hash: Mutex<Option<blake3::Hash>>,
    prefix_hit_streak: Mutex<u64>,
    report: Mutex<TokenSavingsReport>,
}

impl PrefixCacheSaver {
    pub fn new() -> Self {
        Self::with_config(PrefixCacheConfig::default())
    }

    pub fn with_config(config: PrefixCacheConfig) -> Self {
        Self {
            config,
            last_prefix_hash: Mutex::new(None),
            prefix_hit_streak: Mutex::new(0),
            report: Mutex::new(TokenSavingsReport::default()),
        }
    }

    /// Separate messages into stable prefix (system) and dynamic tail.
    fn partition_messages(&self, messages: &[Message]) -> (Vec<usize>, Vec<usize>) {
        let mut stable = Vec::new();
        let mut dynamic = Vec::new();
        for (i, msg) in messages.iter().enumerate() {
            if self.config.stable_roles.contains(&msg.role) {
                stable.push(i);
            } else {
                dynamic.push(i);
            }
        }
        (stable, dynamic)
    }

    /// Compute a deterministic hash of the stable prefix portion.
    fn hash_prefix(messages: &[Message], tools: &[ToolSchema]) -> blake3::Hash {
        let mut hasher = blake3::Hasher::new();
        for msg in messages {
            hasher.update(msg.role.as_bytes());
            hasher.update(msg.content.as_bytes());
        }
        for tool in tools {
            hasher.update(tool.name.as_bytes());
            hasher.update(tool.description.as_bytes());
            hasher.update(tool.parameters.to_string().as_bytes());
        }
        hasher.finalize()
    }

    /// Look up the discount rate for the current model.
    fn discount_for_model(&self, model: &str) -> f64 {
        // Try exact match first, then prefix match
        if let Some(&d) = self.config.model_discounts.get(model) {
            return d;
        }
        for (pattern, &discount) in &self.config.model_discounts {
            if model.starts_with(pattern.as_str()) {
                return discount;
            }
        }
        // Conservative default
        0.50
    }
}

#[async_trait::async_trait]
impl TokenSaver for PrefixCacheSaver {
    fn name(&self) -> &str { "prefix-cache" }
    fn stage(&self) -> SaverStage { SaverStage::PromptAssembly }
    fn priority(&self) -> u32 { 10 }

    async fn process(
        &self,
        input: SaverInput,
        ctx: &SaverContext,
    ) -> Result<SaverOutput, SaverError> {
        let (stable_idxs, dynamic_idxs) = self.partition_messages(&input.messages);

        // Reorder: stable messages first (prefix), dynamic after
        let mut ordered_messages: Vec<Message> = Vec::with_capacity(input.messages.len());
        for &i in &stable_idxs {
            ordered_messages.push(input.messages[i].clone());
        }
        for &i in &dynamic_idxs {
            ordered_messages.push(input.messages[i].clone());
        }

        // Sort tools deterministically if configured
        let mut tools = input.tools.clone();
        if self.config.sort_tools {
            tools.sort_by(|a, b| a.name.cmp(&b.name));
        }

        // Compute prefix hash (stable messages + sorted tools)
        let prefix_messages: Vec<Message> = stable_idxs.iter()
            .map(|&i| input.messages[i].clone())
            .collect();
        let prefix_hash = Self::hash_prefix(&prefix_messages, &tools);

        // Calculate prefix token count
        let prefix_tokens: usize = prefix_messages.iter()
            .map(|m| m.token_count)
            .sum::<usize>()
            + tools.iter().map(|t| t.token_count).sum::<usize>();

        let total_tokens: usize = ordered_messages.iter()
            .map(|m| m.token_count)
            .sum::<usize>()
            + tools.iter().map(|t| t.token_count).sum::<usize>();

        // Check if prefix is cacheable (≥ min_prefix_tokens)
        let mut cache_hit = false;
        if prefix_tokens >= self.config.min_prefix_tokens {
            let mut last = self.last_prefix_hash.lock().unwrap();
            if *last == Some(prefix_hash) {
                cache_hit = true;
                let mut streak = self.prefix_hit_streak.lock().unwrap();
                *streak += 1;
            } else {
                let mut streak = self.prefix_hit_streak.lock().unwrap();
                *streak = 0;
            }
            *last = Some(prefix_hash);
        }

        // Calculate savings
        let discount = self.discount_for_model(&ctx.model);
        let tokens_saved = if cache_hit {
            (prefix_tokens as f64 * discount) as usize
        } else {
            0
        };

        let description = if cache_hit {
            let streak = self.prefix_hit_streak.lock().unwrap();
            format!(
                "Prefix cache HIT: {} prefix tokens cached, {:.0}% discount for model '{}', streak: {}",
                prefix_tokens, discount * 100.0, ctx.model, *streak
            )
        } else if prefix_tokens < self.config.min_prefix_tokens {
            format!(
                "Prefix too short ({} tokens < {} minimum). Reordered for future caching.",
                prefix_tokens, self.config.min_prefix_tokens
            )
        } else {
            format!(
                "Prefix cache MISS (first turn or prefix changed). Stabilized {} prefix tokens for next turn.",
                prefix_tokens
            )
        };

        let report = TokenSavingsReport {
            technique: "prefix-cache".into(),
            tokens_before: total_tokens,
            tokens_after: total_tokens.saturating_sub(tokens_saved),
            tokens_saved,
            description,
        };
        *self.report.lock().unwrap() = report;

        Ok(SaverOutput {
            messages: ordered_messages,
            tools,
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

    fn sys_msg(content: &str, tokens: usize) -> Message {
        Message {
            role: "system".into(),
            content: content.into(),
            images: vec![],
            tool_call_id: None,
            token_count: tokens,
        }
    }

    fn user_msg(content: &str, tokens: usize) -> Message {
        Message {
            role: "user".into(),
            content: content.into(),
            images: vec![],
            tool_call_id: None,
            token_count: tokens,
        }
    }

    fn tool(name: &str, tokens: usize) -> ToolSchema {
        ToolSchema {
            name: name.into(),
            description: format!("{} tool", name),
            parameters: serde_json::json!({}),
            token_count: tokens,
        }
    }

    #[tokio::test]
    async fn test_prefix_reorder_and_stability() {
        let saver = PrefixCacheSaver::new();
        let ctx = SaverContext {
            model: "gpt-4o".into(),
            turn_number: 1,
            ..Default::default()
        };
        let input = SaverInput {
            messages: vec![
                user_msg("hello", 100),
                sys_msg("You are helpful.", 600),
            ],
            tools: vec![tool("read_file", 500)],
            images: vec![],
            turn_number: 1,
        };

        let out = saver.process(input, &ctx).await.unwrap();
        // System message should be first (stable prefix)
        assert_eq!(out.messages[0].role, "system");
        assert_eq!(out.messages[1].role, "user");

        let report = saver.last_savings();
        // First call = miss
        assert_eq!(report.tokens_saved, 0);
    }

    #[tokio::test]
    async fn test_cache_hit_on_second_call() {
        let saver = PrefixCacheSaver::new();
        let ctx = SaverContext {
            model: "gpt-5".into(),
            turn_number: 1,
            ..Default::default()
        };
        let input = SaverInput {
            messages: vec![
                sys_msg("You are a coding assistant. Help the user with their code.", 1100),
            ],
            tools: vec![tool("read_file", 200)],
            images: vec![],
            turn_number: 1,
        };

        // First call: miss
        let _ = saver.process(input.clone(), &ctx).await.unwrap();
        assert_eq!(saver.last_savings().tokens_saved, 0);

        // Second call with same prefix: hit
        let _ = saver.process(input, &ctx).await.unwrap();
        let report = saver.last_savings();
        // GPT-5 = 90% discount on 1300 prefix tokens
        assert!(report.tokens_saved > 0);
        assert!(report.description.contains("HIT"));
    }
}
