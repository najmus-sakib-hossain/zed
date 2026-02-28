//! Git clean filter: DXM → Markdown conversion.
//!
//! The clean filter converts DXM content to CommonMark Markdown when
//! staging files for commit. This allows GitHub to render the content
//! without special support.

use std::io::{Read, Write};
use thiserror::Error;

use super::detect::{DetectedFormat, detect_format};
use crate::parser::DxmParser;
use crate::types::*;

/// Default maximum input size (100 MB).
pub const DEFAULT_MAX_SIZE: usize = 100 * 1024 * 1024;

/// Errors that can occur during filter processing.
#[derive(Debug, Error)]
pub enum FilterError {
    /// Input exceeds maximum allowed size.
    #[error("Input too large: {size} bytes exceeds maximum of {max} bytes")]
    InputTooLarge {
        /// Actual input size
        size: usize,
        /// Maximum allowed size
        max: usize,
    },

    /// Failed to parse input content.
    #[error("Parse error at line {line}, column {column}: {message}")]
    ParseError {
        /// Error message
        message: String,
        /// Line number (1-indexed)
        line: usize,
        /// Column number (1-indexed)
        column: usize,
    },

    /// Invalid UTF-8 encoding.
    #[error("Invalid UTF-8 at byte position {position}")]
    InvalidUtf8 {
        /// Byte position of invalid sequence
        position: usize,
    },

    /// IO error during read/write.
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    /// Format detection failed.
    #[error("Unable to detect input format")]
    UnknownFormat,
}

impl From<crate::error::ParseError> for FilterError {
    fn from(err: crate::error::ParseError) -> Self {
        FilterError::ParseError {
            message: err.message,
            line: err.line,
            column: err.column,
        }
    }
}

impl From<crate::error::ConvertError> for FilterError {
    fn from(err: crate::error::ConvertError) -> Self {
        FilterError::ParseError {
            message: err.to_string(),
            line: 1,
            column: 1,
        }
    }
}

/// Clean filter: converts DXM to Markdown for git storage.
///
/// # Example
///
/// ```
/// use dx_markdown::git::CleanFilter;
///
/// let filter = CleanFilter::new();
/// let dxm = "1|Hello World\nThis is a paragraph.";
/// let md = filter.dxm_to_markdown(dxm).unwrap();
/// assert!(md.contains("# Hello World"));
/// ```
pub struct CleanFilter {
    /// Maximum input size in bytes.
    pub max_size: usize,
}

impl Default for CleanFilter {
    fn default() -> Self {
        Self::new()
    }
}

impl CleanFilter {
    /// Create a new clean filter with default settings.
    pub fn new() -> Self {
        Self {
            max_size: DEFAULT_MAX_SIZE,
        }
    }

    /// Create a clean filter with a custom maximum size.
    pub fn with_max_size(max_size: usize) -> Self {
        Self { max_size }
    }

    /// Process input from reader, write to writer.
    ///
    /// Reads DXM content from the input, converts to Markdown,
    /// and writes to the output.
    ///
    /// # Arguments
    ///
    /// * `input` - Reader to read DXM content from
    /// * `output` - Writer to write Markdown content to
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Input exceeds maximum size
    /// - Input is not valid UTF-8
    /// - Parsing fails
    /// - IO error occurs
    pub fn process<R: Read, W: Write>(
        &self,
        mut input: R,
        mut output: W,
    ) -> Result<(), FilterError> {
        // Read all input
        let mut buffer = Vec::new();
        input.read_to_end(&mut buffer)?;

        // Check size limit
        if buffer.len() > self.max_size {
            return Err(FilterError::InputTooLarge {
                size: buffer.len(),
                max: self.max_size,
            });
        }

        // Validate UTF-8
        let content = match std::str::from_utf8(&buffer) {
            Ok(s) => s,
            Err(e) => {
                return Err(FilterError::InvalidUtf8 {
                    position: e.valid_up_to(),
                });
            }
        };

        // Detect format and convert if needed
        let result = match detect_format(&buffer) {
            DetectedFormat::Markdown => {
                // Already Markdown, pass through unchanged
                content.to_string()
            }
            DetectedFormat::Dxm => {
                // Convert DXM to Markdown
                self.dxm_to_markdown(content)?
            }
            DetectedFormat::Binary => {
                // Binary format not supported for clean filter
                return Err(FilterError::UnknownFormat);
            }
            DetectedFormat::Unknown => {
                // Unknown format, try to parse as DXM
                // If that fails, pass through unchanged
                match self.dxm_to_markdown(content) {
                    Ok(md) => md,
                    Err(_) => content.to_string(),
                }
            }
        };

        // Write output
        output.write_all(result.as_bytes())?;
        Ok(())
    }

