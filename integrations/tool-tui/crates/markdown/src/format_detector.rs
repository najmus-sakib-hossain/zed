//! Format detection for DXM content.
//!
//! This module provides automatic detection of DXM format types
//! (LLM, Human, Machine) based on content analysis.

/// Detected DXM format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DxmFormat {
    /// LLM format (token-optimized)
    Llm,
    /// Human format (Markdown-like)
    Human,
    /// Machine format (binary)
    Machine,
    /// Unknown format
    Unknown,
}

impl std::fmt::Display for DxmFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DxmFormat::Llm => write!(f, "LLM"),
            DxmFormat::Human => write!(f, "Human"),
            DxmFormat::Machine => write!(f, "Machine"),
            DxmFormat::Unknown => write!(f, "Unknown"),
        }
    }
}

/// Magic bytes for DXM binary format.
pub const DXMB_MAGIC: &[u8] = b"DXMB";

/// Format detector for DXM content.
#[derive(Debug, Clone, Default)]
pub struct FormatDetector;

impl FormatDetector {
    /// Create a new format detector.
    pub fn new() -> Self {
        Self
    }

    /// Detect the format of DXM content from bytes.
    pub fn detect(content: &[u8]) -> DxmFormat {
        // Check for Machine format (binary) first
        if Self::is_machine_format(content) {
            return DxmFormat::Machine;
        }

        // Try to interpret as UTF-8 string for text formats
        if let Ok(text) = std::str::from_utf8(content) {
            Self::detect_str(text)
        } else {
            // Non-UTF8 content that's not machine format
            DxmFormat::Unknown
        }
    }

    /// Detect format from string content.
    pub fn detect_str(content: &str) -> DxmFormat {
        let trimmed = content.trim_start();

        // Check for LLM format markers
        if Self::is_llm_format(trimmed) {
            return DxmFormat::Llm;
        }

        // Check for Human format markers
        if Self::is_human_format(trimmed) {
            return DxmFormat::Human;
        }

        DxmFormat::Unknown
    }

    /// Check if content is LLM format.
    ///
    /// LLM format is identified by:
    /// - Starting with `@dxm|` (brain header)
    /// - Lines matching `N|` pattern (numbered headers)
    /// - Lines starting with `#:` (reference definitions)
    fn is_llm_format(content: &str) -> bool {
        // Check for brain header
        if content.starts_with("@dxm|") {
            return true;
        }

        // Check for @meta| header
        if content.starts_with("@meta|") {
            return true;
        }

        // Check first few lines for LLM patterns
        for line in content.lines().take(10) {
            let line = line.trim();

            // Skip empty lines
            if line.is_empty() {
                continue;
            }

            // Check for numbered header pattern (N|text)
            if Self::is_numbered_header(line) {
                return true;
            }

            // Check for reference definition (#:key|value)
            if line.starts_with("#:") && line.contains('|') {
                return true;
            }

            // Check for LLM table syntax (#t(schema))
            if line.starts_with("#t(") {
                return true;
            }

            // Check for LLM semantic blocks (#!, #?, #>, #i, #x)
            if line.starts_with("#!")
                || line.starts_with("#?")
                || line.starts_with("#>")
                || line.starts_with("#i")
                || line.starts_with("#x")
            {
                return true;
            }

            // Check for LLM code block (@lang)
            if line.starts_with('@') && !line.starts_with("@dxm") && !line.starts_with("@meta") {
                // Could be a code block start
                let rest = &line[1..];
                if rest.chars().all(|c| c.is_alphanumeric() || c == '_') {
                    return true;
                }
            }
        }

        false
    }

    /// Check if a line is a numbered header (N|text).
    fn is_numbered_header(line: &str) -> bool {
        let mut chars = line.chars();

        // Must start with a digit
        let first = chars.next();
        if !first.map(|c| c.is_ascii_digit()).unwrap_or(false) {
            return false;
        }

        // Find the pipe character
        for c in chars {
            if c == '|' {
                return true;
            }
            if !c.is_ascii_digit() {
                return false;
            }
        }

        false
    }

