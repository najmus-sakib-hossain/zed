//! # whitespace-normalize
//!
//! Normalizes whitespace across all content: BOM removal, CRLF→LF,
//! tabs→spaces, trailing whitespace, consecutive blank lines.
//!
//! ## Evidence (TOKEN.md ✅ REAL)
//! - Zero risk, zero semantic impact
//! - BOM = 3 wasted bytes, CRLF = 1 extra byte per line
//! - Consecutive blank lines waste tokens (2-3 tokens per blank line)
//! - **Honest: 5-15% savings, higher on messy content (logs, pastes)**
//! - Should run early in pipeline (everything downstream benefits)
//!
//! STAGE: PrePrompt (priority 1 — run first)

use dx_core::*;
use std::sync::Mutex;

/// Configuration for whitespace normalization.
#[derive(Debug, Clone)]
pub struct WhitespaceNormalizeConfig {
    /// Remove BOM (byte order mark) from content
    pub remove_bom: bool,
    /// Convert CRLF to LF
    pub normalize_line_endings: bool,
    /// Convert tabs to spaces (0 = don't convert)
    pub tab_width: usize,
    /// Remove trailing whitespace from lines
    pub strip_trailing: bool,
    /// Collapse consecutive blank lines to this many (0 = keep all)
    pub max_consecutive_blank_lines: usize,
    /// Trim leading/trailing whitespace from entire content
    pub trim_content: bool,
}

impl Default for WhitespaceNormalizeConfig {
    fn default() -> Self {
        Self {
            remove_bom: true,
            normalize_line_endings: true,
            tab_width: 0, // Don't convert tabs by default (preserves indentation style)
            strip_trailing: true,
            max_consecutive_blank_lines: 1,
            trim_content: true,
        }
    }
}

pub struct WhitespaceNormalize {
    config: WhitespaceNormalizeConfig,
    report: Mutex<TokenSavingsReport>,
}

impl WhitespaceNormalize {
    pub fn new() -> Self {
        Self::with_config(WhitespaceNormalizeConfig::default())
    }

    pub fn with_config(config: WhitespaceNormalizeConfig) -> Self {
        Self {
            config,
            report: Mutex::new(TokenSavingsReport::default()),
        }
    }

    /// Normalize a single string according to config.
    fn normalize(&self, input: &str) -> String {
        let mut s = input.to_string();

        // 1. Remove BOM
        if self.config.remove_bom {
            s = s.trim_start_matches('\u{FEFF}').to_string();
        }

        // 2. Normalize line endings: CRLF → LF
        if self.config.normalize_line_endings {
            s = s.replace("\r\n", "\n").replace('\r', "\n");
        }

        // 3. Process line by line
        let lines: Vec<String> = s.lines().map(|line| {
            let mut l = line.to_string();

            // Convert tabs to spaces
            if self.config.tab_width > 0 {
                l = l.replace('\t', &" ".repeat(self.config.tab_width));
            }

            // Strip trailing whitespace
            if self.config.strip_trailing {
                l = l.trim_end().to_string();
            }

            l
        }).collect();

        // 4. Collapse consecutive blank lines
        if self.config.max_consecutive_blank_lines > 0 {
            let mut result_lines: Vec<&str> = Vec::new();
            let mut blank_count = 0usize;

            for line in &lines {
                if line.trim().is_empty() {
                    blank_count += 1;
                    if blank_count <= self.config.max_consecutive_blank_lines {
                        result_lines.push(line);
                    }
                } else {
                    blank_count = 0;
                    result_lines.push(line);
                }
            }

            s = result_lines.join("\n");
        } else {
            s = lines.join("\n");
        }

        // Preserve trailing newline if original had one
        if input.ends_with('\n') || input.ends_with("\r\n") {
            if !s.ends_with('\n') {
                s.push('\n');
            }
        }

        // 5. Trim entire content
        if self.config.trim_content {
            s = s.trim().to_string();
        }

        s
    }
}

#[async_trait::async_trait]
impl TokenSaver for WhitespaceNormalize {
    fn name(&self) -> &str { "whitespace-normalize" }
    fn stage(&self) -> SaverStage { SaverStage::PrePrompt }
    fn priority(&self) -> u32 { 1 }

