//! # output-truncator
//!
//! Smart head+tail truncation of long tool outputs.
//!
//! ## Evidence (TOKEN.md ⚠️ Partly Real)
//! - 95% truncation is extreme — showing 5% of data may miss critical info
//! - Works great for huge file listings, error logs, directory trees
//! - Dangerous for code files where the middle matters
//! - **Honest savings: 30-60% safely, more on genuinely long outputs**
//!
//! STAGE: PostResponse (priority 10)

use dx_core::*;
use std::sync::Mutex;

/// Configuration for output truncation.
#[derive(Debug, Clone)]
pub struct OutputTruncatorConfig {
    /// Max tokens for a single tool output before truncation
    pub max_output_tokens: usize,
    /// Number of head lines to preserve
    pub head_lines: usize,
    /// Number of tail lines to preserve
    pub tail_lines: usize,
    /// Patterns that indicate the output should NOT be truncated
    /// (e.g., code, JSON structures — middle matters)
    pub no_truncate_patterns: Vec<String>,
    /// Patterns that indicate aggressive truncation is safe
    /// (e.g., directory listings, logs)
    pub safe_truncate_patterns: Vec<String>,
}

impl Default for OutputTruncatorConfig {
    fn default() -> Self {
        Self {
            max_output_tokens: 1500,
            head_lines: 30,
            tail_lines: 30,
            no_truncate_patterns: vec![
                "fn ".into(), "def ".into(), "class ".into(),
                "impl ".into(), "pub ".into(), "{".into(),
            ],
            safe_truncate_patterns: vec![
                "total ".into(), "drwx".into(), "-rw-".into(),  // ls output
                "warning:".into(), "error:".into(),              // compiler output
                "│".into(), "├".into(), "└".into(),             // tree output
            ],
        }
    }
}

pub struct OutputTruncatorSaver {
    config: OutputTruncatorConfig,
    report: Mutex<TokenSavingsReport>,
}

impl OutputTruncatorSaver {
    pub fn new() -> Self {
        Self::with_config(OutputTruncatorConfig::default())
    }

    pub fn with_config(config: OutputTruncatorConfig) -> Self {
        Self {
            config,
            report: Mutex::new(TokenSavingsReport::default()),
        }
    }

    /// Classify whether truncation is safe for this content.
    fn classify_content(&self, content: &str) -> TruncationSafety {
        let lines: Vec<&str> = content.lines().take(20).collect();
        let sample = lines.join("\n").to_lowercase();

        // Check if it looks like code (dangerous to truncate)
        let code_signals: usize = self.config.no_truncate_patterns.iter()
            .filter(|p| sample.contains(p.as_str()))
            .count();

        // Check if it looks like logs/listings (safe to truncate)
        let safe_signals: usize = self.config.safe_truncate_patterns.iter()
            .filter(|p| sample.contains(p.as_str()))
            .count();

        if code_signals >= 3 {
            TruncationSafety::Dangerous
        } else if safe_signals >= 2 {
            TruncationSafety::Safe
        } else if content.lines().count() > 100 {
            TruncationSafety::Moderate // long content, probably ok
        } else {
            TruncationSafety::Dangerous
        }
    }

    /// Truncate content to head + tail with an omission marker.
    fn truncate(&self, content: &str, safety: TruncationSafety) -> String {
        let lines: Vec<&str> = content.lines().collect();
        let total_lines = lines.len();

        let (head, tail) = match safety {
            TruncationSafety::Safe => (self.config.head_lines, self.config.tail_lines),
            TruncationSafety::Moderate => {
                // More conservative: keep more context
                (self.config.head_lines + 10, self.config.tail_lines + 10)
            }
            TruncationSafety::Dangerous => {
                // Very conservative: keep most content
                let keep = total_lines * 2 / 3; // keep 66%
                (keep / 2, keep / 2)
            }
        };

        if total_lines <= head + tail + 5 {
            return content.to_string(); // Not worth truncating
        }

        let omitted = total_lines - head - tail;
        let mut result = String::new();

        for line in &lines[..head] {
            result.push_str(line);
            result.push('\n');
        }

        result.push_str(&format!(
            "\n[... {} lines omitted ({} → {} lines kept) ...]\n\n",
            omitted, total_lines, head + tail
        ));

        for line in &lines[total_lines - tail..] {
            result.push_str(line);
            result.push('\n');
        }

        result
    }
}

