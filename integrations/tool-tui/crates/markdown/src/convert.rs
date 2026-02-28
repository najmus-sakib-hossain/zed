//! Format conversion functions for DXM.
//!
//! Provides conversion between the three DXM formats:
//! - LLM Format (token-optimized text)
//! - Human Format (readable text with Unicode tables)
//! - Machine Format (RKYV binary)

// OLD CUSTOM BINARY FORMAT - COMMENTED OUT
// use crate::binary::{BinaryBuilder, BinaryReader};
use crate::error::{ConvertError, ConvertResult, ParseError, ParseResult};
use crate::human_formatter::HumanFormatter;
use crate::human_parser::HumanParser;
use crate::machine;
use crate::parser::DxmParser;
use crate::serializer::LlmSerializer;
use crate::types::DxmDocument;

// ============================================================================
// LLM Format Conversions
// ============================================================================

/// Convert LLM format to Human format.
///
/// Parses the LLM format and serializes to human-readable format.
pub fn llm_to_human(input: &str) -> ParseResult<String> {
    let doc = DxmParser::parse(input)?;
    Ok(HumanFormatter::default().format(&doc))
}

/// Convert Human format to LLM format.
///
/// Parses the human format and serializes to LLM format.
/// For regular markdown files, parses them as markdown and converts to DXM.
pub fn human_to_llm(input: &str) -> ParseResult<String> {
    // Try HumanParser first (for files with [meta] sections)
    if input.contains("[meta]") || input.contains("[refs]") {
        let doc = HumanParser::parse(input)?;
        Ok(LlmSerializer::default().serialize(&doc))
    } else {
        // For regular markdown, parse it and convert to DXM format
        use crate::markdown::MarkdownParser;
        let doc = MarkdownParser::parse(input)
            .map_err(|e| ParseError::new(format!("markdown parsing failed: {}", e), 1, 1))?;
        Ok(LlmSerializer::default().serialize(&doc))
    }
}

// ============================================================================
// Machine Format Conversions
// ============================================================================

/// Convert LLM format to Machine (RKYV binary) format.
///
/// DX-Markdown Machine Format IS RKYV.
pub fn llm_to_machine(input: &str) -> ConvertResult<Vec<u8>> {
    let doc = DxmParser::parse(input).map_err(ConvertError::Parse)?;
    machine::serialize_machine(&doc).map_err(ConvertError::from)
}

/// Convert Machine (RKYV binary) format to LLM format.
///
/// DX-Markdown Machine Format IS RKYV - zero-copy deserialization.
pub fn machine_to_llm(input: &[u8]) -> ConvertResult<String> {
    let doc = machine::deserialize_machine(input)?;
    Ok(LlmSerializer::default().serialize(&doc))
}

/// Convert Human format to Machine (RKYV binary) format.
///
/// DX-Markdown Machine Format IS RKYV.
pub fn human_to_machine(input: &str) -> ConvertResult<Vec<u8>> {
    let doc = HumanParser::parse(input).map_err(ConvertError::Parse)?;
    machine::serialize_machine(&doc).map_err(ConvertError::from)
}

/// Convert Machine (RKYV binary) format to Human format.
///
/// DX-Markdown Machine Format IS RKYV - zero-copy deserialization.
pub fn machine_to_human(input: &[u8]) -> ConvertResult<String> {
    let doc = machine::deserialize_machine(input)?;
    Ok(HumanFormatter::default().format(&doc))
}

// ============================================================================
// Document-based Conversions
// ============================================================================

/// Convert a DxmDocument to LLM format string.
pub fn doc_to_llm(doc: &DxmDocument) -> String {
    LlmSerializer::default().serialize(doc)
}

/// Convert a DxmDocument to Human format string.
pub fn doc_to_human(doc: &DxmDocument) -> String {
    let mut formatter = HumanFormatter::default();
    formatter.format(doc)
}

/// Convert a DxmDocument to Machine (RKYV binary) format.
///
/// DX-Markdown Machine Format IS RKYV.
pub fn doc_to_machine(doc: &DxmDocument) -> ConvertResult<Vec<u8>> {
    machine::serialize_machine(doc).map_err(ConvertError::from)
}

// ============================================================================
// Format Detection
// ============================================================================

/// Detected format type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormatType {
    /// LLM format (token-optimized)
    Llm,
    /// Human format (readable)
    Human,
    /// Machine format (binary)
    Machine,
    /// Unknown format
    Unknown,
}

/// Detect the format of input data.
///
/// Returns the detected format type based on content analysis.
pub fn detect_format(input: &[u8]) -> FormatType {
    // Check for RKYV binary format (machine format)
    // RKYV doesn't have a magic header, so we check if it's valid UTF-8 first
    // If it's not valid UTF-8, assume it's binary (RKYV)
    if std::str::from_utf8(input).is_err() {
        return FormatType::Machine;
    }

    // OLD CUSTOM BINARY FORMAT - COMMENTED OUT
    // if input.len() >= 4 && &input[0..4] == b"DXMB" {
    //     return FormatType::Machine;
    // }

    // Try to interpret as text
    if let Ok(text) = std::str::from_utf8(input) {
        // Human format indicators
        if text.contains("[meta]") || text.contains("[refs]") || text.contains("┌") {
            return FormatType::Human;
        }

        // LLM format indicators (level|content syntax)
        if text.lines().any(|line| {
            let trimmed = line.trim();
            trimmed.len() >= 2
                && trimmed.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false)
                && trimmed.chars().nth(1) == Some('|')
        }) {
            return FormatType::Llm;
        }

        // Check for DXM-specific syntax
        if text.contains("#:") || text.contains("#t(") || text.contains("#!") || text.contains("#?")
        {
            return FormatType::Llm;
        }
    }

    FormatType::Unknown
}

