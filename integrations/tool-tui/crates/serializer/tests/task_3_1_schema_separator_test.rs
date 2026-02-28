//! Task 3.1: Schema Parsing with Space and Comma Separators
//!
//! Validates Requirements 3.1, 3.2

use serializer::llm::parser::LlmParser;
use serializer::llm::types::DxLlmValue;

#[test]
fn test_space_separated_schema() {
    // Space-separated schema (new format)
    let input = "users:2(id name email)[\n1 Alice alice@ex.com\n2 Bob bob@ex.com\n]";
    let doc = LlmParser::parse(input).expect("Failed to parse space-separated schema");

    assert_eq!(doc.sections.len(), 1);
    let section = doc.sections.values().next().unwrap();
    assert_eq!(section.schema, vec!["id", "name", "email"]);
    assert_eq!(section.rows.len(), 2);

    // Verify first row
    assert_eq!(section.rows[0].len(), 3);
    assert_eq!(section.rows[0][0], DxLlmValue::Num(1.0));
    assert_eq!(section.rows[0][1], DxLlmValue::Str("Alice".to_string()));
    assert_eq!(section.rows[0][2], DxLlmValue::Str("alice@ex.com".to_string()));
}

#[test]
fn test_comma_separated_schema_backward_compat() {
    // Comma-separated schema (legacy format)
    let input = "users:2(id,name,email)[\n1 Alice alice@ex.com\n2 Bob bob@ex.com\n]";
    let doc = LlmParser::parse(input).expect("Failed to parse comma-separated schema");

    assert_eq!(doc.sections.len(), 1);
    let section = doc.sections.values().next().unwrap();
    assert_eq!(section.schema, vec!["id", "name", "email"]);
    assert_eq!(section.rows.len(), 2);

    // Verify first row
    assert_eq!(section.rows[0].len(), 3);
    assert_eq!(section.rows[0][0], DxLlmValue::Num(1.0));
    assert_eq!(section.rows[0][1], DxLlmValue::Str("Alice".to_string()));
    assert_eq!(section.rows[0][2], DxLlmValue::Str("alice@ex.com".to_string()));
}

#[test]
fn test_comma_separated_schema_with_spaces() {
    // Comma-separated with spaces (legacy format with whitespace)
    let input = "users:2(id, name, email)[\n1 Alice alice@ex.com\n2 Bob bob@ex.com\n]";
    let doc = LlmParser::parse(input).expect("Failed to parse comma-separated schema with spaces");

    assert_eq!(doc.sections.len(), 1);
    let section = doc.sections.values().next().unwrap();
    assert_eq!(section.schema, vec!["id", "name", "email"]);
    assert_eq!(section.rows.len(), 2);
}

#[test]
fn test_space_separated_schema_inline_rows() {
    // Space-separated schema with inline comma-separated rows
    let input =
        "users:3(id name email)[1 Alice alice@ex.com, 2 Bob bob@ex.com, 3 Carol carol@ex.com]";
    let doc =
        LlmParser::parse(input).expect("Failed to parse space-separated schema with inline rows");

    assert_eq!(doc.sections.len(), 1);
    let section = doc.sections.values().next().unwrap();
    assert_eq!(section.schema, vec!["id", "name", "email"]);
    assert_eq!(section.rows.len(), 3);
}

#[test]
fn test_single_column_space_schema() {
    // Single column schema (space-separated, but only one column)
    let input = "tags:2(name)[\nrust\npython\n]";
    let doc = LlmParser::parse(input).expect("Failed to parse single column schema");

    assert_eq!(doc.sections.len(), 1);
    let section = doc.sections.values().next().unwrap();
    assert_eq!(section.schema, vec!["name"]);
    assert_eq!(section.rows.len(), 2);
}

#[test]
fn test_many_columns_space_schema() {
    // Many columns with space-separated schema
    let input = "data:1(a b c d e f g h)[\n1 2 3 4 5 6 7 8\n]";
    let doc = LlmParser::parse(input).expect("Failed to parse many column schema");

    assert_eq!(doc.sections.len(), 1);
    let section = doc.sections.values().next().unwrap();
    assert_eq!(section.schema, vec!["a", "b", "c", "d", "e", "f", "g", "h"]);
    assert_eq!(section.rows.len(), 1);
    assert_eq!(section.rows[0].len(), 8);
}
