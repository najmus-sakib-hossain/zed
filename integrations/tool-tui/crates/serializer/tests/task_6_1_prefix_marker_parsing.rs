//! Task 6.1: Prefix Marker Parsing Tests
//!
//! Tests for parsing @prefix and @@suffix markers before table data.
//! These tests verify that the parser correctly recognizes and extracts
//! prefix and suffix patterns for prefix elimination.

use serializer::llm::parser::LlmParser;

#[test]
fn test_single_prefix_marker() {
    // Test parsing a single @prefix marker
    let input = "table:2(id name)@/api/[1 Alice, 2 Bob]";
    let result = LlmParser::parse(input);

    // Should parse successfully (prefix markers are parsed but not yet applied)
    assert!(result.is_ok(), "Failed to parse single prefix marker: {:?}", result.err());
}

#[test]
fn test_multiple_prefix_markers() {
    // Test parsing multiple @prefix markers
    let input = "table:2(id name)@/api/ @v1/[1 Alice, 2 Bob]";
    let result = LlmParser::parse(input);

    assert!(result.is_ok(), "Failed to parse multiple prefix markers: {:?}", result.err());
}

#[test]
fn test_suffix_marker() {
    // Test parsing @@suffix marker (double @)
    let input = "table:2(id email)@@example.com[1 alice, 2 bob]";
    let result = LlmParser::parse(input);

    assert!(result.is_ok(), "Failed to parse suffix marker: {:?}", result.err());
}

#[test]
fn test_mixed_prefix_and_suffix_markers() {
    // Test parsing both prefix and suffix markers
    let input = "table:2(id email)@user_ @@example.com[1 alice, 2 bob]";
    let result = LlmParser::parse(input);

    assert!(
        result.is_ok(),
        "Failed to parse mixed prefix and suffix markers: {:?}",
        result.err()
    );
}

#[test]
fn test_prefix_markers_with_spaces() {
    // Test that spaces between markers are handled correctly
    let input = "table:2(id name)@/api/  @v1/  [1 Alice, 2 Bob]";
    let result = LlmParser::parse(input);

    assert!(result.is_ok(), "Failed to parse prefix markers with spaces: {:?}", result.err());
}

#[test]
fn test_no_prefix_markers() {
    // Test that tables without prefix markers still parse correctly
    let input = "table:2(id name)[1 Alice, 2 Bob]";
    let result = LlmParser::parse(input);

    assert!(
        result.is_ok(),
        "Failed to parse table without prefix markers: {:?}",
        result.err()
    );
    let doc = result.unwrap();
    assert_eq!(doc.sections.len(), 1, "Should have one section");
}

#[test]
fn test_prefix_marker_before_newline_separated_rows() {
    // Test prefix markers with newline-separated rows
    let input = "table:2(endpoint status)@/api/[\nusers 200\nposts 404\n]";
    let result = LlmParser::parse(input);

    assert!(
        result.is_ok(),
        "Failed to parse prefix marker with newline rows: {:?}",
        result.err()
    );
}

#[test]
fn test_empty_prefix_marker() {
    // Test that @ followed immediately by [ is handled (empty prefix)
    let input = "table:2(id name)@[1 Alice, 2 Bob]";
    let result = LlmParser::parse(input);

    // Should parse - empty prefix is ignored
    assert!(result.is_ok(), "Failed to parse empty prefix marker: {:?}", result.err());
}

#[test]
fn test_prefix_marker_with_special_chars() {
    // Test prefix markers containing special characters
    let input = "table:2(id path)@/api/v1/users/[1 alice, 2 bob]";
    let result = LlmParser::parse(input);

    assert!(result.is_ok(), "Failed to parse prefix with special chars: {:?}", result.err());
}

#[test]
fn test_suffix_marker_with_domain() {
    // Test suffix marker with domain-like pattern
    let input = "table:2(id email)@@.example.com[1 alice, 2 bob]";
    let result = LlmParser::parse(input);

    assert!(result.is_ok(), "Failed to parse suffix with domain: {:?}", result.err());
}