/// Auto-convert input to a DxmDocument, detecting format automatically.
pub fn auto_parse(input: &[u8]) -> ConvertResult<DxmDocument> {
    match detect_format(input) {
        FormatType::Machine => {
            // DX-Markdown Machine Format IS RKYV
            machine::deserialize_machine(input).map_err(ConvertError::from)

            // OLD CUSTOM BINARY FORMAT - COMMENTED OUT
            // let mut reader = BinaryReader::new(input)?;
            // Ok(reader.read_document()?)
        }
        FormatType::Llm => {
            let text =
                std::str::from_utf8(input).map_err(|e| ConvertError::InvalidUtf8(e.to_string()))?;
            DxmParser::parse(text).map_err(|e| ConvertError::ParseError(e.to_string()))
        }
        FormatType::Human => {
            let text =
                std::str::from_utf8(input).map_err(|e| ConvertError::InvalidUtf8(e.to_string()))?;
            HumanParser::parse(text).map_err(|e| ConvertError::ParseError(e.to_string()))
        }
        FormatType::Unknown => Err(ConvertError::UnknownFormat),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{DxmNode, HeaderNode, InlineNode};

    #[test]
    fn test_llm_to_human_basic() {
        let llm_input = "1|Hello World";
        let human = llm_to_human(llm_input).unwrap();
        assert!(human.contains("[meta]"));
        assert!(human.contains("Hello World"));
    }

    #[test]
    fn test_llm_to_machine_roundtrip() {
        let llm_input = "1|Hello World\nThis is a paragraph.";
        let binary = llm_to_machine(llm_input).unwrap();
        let back_to_llm = machine_to_llm(&binary).unwrap();

        // Parse both and compare structure
        let doc1 = DxmParser::parse(llm_input).unwrap();
        let doc2 = DxmParser::parse(&back_to_llm).unwrap();

        assert_eq!(doc1.nodes.len(), doc2.nodes.len());
    }

    #[test]
    fn test_human_to_llm_basic() {
        // Use human format input with [meta] section
        let human_input = "[meta]\nversion = 1.0\n\n# Hello World";
        let result = human_to_llm(human_input);
        // Just verify it parses without error
        assert!(result.is_ok());
    }

    #[test]
    fn test_detect_format_binary() {
        // RKYV binary format (non-UTF8 bytes)
        let binary = &[0xFF, 0xFE, 0xFD, 0xFC];
        assert_eq!(detect_format(binary), FormatType::Machine);
    }

    #[test]
    fn test_detect_format_llm() {
        let llm = b"1|Hello World";
        assert_eq!(detect_format(llm), FormatType::Llm);
    }

    #[test]
    fn test_detect_format_human() {
        let human = b"[meta]\nversion = 1.0";
        assert_eq!(detect_format(human), FormatType::Human);
    }

    #[test]
    fn test_detect_format_human_unicode_table() {
        let human = "┌───┬───┐".as_bytes();
        assert_eq!(detect_format(human), FormatType::Human);
    }

    #[test]
    fn test_auto_parse_llm() {
        let llm = b"1|Hello World";
        let doc = auto_parse(llm).unwrap();
        assert!(!doc.nodes.is_empty());
    }

    #[test]
    fn test_doc_to_llm() {
        let mut doc = DxmDocument::default();
        doc.nodes.push(DxmNode::Header(HeaderNode {
            level: 1,
            content: vec![InlineNode::Text("Test".to_string())],
            priority: None,
        }));
        let llm = doc_to_llm(&doc);
        assert!(llm.contains("Test"));
    }

    #[test]
    fn test_doc_to_human() {
        let mut doc = DxmDocument::default();
        doc.nodes.push(DxmNode::Header(HeaderNode {
            level: 1,
            content: vec![InlineNode::Text("Test".to_string())],
            priority: None,
        }));
        let human = doc_to_human(&doc);
        assert!(human.contains("[meta]"));
        assert!(human.contains("Test"));
    }

    #[test]
    fn test_doc_to_machine() {
        let mut doc = DxmDocument::default();
        doc.nodes.push(DxmNode::Header(HeaderNode {
            level: 1,
            content: vec![InlineNode::Text("Test".to_string())],
            priority: None,
        }));
        let binary = doc_to_machine(&doc).expect("Failed to convert to machine format");
        // RKYV binary format
        assert!(!binary.is_empty());

        // Verify round-trip
        let parsed = machine::deserialize_machine(&binary).expect("Failed to deserialize");
        assert_eq!(parsed.nodes.len(), 1);
    }
}
