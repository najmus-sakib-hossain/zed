//! Format detection for DXM Git filters.
//!
//! Detects whether input content is DXM, Markdown, or binary format
//! to enable pass-through behavior when content is already in target format.

/// Detected format type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetectedFormat {
    /// DXM LLM format (starts with @dxm| or N|)
    Dxm,
    /// Standard Markdown
    Markdown,
    /// Binary DXM format (starts with DXMB magic bytes)
    Binary,
    /// Unknown format
    Unknown,
}

/// Magic bytes for binary DXM format.
pub const DXMB_MAGIC: &[u8; 4] = b"DXMB";

/// Detect the format of input content.
///
/// # Arguments
///
/// * `content` - The raw bytes to analyze
///
/// # Returns
///
/// The detected format type.
///
/// # Examples
///
/// ```
/// use dx_markdown::git::detect::{detect_format, DetectedFormat};
///
/// // DXM format
/// assert_eq!(detect_format(b"1|Hello World"), DetectedFormat::Dxm);
/// assert_eq!(detect_format(b"@dxm|1.0"), DetectedFormat::Dxm);
///
/// // Markdown format
/// assert_eq!(detect_format(b"# Hello World"), DetectedFormat::Markdown);
///
/// // Binary format
/// assert_eq!(detect_format(b"DXMB\x00\x01"), DetectedFormat::Binary);
/// ```
pub fn detect_format(content: &[u8]) -> DetectedFormat {
    if content.is_empty() {
        return DetectedFormat::Unknown;
    }

    // Check for binary DXM magic bytes
    if content.len() >= 4 && &content[0..4] == DXMB_MAGIC {
        return DetectedFormat::Binary;
    }

    // Try to interpret as UTF-8 for text-based detection
    let text = match std::str::from_utf8(content) {
        Ok(s) => s,
        Err(_) => return DetectedFormat::Unknown,
    };

    // Skip leading whitespace for detection
    let trimmed = text.trim_start();

    if trimmed.is_empty() {
        return DetectedFormat::Unknown;
    }

    // Check for DXM format indicators
    if is_dxm_format(trimmed) {
        return DetectedFormat::Dxm;
    }

    // Check for Markdown format indicators
    if is_markdown_format(trimmed) {
        return DetectedFormat::Markdown;
    }

    // Default to unknown
    DetectedFormat::Unknown
}

/// Check if content appears to be DXM format.
fn is_dxm_format(content: &str) -> bool {
    // Check for @dxm| header
    if content.starts_with("@dxm|") {
        return true;
    }

    // Check for N| header pattern (1|, 2|, etc.)
    if let Some(first_char) = content.chars().next()
        && first_char.is_ascii_digit()
    {
        // Look for digit followed by |
        let chars = content.chars();
        for c in chars {
            if c == '|' {
                return true;
            }
            if !c.is_ascii_digit() {
                break;
            }
        }
    }

    // Check for DXM-specific patterns in content
    // Code blocks: @lang ... @
    if content.contains("\n@") && !content.contains("```") {
        // Has @ code block markers but no markdown code blocks
        return true;
    }

    // Check for DXM inline styles (postfix notation)
    // Bold: word! (but not !! or !!! which are priority markers)
    // Italic: word/
    // These are harder to detect reliably, so we rely on other indicators

    // Check for DXM table syntax: #t(schema)
    if content.contains("#t(") {
        return true;
    }

    // Check for DXM semantic blocks: #!, #?, #>, #i, #x
    if content.contains("\n#!")
        || content.contains("\n#?")
        || content.contains("\n#>")
        || content.contains("\n#i")
        || content.contains("\n#x")
    {
        return true;
    }

    // Check for reference syntax: ^key
    // This is DXM-specific (Markdown uses [text][ref] or [ref])
    if content.contains("^") {
        // Look for ^word pattern (reference usage)
        let mut in_code = false;
        let mut prev_char = ' ';
        for c in content.chars() {
            if c == '`' {
                in_code = !in_code;
            }
            if !in_code && c == '^' && !prev_char.is_alphanumeric() {
                return true;
            }
            prev_char = c;
        }
    }

    false
}