    /// Check if content is Human format.
    ///
    /// Human format is identified by:
    /// - Starting with `[meta]` section
    /// - Starting with `#` (Markdown header)
    /// - Containing `[refs]` section
    /// - Unicode box-drawing characters for tables
    fn is_human_format(content: &str) -> bool {
        // Check for TOML-like sections
        if content.starts_with("[meta]") {
            return true;
        }

        if content.starts_with("[refs]") {
            return true;
        }

        // Check for Markdown-style header (# followed by space or text)
        if let Some(rest) = content.strip_prefix('#') {
            // Make sure it's not an LLM semantic block
            if rest.starts_with(' ') || rest.starts_with('#') {
                return true;
            }
            // Check if it's a Markdown header (not #!, #?, etc.)
            if let Some(c) = rest.chars().next()
                && c.is_alphanumeric()
            {
                return true;
            }
        }

        // Check for semantic block syntax (> [!TYPE])
        if content.starts_with("> [!") {
            return true;
        }

        // Check for Unicode table start
        if content.starts_with('┌') {
            return true;
        }

        // Check for GFM-style table (| header |)
        if content.starts_with('|') {
            return true;
        }

        // Check for code fence
        if content.starts_with("```") {
            return true;
        }

        // Check first few lines for Human format patterns
        for line in content.lines().take(10) {
            let line = line.trim();

            if line.starts_with("[meta]") || line.starts_with("[refs]") {
                return true;
            }

            // Check for reference usage [^key]
            if line.contains("[^") && line.contains(']') {
                return true;
            }
        }

        false
    }

    /// Check if content is Machine format (binary).
    ///
    /// Machine format is identified by DXMB magic bytes at the start.
    fn is_machine_format(content: &[u8]) -> bool {
        content.len() >= 4 && &content[..4] == DXMB_MAGIC
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_llm_format_brain_header() {
        let content = "@dxm|1.0\n@meta|tokens:100\n1|Hello World";
        assert_eq!(FormatDetector::detect_str(content), DxmFormat::Llm);
    }

    #[test]
    fn test_detect_llm_format_numbered_header() {
        let content = "1|Hello World\nThis is content.";
        assert_eq!(FormatDetector::detect_str(content), DxmFormat::Llm);
    }

    #[test]
    fn test_detect_llm_format_reference() {
        let content = "#:doc|https://example.com\n1|Title";
        assert_eq!(FormatDetector::detect_str(content), DxmFormat::Llm);
    }

    #[test]
    fn test_detect_llm_format_table() {
        let content = "#t(id|name)\n1|Alice\n2|Bob";
        assert_eq!(FormatDetector::detect_str(content), DxmFormat::Llm);
    }

    #[test]
    fn test_detect_llm_format_semantic_block() {
        let content = "#!This is a warning";
        assert_eq!(FormatDetector::detect_str(content), DxmFormat::Llm);
    }

    #[test]
    fn test_detect_human_format_meta_section() {
        let content = "[meta]\nversion = 1.0\n\n# Hello World";
        assert_eq!(FormatDetector::detect_str(content), DxmFormat::Human);
    }

    #[test]
    fn test_detect_human_format_markdown_header() {
        let content = "# Hello World\n\nThis is a paragraph.";
        assert_eq!(FormatDetector::detect_str(content), DxmFormat::Human);
    }

    #[test]
    fn test_detect_human_format_semantic_block() {
        let content = "> [!WARNING]\n> This is important.";
        assert_eq!(FormatDetector::detect_str(content), DxmFormat::Human);
    }

    #[test]
    fn test_detect_human_format_unicode_table() {
        let content = "┌───┬───┐\n│ a │ b │\n└───┴───┘";
        assert_eq!(FormatDetector::detect_str(content), DxmFormat::Human);
    }

    #[test]
    fn test_detect_human_format_code_fence() {
        let content = "```rust\nfn main() {}\n```";
        assert_eq!(FormatDetector::detect_str(content), DxmFormat::Human);
    }

    #[test]
    fn test_detect_machine_format() {
        let content = b"DXMB\x00\x01\x00\x00some binary data";
        assert_eq!(FormatDetector::detect(content), DxmFormat::Machine);
    }

    #[test]
    fn test_detect_unknown_format() {
        let content = "Just some random text without any markers.";
        assert_eq!(FormatDetector::detect_str(content), DxmFormat::Unknown);
    }

    #[test]
    fn test_detect_empty_content() {
        assert_eq!(FormatDetector::detect_str(""), DxmFormat::Unknown);
        assert_eq!(FormatDetector::detect(b""), DxmFormat::Unknown);
    }

    #[test]
    fn test_is_numbered_header() {
        assert!(FormatDetector::is_numbered_header("1|Hello"));
        assert!(FormatDetector::is_numbered_header("12|Title"));
        assert!(!FormatDetector::is_numbered_header("Hello|World"));
        assert!(!FormatDetector::is_numbered_header("# Header"));
    }

    #[test]
    fn test_format_display() {
        assert_eq!(format!("{}", DxmFormat::Llm), "LLM");
        assert_eq!(format!("{}", DxmFormat::Human), "Human");
        assert_eq!(format!("{}", DxmFormat::Machine), "Machine");
        assert_eq!(format!("{}", DxmFormat::Unknown), "Unknown");
    }
}

#[cfg(test)]
mod prop_tests {
    use super::*;
    use proptest::prelude::*;