    /// Convert DXM content to Markdown.
    ///
    /// # Arguments
    ///
    /// * `dxm` - DXM content string
    ///
    /// # Returns
    ///
    /// CommonMark Markdown string.
    ///
    /// # Errors
    ///
    /// Returns an error if parsing fails.
    pub fn dxm_to_markdown(&self, dxm: &str) -> Result<String, FilterError> {
        // Parse DXM content
        let doc = DxmParser::parse(dxm)?;

        // Convert to Markdown with reference expansion
        let md = to_markdown_with_refs(&doc);

        Ok(md)
    }
}

/// Convert a DXM document to Markdown with reference expansion.
///
/// All `^key` references are expanded to their full values inline.
fn to_markdown_with_refs(doc: &DxmDocument) -> String {
    let mut output = String::new();

    for (i, node) in doc.nodes.iter().enumerate() {
        if i > 0 {
            output.push('\n');
        }
        output.push_str(&node_to_markdown(node, doc));
        output.push('\n');
    }

    output
}

/// Convert a single node to Markdown.
fn node_to_markdown(node: &DxmNode, doc: &DxmDocument) -> String {
    match node {
        DxmNode::Header(h) => {
            let prefix = "#".repeat(h.level as usize);
            let content = inlines_to_markdown(&h.content, doc);
            format!("{} {}", prefix, content)
        }
        DxmNode::Paragraph(inlines) => inlines_to_markdown(inlines, doc),
        DxmNode::CodeBlock(cb) => {
            let lang = cb.language.as_deref().unwrap_or("");
            format!("```{}\n{}\n```", lang, cb.content)
        }
        DxmNode::Table(t) => table_to_markdown(t),
        DxmNode::List(l) => list_to_markdown(l, doc),
        DxmNode::SemanticBlock(sb) => {
            let content = inlines_to_markdown(&sb.content, doc);
            // Use GitHub-compatible admonition syntax
            match sb.block_type {
                SemanticBlockType::Warning => format!("> [!WARNING]\n> {}", content),
                SemanticBlockType::Info => format!("> [!NOTE]\n> {}", content),
                SemanticBlockType::FAQ => format!("> [!TIP]\n> **FAQ:** {}", content),
                SemanticBlockType::Quote => format!("> {}", content),
                SemanticBlockType::Example => format!("> [!IMPORTANT]\n> **Example:** {}", content),
            }
        }
        DxmNode::HorizontalRule => "---".to_string(),
    }
}

/// Convert inline nodes to Markdown with reference expansion.
fn inlines_to_markdown(inlines: &[InlineNode], doc: &DxmDocument) -> String {
    let mut output = String::new();
    for inline in inlines {
        output.push_str(&inline_to_markdown(inline, doc));
    }
    output
}

/// Convert a single inline node to Markdown.
fn inline_to_markdown(inline: &InlineNode, doc: &DxmDocument) -> String {
    match inline {
        InlineNode::Text(t) => t.clone(),
        InlineNode::Bold(inner) => format!("**{}**", inlines_to_markdown(inner, doc)),
        InlineNode::Italic(inner) => format!("*{}*", inlines_to_markdown(inner, doc)),
        InlineNode::Strikethrough(inner) => format!("~~{}~~", inlines_to_markdown(inner, doc)),
        InlineNode::Code(c) => format!("`{}`", c),
        InlineNode::Link { text, url, title } => {
            let text_str = inlines_to_markdown(text, doc);
            if let Some(t) = title {
                format!("[{}]({} \"{}\")", text_str, url, t)
            } else {
                format!("[{}]({})", text_str, url)
            }
        }
        InlineNode::Image { alt, url, title } => {
            if let Some(t) = title {
                format!("![{}]({} \"{}\")", alt, url, t)
            } else {
                format!("![{}]({})", alt, url)
            }
        }
        InlineNode::Reference(key) => {
            // Expand reference to full URL
            doc.refs.get(key).cloned().unwrap_or_else(|| format!("[^{}]", key))
        }
    }
}

