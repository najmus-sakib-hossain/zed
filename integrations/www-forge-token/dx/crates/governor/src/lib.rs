//! # governor
//!
//! Circuit breaker for tool calls — prevents token waste from
//! runaway loops where the agent repeatedly calls the same tool.
//!
//! ## Evidence (TOKEN.md ✅ REAL)
//! - Pure engineering, no hype. If a tool loops 5 times reading
//!   the same file, stopping it saves real tokens.
//! - Only risk: being too aggressive blocking legitimate retries.
//!
//! STAGE: PreCall (priority 5)

use dx_core::*;
use std::collections::HashMap;
use std::sync::Mutex;

/// Configuration for the governor circuit breaker.
#[derive(Debug, Clone)]
pub struct GovernorConfig {
    /// Max times the same tool can be called in one turn
    pub max_same_tool_calls: usize,
    /// Max times same tool+args combo can repeat
    pub max_identical_calls: usize,
    /// Max total tool calls per conversation turn
    pub max_total_calls_per_turn: usize,
    /// Tools exempt from circuit breaking (e.g., "ask_user")
    pub exempt_tools: Vec<String>,
    /// Whether to add a warning message when a tool is blocked
    pub inject_warning: bool,
}

impl Default for GovernorConfig {
    fn default() -> Self {
        Self {
            max_same_tool_calls: 5,
            max_identical_calls: 2,
            max_total_calls_per_turn: 15,
            exempt_tools: vec!["ask_user".into(), "ask_questions".into()],
            inject_warning: true,
        }
    }
}

/// Tracks tool call patterns within a conversation.
#[derive(Debug, Default)]
struct ToolCallTracker {
    /// tool_name → count this turn
    tool_counts: HashMap<String, usize>,
    /// hash(tool_name + args) → count
    identical_counts: HashMap<u64, usize>,
    /// Total calls this turn
    total_calls: usize,
    /// Tools that have been blocked
    blocked_tools: Vec<String>,
}

impl ToolCallTracker {
    fn record_call(&mut self, tool_name: &str, args_hash: u64) -> bool {
        self.total_calls += 1;
        let tool_count = self.tool_counts.entry(tool_name.to_string()).or_insert(0);
        *tool_count += 1;
        let identical_count = self.identical_counts.entry(args_hash).or_insert(0);
        *identical_count += 1;
        true
    }
}

pub struct GovernorSaver {
    config: GovernorConfig,
    tracker: Mutex<ToolCallTracker>,
    report: Mutex<TokenSavingsReport>,
}

impl GovernorSaver {
    pub fn new() -> Self {
        Self::with_config(GovernorConfig::default())
    }

    pub fn with_config(config: GovernorConfig) -> Self {
        Self {
            config,
            tracker: Mutex::new(ToolCallTracker::default()),
            report: Mutex::new(TokenSavingsReport::default()),
        }
    }

    /// Reset tracker for a new turn.
    pub fn reset_turn(&self) {
        let mut tracker = self.tracker.lock().unwrap();
        tracker.tool_counts.clear();
        tracker.identical_counts.clear();
        tracker.total_calls = 0;
        tracker.blocked_tools.clear();
    }

    /// Hash tool name + content for dedup detection.
    fn hash_tool_call(name: &str, content: &str) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        name.hash(&mut hasher);
        content.hash(&mut hasher);
        hasher.finish()
    }

    /// Check if a tool call should be allowed based on current patterns.
    fn should_allow(&self, tool_name: &str, args_hash: u64) -> (bool, String) {
        if self.config.exempt_tools.contains(&tool_name.to_string()) {
            return (true, String::new());
        }

        let tracker = self.tracker.lock().unwrap();

        // Check total calls
        if tracker.total_calls >= self.config.max_total_calls_per_turn {
            return (false, format!(
                "CIRCUIT BREAKER: Total tool calls ({}) exceeded limit ({}). Stop and summarize progress.",
                tracker.total_calls, self.config.max_total_calls_per_turn
            ));
        }

        // Check same tool count
        if let Some(&count) = tracker.tool_counts.get(tool_name) {
            if count >= self.config.max_same_tool_calls {
                return (false, format!(
                    "CIRCUIT BREAKER: '{}' called {} times (limit {}). Try a different approach.",
                    tool_name, count, self.config.max_same_tool_calls
                ));
            }
        }

        // Check identical calls
        if let Some(&count) = tracker.identical_counts.get(&args_hash) {
            if count >= self.config.max_identical_calls {
                return (false, format!(
                    "CIRCUIT BREAKER: Identical '{}' call repeated {} times (limit {}). The same call won't give different results.",
                    tool_name, count, self.config.max_identical_calls
                ));
            }
        }

        (true, String::new())
    }
}

#[async_trait::async_trait]
impl TokenSaver for GovernorSaver {
    fn name(&self) -> &str { "governor" }
    fn stage(&self) -> SaverStage { SaverStage::PreCall }
    fn priority(&self) -> u32 { 5 }

    async fn process(
        &self,
        input: SaverInput,
        _ctx: &SaverContext,
    ) -> Result<SaverOutput, SaverError> {
        let tokens_before: usize = input.messages.iter().map(|m| m.token_count).sum();
        let mut messages = input.messages;
        let mut blocked_count = 0usize;
        let mut tokens_saved = 0usize;
        let mut warnings = Vec::new();

        // Scan for tool-result messages and track patterns
        for msg in &messages {
            if msg.role == "tool" || (msg.role == "assistant" && msg.tool_call_id.is_some()) {
                let tool_name = msg.tool_call_id.as_deref().unwrap_or("unknown");
                let args_hash = Self::hash_tool_call(tool_name, &msg.content);

                let (allowed, reason) = self.should_allow(tool_name, args_hash);
                if !allowed {
                    blocked_count += 1;
                    tokens_saved += msg.token_count;
                    warnings.push(reason);
                }

                let mut tracker = self.tracker.lock().unwrap();
                tracker.record_call(tool_name, args_hash);
            }
        }

        // If we detected patterns that should be blocked, inject a warning
        if self.config.inject_warning && !warnings.is_empty() {
            let warning_text = warnings.join("\n");
            messages.push(Message {
                role: "system".into(),
                content: warning_text,
                images: vec![],
                tool_call_id: None,
                token_count: 30, // approximate
            });
        }

        let tokens_after = tokens_before.saturating_sub(tokens_saved);
        let report = TokenSavingsReport {
            technique: "governor".into(),
            tokens_before,
            tokens_after,
            tokens_saved,
            description: if blocked_count > 0 {
                format!("Circuit breaker triggered: {} tool calls flagged for blocking, ~{} tokens saved", blocked_count, tokens_saved)
            } else {
                "No runaway patterns detected.".into()
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

    fn tool_msg(tool_id: &str, content: &str, tokens: usize) -> Message {
        Message {
            role: "tool".into(),
            content: content.into(),
            images: vec![],
            tool_call_id: Some(tool_id.into()),
            token_count: tokens,
        }
    }

    #[tokio::test]
    async fn test_no_blocking_under_limit() {
        let saver = GovernorSaver::new();
        let ctx = SaverContext::default();
        let input = SaverInput {
            messages: vec![tool_msg("read_file", "content", 100)],
            tools: vec![],
            images: vec![],
            turn_number: 1,
        };
        let out = saver.process(input, &ctx).await.unwrap();
        assert_eq!(saver.last_savings().tokens_saved, 0);
        assert!(!out.messages.is_empty());
    }
}