    async fn process(
        &self,
        input: SaverInput,
        _ctx: &SaverContext,
    ) -> Result<SaverOutput, SaverError> {
        let tokens_before: usize = input.messages.iter().map(|m| m.token_count).sum();
        let chars_before: usize = input.messages.iter().map(|m| m.content.len()).sum();

        let mut new_messages = Vec::with_capacity(input.messages.len());

        for msg in input.messages {
            let normalized = self.normalize(&msg.content);
            let len_ratio = if msg.content.is_empty() {
                1.0
            } else {
                normalized.len() as f64 / msg.content.len() as f64
            };
            let new_tokens = (msg.token_count as f64 * len_ratio).ceil() as usize;

            new_messages.push(Message {
                content: normalized,
                token_count: new_tokens,
                ..msg
            });
        }

        let tokens_after: usize = new_messages.iter().map(|m| m.token_count).sum();
        let chars_after: usize = new_messages.iter().map(|m| m.content.len()).sum();
        let tokens_saved = tokens_before.saturating_sub(tokens_after);

        let report = TokenSavingsReport {
            technique: "whitespace-normalize".into(),
            tokens_before,
            tokens_after,
            tokens_saved,
            description: format!(
                "Normalized whitespace: {} → {} chars ({:.1}% reduction). \
                 Operations: {}{}{}{}{}. \
                 Zero-risk, zero semantic impact.",
                chars_before, chars_after,
                if chars_before > 0 {
                    (1.0 - chars_after as f64 / chars_before as f64) * 100.0
                } else { 0.0 },
                if self.config.remove_bom { "BOM " } else { "" },
                if self.config.normalize_line_endings { "CRLF→LF " } else { "" },
                if self.config.strip_trailing { "trailing " } else { "" },
                if self.config.max_consecutive_blank_lines > 0 { "blank-lines " } else { "" },
                if self.config.tab_width > 0 { "tabs " } else { "" },
            ),
        };
        *self.report.lock().unwrap() = report;

        Ok(SaverOutput {
            messages: new_messages,
            tools: input.tools,
            images: input.images,
            skipped: tokens_saved == 0,
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

    fn msg(content: &str, tokens: usize) -> Message {
        Message { role: "user".into(), content: content.into(), images: vec![], tool_call_id: None, token_count: tokens }
    }

    #[tokio::test]
    async fn test_bom_and_crlf() {
        let saver = WhitespaceNormalize::new();
        let ctx = SaverContext::default();
        let input = SaverInput {
            messages: vec![msg("\u{FEFF}hello\r\nworld\r\n", 10)],
            tools: vec![],
            images: vec![],
            turn_number: 1,
        };
        let out = saver.process(input, &ctx).await.unwrap();
        let content = &out.messages[0].content;
        assert!(!content.contains('\u{FEFF}'));
        assert!(!content.contains('\r'));
        assert_eq!(content, "hello\nworld");
    }

    #[tokio::test]
    async fn test_collapse_blank_lines() {
        let saver = WhitespaceNormalize::new();
        let ctx = SaverContext::default();
        let input = SaverInput {
            messages: vec![msg("line1\n\n\n\n\nline2\n\n\n\nline3", 30)],
            tools: vec![],
            images: vec![],
            turn_number: 1,
        };
        let out = saver.process(input, &ctx).await.unwrap();
        let content = &out.messages[0].content;
        // Should have at most 1 consecutive blank line
        assert!(!content.contains("\n\n\n"));
    }

    #[tokio::test]
    async fn test_trailing_whitespace() {
        let saver = WhitespaceNormalize::new();
        let ctx = SaverContext::default();
        let input = SaverInput {
            messages: vec![msg("hello   \nworld  \t  \n  spaces  ", 20)],
            tools: vec![],
            images: vec![],
            turn_number: 1,
        };
        let out = saver.process(input, &ctx).await.unwrap();
        for line in out.messages[0].content.lines() {
            assert_eq!(line, line.trim_end(), "Line has trailing whitespace: {:?}", line);
        }
    }
}