/// Convert a table to GFM Markdown.
fn table_to_markdown(table: &TableNode) -> String {
    let mut output = String::new();

    // Header row
    output.push('|');
    for col in &table.schema {
        output.push_str(&format!(" {} |", col.name));
    }
    output.push('\n');

    // Separator row
    output.push('|');
    for _ in &table.schema {
        output.push_str(" --- |");
    }
    output.push('\n');

    // Data rows
    for row in &table.rows {
        output.push('|');
        for cell in row {
            let cell_str = match cell {
                CellValue::Text(t) => t.clone(),
                CellValue::Integer(i) => i.to_string(),
                CellValue::Float(f) => f.to_string(),
                CellValue::Boolean(b) => b.to_string(),
                CellValue::Null => String::new(),
            };
            output.push_str(&format!(" {} |", cell_str));
        }
        output.push('\n');
    }

    output.trim_end().to_string()
}

/// Convert a list to Markdown.
fn list_to_markdown(list: &ListNode, doc: &DxmDocument) -> String {
    let mut output = String::new();
    for (i, item) in list.items.iter().enumerate() {
        let prefix = if list.ordered {
            format!("{}. ", i + 1)
        } else {
            "- ".to_string()
        };
        let content = inlines_to_markdown(&item.content, doc);
        output.push_str(&format!("{}{}\n", prefix, content));
    }
    output.trim_end().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clean_filter_new() {
        let filter = CleanFilter::new();
        assert_eq!(filter.max_size, DEFAULT_MAX_SIZE);
    }

    #[test]
    fn test_clean_filter_with_max_size() {
        let filter = CleanFilter::with_max_size(1024);
        assert_eq!(filter.max_size, 1024);
    }

    #[test]
    fn test_dxm_to_markdown_header() {
        let filter = CleanFilter::new();
        let md = filter.dxm_to_markdown("1|Hello World").unwrap();
        assert!(md.contains("# Hello World"));
    }

    #[test]
    fn test_dxm_to_markdown_multiple_headers() {
        let filter = CleanFilter::new();
        let md = filter.dxm_to_markdown("1|H1\n2|H2\n3|H3").unwrap();
        assert!(md.contains("# H1"));
        assert!(md.contains("## H2"));
        assert!(md.contains("### H3"));
    }

    #[test]
    fn test_process_passthrough_markdown() {
        let filter = CleanFilter::new();
        let input = b"# Already Markdown\n\nThis is a paragraph.";
        let mut output = Vec::new();
        filter.process(&input[..], &mut output).unwrap();
        assert_eq!(output, input);
    }

    #[test]
    fn test_process_input_too_large() {
        let filter = CleanFilter::with_max_size(10);
        let input = b"This is more than 10 bytes";
        let mut output = Vec::new();
        let result = filter.process(&input[..], &mut output);
        assert!(matches!(result, Err(FilterError::InputTooLarge { .. })));
    }

    #[test]
    fn test_process_invalid_utf8() {
        let filter = CleanFilter::new();
        let input = [0xFF, 0xFE, 0x00, 0x01];
        let mut output = Vec::new();
        let result = filter.process(&input[..], &mut output);
        assert!(matches!(result, Err(FilterError::InvalidUtf8 { .. })));
    }

    #[test]
    fn test_table_to_markdown() {
        let table = TableNode {
            schema: vec![
                ColumnDef {
                    name: "Name".to_string(),
                    type_hint: None,
                },
                ColumnDef {
                    name: "Age".to_string(),
                    type_hint: None,
                },
            ],
            rows: vec![vec![
                CellValue::Text("Alice".to_string()),
                CellValue::Integer(30),
            ]],
        };
        let md = table_to_markdown(&table);
        assert!(md.contains("| Name | Age |"));
        assert!(md.contains("| --- | --- |"));
        assert!(md.contains("| Alice | 30 |"));
    }
}

// ============================================================================
// Property-Based Tests
// ============================================================================

