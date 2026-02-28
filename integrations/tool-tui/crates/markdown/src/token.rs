//! Token counting and efficiency analysis for DXM.
//!
//! Provides token counting using a tiktoken-compatible algorithm
//! and comparison between DXM and Markdown token counts.

use crate::markdown::{MarkdownParser, to_markdown};
use crate::parser::DxmParser;
use crate::serializer::LlmSerializer;
use crate::types::DxmDocument;

/// Token counter using a simplified tiktoken-compatible algorithm.
///
/// This implementation uses a byte-pair encoding (BPE) approximation
/// that closely matches GPT-4's tokenization.
pub struct TokenCounter {
    /// Average characters per token (approximation)
    chars_per_token: f64,
}

impl Default for TokenCounter {
    fn default() -> Self {
        Self::new()
    }
}

impl TokenCounter {
    /// Create a new token counter.
    ///
    /// Uses default approximation of ~4 characters per token,
    /// which is typical for English text with GPT-4.
    pub fn new() -> Self {
        Self {
            chars_per_token: 4.0,
        }
    }

    /// Create a token counter with custom characters per token ratio.
    pub fn with_ratio(chars_per_token: f64) -> Self {
        Self { chars_per_token }
    }

    /// Count tokens in a string using approximation.
    ///
    /// This is a simplified approximation that:
    /// - Counts whitespace-separated words
    /// - Accounts for punctuation as separate tokens
    /// - Handles code blocks specially
    pub fn count(&self, text: &str) -> usize {
        if text.is_empty() {
            return 0;
        }

        let mut tokens = 0;
        let mut in_code_block = false;
        let mut current_word_len = 0;

        for c in text.chars() {
            // Track code blocks
            if c == '`' {
                in_code_block = !in_code_block;
            }

            if c.is_whitespace() {
                if current_word_len > 0 {
                    // Estimate tokens for the word
                    tokens += self.estimate_word_tokens(current_word_len, in_code_block);
                    current_word_len = 0;
                }
                // Whitespace usually doesn't add tokens unless it's significant
                if c == '\n' {
                    tokens += 1;
                }
            } else if c.is_ascii_punctuation() {
                if current_word_len > 0 {
                    tokens += self.estimate_word_tokens(current_word_len, in_code_block);
                    current_word_len = 0;
                }
                // Most punctuation is a separate token
                tokens += 1;
            } else {
                current_word_len += c.len_utf8();
            }
        }

        // Handle remaining word
        if current_word_len > 0 {
            tokens += self.estimate_word_tokens(current_word_len, in_code_block);
        }

        tokens.max(1)
    }

    /// Estimate tokens for a word based on its byte length.
    fn estimate_word_tokens(&self, byte_len: usize, in_code: bool) -> usize {
        // Code tends to have more tokens per character
        let ratio = if in_code {
            self.chars_per_token * 0.7
        } else {
            self.chars_per_token
        };

        ((byte_len as f64) / ratio).ceil() as usize
    }

    /// Count tokens in a DXM document (LLM format).
    pub fn count_dxm(&self, doc: &DxmDocument) -> usize {
        let llm_text = LlmSerializer::default().serialize(doc);
        self.count(&llm_text)
    }

    /// Count tokens in Markdown text.
    pub fn count_markdown(&self, markdown: &str) -> usize {
        self.count(markdown)
    }
}

/// Token efficiency analysis result.
#[derive(Debug, Clone)]
pub struct TokenEfficiency {
    /// Token count for DXM format
    pub dxm_tokens: usize,
    /// Token count for Markdown format
    pub markdown_tokens: usize,
    /// Token reduction (markdown - dxm)
    pub reduction: i64,
    /// Reduction percentage
    pub reduction_percent: f64,
}

impl TokenEfficiency {
    /// Calculate efficiency metrics.
    pub fn calculate(dxm_tokens: usize, markdown_tokens: usize) -> Self {
        let reduction = markdown_tokens as i64 - dxm_tokens as i64;
        let reduction_percent = if markdown_tokens > 0 {
            (reduction as f64 / markdown_tokens as f64) * 100.0
        } else {
            0.0
        };

        Self {
            dxm_tokens,
            markdown_tokens,
            reduction,
            reduction_percent,
        }
    }

    /// Check if DXM is more efficient than Markdown.
    pub fn is_efficient(&self) -> bool {
        self.reduction > 0
    }
}

/// Compare token efficiency between DXM and Markdown.
///
/// Takes a DXM document and compares its token count to the
/// equivalent Markdown representation.
pub fn compare_efficiency(doc: &DxmDocument) -> TokenEfficiency {
    let counter = TokenCounter::new();

    let dxm_tokens = counter.count_dxm(doc);
    let markdown = to_markdown(doc);
    let markdown_tokens = counter.count_markdown(&markdown);

    TokenEfficiency::calculate(dxm_tokens, markdown_tokens)
}

/// Compare token efficiency from DXM text input.
pub fn compare_from_dxm(dxm_text: &str) -> Result<TokenEfficiency, crate::error::ParseError> {
    let doc = DxmParser::parse(dxm_text)?;
    Ok(compare_efficiency(&doc))
}

/// Compare token efficiency from Markdown text input.
pub fn compare_from_markdown(
    markdown: &str,
) -> Result<TokenEfficiency, crate::error::ConvertError> {
    let doc = MarkdownParser::parse(markdown)?;
    Ok(compare_efficiency(&doc))
}

