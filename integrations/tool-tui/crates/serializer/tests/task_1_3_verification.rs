// Verification test for Task 1.3: Nested array support in inline objects
// Requirements: 1.3

use serializer::llm::parser::LlmParser;
use serializer::llm::types::DxLlmValue;

#[test]
fn test_nested_array_space_separated() {
    // Test key[count]=item1 item2 item3 syntax with space separators
    let input = "config:2[tags[3]=web api mobile host=localhost]";
    let doc = LlmParser::parse(input).unwrap();

    assert!(doc.context.contains_key("config"));
    if let Some(DxLlmValue::Obj(fields)) = doc.context.get("config") {
        assert_eq!(fields.len(), 2);

        // Check tags array
        if let Some(DxLlmValue::Arr(tags)) = fields.get("tags") {
            assert_eq!(tags.len(), 3);
            assert_eq!(tags[0].as_str(), Some("web"));
            assert_eq!(tags[1].as_str(), Some("api"));
            assert_eq!(tags[2].as_str(), Some("mobile"));
        } else {
            panic!("Expected tags to be an array");
        }

        // Check host field
        assert_eq!(fields.get("host").unwrap().as_str(), Some("localhost"));
    } else {
        panic!("Expected Obj variant");
    }
}

#[test]
fn test_nested_array_with_numbers() {
    // Test nested array with numeric values
    let input = "data:2[ports[3]=8080 8081 8082 name=server]";
    let doc = LlmParser::parse(input).unwrap();

    if let Some(DxLlmValue::Obj(fields)) = doc.context.get("data") {
        assert_eq!(fields.len(), 2);

        if let Some(DxLlmValue::Arr(ports)) = fields.get("ports") {
            assert_eq!(ports.len(), 3);
            assert_eq!(ports[0].as_num(), Some(8080.0));
            assert_eq!(ports[1].as_num(), Some(8081.0));
            assert_eq!(ports[2].as_num(), Some(8082.0));
        } else {
            panic!("Expected ports to be an array");
        }

        assert_eq!(fields.get("name").unwrap().as_str(), Some("server"));
    } else {
        panic!("Expected Obj variant");
    }
}

#[test]
fn test_nested_array_comma_separated_object() {
    // Test nested array in comma-separated object (backward compatibility)
    let input = "config:2[tags[3]=web,api,mobile,host=localhost]";
    let doc = LlmParser::parse(input).unwrap();

    if let Some(DxLlmValue::Obj(fields)) = doc.context.get("config") {
        assert_eq!(fields.len(), 2);

        if let Some(DxLlmValue::Arr(tags)) = fields.get("tags") {
            assert_eq!(tags.len(), 3);
            assert_eq!(tags[0].as_str(), Some("web"));
            assert_eq!(tags[1].as_str(), Some("api"));
            assert_eq!(tags[2].as_str(), Some("mobile"));
        } else {
            panic!("Expected tags to be an array");
        }

        assert_eq!(fields.get("host").unwrap().as_str(), Some("localhost"));
    } else {
        panic!("Expected Obj variant");
    }
}

#[test]
fn test_multiple_nested_arrays() {
    // Test multiple nested arrays in the same object
    let input = "server:3[hosts[2]=web1 web2 ports[2]=80 443 ssl=true]";
    let doc = LlmParser::parse(input).unwrap();

    if let Some(DxLlmValue::Obj(fields)) = doc.context.get("server") {
        assert_eq!(fields.len(), 3);

        // Check hosts array
        if let Some(DxLlmValue::Arr(hosts)) = fields.get("hosts") {
            assert_eq!(hosts.len(), 2);
            assert_eq!(hosts[0].as_str(), Some("web1"));
            assert_eq!(hosts[1].as_str(), Some("web2"));
        } else {
            panic!("Expected hosts to be an array");
        }

        // Check ports array
        if let Some(DxLlmValue::Arr(ports)) = fields.get("ports") {
            assert_eq!(ports.len(), 2);
            assert_eq!(ports[0].as_num(), Some(80.0));
            assert_eq!(ports[1].as_num(), Some(443.0));
        } else {
            panic!("Expected ports to be an array");
        }

        // Check ssl field
        assert_eq!(fields.get("ssl").unwrap().as_bool(), Some(true));
    } else {
        panic!("Expected Obj variant");
    }
}

#[test]
fn test_nested_array_empty() {
    // Test empty nested array
    let input = "config:2[tags[0]= host=localhost]";
    let doc = LlmParser::parse(input).unwrap();

    if let Some(DxLlmValue::Obj(fields)) = doc.context.get("config") {
        assert_eq!(fields.len(), 2);

        if let Some(DxLlmValue::Arr(tags)) = fields.get("tags") {
            assert_eq!(tags.len(), 0);
        } else {
            panic!("Expected tags to be an array");
        }

        assert_eq!(fields.get("host").unwrap().as_str(), Some("localhost"));
    } else {
        panic!("Expected Obj variant");
    }
}

#[test]
fn test_nested_array_single_item() {
    // Test nested array with single item
    let input = "config:2[tags[1]=production host=localhost]";
    let doc = LlmParser::parse(input).unwrap();

    if let Some(DxLlmValue::Obj(fields)) = doc.context.get("config") {
        assert_eq!(fields.len(), 2);

        if let Some(DxLlmValue::Arr(tags)) = fields.get("tags") {
            assert_eq!(tags.len(), 1);
            assert_eq!(tags[0].as_str(), Some("production"));
        } else {
            panic!("Expected tags to be an array");
        }

        assert_eq!(fields.get("host").unwrap().as_str(), Some("localhost"));
    } else {
        panic!("Expected Obj variant");
    }
}

#[test]
fn test_nested_array_with_boolean_values() {
    // Test nested array with boolean values
    let input = "flags:2[enabled[3]=true false true name=test]";
    let doc = LlmParser::parse(input).unwrap();

    if let Some(DxLlmValue::Obj(fields)) = doc.context.get("flags") {
        assert_eq!(fields.len(), 2);

        if let Some(DxLlmValue::Arr(enabled)) = fields.get("enabled") {
            assert_eq!(enabled.len(), 3);
            assert_eq!(enabled[0].as_bool(), Some(true));
            assert_eq!(enabled[1].as_bool(), Some(false));
            assert_eq!(enabled[2].as_bool(), Some(true));
        } else {
            panic!("Expected enabled to be an array");
        }

        assert_eq!(fields.get("name").unwrap().as_str(), Some("test"));
    } else {
        panic!("Expected Obj variant");
    }
}

#[test]
fn test_nested_array_mixed_types() {
    // Test nested array with mixed value types
    let input = "data:2[values[4]=100 test 3.14 true name=mixed]";
    let doc = LlmParser::parse(input).unwrap();

    if let Some(DxLlmValue::Obj(fields)) = doc.context.get("data") {
        assert_eq!(fields.len(), 2);

        if let Some(DxLlmValue::Arr(values)) = fields.get("values") {
            assert_eq!(values.len(), 4);
            assert_eq!(values[0].as_num(), Some(100.0));
            assert_eq!(values[1].as_str(), Some("test"));
            assert_eq!(values[2].as_num(), Some(3.14));
            assert_eq!(values[3].as_bool(), Some(true));
        } else {
            panic!("Expected values to be an array");
        }

        assert_eq!(fields.get("name").unwrap().as_str(), Some("mixed"));
    } else {
        panic!("Expected Obj variant");
    }
}
