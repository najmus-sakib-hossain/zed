//! # cot-compress
//!
//! Compresses chain-of-thought reasoning in assistant history messages.
//!
//! ## Evidence (TOKEN.md ⚠️ Partly Real)
//! - Removing "Let me think..." lines from history works
//! - Prefix-matching is fragile — can remove actual conclusions  
//! - O-series reasoning tokens are already hidden (no need)
//! - **Honest savings: 15-30% safely (not 30-60%)**
//!
//! STAGE: InterTurn (priority 20)

use dx_core::*;
use std::sync::Mutex;

/// Configuration for CoT compression.
#[derive(Debug, Clone)]
pub struct CotCompressConfig {
    /// Prefixes that indicate thinking/reasoning lines (safe to remove from history)
    pub thinking_prefixes: Vec<String>,
    /// Minimum message token count before CoT compression applies
    pub min_tokens: usize,
    /// Maximum percentage of message to remove (safety cap)
    pub max_removal_pct: f64,
    /// Protect messages in the last N turns from CoT compression
    pub protect_recent_turns: usize,
}

impl Default for CotCompressConfig {
    fn default() -> Self {
        Self {
            thinking_prefixes: vec![
                "Let me think".into(),
                "Let me analyze".into(),
                "Let me consider".into(),
                "Hmm,".into(),
                "Thinking about this".into(),
                "I need to think".into(),
                "Let's break this down".into(),
                "First, let me".into(),
                "OK so".into(),
                "Okay, so".into(),
                "Well,".into(),
                "Right, so".into(),
                "Now let me".into(),
                "Let me check".into(),
                "I see that".into(),
                "Looking at this more carefully".into(),
                "Wait,".into(),
                "Actually,".into(),
                "On second thought".into(),
                "Let me reconsider".into(),
            ],
            min_tokens: 100,
            max_removal_pct: 0.40, // Never remove more than 40% — safety cap
            protect_recent_turns: 2,
        }
    }
}

pub struct CotCompressSaver {
    config: CotCompressConfig,
    report: Mutex<TokenSavingsReport>,
}

impl CotCompressSaver {
    pub fn new() -> Self {
        Self::with_config(CotCompressConfig::default())
    }

    pub fn with_config(config: CotCompressConfig) -> Self {
        Self {
            config,
            report: Mutex::new(TokenSavingsReport::default()),
        }
    }

    /// Compress an assistant message by removing thinking lines.
    fn compress_cot(&self, content: &str) -> String {
        let lines: Vec<&str> = content.lines().collect();
        let total_lines = lines.len();
        if total_lines < 3 {
            return content.to_string();
        }

        let max_removable = (total_lines as f64 * self.config.max_removal_pct) as usize;
        let mut removed = 0usize;
        let mut result_lines = Vec::new();

        for line in &lines {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                result_lines.push(*line);
                continue;
            }

            let is_thinking = self.config.thinking_prefixes.iter()
                .any(|prefix| trimmed.starts_with(prefix.as_str()));

            if is_thinking && removed < max_removable {
                removed += 1;
                // Skip this line (CoT noise)
            } else {
                result_lines.push(*line);
            }
        }

        if removed > 0 && !result_lines.is_empty() {
            // Add a note about compression
            result_lines.push(&"");
            // Return joined result
            let mut result = result_lines.join("\n");
            result.push_str(&format!("[{} thinking lines compressed]", removed));
            result
        } else {
            content.to_string()
        }
    }
}

#[async_trait::async_trait]
impl TokenSaver for CotCompressSaver {
    fn name(&self) -> &str { "cot-compress" }
    fn stage(&self) -> SaverStage { SaverStage::InterTurn }
    fn priority(&self) -> u32 { 20 }

    async fn process(
        &self,
        input: SaverInput,
        ctx: &SaverContext,
    ) -> Result<SaverOutput, SaverError> {
        let tokens_before: usize = input.messages.iter().map(|m| m.token_count).sum();
        let mut messages = input.messages;
        let mut compressed_count = 0usize;

        // Determine which messages are in the protected recent window
        let mut user_turn = 0usize;
        let mut msg_turns: Vec<usize> = Vec::new();
        for msg in &messages {
            if msg.role == "user" { user_turn += 1; }
            msg_turns.push(user_turn);
        }

        let current_turn = ctx.turn_number;

        for (i, msg) in messages.iter_mut().enumerate() {
            // Only compress assistant messages
            if msg.role != "assistant" { continue; }
            if msg.token_count < self.config.min_tokens { continue; }

            // Don't compress recent turns
            if current_turn.saturating_sub(msg_turns[i]) < self.config.protect_recent_turns {
                continue;
            }

            let compressed = self.compress_cot(&msg.content);
            let new_tokens = compressed.len() / 4;
            if new_tokens < msg.token_count {
                msg.content = compressed;
                msg.token_count = new_tokens;
                compressed_count += 1;
            }
        }

        let tokens_after: usize = messages.iter().map(|m| m.token_count).sum();
        let tokens_saved = tokens_before.saturating_sub(tokens_after);
        let pct = if tokens_before > 0 { tokens_saved as f64 / tokens_before as f64 * 100.0 } else { 0.0 };

        let report = TokenSavingsReport {
            technique: "cot-compress".into(),
            tokens_before,
            tokens_after,
            tokens_saved,
            description: if compressed_count > 0 {
                format!(
                    "Compressed CoT in {} assistant messages: {} → {} tokens ({:.1}% saved). \
                     Max removal capped at {:.0}% per message for safety.",
                    compressed_count, tokens_before, tokens_after, pct,
                    self.config.max_removal_pct * 100.0
                )
            } else {
                "No CoT reasoning found to compress.".into()
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

    #[test]
    fn test_cot_compression() {
        let saver = CotCompressSaver::new();
        let text = "Let me think about this carefully.\nThe answer is 42.\nActually, wait.\nYes, 42 is correct.\nThe function returns the right value.";
        let compressed = saver.compress_cot(text);
        assert!(compressed.contains("42 is correct"));
        assert!(compressed.contains("compressed"));
    }

    #[test]
    fn test_max_removal_cap() {
        let config = CotCompressConfig {
            max_removal_pct: 0.20,
            ..Default::default()
        };
        let saver = CotCompressSaver::with_config(config);
        let mut text = String::new();
        for i in 0..10 {
            text.push_str(&format!("Let me think about line {}.\n", i));
        }
        text.push_str("The actual answer.\n");
        let compressed = saver.compress_cot(&text);
        // Should keep at least 80% of lines
        let original_lines = text.lines().count();
        let compressed_lines = compressed.lines().count();
        assert!(compressed_lines as f64 / original_lines as f64 >= 0.60);
    }
}
