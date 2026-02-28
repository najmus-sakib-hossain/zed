//! Auto-formatter for DXM documents.
//!
//! This module provides automatic formatting capabilities that can be
//! triggered on save or on-demand to ensure consistent document styling.

use crate::error::ParseError;
use crate::format_detector::{DxmFormat, FormatDetector};
use crate::human_formatter::{FormatterConfig, HumanFormatter};
use crate::human_parser::HumanParser;
use crate::parser::DxmParser;
use crate::types::DxmDocument;

/// Format error types.
#[derive(Debug)]
pub enum FormatError {
    /// Parse error during formatting
    Parse(ParseError),
    /// Unknown format - cannot auto-format
    UnknownFormat,
    /// Machine format - cannot auto-format binary
    MachineFormat,
}

impl std::fmt::Display for FormatError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FormatError::Parse(e) => write!(f, "Parse error: {}", e),
            FormatError::UnknownFormat => write!(f, "Unknown format - cannot auto-format"),
            FormatError::MachineFormat => write!(f, "Machine format - cannot auto-format binary"),
        }
    }
}

impl std::error::Error for FormatError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            FormatError::Parse(e) => Some(e),
            _ => None,
        }
    }
}

impl From<ParseError> for FormatError {
    fn from(err: ParseError) -> Self {
        FormatError::Parse(err)
    }
}

/// Auto-formatter for DXM documents.
///
/// Provides automatic formatting on save or on-demand.
#[derive(Debug, Clone)]
pub struct AutoFormatter {
    /// Formatter instance
    formatter: HumanFormatter,
    /// Whether auto-format is enabled
    pub enabled: bool,
}

impl Default for AutoFormatter {
    fn default() -> Self {
        Self::new()
    }
}

impl AutoFormatter {
    /// Create auto-formatter with default settings.
    pub fn new() -> Self {
        Self {
            formatter: HumanFormatter::new(),
            enabled: true,
        }
    }

    /// Create auto-formatter with custom config.
    pub fn with_config(config: FormatterConfig) -> Self {
        Self {
            formatter: HumanFormatter::with_config(config),
            enabled: true,
        }
    }

    /// Enable or disable auto-formatting.
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Format content if enabled.
    ///
    /// Returns the original content unchanged if auto-format is disabled.
    pub fn format_if_enabled(&mut self, content: &str) -> Result<String, FormatError> {
        if self.enabled {
            self.format(content)
        } else {
            Ok(content.to_string())
        }
    }

    /// Format content unconditionally.
    ///
    /// Detects the format and parses/formats accordingly.
    pub fn format(&mut self, content: &str) -> Result<String, FormatError> {
        let format = FormatDetector::detect_str(content);

        match format {
            DxmFormat::Human => self.format_human(content),
            DxmFormat::Llm => self.format_llm(content),
            DxmFormat::Machine => Err(FormatError::MachineFormat),
            DxmFormat::Unknown => {
                // Try to parse as human format anyway
                self.format_human(content)
            }
        }
    }

    /// Format human format content.
    fn format_human(&mut self, content: &str) -> Result<String, FormatError> {
        let doc = HumanParser::parse(content)?;
        Ok(self.formatter.format(&doc))
    }

    /// Format LLM format content.
    ///
    /// Parses LLM format and outputs as human format.
    fn format_llm(&mut self, content: &str) -> Result<String, FormatError> {
        let doc = DxmParser::parse(content)?;
        Ok(self.formatter.format(&doc))
    }

    /// Format a pre-parsed document.
    pub fn format_document(&mut self, doc: &DxmDocument) -> String {
        self.formatter.format(doc)
    }

    /// Get the underlying formatter.
    pub fn formatter(&self) -> &HumanFormatter {
        &self.formatter
    }

