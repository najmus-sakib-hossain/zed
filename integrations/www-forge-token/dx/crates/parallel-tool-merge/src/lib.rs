//! # parallel-tool-merge
//!
//! Merges parallel tool call results into a single consolidated message.
//!
//! ## Evidence (TOKEN.md 🚨 HALLUCINATION — re-evaluated as ⚠️)
//! - Original claim of huge savings was overblown
//! - Realistic savings: 3-5% by removing per-call framing overhead
//! - The actual tool output content can't be compressed by merging
//! - **Honest: 3-5% savings from reduced message framing overhead**
//! - Main benefit is structural clarity, not token savings
//!
//! STAGE: PostResponse (priority 5)

use dx_core::*;
use std::sync::Mutex;

/// Configuration for parallel tool merge.
#[derive(Debug, Clone)]
pub struct ParallelToolMergeConfig {
    /// Minimum number of tool results in a row to trigger merge
    pub min_consecutive_tools: usize,
    /// Maximum merged message size (tokens) before we stop merging
    pub max_merged_tokens: usize,
    /// Separator between merged tool outputs
    pub separator: String,
}

impl Default for ParallelToolMergeConfig {
    fn default() -> Self {
        Self {
            min_consecutive_tools: 2,
            max_merged_tokens: 10_000,
            separator: "\n---\n".into(),
        }
    }
}

pub struct ParallelToolMerge {
    config: ParallelToolMergeConfig,
    report: Mutex<TokenSavingsReport>,
}

impl ParallelToolMerge {
    pub fn new() -> Self {
        Self::with_config(ParallelToolMergeConfig::default())
    }

    pub fn with_config(config: ParallelToolMergeConfig) -> Self {
        Self {
            config,
            report: Mutex::new(TokenSavingsReport::default()),
        }
    }
}

#[async_trait::async_trait]
impl TokenSaver for ParallelToolMerge {
    fn name(&self) -> &str { "parallel-tool-merge" }
    fn stage(&self) -> SaverStage { SaverStage::PostResponse }
    fn priority(&self) -> u32 { 5 }

