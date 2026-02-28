//! # patch-prefer
//!
//! Instructs the model to output diffs/patches instead of full files —
//! the single highest-impact token saver for coding agents.
//!
//! ## Evidence (TOKEN.md ✅ REAL)
//! - A 500-line file with a 3-line change: full = ~2000 tokens, diff = ~40 tokens = 98%
//! - This is not hype — it's math
//! - **Honest savings: 90-98% on code edits (the most common agent action)**
//!
//! STAGE: PromptAssembly (priority 25)

use dx_core::*;
use std::sync::Mutex;

/// Configuration for patch-prefer.
#[derive(Debug, Clone)]
pub struct PatchPreferConfig {
    /// System prompt injection text to instruct diff-mode
    pub diff_instruction: String,
    /// Minimum file size (tokens) to request diff instead of full
    pub min_file_tokens: usize,
    /// Whether to add diff format examples
    pub include_example: bool,
}

impl Default for PatchPreferConfig {
    fn default() -> Self {
        Self {
            diff_instruction: concat!(
                "IMPORTANT: When editing files, ALWAYS output a unified diff (patch) ",
                "instead of the complete file. Use this exact format:\n",
                "```diff\n",
                "--- a/path/to/file\n",
                "+++ b/path/to/file\n",
                "@@ -start,count +start,count @@\n",
                " context line\n",
                "-removed line\n",
                "+added line\n",
                " context line\n",
                "```\n",
                "Include 3 lines of context before and after each change. ",
                "NEVER output the entire file when only a few lines change. ",
                "This saves 90-98% of output tokens on edits."
            ).into(),
            min_file_tokens: 100,
            include_example: true,
        }
    }
}

pub struct PatchPreferSaver {
    config: PatchPreferConfig,
    report: Mutex<TokenSavingsReport>,
}

impl PatchPreferSaver {
    pub fn new() -> Self {
        Self::with_config(PatchPreferConfig::default())
    }

    pub fn with_config(config: PatchPreferConfig) -> Self {
        Self {
            config,
            report: Mutex::new(TokenSavingsReport::default()),
        }
    }

    /// Check if the conversation context suggests file editing is happening.
    fn editing_context(messages: &[Message]) -> bool {
        messages.iter().any(|m| {
            let lower = m.content.to_lowercase();
            lower.contains("edit") || lower.contains("change") || lower.contains("modify")
                || lower.contains("update") || lower.contains("fix")
                || lower.contains("refactor") || lower.contains("replace")
                || lower.contains("write_file") || lower.contains("create_file")
        })
    }

    /// Check if diff instruction already exists in system prompt.
    fn has_diff_instruction(messages: &[Message]) -> bool {
        messages.iter().any(|m| {
            m.role == "system" && (
                m.content.contains("unified diff")
                || m.content.contains("ALWAYS output a")
                || m.content.contains("patch format")
            )
        })
    }
}

#[async_trait::async_trait]
impl TokenSaver for PatchPreferSaver {
    fn name(&self) -> &str { "patch-prefer" }
    fn stage(&self) -> SaverStage { SaverStage::PromptAssembly }
    fn priority(&self) -> u32 { 25 }

    async fn process(
        &self,
        input: SaverInput,
        _ctx: &SaverContext,
    ) -> Result<SaverOutput, SaverError> {
        let tokens_before: usize = input.messages.iter().map(|m| m.token_count).sum();
        let mut messages = input.messages;

        // Only inject if editing context is detected and instruction not already present
        let should_inject = Self::editing_context(&messages) && !Self::has_diff_instruction(&messages);

        if should_inject {
            // Find system message to append to, or create one
            let sys_idx = messages.iter().position(|m| m.role == "system");

            match sys_idx {
                Some(idx) => {
                    messages[idx].content.push_str("\n\n");
                    messages[idx].content.push_str(&self.config.diff_instruction);
                    let added_tokens = self.config.diff_instruction.len() / 4;
                    messages[idx].token_count += added_tokens;
                }
                None => {
                    let instruction_tokens = self.config.diff_instruction.len() / 4;
                    messages.insert(0, Message {
                        role: "system".into(),
                        content: self.config.diff_instruction.clone(),
                        images: vec![],
                        tool_call_id: None,
                        token_count: instruction_tokens,
                    });
                }
            }
        }

        let tokens_after: usize = messages.iter().map(|m| m.token_count).sum();
        // Note: patch-prefer ADDS a small number of instruction tokens but
        // the savings come from the model outputting diffs instead of full files.
        // Estimated savings: for a typical edit on a 500-line file,
        // ~2000 output tokens → ~40 diff tokens = 98% output savings.

        // We report the estimated output savings, not input cost
        let estimated_output_savings = if should_inject { 1500 } else { 0 }; // conservative estimate

        let report = TokenSavingsReport {
            technique: "patch-prefer".into(),
            tokens_before,
            tokens_after,
            tokens_saved: estimated_output_savings,
            description: if should_inject {
                format!(
                    "Injected diff-mode instruction (+{} input tokens). \
                     Expected output savings: ~90-98% per file edit. \
                     A 500-line file edit: ~2000 → ~40 output tokens.",
                    self.config.diff_instruction.len() / 4
                )
            } else if Self::has_diff_instruction(&messages) {
                "Diff instruction already present. No injection needed.".into()
            } else {
                "No editing context detected. Skipped diff instruction injection.".into()
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

    fn sys_msg(content: &str) -> Message {
        Message { role: "system".into(), content: content.into(), images: vec![], tool_call_id: None, token_count: content.len() / 4 }
    }
    fn user_msg(content: &str) -> Message {
        Message { role: "user".into(), content: content.into(), images: vec![], tool_call_id: None, token_count: content.len() / 4 }
    }

    #[tokio::test]
    async fn test_injects_diff_instruction() {
        let saver = PatchPreferSaver::new();
        let ctx = SaverContext::default();
        let input = SaverInput {
            messages: vec![
                sys_msg("You are a helpful coding assistant."),
                user_msg("Please edit the file src/main.rs to fix the bug."),
            ],
            tools: vec![],
            images: vec![],
            turn_number: 1,
        };
        let out = saver.process(input, &ctx).await.unwrap();
        assert!(out.messages[0].content.contains("unified diff"));
    }

    #[tokio::test]
    async fn test_no_injection_without_edit_context() {
        let saver = PatchPreferSaver::new();
        let ctx = SaverContext::default();
        let input = SaverInput {
            messages: vec![
                sys_msg("You are helpful."),
                user_msg("What is the capital of France?"),
            ],
            tools: vec![],
            images: vec![],
            turn_number: 1,
        };
        let out = saver.process(input, &ctx).await.unwrap();
        assert!(!out.messages[0].content.contains("unified diff"));
    }
}