#[derive(Debug, Clone, Copy)]
enum TruncationSafety {
    Safe,
    Moderate,
    Dangerous,
}

#[async_trait::async_trait]
impl TokenSaver for OutputTruncatorSaver {
    fn name(&self) -> &str { "output-truncator" }
    fn stage(&self) -> SaverStage { SaverStage::PostResponse }
    fn priority(&self) -> u32 { 10 }

    async fn process(
        &self,
        input: SaverInput,
        _ctx: &SaverContext,
    ) -> Result<SaverOutput, SaverError> {
        let tokens_before: usize = input.messages.iter().map(|m| m.token_count).sum();
        let mut messages = input.messages;
        let mut truncated_count = 0usize;

        for msg in &mut messages {
            // Only truncate tool outputs and long assistant messages
            if (msg.role == "tool" || msg.role == "assistant")
                && msg.token_count > self.config.max_output_tokens
            {
                let safety = self.classify_content(&msg.content);
                let truncated = self.truncate(&msg.content, safety);
                let new_tokens = truncated.len() / 4;

                if new_tokens < msg.token_count {
                    msg.content = truncated;
                    msg.token_count = new_tokens;
                    truncated_count += 1;
                }
            }
        }

        let tokens_after: usize = messages.iter().map(|m| m.token_count).sum();
        let tokens_saved = tokens_before.saturating_sub(tokens_after);
        let pct = if tokens_before > 0 { tokens_saved as f64 / tokens_before as f64 * 100.0 } else { 0.0 };

        let report = TokenSavingsReport {
            technique: "output-truncator".into(),
            tokens_before,
            tokens_after,
            tokens_saved,
            description: if truncated_count > 0 {
                format!("Truncated {} long outputs: {} → {} tokens ({:.1}% saved). Content-aware safety classification applied.", truncated_count, tokens_before, tokens_after, pct)
            } else {
                "No outputs exceeded truncation threshold.".into()
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

    fn tool_msg(content: &str) -> Message {
        Message {
            role: "tool".into(),
            content: content.into(),
            images: vec![],
            tool_call_id: Some("tc_1".into()),
            token_count: content.len() / 4,
        }
    }

    #[tokio::test]
    async fn test_no_truncation_short_output() {
        let saver = OutputTruncatorSaver::new();
        let ctx = SaverContext::default();
        let input = SaverInput {
            messages: vec![tool_msg("short output")],
            tools: vec![],
            images: vec![],
            turn_number: 1,
        };
        let _ = saver.process(input, &ctx).await.unwrap();
        assert_eq!(saver.last_savings().tokens_saved, 0);
    }

    #[tokio::test]
    async fn test_truncation_long_log_output() {
        let config = OutputTruncatorConfig {
            max_output_tokens: 100,
            head_lines: 5,
            tail_lines: 5,
            ..Default::default()
        };
        let saver = OutputTruncatorSaver::with_config(config);
        let ctx = SaverContext::default();

        // Simulate long log output (safe to truncate)
        let mut log_content = String::new();
        for i in 0..200 {
            log_content.push_str(&format!("warning: unused variable `x{}` at line {}\n", i, i));
        }

        let input = SaverInput {
            messages: vec![tool_msg(&log_content)],
            tools: vec![],
            images: vec![],
            turn_number: 1,
        };
        let out = saver.process(input, &ctx).await.unwrap();
        assert!(saver.last_savings().tokens_saved > 0);
        assert!(out.messages[0].content.contains("omitted"));
    }
}
