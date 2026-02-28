//! # prompt-compress
//!
//! Rule-based prompt compression — removes filler phrases and collapses whitespace.
//!
//! ## Evidence (TOKEN.md ⚠️ Overclaimed 3x)
//! - Your rule-based approach saves **5-15%**, NOT 15-40%
//! - 15-40% requires neural LLMLingua (GPT-2/LLaMA perplexity-based token selection)
//! - Rule-based filler removal + whitespace is "whitespace normalization, not prompt compression"
//! - **Honest savings: 5-15% with rule-based approach**
//!
//! STAGE: PrePrompt (priority 25)

use dx_core::*;
use std::sync::Mutex;

/// Configuration for prompt compression.
#[derive(Debug, Clone)]
pub struct PromptCompressConfig {
    /// Filler phrases to remove (case-insensitive)
    pub filler_phrases: Vec<String>,
    /// Whether to collapse multiple spaces/newlines
    pub collapse_whitespace: bool,
    /// Whether to remove empty lines
    pub remove_empty_lines: bool,
    /// Maximum consecutive newlines to allow
    pub max_consecutive_newlines: usize,
    /// Roles to compress (don't compress system prompts by default)
    pub compress_roles: Vec<String>,
    /// Minimum message tokens before compression kicks in
    pub min_tokens: usize,
}

impl Default for PromptCompressConfig {
    fn default() -> Self {
        Self {
            filler_phrases: vec![
                "I think that ".into(),
                "I believe that ".into(),
                "I would like you to ".into(),
                "Could you please ".into(),
                "Would you mind ".into(),
                "Please note that ".into(),
                "It is important to note that ".into(),
                "As you can see, ".into(),
                "In other words, ".into(),
                "Basically, ".into(),
                "Essentially, ".into(),
                "As I mentioned earlier, ".into(),
                "To be honest, ".into(),
                "Kind of ".into(),
                "Sort of ".into(),
                "That being said, ".into(),
                "At the end of the day, ".into(),
                "Moving forward, ".into(),
                "In terms of ".into(),
                "With respect to ".into(),
                "In fact, ".into(),
            ],
            collapse_whitespace: true,
            remove_empty_lines: true,
            max_consecutive_newlines: 2,
            compress_roles: vec!["user".into(), "assistant".into(), "tool".into()],
            min_tokens: 50,
        }
    }
}

pub struct PromptCompressSaver {
    config: PromptCompressConfig,
    report: Mutex<TokenSavingsReport>,
}

impl PromptCompressSaver {
    pub fn new() -> Self {
        Self::with_config(PromptCompressConfig::default())
    }

    pub fn with_config(config: PromptCompressConfig) -> Self {
        Self {
            config,
            report: Mutex::new(TokenSavingsReport::default()),
        }
    }

    /// Compress a single text string using rule-based techniques.
    fn compress_text(&self, text: &str) -> String {
        let mut result = text.to_string();

        // Remove filler phrases (case-insensitive)
        for filler in &self.config.filler_phrases {
            let lower = result.to_lowercase();
            let filler_lower = filler.to_lowercase();
            while let Some(pos) = lower.find(&filler_lower) {
                // Only remove if at start of sentence or after punctuation/newline
                let before = if pos > 0 { result.as_bytes().get(pos - 1).copied() } else { Some(b'\n') };
                if matches!(before, Some(b'\n') | Some(b'.') | Some(b'!') | Some(b'?') | Some(b' ') | None) {
                    result = format!("{}{}", &result[..pos], &result[pos + filler.len()..]);
                    break; // Re-search from scratch since indices shifted
                } else {
                    break;
                }
            }
        }

        // Collapse multiple spaces
        if self.config.collapse_whitespace {
            while result.contains("  ") {
                result = result.replace("  ", " ");
            }
        }

        // Collapse multiple newlines
        if self.config.remove_empty_lines {
            let max_nl = "\n".repeat(self.config.max_consecutive_newlines);
            let excess = "\n".repeat(self.config.max_consecutive_newlines + 1);
            while result.contains(&excess) {
                result = result.replace(&excess, &max_nl);
            }
        }

        // Trim trailing whitespace per line
        result = result.lines()
            .map(|line| line.trim_end())
            .collect::<Vec<_>>()
            .join("\n");

        result
    }
}

#[async_trait::async_trait]
impl TokenSaver for PromptCompressSaver {
    fn name(&self) -> &str { "prompt-compress" }
    fn stage(&self) -> SaverStage { SaverStage::PrePrompt }
    fn priority(&self) -> u32 { 25 }

    async fn process(
        &self,
        input: SaverInput,
        _ctx: &SaverContext,
    ) -> Result<SaverOutput, SaverError> {
        let tokens_before: usize = input.messages.iter().map(|m| m.token_count).sum();
        let mut messages = input.messages;

        for msg in &mut messages {
            if !self.config.compress_roles.contains(&msg.role) { continue; }
            if msg.token_count < self.config.min_tokens { continue; }

            let compressed = self.compress_text(&msg.content);
            let new_tokens = compressed.len() / 4;
            if new_tokens < msg.token_count {
                msg.content = compressed;
                msg.token_count = new_tokens;
            }
        }

        let tokens_after: usize = messages.iter().map(|m| m.token_count).sum();
        let tokens_saved = tokens_before.saturating_sub(tokens_after);
        let pct = if tokens_before > 0 { tokens_saved as f64 / tokens_before as f64 * 100.0 } else { 0.0 };

        let report = TokenSavingsReport {
            technique: "prompt-compress".into(),
            tokens_before,
            tokens_after,
            tokens_saved,
            description: format!(
                "Rule-based compression: {} → {} tokens ({:.1}% saved). \
                 Filler removal + whitespace collapse. \
                 NOTE: For 15-40% savings, integrate LLMLingua neural compression.",
                tokens_before, tokens_after, pct
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

    #[test]
    fn test_filler_removal() {
        let saver = PromptCompressSaver::new();
        let text = "I think that the function should return an error. Basically, you need to handle the edge case.";
        let compressed = saver.compress_text(text);
        assert!(!compressed.contains("I think that"));
        assert!(compressed.contains("function"));
    }

    #[test]
    fn test_whitespace_collapse() {
        let saver = PromptCompressSaver::new();
        let text = "hello    world\n\n\n\n\nfoo";
        let compressed = saver.compress_text(text);
        assert!(!compressed.contains("    "));
        assert!(!compressed.contains("\n\n\n"));
    }
}
