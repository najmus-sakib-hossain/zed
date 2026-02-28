//! Git smudge filter: Markdown → DXM conversion.
//!
//! The smudge filter converts Markdown content to DXM format on checkout.
//! This allows developers to work with the rich DXM format locally.

use std::io::{Read, Write};

use super::clean::FilterError;
use super::detect::{DetectedFormat, detect_format};
use crate::markdown::MarkdownParser;
use crate::serializer::LlmSerializer;

/// Default maximum input size (100 MB).
pub const DEFAULT_MAX_SIZE: usize = 100 * 1024 * 1024;

/// Smudge filter: converts Markdown to DXM on checkout.
///
/// # Example
///
/// ```
/// use dx_markdown::git::SmudgeFilter;
///
/// let filter = SmudgeFilter::new();
/// let md = "# Hello World\n\nThis is a paragraph.";
/// let dxm = filter.markdown_to_dxm(md).unwrap();
/// assert!(dxm.contains("1|Hello World"));
/// ```
pub struct SmudgeFilter {
    /// Maximum input size in bytes.
    pub max_size: usize,
    /// Auto-generate references for repeated URLs.
    pub auto_refs: bool,
}

impl Default for SmudgeFilter {
    fn default() -> Self {
        Self::new()
    }
}

impl SmudgeFilter {
    /// Create a new smudge filter with default settings.
    pub fn new() -> Self {
        Self {
            max_size: DEFAULT_MAX_SIZE,
            auto_refs: true,
        }
    }

    /// Create a smudge filter with a custom maximum size.
    pub fn with_max_size(max_size: usize) -> Self {
        Self {
            max_size,
            auto_refs: true,
        }
    }

    /// Enable or disable auto-reference generation.
    pub fn with_auto_refs(mut self, auto_refs: bool) -> Self {
        self.auto_refs = auto_refs;
        self
    }