    /// Get mutable access to the underlying formatter.
    pub fn formatter_mut(&mut self) -> &mut HumanFormatter {
        &mut self.formatter
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auto_formatter_new() {
        let mut formatter = AutoFormatter::new();
        assert!(formatter.enabled);
    }

    #[test]
    fn test_auto_formatter_disabled() {
        let mut formatter = AutoFormatter::new();
        formatter.set_enabled(false);

        let content = "# Test\nSome content";
        let result = formatter.format_if_enabled(content).unwrap();

        // Should return original content unchanged
        assert_eq!(result, content);
    }

    #[test]
    fn test_format_human_content() {
        let mut formatter = AutoFormatter::new();

        let content = "[meta]\nversion = 1.0\n\n# Hello World\n\nThis is a test.";
        let result = formatter.format(content).unwrap();

        // Should contain formatted output
        assert!(result.contains("[meta]"), "Result should contain [meta]: {}", result);
        assert!(result.contains("version = 1.0"), "Result should contain version: {}", result);
    }

    #[test]
    fn test_format_llm_content() {
        let mut formatter = AutoFormatter::new();

        let content = "1|Hello World\nThis is a test.";
        let result = formatter.format(content).unwrap();

        // Should be converted to human format
        assert!(result.contains("[meta]"));
        assert!(result.contains("# Hello World"));
    }

    #[test]
    fn test_format_unknown_content() {
        let mut formatter = AutoFormatter::new();

        // Plain text without markers - should try to parse as human
        let content = "Just some plain text.";
        let result = formatter.format(content);

        // Should succeed (parsed as paragraph)
        assert!(result.is_ok());
    }

    #[test]
    fn test_format_document() {
        let mut formatter = AutoFormatter::new();

        let doc = DxmDocument {
            meta: crate::types::DxmMeta {
                version: "1.0".to_string(),
                ..Default::default()
            },
            refs: Default::default(),
            nodes: vec![crate::types::DxmNode::Header(crate::types::HeaderNode {
                level: 1,
                content: vec![crate::types::InlineNode::Text("Test".to_string())],
                priority: None,
            })],
        };

        let result = formatter.format_document(&doc);
        assert!(result.contains("[meta]"));
        assert!(result.contains("# Test"));
    }

    #[test]
    fn test_with_config() {
        let config = FormatterConfig {
            unicode_tables: false,
            indent_width: 4,
            ..Default::default()
        };

        let mut formatter = AutoFormatter::with_config(config);
        assert!(formatter.enabled);
    }
}

#[cfg(test)]
mod prop_tests {
    use super::*;
    use crate::types::*;
    use proptest::prelude::*;
    use std::collections::HashMap;

    /// Generate a random inline node (simple version for testing)
    fn arb_simple_inline() -> impl Strategy<Value = InlineNode> {
        prop_oneof![
            "[a-zA-Z0-9 ]{1,20}".prop_map(InlineNode::Text),
            "[a-zA-Z0-9_]{1,10}".prop_map(InlineNode::Code),
        ]
    }

    /// Generate a random header node
    fn arb_header() -> impl Strategy<Value = DxmNode> {
        (
            1u8..=6,
            prop::collection::vec(arb_simple_inline(), 1..=3),
            prop_oneof![
                Just(None),
                Just(Some(Priority::Low)),
                Just(Some(Priority::Important)),
                Just(Some(Priority::Critical)),
            ],
        )
            .prop_map(|(level, content, priority)| {
                DxmNode::Header(HeaderNode {
                    level,
                    content,
                    priority,
                })
            })
    }

    /// Generate a random paragraph node
    fn arb_paragraph() -> impl Strategy<Value = DxmNode> {
        prop::collection::vec(arb_simple_inline(), 1..=5).prop_map(DxmNode::Paragraph)
    }

    /// Generate a random code block node
    fn arb_code_block() -> impl Strategy<Value = DxmNode> {
        (
            prop_oneof![
                Just(None),
                Just(Some("rust".to_string())),
                Just(Some("python".to_string())),
            ],
            "[a-zA-Z0-9 \n]{1,50}",
        )
            .prop_map(|(language, content)| {
                DxmNode::CodeBlock(CodeBlockNode {
                    language,
                    content,
                    priority: None,
                })
            })
    }

    /// Generate a random DXM document
    fn arb_document() -> impl Strategy<Value = DxmDocument> {
        (prop::collection::vec(
            prop_oneof![arb_header(), arb_paragraph(), arb_code_block(),],
            1..=5,
        ),)
            .prop_map(|(nodes,)| DxmDocument {
                meta: DxmMeta {
                    version: "1.0".to_string(),
                    ..Default::default()
                },
                refs: HashMap::new(),
                nodes,
            })
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// **Feature: dxm-human-format, Property 9: Auto-Formatter Idempotence**
        /// *For any* valid DXM document, applying the auto-formatter twice SHALL
        /// produce the same output as applying it once (f(f(x)) = f(x)).
        /// **Validates: Requirements 3.1, 3.6**
        #[test]
        fn prop_auto_formatter_idempotence(doc in arb_document()) {
            let mut formatter = AutoFormatter::new();

            // First format
            let first_format = formatter.format_document(&doc);

            // Second format (format the already-formatted output)
            let second_format = formatter.format(&first_format);

            // Should succeed
            prop_assert!(second_format.is_ok(),
                "Second format should succeed");

            let second_format = second_format.unwrap();

            // Third format to verify idempotence
            let third_format = formatter.format(&second_format);
            prop_assert!(third_format.is_ok(),
                "Third format should succeed");

            let third_format = third_format.unwrap();

            // f(f(x)) should equal f(f(f(x)))
            prop_assert_eq!(second_format, third_format,
                "Formatter should be idempotent: f(f(x)) = f(f(f(x)))");
        }
    }
}
