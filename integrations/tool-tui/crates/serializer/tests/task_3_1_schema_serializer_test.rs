//! Task 3.1: Schema Serialization with Space Separators
//!
//! Validates that serializer outputs space-separated schemas

use serializer::llm::parser::LlmParser;
use serializer::llm::serializer::LlmSerializer;

#[test]
fn test_serializer_outputs_space_separated_schema() {
    // Parse a table with space-separated schema
    let input = "users:2(id name email)[\n1 Alice alice@ex.com\n2 Bob bob@ex.com\n]";
    let doc = LlmParser::parse(input).expect("Failed to parse");

    // Serialize it back
    let serializer = LlmSerializer::new();
    let output = serializer.serialize(&doc);

    // Verify the output contains space-separated schema
    assert!(
        output.contains("(id name email)"),
        "Schema should be space-separated, got: {}",
        output
    );
    assert!(!output.contains("(id,name,email)"), "Schema should not be comma-separated");
}

#[test]
fn test_roundtrip_preserves_schema() {
    // Parse with space-separated schema
    let input =
        "users:3(id name email)[\n1 Alice alice@ex.com\n2 Bob bob@ex.com\n3 Carol carol@ex.com\n]";
    let doc1 = LlmParser::parse(input).expect("Failed to parse");

    // Serialize
    let serializer = LlmSerializer::new();
    let output = serializer.serialize(&doc1);

    // Parse again
    let doc2 = LlmParser::parse(&output).expect("Failed to parse serialized output");

    // Verify schemas match
    assert_eq!(doc1.sections.len(), doc2.sections.len());
    let section1 = doc1.sections.values().next().unwrap();
    let section2 = doc2.sections.values().next().unwrap();
    assert_eq!(section1.schema, section2.schema);
    assert_eq!(section1.rows.len(), section2.rows.len());
}

#[test]
fn test_comma_schema_converts_to_space_on_roundtrip() {
    // Parse with comma-separated schema (legacy)
    let input = "users:2(id,name,email)[\n1 Alice alice@ex.com\n2 Bob bob@ex.com\n]";
    let doc = LlmParser::parse(input).expect("Failed to parse");

    // Serialize (should output space-separated)
    let serializer = LlmSerializer::new();
    let output = serializer.serialize(&doc);

    // Verify output uses space-separated format
    assert!(
        output.contains("(id name email)"),
        "Should convert to space-separated, got: {}",
        output
    );
}