    /// Generate LLM format content
    fn arb_llm_content() -> impl Strategy<Value = String> {
        prop_oneof![
            // Brain header format
            "[a-zA-Z0-9 ]{1,50}".prop_map(|text| format!("@dxm|1.0\n1|{}", text)),
            // Numbered header format
            (1u8..=6, "[a-zA-Z0-9 ]{1,50}").prop_map(|(level, text)| format!("{}|{}", level, text)),
            // Reference definition format
            ("[a-zA-Z][a-zA-Z0-9_]{0,10}", "[a-zA-Z0-9:/._-]{1,50}")
                .prop_map(|(key, value)| format!("#:{}|{}", key, value)),
            // Table format
            "[a-zA-Z][a-zA-Z0-9_]{0,5}".prop_map(|col| format!("#t({}|name)\n1|Alice", col)),
            // Semantic block format
            prop_oneof![
                "[a-zA-Z0-9 ]{1,30}".prop_map(|text| format!("#!{}", text)),
                "[a-zA-Z0-9 ]{1,30}".prop_map(|text| format!("#?{}", text)),
                "[a-zA-Z0-9 ]{1,30}".prop_map(|text| format!("#>{}", text)),
                "[a-zA-Z0-9 ]{1,30}".prop_map(|text| format!("#i{}", text)),
                "[a-zA-Z0-9 ]{1,30}".prop_map(|text| format!("#x{}", text)),
            ],
        ]
    }

    /// Generate Human format content
    fn arb_human_content() -> impl Strategy<Value = String> {
        prop_oneof![
            // Meta section format
            "[a-zA-Z0-9 ]{1,50}".prop_map(|text| format!("[meta]\nversion = 1.0\n\n# {}", text)),
            // Markdown header format
            (1usize..=6, "[a-zA-Z0-9 ]{1,50}").prop_map(|(level, text)| format!(
                "{} {}",
                "#".repeat(level),
                text
            )),
            // Semantic block format
            "[a-zA-Z0-9 ]{1,30}".prop_map(|text| format!("> [!WARNING]\n> {}", text)),
            // Unicode table format
            Just("┌───┬───┐\n│ a │ b │\n└───┴───┘".to_string()),
            // Code fence format
            "[a-zA-Z0-9 \n]{1,50}".prop_map(|code| format!("```rust\n{}\n```", code)),
            // Reference usage format
            ("[a-zA-Z][a-zA-Z0-9_]{0,5}", "[a-zA-Z0-9 ]{1,30}")
                .prop_map(|(key, text)| format!("{} [^{}] more text", text, key)),
        ]
    }

    /// Generate Machine format content (binary)
    fn arb_machine_content() -> impl Strategy<Value = Vec<u8>> {
        prop::collection::vec(any::<u8>(), 0..100).prop_map(|mut data| {
            let mut content = DXMB_MAGIC.to_vec();
            content.append(&mut data);
            content
        })
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// **Feature: dxm-human-format, Property 2: Format Detection Accuracy**
        /// *For any* DXM content in LLM format (starting with `@dxm|` or `N|`),
        /// Human format (starting with `[meta]` or `#`), or Machine format
        /// (starting with `DXMB`), the Format_Detector SHALL correctly identify the format.
        /// **Validates: Requirements 6.1, 6.2, 6.3**
        #[test]
        fn prop_format_detection_llm(content in arb_llm_content()) {
            let detected = FormatDetector::detect_str(&content);
            prop_assert_eq!(detected, DxmFormat::Llm,
                "Expected LLM format for content: {:?}", content);
        }

        #[test]
        fn prop_format_detection_human(content in arb_human_content()) {
            let detected = FormatDetector::detect_str(&content);
            prop_assert_eq!(detected, DxmFormat::Human,
                "Expected Human format for content: {:?}", content);
        }

        #[test]
        fn prop_format_detection_machine(content in arb_machine_content()) {
            let detected = FormatDetector::detect(&content);
            prop_assert_eq!(detected, DxmFormat::Machine,
                "Expected Machine format for content with DXMB magic bytes");
        }
    }
}