    async fn process(
        &self,
        input: SaverInput,
        _ctx: &SaverContext,
    ) -> Result<SaverOutput, SaverError> {
        let tokens_before: usize = input.messages.iter().map(|m| m.token_count).sum();

        // Find consecutive runs of tool messages
        let mut new_messages: Vec<Message> = Vec::new();
        let mut tool_run: Vec<Message> = Vec::new();
        let mut merged_count = 0usize;
        let mut framing_saved = 0usize;

        for msg in input.messages {
            if msg.role == "tool" {
                tool_run.push(msg);
            } else {
                // Flush accumulated tool messages
                if tool_run.len() >= self.config.min_consecutive_tools {
                    // Merge the run
                    let run_tokens: usize = tool_run.iter().map(|m| m.token_count).sum();
                    if run_tokens <= self.config.max_merged_tokens {
                        let mut merged_content = String::new();
                        let tool_ids: Vec<String> = tool_run.iter()
                            .filter_map(|m| m.tool_call_id.clone())
                            .collect();

                        for (i, tool_msg) in tool_run.iter().enumerate() {
                            if i > 0 {
                                merged_content.push_str(&self.config.separator);
                            }
                            if let Some(ref id) = tool_msg.tool_call_id {
                                merged_content.push_str(&format!("[{}] ", id));
                            }
                            merged_content.push_str(&tool_msg.content);
                        }

                        // Framing overhead: each message has ~4 tokens of framing
                        // Merging N messages into 1 saves (N-1) * 4 tokens
                        let overhead_per_msg = 4;
                        let saved = (tool_run.len() - 1) * overhead_per_msg;
                        framing_saved += saved;
                        merged_count += tool_run.len();

                        let merged_tokens = run_tokens.saturating_sub(saved);
                        new_messages.push(Message {
                            role: "tool".into(),
                            content: merged_content,
                            images: vec![],
                            tool_call_id: tool_ids.into_iter().next(), // Keep first ID
                            token_count: merged_tokens,
                        });
                    } else {
                        // Too large to merge, keep individual
                        new_messages.extend(tool_run.drain(..));
                    }
                } else {
                    new_messages.extend(tool_run.drain(..));
                }
                tool_run.clear();
                new_messages.push(msg);
            }
        }

        // Flush final run
        if tool_run.len() >= self.config.min_consecutive_tools {
            let run_tokens: usize = tool_run.iter().map(|m| m.token_count).sum();
            if run_tokens <= self.config.max_merged_tokens {
                let mut merged_content = String::new();
                let tool_ids: Vec<String> = tool_run.iter()
                    .filter_map(|m| m.tool_call_id.clone())
                    .collect();

                for (i, tool_msg) in tool_run.iter().enumerate() {
                    if i > 0 {
                        merged_content.push_str(&self.config.separator);
                    }
                    if let Some(ref id) = tool_msg.tool_call_id {
                        merged_content.push_str(&format!("[{}] ", id));
                    }
                    merged_content.push_str(&tool_msg.content);
                }

                let overhead_per_msg = 4;
                let saved = (tool_run.len() - 1) * overhead_per_msg;
                framing_saved += saved;
                merged_count += tool_run.len();

                let merged_tokens = run_tokens.saturating_sub(saved);
                new_messages.push(Message {
                    role: "tool".into(),
                    content: merged_content,
                    images: vec![],
                    tool_call_id: tool_ids.into_iter().next(),
                    token_count: merged_tokens,
                });
            } else {
                new_messages.extend(tool_run);
            }
        } else {
            new_messages.extend(tool_run);
        }

        let tokens_after = tokens_before.saturating_sub(framing_saved);

        let report = TokenSavingsReport {
            technique: "parallel-tool-merge".into(),
            tokens_before,
            tokens_after,
            tokens_saved: framing_saved,
            description: if merged_count > 0 {
                format!(
                    "Merged {} tool messages, saved ~{} tokens of framing overhead. \
                     HONEST: Only 3-5% savings from reduced message framing. \
                     The actual content can't be compressed by merging — \
                     main benefit is structural clarity.",
                    merged_count, framing_saved
                )
            } else {
                "No consecutive tool runs found to merge.".into()
            },
        };
        *self.report.lock().unwrap() = report;

        Ok(SaverOutput {
            messages: new_messages,
            tools: input.tools,
            images: input.images,
            skipped: merged_count == 0,
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

    fn tool_msg(id: &str, content: &str, tokens: usize) -> Message {
        Message { role: "tool".into(), content: content.into(), images: vec![], tool_call_id: Some(id.into()), token_count: tokens }
    }

    #[tokio::test]
    async fn test_merges_consecutive_tools() {
        let saver = ParallelToolMerge::new();
        let ctx = SaverContext::default();
        let input = SaverInput {
            messages: vec![
                Message { role: "assistant".into(), content: "calling tools".into(), images: vec![], tool_call_id: None, token_count: 5 },
                tool_msg("call_1", "result one", 50),
                tool_msg("call_2", "result two", 50),
                tool_msg("call_3", "result three", 50),
            ],
            tools: vec![],
            images: vec![],
            turn_number: 1,
        };
        let out = saver.process(input, &ctx).await.unwrap();
        // 1 assistant + 1 merged tool = 2 messages
        assert_eq!(out.messages.len(), 2);
        assert!(saver.last_savings().tokens_saved > 0);
    }

    #[tokio::test]
    async fn test_no_merge_single_tool() {
        let saver = ParallelToolMerge::new();
        let ctx = SaverContext::default();
        let input = SaverInput {
            messages: vec![
                Message { role: "user".into(), content: "hi".into(), images: vec![], tool_call_id: None, token_count: 5 },
                tool_msg("call_1", "result", 50),
                Message { role: "assistant".into(), content: "done".into(), images: vec![], tool_call_id: None, token_count: 5 },
            ],
            tools: vec![],
            images: vec![],
            turn_number: 1,
        };
        let out = saver.process(input, &ctx).await.unwrap();
        assert_eq!(out.messages.len(), 3); // No merge
        assert!(out.skipped);
    }
}