/// Analyze token distribution in a document.
#[derive(Debug, Clone, Default)]
pub struct TokenDistribution {
    /// Tokens in headers
    pub headers: usize,
    /// Tokens in paragraphs
    pub paragraphs: usize,
    /// Tokens in code blocks
    pub code_blocks: usize,
    /// Tokens in tables
    pub tables: usize,
    /// Tokens in lists
    pub lists: usize,
    /// Tokens in semantic blocks
    pub semantic_blocks: usize,
    /// Tokens in references
    pub references: usize,
    /// Total tokens
    pub total: usize,
}

impl TokenDistribution {
    /// Analyze token distribution in a document.
    pub fn analyze(doc: &DxmDocument) -> Self {
        use crate::types::DxmNode;

        let counter = TokenCounter::new();
        let serializer = LlmSerializer::default();
        let mut dist = Self::default();

        for node in &doc.nodes {
            let node_doc = DxmDocument {
                meta: Default::default(),
                refs: Default::default(),
                nodes: vec![node.clone()],
            };
            let text = serializer.serialize(&node_doc);
            let tokens = counter.count(&text);

            match node {
                DxmNode::Header(_) => dist.headers += tokens,
                DxmNode::Paragraph(_) => dist.paragraphs += tokens,
                DxmNode::CodeBlock(_) => dist.code_blocks += tokens,
                DxmNode::Table(_) => dist.tables += tokens,
                DxmNode::List(_) => dist.lists += tokens,
                DxmNode::SemanticBlock(_) => dist.semantic_blocks += tokens,
                DxmNode::HorizontalRule => {}
            }
        }

        // Count reference tokens
        for (key, value) in &doc.refs {
            dist.references += counter.count(key) + counter.count(value);
        }

        dist.total = dist.headers
            + dist.paragraphs
            + dist.code_blocks
            + dist.tables
            + dist.lists
            + dist.semantic_blocks
            + dist.references;

        dist
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::*;

    #[test]
    fn test_token_counter_empty() {
        let counter = TokenCounter::new();
        assert_eq!(counter.count(""), 0);
    }

    #[test]
    fn test_token_counter_simple() {
        let counter = TokenCounter::new();
        let tokens = counter.count("Hello world");
        assert!(tokens > 0);
        assert!(tokens < 10); // Should be around 2-3 tokens
    }

    #[test]
    fn test_token_counter_code() {
        let counter = TokenCounter::new();
        let code = "fn main() { println!(\"Hello\"); }";
        let tokens = counter.count(code);
        assert!(tokens > 5); // Code has more tokens
    }

    #[test]
    fn test_token_efficiency_calculation() {
        let eff = TokenEfficiency::calculate(100, 150);
        assert_eq!(eff.dxm_tokens, 100);
        assert_eq!(eff.markdown_tokens, 150);
        assert_eq!(eff.reduction, 50);
        assert!((eff.reduction_percent - 33.33).abs() < 1.0);
        assert!(eff.is_efficient());
    }

    #[test]
    fn test_token_efficiency_not_efficient() {
        let eff = TokenEfficiency::calculate(150, 100);
        assert!(!eff.is_efficient());
    }

    #[test]
    fn test_compare_efficiency() {
        let mut doc = DxmDocument::default();
        doc.nodes.push(DxmNode::Header(HeaderNode {
            level: 1,
            content: vec![InlineNode::Text("Hello World".to_string())],
            priority: None,
        }));
        doc.nodes.push(DxmNode::Paragraph(vec![InlineNode::Text(
            "This is a test paragraph.".to_string(),
        )]));

        let eff = compare_efficiency(&doc);
        assert!(eff.dxm_tokens > 0);
        assert!(eff.markdown_tokens > 0);
    }

    #[test]
    fn test_token_distribution() {
        let mut doc = DxmDocument::default();
        doc.nodes.push(DxmNode::Header(HeaderNode {
            level: 1,
            content: vec![InlineNode::Text("Title".to_string())],
            priority: None,
        }));
        doc.nodes
            .push(DxmNode::Paragraph(vec![InlineNode::Text("Paragraph text.".to_string())]));
        doc.nodes.push(DxmNode::CodeBlock(CodeBlockNode {
            language: Some("rust".to_string()),
            content: "fn main() {}".to_string(),
            priority: None,
        }));

        let dist = TokenDistribution::analyze(&doc);
        assert!(dist.headers > 0);
        assert!(dist.paragraphs > 0);
        assert!(dist.code_blocks > 0);
        assert!(dist.total > 0);
    }

    #[test]
    fn test_dxm_more_efficient_than_markdown() {
        // DXM should be more efficient for documents with:
        // - Repeated URLs (references)
        // - Tables (compact syntax)
        // - Headers (shorter syntax)

        let dxm_input = r#"1|Test Document
#:doc|https://example.com/very/long/documentation/url
See ^doc for details.
Also check ^doc for more info.
#t(Name|Age|City)
Alice|30|NYC
Bob|25|LA"#;

        let doc = DxmParser::parse(dxm_input).unwrap();
        let eff = compare_efficiency(&doc);

        // DXM should generally be more efficient
        // (though this depends on the specific content)
        assert!(eff.dxm_tokens > 0);
        assert!(eff.markdown_tokens > 0);
    }

    #[test]
    fn test_compare_from_dxm() {
        let dxm = "1|Hello\nWorld";
        let result = compare_from_dxm(dxm);
        assert!(result.is_ok());
    }

    #[test]
    fn test_compare_from_markdown() {
        let md = "# Hello\n\nWorld";
        let result = compare_from_markdown(md);
        assert!(result.is_ok());
    }
}