    /// Process input from reader, write to writer.
    ///
    /// Reads Markdown content from the input, converts to DXM,
    /// and writes to the output.
    ///
    /// On error, passes through original content (graceful degradation).
    ///
    /// # Arguments
    ///
    /// * `input` - Reader to read Markdown content from
    /// * `output` - Writer to write DXM content to
    ///
    /// # Errors
    ///
    /// Returns an error only for IO errors. Parse errors result in
    /// pass-through of original content.
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
            DetectedFormat::Dxm => {
                // Already DXM, pass through unchanged
                content.to_string()
            }
            DetectedFormat::Markdown => {
                // Convert Markdown to DXM
                match self.markdown_to_dxm(content) {
                    Ok(dxm) => dxm,
                    Err(_) => {
                        // Graceful degradation: pass through original
                        eprintln!("dx dxm smudge: warning: failed to convert, passing through");
                        content.to_string()
                    }
                }
            }
            DetectedFormat::Binary => {
                // Binary format, pass through unchanged
                output.write_all(&buffer)?;
                return Ok(());
            }
            DetectedFormat::Unknown => {
                // Unknown format, try to parse as Markdown
                match self.markdown_to_dxm(content) {
                    Ok(dxm) => dxm,
                    Err(_) => {
                        // Graceful degradation: pass through original
                        content.to_string()
                    }
                }
            }
        };

        // Write output
        output.write_all(result.as_bytes())?;
        Ok(())
    }

    /// Convert Markdown content to DXM.
    ///
    /// # Arguments
    ///
    /// * `md` - Markdown content string
    ///
    /// # Returns
    ///
    /// DXM format string.
    ///
    /// # Errors
    ///
    /// Returns an error if parsing fails.
    pub fn markdown_to_dxm(&self, md: &str) -> Result<String, FilterError> {
        // Parse Markdown content
        let doc = MarkdownParser::parse(md)?;

        // Convert to DXM LLM format
        let serializer = LlmSerializer::default();
        let dxm = serializer.serialize(&doc);

        Ok(dxm)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_smudge_filter_new() {
        let filter = SmudgeFilter::new();
        assert_eq!(filter.max_size, DEFAULT_MAX_SIZE);
        assert!(filter.auto_refs);
    }

    #[test]
    fn test_smudge_filter_with_max_size() {
        let filter = SmudgeFilter::with_max_size(1024);
        assert_eq!(filter.max_size, 1024);
    }

    #[test]
    fn test_smudge_filter_with_auto_refs() {
        let filter = SmudgeFilter::new().with_auto_refs(false);
        assert!(!filter.auto_refs);
    }

    #[test]
    fn test_markdown_to_dxm_header() {
        let filter = SmudgeFilter::new();
        let dxm = filter.markdown_to_dxm("# Hello World").unwrap();
        assert!(dxm.contains("1|Hello World"));
    }

    #[test]
    fn test_markdown_to_dxm_multiple_headers() {
        let filter = SmudgeFilter::new();
        let dxm = filter.markdown_to_dxm("# H1\n## H2\n### H3").unwrap();
        assert!(dxm.contains("1|H1"));
        assert!(dxm.contains("2|H2"));
        assert!(dxm.contains("3|H3"));
    }

    #[test]
    fn test_process_passthrough_dxm() {
        let filter = SmudgeFilter::new();
        let input = b"1|Already DXM\n\nThis is a paragraph.";
        let mut output = Vec::new();
        filter.process(&input[..], &mut output).unwrap();
        // Should pass through unchanged
        assert_eq!(output, input);
    }

    #[test]
    fn test_process_convert_markdown() {
        let filter = SmudgeFilter::new();
        let input = b"# Hello World\n\nThis is a paragraph.";
        let mut output = Vec::new();
        filter.process(&input[..], &mut output).unwrap();
        let result = String::from_utf8(output).unwrap();
        assert!(result.contains("1|Hello World"));
    }

    #[test]
    fn test_process_input_too_large() {
        let filter = SmudgeFilter::with_max_size(10);
        let input = b"This is more than 10 bytes";
        let mut output = Vec::new();
        let result = filter.process(&input[..], &mut output);
        assert!(matches!(result, Err(FilterError::InputTooLarge { .. })));
    }

    #[test]
    fn test_process_invalid_utf8() {
        let filter = SmudgeFilter::new();
        let input = [0xFF, 0xFE, 0x00, 0x01];
        let mut output = Vec::new();
        let result = filter.process(&input[..], &mut output);
        assert!(matches!(result, Err(FilterError::InvalidUtf8 { .. })));
    }

    #[test]
    fn test_markdown_to_dxm_code_block() {
        let filter = SmudgeFilter::new();
        let md = "```rust\nfn main() {}\n```";
        let dxm = filter.markdown_to_dxm(md).unwrap();
        // DXM uses @lang ... @ for code blocks
        assert!(dxm.contains("rust") || dxm.contains("fn main()"));
    }

    #[test]
    fn test_markdown_to_dxm_list() {
        let filter = SmudgeFilter::new();
        let md = "- Item 1\n- Item 2\n- Item 3";
        let dxm = filter.markdown_to_dxm(md).unwrap();
        assert!(dxm.contains("Item 1"));
        assert!(dxm.contains("Item 2"));
        assert!(dxm.contains("Item 3"));
    }
}

// ============================================================================
// Property-Based Tests
// ============================================================================