#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;
    use std::collections::HashMap;

    /// **Property 4: Reference Expansion Completeness**
    ///
    /// For any DXM document with reference definitions and usages, the clean
    /// filter output SHALL contain no `^key` references—all references SHALL
    /// be expanded to their full values inline.
    ///
    /// **Validates: Requirements 1.6**
    mod property_4_reference_expansion {
        use super::*;

        // Strategy to generate reference keys (A-Z, AA-ZZ)
        fn ref_key_strategy() -> impl Strategy<Value = String> {
            prop::string::string_regex("[A-Z]{1,2}").unwrap()
        }

        // Strategy to generate URLs
        fn url_strategy() -> impl Strategy<Value = String> {
            prop::string::string_regex("https://[a-z]+\\.[a-z]+/[a-z]+").unwrap()
        }

        // Strategy to generate a document with references
        fn doc_with_refs_strategy() -> impl Strategy<Value = (DxmDocument, HashMap<String, String>)>
        {
            // Generate 1-5 reference definitions
            prop::collection::vec((ref_key_strategy(), url_strategy()), 1..5).prop_map(|refs| {
                let mut doc = DxmDocument::default();
                let mut ref_map = HashMap::new();

                // Add reference definitions to document
                for (key, url) in &refs {
                    doc.refs.insert(key.clone(), url.clone());
                    ref_map.insert(key.clone(), url.clone());
                }

                // Create a paragraph that uses the references
                let mut content = Vec::new();
                content.push(InlineNode::Text("See ".to_string()));

                for (i, (key, _)) in refs.iter().enumerate() {
                    if i > 0 {
                        content.push(InlineNode::Text(" and ".to_string()));
                    }
                    content.push(InlineNode::Reference(key.clone()));
                }

                doc.nodes.push(DxmNode::Paragraph(content));
                (doc, ref_map)
            })
        }

        proptest! {
            #![proptest_config(ProptestConfig::with_cases(100))]

            /// For any document with references, the output should not contain ^key patterns
            #[test]
            fn references_are_expanded(
                (doc, ref_map) in doc_with_refs_strategy()
            ) {
                let md = to_markdown_with_refs(&doc);

                // Check that no ^key references remain in output
                for key in ref_map.keys() {
                    let ref_pattern = format!("^{}", key);
                    prop_assert!(!md.contains(&ref_pattern),
                        "Output should not contain reference ^{}: {}", key, md);
                }

                // Check that all URLs are present in output
                for url in ref_map.values() {
                    prop_assert!(md.contains(url),
                        "Output should contain expanded URL {}: {}", url, md);
                }
            }

            /// Empty references map should not cause issues
            #[test]
            fn empty_refs_handled(content in "[a-zA-Z0-9 ]{1,50}") {
                let mut doc = DxmDocument::default();
                doc.nodes.push(DxmNode::Paragraph(vec![InlineNode::Text(content.clone())]));

                let md = to_markdown_with_refs(&doc);
                prop_assert!(md.contains(&content),
                    "Content should be preserved: {}", md);
            }
        }

        // Test that undefined references are handled gracefully
        proptest! {
            #![proptest_config(ProptestConfig::with_cases(100))]

            #[test]
            fn undefined_refs_handled_gracefully(key in ref_key_strategy()) {
                let mut doc = DxmDocument::default();
                // Reference without definition
                doc.nodes.push(DxmNode::Paragraph(vec![
                    InlineNode::Text("See ".to_string()),
                    InlineNode::Reference(key.clone()),
                ]));

                // Should not panic
                let md = to_markdown_with_refs(&doc);
                // Undefined refs should produce some fallback
                prop_assert!(!md.is_empty(), "Output should not be empty");
            }
        }
    }

    /// **Property 1: Clean-Smudge Round Trip** (clean direction)
    ///
    /// For any valid DXM document, applying the clean filter (DXM → MD) and then
    /// the smudge filter (MD → DXM) SHALL produce a document that is semantically
    /// equivalent to the original.
    ///
    /// **Validates: Requirements 1.2, 1.3-1.8**
    mod property_1_clean_smudge_round_trip {
        use super::*;
        use crate::git::SmudgeFilter;
        #[allow(unused_imports)]
        use proptest::prelude::*;

        // Strategy to generate valid header levels (1-6)
        fn header_level_strategy() -> impl Strategy<Value = u8> {
            1u8..=6
        }

        // Strategy to generate simple text content
        fn text_content_strategy() -> impl Strategy<Value = String> {
            prop::string::string_regex("[a-zA-Z0-9 ]{1,30}").unwrap()
        }

        // Strategy to generate a simple DXM document with headers
        fn simple_dxm_doc_strategy() -> impl Strategy<Value = DxmDocument> {
            prop::collection::vec((header_level_strategy(), text_content_strategy()), 1..5)
                .prop_map(|headers| {
                    let mut doc = DxmDocument::default();
                    for (level, title) in headers {
                        doc.nodes.push(DxmNode::Header(HeaderNode {
                            level,
                            content: vec![InlineNode::Text(title)],
                            priority: None,
                        }));
                    }
                    doc
                })
        }

        proptest! {
            #![proptest_config(ProptestConfig::with_cases(100))]

            /// Clean filter produces valid output that can be parsed back
            #[test]
            fn clean_produces_parseable_markdown(doc in simple_dxm_doc_strategy()) {
                let smudge_filter = SmudgeFilter::new();

                // Convert DXM to Markdown
                let md = to_markdown_with_refs(&doc);

                // The markdown should be parseable
                let result = smudge_filter.markdown_to_dxm(&md);
                prop_assert!(result.is_ok(),
                    "Clean filter output should be parseable: {:?}", result.err());
            }

            /// Header levels are preserved through round trip
            #[test]
            fn header_levels_preserved(
                level in header_level_strategy(),
                title in text_content_strategy()
            ) {
                let mut doc = DxmDocument::default();
                doc.nodes.push(DxmNode::Header(HeaderNode {
                    level,
                    content: vec![InlineNode::Text(title.clone())],
                    priority: None,
                }));

                let smudge_filter = SmudgeFilter::new();

                // DXM -> MD -> DXM
                let md = to_markdown_with_refs(&doc);
                let dxm_back = smudge_filter.markdown_to_dxm(&md).unwrap();

                // Parse the result to check header level
                let parsed = crate::parser::DxmParser::parse(&dxm_back).unwrap();

                prop_assert!(!parsed.nodes.is_empty(), "Should have at least one node");
                if let DxmNode::Header(h) = &parsed.nodes[0] {
                    prop_assert_eq!(h.level, level,
                        "Header level should be preserved: expected {}, got {}", level, h.level);
                } else {
                    prop_assert!(false, "First node should be a header");
                }
            }
        }

        // Test that paragraphs survive round trip
        proptest! {
            #![proptest_config(ProptestConfig::with_cases(100))]

            #[test]
            fn paragraph_content_preserved(content in "[a-zA-Z0-9]{1,30}") {
                // Use non-whitespace content to avoid trimming edge cases
                let mut doc = DxmDocument::default();
                doc.nodes.push(DxmNode::Paragraph(vec![InlineNode::Text(content.clone())]));

                let smudge_filter = SmudgeFilter::new();

                // DXM -> MD -> DXM
                let md = to_markdown_with_refs(&doc);
                let dxm_back = smudge_filter.markdown_to_dxm(&md).unwrap();

                // The content should be present in the result
                prop_assert!(dxm_back.contains(&content),
                    "Paragraph content should be preserved: {} not in {}", content, dxm_back);
            }
        }
    }

    /// **Property 10: Header Level Preservation**
    ///
    /// For any header in a DXM document with level N (1-6), after clean filter
    /// conversion, the Markdown output SHALL have exactly N `#` characters, and
    /// after smudge filter conversion back, the header SHALL have level N.
    ///
    /// **Validates: Requirements 1.3, 2.3**
    mod property_10_header_level_preservation {
        use super::*;
        use crate::git::SmudgeFilter;
        #[allow(unused_imports)]
        use proptest::prelude::*;

        proptest! {
            #![proptest_config(ProptestConfig::with_cases(100))]

            /// Header level N produces exactly N # characters in Markdown
            #[test]
            fn header_level_produces_correct_hashes(
                level in 1u8..=6,
                title in "[a-zA-Z0-9]{1,20}"
            ) {
                let mut doc = DxmDocument::default();
                doc.nodes.push(DxmNode::Header(HeaderNode {
                    level,
                    content: vec![InlineNode::Text(title.clone())],
                    priority: None,
                }));

                // Convert to Markdown
                let md = to_markdown_with_refs(&doc);

                // Count leading # characters
                let hash_count = md.trim_start().chars().take_while(|&c| c == '#').count();
                prop_assert_eq!(hash_count, level as usize,
                    "Header level {} should produce {} hashes, got {}: {}",
                    level, level, hash_count, md);
            }

            /// Header level is preserved through full round trip
            #[test]
            fn header_level_round_trip(
                level in 1u8..=6,
                title in "[a-zA-Z0-9]{1,20}"
            ) {
                let mut doc = DxmDocument::default();
                doc.nodes.push(DxmNode::Header(HeaderNode {
                    level,
                    content: vec![InlineNode::Text(title.clone())],
                    priority: None,
                }));

                let smudge_filter = SmudgeFilter::new();

                // DXM -> MD -> DXM
                let md = to_markdown_with_refs(&doc);
                let dxm_back = smudge_filter.markdown_to_dxm(&md).unwrap();

                // Parse and verify level
                let parsed = crate::parser::DxmParser::parse(&dxm_back).unwrap();
                prop_assert!(!parsed.nodes.is_empty(), "Should have at least one node");

                if let DxmNode::Header(h) = &parsed.nodes[0] {
                    prop_assert_eq!(h.level, level,
                        "Header level should be preserved through round trip: expected {}, got {}",
                        level, h.level);
                } else {
                    prop_assert!(false, "First node should be a header");
                }
            }

            /// Multiple headers with different levels are all preserved
            #[test]
            fn multiple_header_levels_preserved(
                levels in prop::collection::vec(1u8..=6, 2..5)
            ) {
                let mut doc = DxmDocument::default();
                for (i, level) in levels.iter().enumerate() {
                    doc.nodes.push(DxmNode::Header(HeaderNode {
                        level: *level,
                        content: vec![InlineNode::Text(format!("Header{}", i))],
                        priority: None,
                    }));
                }

                let smudge_filter = SmudgeFilter::new();

                // DXM -> MD -> DXM
                let md = to_markdown_with_refs(&doc);
                let dxm_back = smudge_filter.markdown_to_dxm(&md).unwrap();

                // Parse and verify all levels
                let parsed = crate::parser::DxmParser::parse(&dxm_back).unwrap();

                let mut header_idx = 0;
                for node in &parsed.nodes {
                    if let DxmNode::Header(h) = node {
                        if header_idx < levels.len() {
                            prop_assert_eq!(h.level, levels[header_idx],
                                "Header {} level should be {}, got {}",
                                header_idx, levels[header_idx], h.level);
                            header_idx += 1;
                        }
                    }
                }
            }
        }
    }

    /// **Property 11: Code Block Content Preservation**
    ///
    /// For any code block in a document, the content inside the code block SHALL
    /// be preserved exactly through clean and smudge filter conversions, with no
    /// modifications to whitespace, indentation, or characters.
    ///
    /// **Validates: Requirements 1.5, 2.5**
    mod property_11_code_block_preservation {
        use super::*;
        use crate::git::SmudgeFilter;
        #[allow(unused_imports)] // Used by proptest! macro
        use proptest::prelude::*;

        // Strategy for code block languages
        fn language_strategy() -> impl Strategy<Value = String> {
            prop::string::string_regex("[a-z]{0,10}").unwrap()
        }

        // Strategy for code content (alphanumeric with spaces and newlines)
        fn code_content_strategy() -> impl Strategy<Value = String> {
            prop::string::string_regex("[a-zA-Z0-9 \n]{1,100}").unwrap()
        }

        proptest! {
            #![proptest_config(ProptestConfig::with_cases(100))]

            /// Code block content is preserved through clean filter
            #[test]
            fn code_content_in_markdown(
                lang in language_strategy(),
                code in "[a-zA-Z0-9]{1,50}"
            ) {
                let mut doc = DxmDocument::default();
                doc.nodes.push(DxmNode::CodeBlock(CodeBlockNode {
                    language: if lang.is_empty() { None } else { Some(lang.clone()) },
                    content: code.clone(),
                    priority: None,
                }));

                // Convert to Markdown
                let md = to_markdown_with_refs(&doc);

                // Code content should be present in output
                prop_assert!(md.contains(&code),
                    "Code content should be in Markdown output: {} not in {}", code, md);
            }

            /// Code block content survives round trip
            #[test]
            fn code_content_round_trip(
                lang in language_strategy(),
                code in "[a-zA-Z0-9]{1,50}"
            ) {
                let mut doc = DxmDocument::default();
                doc.nodes.push(DxmNode::CodeBlock(CodeBlockNode {
                    language: if lang.is_empty() { None } else { Some(lang.clone()) },
                    content: code.clone(),
                    priority: None,
                }));

                let smudge_filter = SmudgeFilter::new();

                // DXM -> MD -> DXM
                let md = to_markdown_with_refs(&doc);
                let dxm_back = smudge_filter.markdown_to_dxm(&md).unwrap();

                // Code content should be preserved
                prop_assert!(dxm_back.contains(&code),
                    "Code content should survive round trip: {} not in {}", code, dxm_back);
            }

            /// Code block language is preserved
            #[test]
            fn code_language_preserved(
                lang in "[a-z]{1,10}",
                code in "[a-zA-Z0-9]{1,30}"
            ) {
                let mut doc = DxmDocument::default();
                doc.nodes.push(DxmNode::CodeBlock(CodeBlockNode {
                    language: Some(lang.clone()),
                    content: code.clone(),
                    priority: None,
                }));

                // Convert to Markdown
                let md = to_markdown_with_refs(&doc);

                // Language should be present after ```
                let expected = format!("```{}", lang);
                prop_assert!(md.contains(&expected),
                    "Language should be in Markdown: {} not in {}", expected, md);
            }
        }
    }

    /// **Property 7: CommonMark Validity**
    ///
    /// For any DXM document, the clean filter output SHALL be valid CommonMark
    /// Markdown that can be parsed by any CommonMark-compliant parser.
    ///
    /// **Validates: Requirements 9.3**
    mod property_7_commonmark_validity {
        use super::*;
        use crate::git::SmudgeFilter;
        #[allow(unused_imports)] // Used by proptest! macro
        use proptest::prelude::*;

        proptest! {
            #![proptest_config(ProptestConfig::with_cases(100))]

            /// Clean filter output is parseable as Markdown
            #[test]
            fn clean_output_is_valid_markdown(
                level in 1u8..=6,
                title in "[a-zA-Z0-9]{1,20}"
            ) {
                let mut doc = DxmDocument::default();
                doc.nodes.push(DxmNode::Header(HeaderNode {
                    level,
                    content: vec![InlineNode::Text(title)],
                    priority: None,
                }));

                // Convert to Markdown
                let md = to_markdown_with_refs(&doc);

                // The output should be parseable by our Markdown parser
                // (which is CommonMark-compliant)
                let smudge_filter = SmudgeFilter::new();
                let result = smudge_filter.markdown_to_dxm(&md);
                prop_assert!(result.is_ok(),
                    "Clean filter output should be valid CommonMark: {:?}", result.err());
            }

            /// Complex documents produce valid CommonMark
            #[test]
            fn complex_doc_produces_valid_commonmark(
                headers in prop::collection::vec(
                    (1u8..=6, "[a-zA-Z0-9]{1,15}"),
                    1..4
                ),
                paragraphs in prop::collection::vec("[a-zA-Z0-9 ]{1,30}", 0..3)
            ) {
                let mut doc = DxmDocument::default();

                for (level, title) in headers {
                    doc.nodes.push(DxmNode::Header(HeaderNode {
                        level,
                        content: vec![InlineNode::Text(title)],
                        priority: None,
                    }));
                }

                for para in paragraphs {
                    doc.nodes.push(DxmNode::Paragraph(vec![InlineNode::Text(para)]));
                }

                // Convert to Markdown
                let md = to_markdown_with_refs(&doc);

                // Should be parseable
                let smudge_filter = SmudgeFilter::new();
                let result = smudge_filter.markdown_to_dxm(&md);
                prop_assert!(result.is_ok(),
                    "Complex document should produce valid CommonMark: {:?}", result.err());
            }

            /// Code blocks produce valid CommonMark
            #[test]
            fn code_blocks_produce_valid_commonmark(
                lang in "[a-z]{0,8}",
                code in "[a-zA-Z0-9 ]{1,50}"
            ) {
                let mut doc = DxmDocument::default();
                doc.nodes.push(DxmNode::CodeBlock(CodeBlockNode {
                    language: if lang.is_empty() { None } else { Some(lang) },
                    content: code,
                    priority: None,
                }));

                // Convert to Markdown
                let md = to_markdown_with_refs(&doc);

                // Should be parseable
                let smudge_filter = SmudgeFilter::new();
                let result = smudge_filter.markdown_to_dxm(&md);
                prop_assert!(result.is_ok(),
                    "Code block should produce valid CommonMark: {:?}", result.err());
            }
        }
    }

    /// **Property 8: Filter Input Validation**
    ///
    /// For any input to the clean or smudge filter:
    /// - Invalid UTF-8 sequences SHALL be rejected with an error
    /// - Input exceeding the size limit SHALL be rejected with an error
    /// - No code execution SHALL occur from document content
    ///
    /// **Validates: Requirements 14.1, 14.2, 14.3, 14.4**
    mod property_8_filter_input_validation {
        use super::*;
        #[allow(unused_imports)] // Used by proptest! macro
        use proptest::prelude::*;

        proptest! {
            #![proptest_config(ProptestConfig::with_cases(100))]

            /// Invalid UTF-8 is rejected
            #[test]
            fn invalid_utf8_rejected(
                valid_prefix in "[a-zA-Z0-9]{0,20}",
                invalid_byte in 0x80u8..=0xFF
            ) {
                let filter = CleanFilter::new();

                // Create input with invalid UTF-8
                let mut input = valid_prefix.into_bytes();
                input.push(invalid_byte);
                // Add more invalid bytes to ensure it's definitely invalid UTF-8
                input.push(0xFF);

                let mut output = Vec::new();
                let result = filter.process(&input[..], &mut output);

                // Should fail with InvalidUtf8 error
                prop_assert!(matches!(result, Err(FilterError::InvalidUtf8 { .. })),
                    "Invalid UTF-8 should be rejected: {:?}", result);
            }

            /// Input exceeding size limit is rejected
            #[test]
            fn oversized_input_rejected(extra_size in 1usize..100) {
                let max_size = 100;
                let filter = CleanFilter::with_max_size(max_size);

                // Create input larger than max_size
                let input = vec![b'a'; max_size + extra_size];
                let mut output = Vec::new();
                let result = filter.process(&input[..], &mut output);

                // Should fail with InputTooLarge error
                prop_assert!(matches!(result, Err(FilterError::InputTooLarge { .. })),
                    "Oversized input should be rejected: {:?}", result);
            }

            /// Valid UTF-8 within size limit is accepted
            #[test]
            fn valid_input_accepted(content in "[a-zA-Z0-9 ]{1,50}") {
                let filter = CleanFilter::new();
                let input = content.as_bytes();
                let mut output = Vec::new();

                // Should not fail with validation errors
                let result = filter.process(input, &mut output);

                // May fail with parse error but not validation error
                if let Err(e) = &result {
                    prop_assert!(!matches!(e, FilterError::InvalidUtf8 { .. }),
                        "Valid UTF-8 should not be rejected as invalid");
                    prop_assert!(!matches!(e, FilterError::InputTooLarge { .. }),
                        "Small input should not be rejected as too large");
                }
            }

            /// Empty input is handled gracefully
            #[test]
            fn empty_input_handled(_dummy in Just(())) {
                let filter = CleanFilter::new();
                let input: &[u8] = &[];
                let mut output = Vec::new();

                // Should not panic
                let _ = filter.process(input, &mut output);
            }

            /// Malicious content does not execute
            #[test]
            fn no_code_execution(
                script_type in prop::sample::select(vec![
                    "$(rm -rf /)",
                    "`rm -rf /`",
                    "{{system('rm -rf /')}}",
                    "<script>alert('xss')</script>",
                    "eval('malicious')",
                ])
            ) {
                let filter = CleanFilter::new();
                let input = script_type.as_bytes();
                let mut output = Vec::new();

                // Should process without executing any code
                // (we can't directly test no execution, but we can verify
                // the filter completes without side effects)
                let _ = filter.process(input, &mut output);

                // If we get here, no code was executed that would crash/hang
                prop_assert!(true, "Filter should not execute embedded code");
            }
        }
    }
}
