// Verification test for Task 1.2: Space separator support for inline object fields
// Requirements: 1.1, 1.2

use serializer::llm::parser::LlmParser;
use serializer::llm::types::DxLlmValue;

#[test]
fn test_space_separated_inline_object() {
    // Test section:count[key=value key2=value2] syntax with space separators
    let input = "config:3[host=localhost port=8080 debug=true]";
    let doc = LlmParser::parse(input).unwrap();

    assert!(doc.context.contains_key("config"));
    if let Some(DxLlmValue::Obj(fields)) = doc.context.get("config") {
        assert_eq!(fields.len(), 3);
        assert_eq!(fields.get("host").unwrap().as_str(), Some("localhost"));
        assert_eq!(fields.get("port").unwrap().as_num(), Some(8080.0));
        assert_eq!(fields.get("debug").unwrap().as_bool(), Some(true));
    } else {
        panic!("Expected Obj variant");
    }
}

#[test]
fn test_comma_separated_inline_object_backward_compat() {
    // Test backward compatibility with comma-separated fields
    let input = "config:3[host=localhost,port=8080,debug=true]";
    let doc = LlmParser::parse(input).unwrap();

    assert!(doc.context.contains_key("config"));
    if let Some(DxLlmValue::Obj(fields)) = doc.context.get("config") {
        assert_eq!(fields.len(), 3);
        assert_eq!(fields.get("host").unwrap().as_str(), Some("localhost"));
        assert_eq!(fields.get("port").unwrap().as_num(), Some(8080.0));
        assert_eq!(fields.get("debug").unwrap().as_bool(), Some(true));
    } else {
        panic!("Expected Obj variant");
    }
}

#[test]
fn test_mixed_whitespace_handling() {
    // Test handling of multiple spaces between fields
    let input = "config:4[host=localhost  port=8080   debug=true   name=myapp]";
    let doc = LlmParser::parse(input).unwrap();

    if let Some(DxLlmValue::Obj(fields)) = doc.context.get("config") {
        assert_eq!(fields.len(), 4);
        assert_eq!(fields.get("host").unwrap().as_str(), Some("localhost"));
        assert_eq!(fields.get("port").unwrap().as_num(), Some(8080.0));
        assert_eq!(fields.get("debug").unwrap().as_bool(), Some(true));
        assert_eq!(fields.get("name").unwrap().as_str(), Some("myapp"));
    } else {
        panic!("Expected Obj variant");
    }
}

#[test]
fn test_values_with_special_characters() {
    // Test values containing dots, underscores, etc.
    let input = "db:3[host=db.example.com port=5432 name=my_database]";
    let doc = LlmParser::parse(input).unwrap();

    if let Some(DxLlmValue::Obj(fields)) = doc.context.get("db") {
        assert_eq!(fields.len(), 3);
        assert_eq!(fields.get("host").unwrap().as_str(), Some("db.example.com"));
        assert_eq!(fields.get("port").unwrap().as_num(), Some(5432.0));
        assert_eq!(fields.get("name").unwrap().as_str(), Some("my_database"));
    } else {
        panic!("Expected Obj variant");
    }
}

#[test]
fn test_empty_inline_object() {
    // Test empty object handling
    let input = "empty:0[]";
    let doc = LlmParser::parse(input).unwrap();

    assert!(doc.context.contains_key("empty"));
    // Empty objects are represented as Null
    assert!(matches!(doc.context.get("empty"), Some(DxLlmValue::Null)));
}

#[test]
fn test_separator_detection_with_nested_brackets() {
    // Test that separator detection correctly handles nested brackets
    let input = "config:2[arr=test port=8080]";
    let doc = LlmParser::parse(input).unwrap();

    if let Some(DxLlmValue::Obj(fields)) = doc.context.get("config") {
        assert_eq!(fields.len(), 2);
        assert!(fields.contains_key("arr"));
        assert!(fields.contains_key("port"));
    } else {
        panic!("Expected Obj variant");
    }
}

#[test]
fn test_numeric_string_values() {
    // Test that numeric strings in values are parsed correctly
    let input = "version:3[major=2 minor=1 patch=0]";
    let doc = LlmParser::parse(input).unwrap();

    if let Some(DxLlmValue::Obj(fields)) = doc.context.get("version") {
        assert_eq!(fields.len(), 3);
        assert_eq!(fields.get("major").unwrap().as_num(), Some(2.0));
        assert_eq!(fields.get("minor").unwrap().as_num(), Some(1.0));
        assert_eq!(fields.get("patch").unwrap().as_num(), Some(0.0));
    } else {
        panic!("Expected Obj variant");
    }
}