/// Check if content appears to be Markdown format.
fn is_markdown_format(content: &str) -> bool {
    // ATX headers: # Title
    if content.starts_with('#')
        && content.chars().nth(1).map(|c| c == ' ' || c == '#').unwrap_or(false)
    {
        return true;
    }

    // Fenced code blocks: ```
    if content.contains("```") {
        return true;
    }

    // Links: [text](url)
    if content.contains("](") {
        return true;
    }

    // Images: ![alt](url)
    if content.contains("![") {
        return true;
    }

    // Bold: **text** or __text__
    if content.contains("**") || content.contains("__") {
        return true;
    }

    // Italic: *text* (single asterisk)
    // This is tricky because * is also used for lists
    // We look for *word* pattern
    let mut in_code = false;
    let chars: Vec<char> = content.chars().collect();
    for i in 0..chars.len() {
        if chars[i] == '`' {
            in_code = !in_code;
        }
        if !in_code && chars[i] == '*' {
            // Check if it's *word* pattern (not ** or list)
            if i > 0 && i + 1 < chars.len() {
                let prev = chars[i - 1];
                let next = chars[i + 1];
                if prev != '*' && next != '*' && next != ' ' && prev != '\n' {
                    return true;
                }
            }
        }
    }

    // GFM tables: | header |
    if (content.contains("\n|") || content.starts_with('|'))
        && (content.contains("|---") || content.contains("| ---"))
    {
        return true;
    }

    // Blockquotes: > text
    if content.starts_with('>') || content.contains("\n>") {
        return true;
    }

    // Horizontal rules: --- or *** or ___
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.len() >= 3 {
            let first = trimmed.chars().next().unwrap_or(' ');
            if matches!(first, '-' | '*' | '_')
                && trimmed.chars().all(|c| c == first || c.is_whitespace())
            {
                return true;
            }
        }
    }

    false
}

/// Check if content is already in the target format.
///
/// # Arguments
///
/// * `content` - The raw bytes to check
/// * `format` - The target format to check against
///
/// # Returns
///
/// `true` if the content is already in the specified format.
pub fn is_format(content: &[u8], format: DetectedFormat) -> bool {
    detect_format(content) == format
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_empty() {
        assert_eq!(detect_format(b""), DetectedFormat::Unknown);
    }

    #[test]
    fn test_detect_binary() {
        assert_eq!(detect_format(b"DXMB\x00\x01\x00\x00"), DetectedFormat::Binary);
    }

    #[test]
    fn test_detect_dxm_header() {
        assert_eq!(detect_format(b"1|Hello World"), DetectedFormat::Dxm);
        assert_eq!(detect_format(b"2|Section"), DetectedFormat::Dxm);
        assert_eq!(detect_format(b"6|Deep Header"), DetectedFormat::Dxm);
    }

    #[test]
    fn test_detect_dxm_meta() {
        assert_eq!(detect_format(b"@dxm|1.0\n1|Title"), DetectedFormat::Dxm);
    }

    #[test]
    fn test_detect_dxm_table() {
        assert_eq!(detect_format(b"#t(name,age)\nAlice,30"), DetectedFormat::Dxm);
    }

    #[test]
    fn test_detect_markdown_header() {
        assert_eq!(detect_format(b"# Hello World"), DetectedFormat::Markdown);
        assert_eq!(detect_format(b"## Section"), DetectedFormat::Markdown);
        assert_eq!(detect_format(b"### Subsection"), DetectedFormat::Markdown);
    }

    #[test]
    fn test_detect_markdown_code_block() {
        assert_eq!(detect_format(b"```rust\nfn main() {}\n```"), DetectedFormat::Markdown);
    }

    #[test]
    fn test_detect_markdown_link() {
        assert_eq!(detect_format(b"See [docs](https://example.com)"), DetectedFormat::Markdown);
    }

    #[test]
    fn test_detect_markdown_bold() {
        assert_eq!(detect_format(b"This is **bold** text"), DetectedFormat::Markdown);
    }

    #[test]
    fn test_detect_markdown_table() {
        let table = b"| Name | Age |\n|---|---|\n| Alice | 30 |";
        assert_eq!(detect_format(table), DetectedFormat::Markdown);
    }

    #[test]
    fn test_detect_markdown_blockquote() {
        assert_eq!(detect_format(b"> This is a quote"), DetectedFormat::Markdown);
    }

    #[test]
    fn test_is_format() {
        assert!(is_format(b"# Hello", DetectedFormat::Markdown));
        assert!(is_format(b"1|Hello", DetectedFormat::Dxm));
        assert!(!is_format(b"# Hello", DetectedFormat::Dxm));
    }

    #[test]
    fn test_detect_invalid_utf8() {
        // Invalid UTF-8 sequence
        assert_eq!(detect_format(&[0xFF, 0xFE, 0x00, 0x01]), DetectedFormat::Unknown);
    }

    #[test]
    fn test_detect_whitespace_only() {
        assert_eq!(detect_format(b"   \n\t  "), DetectedFormat::Unknown);
    }
}

// ============================================================================
// Property-Based Tests
// ============================================================================