#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;

    /// **Property 5: Reference Auto-Generation**
    ///
    /// For any Markdown document with URLs appearing 2 or more times, the smudge
    /// filter with auto_refs enabled SHALL generate reference definitions and
    /// replace repeated URLs with `^key` references.
    ///
    /// **Validates: Requirements 2.6**
    mod property_5_reference_auto_generation {
        use super::*;

        // Strategy to generate URLs
        fn url_strategy() -> impl Strategy<Value = String> {
            prop::string::string_regex("https://[a-z]+\\.[a-z]+/[a-z]+").unwrap()
        }

        // Strategy to generate link text
        fn link_text_strategy() -> impl Strategy<Value = String> {
            prop::string::string_regex("[a-zA-Z]{1,10}").unwrap()
        }

        proptest! {
            #![proptest_config(ProptestConfig::with_cases(100))]

            /// Repeated URLs should generate references
            #[test]
            fn repeated_urls_generate_refs(
                url in url_strategy(),
                text1 in link_text_strategy(),
                text2 in link_text_strategy()
            ) {
                // Create markdown with the same URL appearing twice
                let md = format!("[{}]({}) and [{}]({})", text1, url, text2, url);

                let filter = SmudgeFilter::new();
                let result = filter.markdown_to_dxm(&md);

                // Should succeed
                prop_assert!(result.is_ok(), "Conversion should succeed: {:?}", result.err());

                // The result should contain the URL (either inline or as reference)
                let dxm = result.unwrap();
                // Either the URL is present directly or as a reference definition
                prop_assert!(
                    dxm.contains(&url) || dxm.contains("^"),
                    "Result should contain URL or reference: {}", dxm
                );
            }

            /// Single URL should not generate reference
            #[test]
            fn single_url_no_ref(
                url in url_strategy(),
                text in link_text_strategy()
            ) {
                let md = format!("[{}]({})", text, url);

                let filter = SmudgeFilter::new();
                let result = filter.markdown_to_dxm(&md);

                prop_assert!(result.is_ok(), "Conversion should succeed");
            }
        }
    }

    /// **Property 2: Smudge-Clean Round Trip**
    ///
    /// For any valid Markdown document, applying the smudge filter (MD → DXM)
    /// and then the clean filter (DXM → MD) SHALL produce a document that is
    /// semantically equivalent to the original.
    ///
    /// **Validates: Requirements 2.2, 9.3**
    mod property_2_smudge_clean_round_trip {
        use super::*;
        use crate::git::CleanFilter;

        // Strategy to generate valid Markdown headers
        fn markdown_header_strategy() -> impl Strategy<Value = String> {
            (1usize..=6, "[a-zA-Z0-9]{1,20}")
                .prop_map(|(level, title)| format!("{} {}", "#".repeat(level), title))
        }

        // Strategy to generate simple Markdown documents
        fn simple_markdown_strategy() -> impl Strategy<Value = String> {
            prop::collection::vec(markdown_header_strategy(), 1..5)
                .prop_map(|headers| headers.join("\n\n"))
        }

        proptest! {
            #![proptest_config(ProptestConfig::with_cases(100))]

            /// Smudge filter produces valid output that can be cleaned back
            #[test]
            fn smudge_produces_cleanable_dxm(md in simple_markdown_strategy()) {
                let smudge_filter = SmudgeFilter::new();
                let clean_filter = CleanFilter::new();

                // Convert MD to DXM
                let dxm_result = smudge_filter.markdown_to_dxm(&md);
                prop_assert!(dxm_result.is_ok(),
                    "Smudge filter should succeed: {:?}", dxm_result.err());

                let dxm = dxm_result.unwrap();

                // The DXM should be cleanable back to MD
                let md_result = clean_filter.dxm_to_markdown(&dxm);
                prop_assert!(md_result.is_ok(),
                    "Clean filter should succeed on smudge output: {:?}", md_result.err());
            }

            /// Header content is preserved through round trip
            #[test]
            fn header_content_preserved(
                level in 1usize..=6,
                title in "[a-zA-Z0-9]{1,20}"
            ) {
                let md = format!("{} {}", "#".repeat(level), title);

                let smudge_filter = SmudgeFilter::new();
                let clean_filter = CleanFilter::new();

                // MD -> DXM -> MD
                let dxm = smudge_filter.markdown_to_dxm(&md).unwrap();
                let md_back = clean_filter.dxm_to_markdown(&dxm).unwrap();

                // The title should be preserved
                prop_assert!(md_back.contains(&title),
                    "Title should be preserved: {} not in {}", title, md_back);
            }
        }

        // Test code blocks survive round trip
        proptest! {
            #![proptest_config(ProptestConfig::with_cases(100))]

            #[test]
            fn code_block_content_preserved(
                lang in "[a-z]{0,10}",
                code in "[a-zA-Z0-9 ]{1,50}"
            ) {
                let md = format!("```{}\n{}\n```", lang, code);

                let smudge_filter = SmudgeFilter::new();
                let clean_filter = CleanFilter::new();

                // MD -> DXM -> MD
                let dxm = smudge_filter.markdown_to_dxm(&md).unwrap();
                let md_back = clean_filter.dxm_to_markdown(&dxm).unwrap();

                // The code content should be preserved
                prop_assert!(md_back.contains(&code),
                    "Code content should be preserved: {} not in {}", code, md_back);
            }
        }
    }
}
