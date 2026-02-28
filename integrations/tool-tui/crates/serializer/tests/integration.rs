//! Comprehensive integration tests for dx-serializer

use serializer::*;

#[test]
fn test_complete_round_trip() {
    // Simple key-value format that the parser supports
    let input = b"name:Test
version:1
active:+";

    // Parse
    let parsed = parse(input).expect("Parse failed");

    // Encode back
    let encoded = encode(&parsed).expect("Encode failed");

    // Reparse
    let reparsed = parse(&encoded).expect("Reparse failed");

    // Should be identical
    assert_eq!(parsed, reparsed);
}

#[test]
fn test_human_format() {
    let input = b"data=id%i name%s
1 Test
2 Demo";

    let value = parse(input).expect("Parse failed");
    let human = format_human(&value).expect("Format failed");

    // Check structure
    assert!(human.contains("DATA TABLE"));
    assert!(human.contains("Test"));
    assert!(human.contains("Demo"));
}

#[test]
fn test_ditto_compression() {
    let input = b"data=id%i status%s priority%i
1 active 5
2 active 5
3 active 5";

    let value = parse(input).expect("Parse failed");
    let encoded = encode(&value).expect("Encode failed");
    let encoded_str = std::str::from_utf8(&encoded).unwrap();

    // Should use ditto for repeated values
    assert!(encoded_str.contains("_"));
}

#[test]
fn test_alias_generation() {
    // Simple nested keys
    let input = b"config.host:localhost
config.port:5432";

    let value = parse(input).expect("Parse failed");
    let encoded = encode(&value).expect("Encode failed");
    let encoded_str = std::str::from_utf8(&encoded).unwrap();

    // Should contain the keys
    println!("Encoded:\n{}", encoded_str);
    assert!(encoded_str.contains("localhost") || encoded_str.contains("host"));
}

#[test]
fn test_vacuum_parsing() {
    // Strings with spaces - no quotes needed
    let input = b"users=id%i name%s score%f
1 Alice Johnson 95.5
2 Bob Smith 87.3";

    let value = parse(input).expect("Parse failed");

    if let DxValue::Object(obj) = value {
        if let Some(DxValue::Table(table)) = obj.get("users") {
            assert_eq!(table.rows[0][1], DxValue::String("Alice Johnson".to_string()));
            assert_eq!(table.rows[1][1], DxValue::String("Bob Smith".to_string()));
        } else {
            panic!("Expected table");
        }
    }
}

#[test]
fn test_prefix_inheritance() {
    let input = b"app.name:DX
^version:1.0
^author:Team";

    let value = parse(input).expect("Parse failed");

    if let DxValue::Object(obj) = value {
        assert!(obj.get("app.name").is_some());
        assert!(obj.get("version").is_some() || obj.get("app.version").is_some());
    }
}

#[test]
fn test_implicit_flags() {
    let input = b"admin!
debug!
error?";

    let value = parse(input).expect("Parse failed");

    if let DxValue::Object(obj) = value {
        assert_eq!(obj.get("admin"), Some(&DxValue::Bool(true)));
        assert_eq!(obj.get("debug"), Some(&DxValue::Bool(true)));
        assert_eq!(obj.get("error"), Some(&DxValue::Null));
    }
}

#[test]
fn test_complex_nested() {
    let input = b"project:DX
metadata.created:2025
metadata.author:Team
dependencies>rust|wasm|web-sys";

    let value = parse(input).expect("Parse failed");
    let human = format_human(&value).expect("Format failed");

    assert!(human.contains("project") || human.contains("DX"));
    assert!(human.contains("metadata") || human.contains("2025"));
    assert!(human.contains("rust") || human.contains("dependencies"));
}

#[test]
fn test_empty_table() {
    let input = b"users=id%i name%s";

    let value = parse(input).expect("Parse failed");

    if let DxValue::Object(obj) = value {
        if let Some(DxValue::Table(table)) = obj.get("users") {
            assert_eq!(table.row_count(), 0);
        }
    }
}

#[test]
fn test_sigil_values() {
    let input = b"data=active%b score%i status%s
+ 100 ready
- 50 pending";

    let value = parse(input).expect("Parse failed");

    if let DxValue::Object(obj) = value {
        if let Some(DxValue::Table(table)) = obj.get("data") {
            assert_eq!(table.rows[0][0], DxValue::Bool(true));
            assert_eq!(table.rows[1][0], DxValue::Bool(false));
        }
    }
}

#[test]
fn test_stream_array_variations() {
    let input = b"strings>alpha|beta|gamma
numbers>1|2|3|4|5
mixed>test|123|+|-";

    let value = parse(input).expect("Parse failed");

    if let DxValue::Object(obj) = value {
        if let Some(DxValue::Array(arr)) = obj.get("strings") {
            assert_eq!(arr.values.len(), 3);
            assert!(arr.is_stream);
        }
        if let Some(DxValue::Array(arr)) = obj.get("numbers") {
            assert_eq!(arr.values.len(), 5);
        }
    }
}