#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;

    /// **Property 3: Format Detection Idempotence**
    ///
    /// For any content that is already in the target format, the filter SHALL
    /// pass it through unchanged. This test verifies that format detection
    /// correctly identifies formats so that:
    /// - Clean filter on Markdown input → unchanged Markdown output
    /// - Smudge filter on DXM input → unchanged DXM output
    ///
    /// **Validates: Requirements 1.9, 2.8**
    mod property_3_format_detection_idempotence {
        use super::*;

        // Strategy to generate valid DXM headers
        fn dxm_header_strategy() -> impl Strategy<Value = String> {
            (1u8..=6, "[a-zA-Z0-9 ]{1,50}")
                .prop_map(|(level, title)| format!("{}|{}", level, title))
        }

        // Strategy to generate valid Markdown headers
        fn markdown_header_strategy() -> impl Strategy<Value = String> {
            (1usize..=6, "[a-zA-Z0-9 ]{1,50}")
                .prop_map(|(level, title)| format!("{} {}", "#".repeat(level), title))
        }

        // Strategy to generate DXM documents
        fn dxm_document_strategy() -> impl Strategy<Value = String> {
            prop::collection::vec(dxm_header_strategy(), 1..5)
                .prop_map(|headers| headers.join("\n"))
        }

        // Strategy to generate Markdown documents
        fn markdown_document_strategy() -> impl Strategy<Value = String> {
            prop::collection::vec(markdown_header_strategy(), 1..5)
                .prop_map(|headers| headers.join("\n\n"))
        }

        proptest! {
            #![proptest_config(ProptestConfig::with_cases(100))]

            /// For any valid DXM document, detect_format should return Dxm
            #[test]
            fn dxm_documents_detected_as_dxm(doc in dxm_document_strategy()) {
                let detected = detect_format(doc.as_bytes());
                prop_assert_eq!(detected, DetectedFormat::Dxm,
                    "DXM document should be detected as DXM: {:?}", doc);
            }

            /// For any valid Markdown document, detect_format should return Markdown
            #[test]
            fn markdown_documents_detected_as_markdown(doc in markdown_document_strategy()) {
                let detected = detect_format(doc.as_bytes());
                prop_assert_eq!(detected, DetectedFormat::Markdown,
                    "Markdown document should be detected as Markdown: {:?}", doc);
            }

            /// Binary format detection is deterministic
            #[test]
            fn binary_magic_always_detected(suffix in prop::collection::vec(any::<u8>(), 0..100)) {
                let mut content = DXMB_MAGIC.to_vec();
                content.extend(suffix);
                let detected = detect_format(&content);
                prop_assert_eq!(detected, DetectedFormat::Binary,
                    "Content starting with DXMB magic should be detected as Binary");
            }

            /// Format detection is consistent (calling twice gives same result)
            #[test]
            fn format_detection_is_consistent(content in prop::collection::vec(any::<u8>(), 0..1000)) {
                let first = detect_format(&content);
                let second = detect_format(&content);
                prop_assert_eq!(first, second,
                    "Format detection should be consistent for same input");
            }
        }

        // Additional property: DXM with @dxm| prefix is always detected
        proptest! {
            #![proptest_config(ProptestConfig::with_cases(100))]

            #[test]
            fn dxm_meta_header_always_detected(version in "[0-9]+\\.[0-9]+", rest in ".*") {
                let doc = format!("@dxm|{}\n{}", version, rest);
                let detected = detect_format(doc.as_bytes());
                prop_assert_eq!(detected, DetectedFormat::Dxm,
                    "Document with @dxm| header should be detected as DXM");
            }
        }

        // Property: Markdown with code blocks is always detected
        proptest! {
            #![proptest_config(ProptestConfig::with_cases(100))]

            #[test]
            fn markdown_code_blocks_always_detected(
                lang in "[a-z]{0,10}",
                code in "[a-zA-Z0-9 \n]{0,100}"
            ) {
                let doc = format!("```{}\n{}\n```", lang, code);
                let detected = detect_format(doc.as_bytes());
                prop_assert_eq!(detected, DetectedFormat::Markdown,
                    "Document with ``` code blocks should be detected as Markdown");
            }
        }

        // Property: Markdown with links is always detected
        proptest! {
            #![proptest_config(ProptestConfig::with_cases(100))]

            #[test]
            fn markdown_links_always_detected(
                text in "[a-zA-Z0-9 ]{1,20}",
                url in "https?://[a-z]+\\.[a-z]+"
            ) {
                let doc = format!("See [{}]({})", text, url);
                let detected = detect_format(doc.as_bytes());
                prop_assert_eq!(detected, DetectedFormat::Markdown,
                    "Document with [text](url) links should be detected as Markdown");
            }
        }

        // Property: Markdown with bold text is always detected
        proptest! {
            #![proptest_config(ProptestConfig::with_cases(100))]

            #[test]
            fn markdown_bold_always_detected(text in "[a-zA-Z0-9]{1,20}") {
                let doc = format!("This is **{}** text", text);
                let detected = detect_format(doc.as_bytes());
                prop_assert_eq!(detected, DetectedFormat::Markdown,
                    "Document with **bold** should be detected as Markdown");
            }
        }
    }
}
