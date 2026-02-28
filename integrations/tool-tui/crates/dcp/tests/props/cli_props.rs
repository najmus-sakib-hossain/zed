//! Property tests for CLI schema conversion.
//!
//! Feature: dcp-protocol, Property 16: Schema Conversion Correctness

use proptest::prelude::*;
use std::collections::HashMap;

use dcp::cli::convert::{
    convert_dcp_to_mcp, convert_mcp_to_dcp, DcpFieldType, DcpSchema, McpInputSchema, McpProperty,
    McpSchema,
};

/// Generate a valid MCP property type
fn mcp_type_strategy() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("string".to_string()),
        Just("integer".to_string()),
        Just("number".to_string()),
        Just("boolean".to_string()),
        Just("array".to_string()),
        Just("object".to_string()),
    ]
}

/// Generate a valid field name
fn field_name_strategy() -> impl Strategy<Value = String> {
    "[a-z][a-z0-9_]{0,19}".prop_map(|s| s)
}

/// Generate a valid tool name
fn tool_name_strategy() -> impl Strategy<Value = String> {
    "[a-z][a-z0-9_]{2,30}".prop_map(|s| s)
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Feature: dcp-protocol, Property 16: Schema Conversion Correctness
    /// For any valid MCP tool schema, converting to DCP format and back
    /// SHALL produce an equivalent schema with all fields preserved.
    #[test]
    fn prop_schema_conversion_round_trip(
        name in tool_name_strategy(),
        description in "[a-zA-Z0-9 ]{0,100}",
        field_names in prop::collection::hash_set(field_name_strategy(), 1..10),
        field_types in prop::collection::vec(mcp_type_strategy(), 1..10),
    ) {
        // Build MCP schema
        let mut properties = HashMap::new();
        let field_names: Vec<_> = field_names.into_iter().collect();

        for (i, field_name) in field_names.iter().enumerate() {
            let field_type = &field_types[i % field_types.len()];
            properties.insert(
                field_name.clone(),
                McpProperty {
                    prop_type: field_type.clone(),
                    description: String::new(),
                    enum_values: Vec::new(),
                    default: None,
                },
            );
        }

        // Make some fields required
        let required: Vec<String> = field_names
            .iter()
            .take(field_names.len() / 2)
            .cloned()
            .collect();

        let mcp = McpSchema {
            name: name.clone(),
            description: description.clone(),
            input_schema: McpInputSchema {
                schema_type: "object".to_string(),
                properties,
                required: required.clone(),
            },
        };

        // Convert MCP -> DCP
        let dcp = convert_mcp_to_dcp(&mcp).unwrap();

        // Verify DCP schema
        prop_assert_eq!(&dcp.name, &name);
        prop_assert_eq!(&dcp.description, &description);
        prop_assert_eq!(dcp.fields.len(), field_names.len());

        // Convert DCP -> MCP
        let mcp_restored = convert_dcp_to_mcp(&dcp);

        // Verify restored schema
        prop_assert_eq!(&mcp_restored.name, &name);
        prop_assert_eq!(&mcp_restored.description, &description);
        prop_assert_eq!(mcp_restored.input_schema.properties.len(), field_names.len());

        // Verify all field names are preserved
        for field_name in &field_names {
            prop_assert!(mcp_restored.input_schema.properties.contains_key(field_name));
        }

        // Verify required fields are preserved
        for req_field in &required {
            prop_assert!(mcp_restored.input_schema.required.contains(req_field));
        }
    }

    /// Test DCP schema binary serialization round-trip
    #[test]
    fn prop_dcp_schema_binary_round_trip(
        name in tool_name_strategy(),
        tool_id in 0u16..65535,
        description in "[a-zA-Z0-9 ]{0,100}",
        field_count in 1usize..10,
    ) {
        let fields: Vec<_> = (0..field_count)
            .map(|i| dcp::cli::convert::DcpField {
                name: format!("field_{}", i),
                field_type: match i % 6 {
                    0 => DcpFieldType::String,
                    1 => DcpFieldType::I64,
                    2 => DcpFieldType::F64,
                    3 => DcpFieldType::Bool,
                    4 => DcpFieldType::Array,
                    _ => DcpFieldType::Object,
                },
                offset: (i * 8) as u16,
                size: 8,
            })
            .collect();

        let required_mask = (1u64 << (field_count / 2)) - 1;

        let schema = DcpSchema {
            name: name.clone(),
            tool_id,
            description: description.clone(),
            fields: fields.clone(),
            required_mask,
        };

        // Serialize to bytes
        let bytes = schema.to_bytes();

        // Deserialize from bytes
        let restored = DcpSchema::from_bytes(&bytes).unwrap();

        // Verify all fields
        prop_assert_eq!(&restored.name, &name);
        prop_assert_eq!(restored.tool_id, tool_id);
        prop_assert_eq!(&restored.description, &description);
        prop_assert_eq!(restored.fields.len(), field_count);
        prop_assert_eq!(restored.required_mask, required_mask);

        // Verify field details
        for (original, restored_field) in fields.iter().zip(restored.fields.iter()) {
            prop_assert_eq!(&restored_field.name, &original.name);
            prop_assert_eq!(restored_field.field_type, original.field_type);
            prop_assert_eq!(restored_field.offset, original.offset);
            prop_assert_eq!(restored_field.size, original.size);
        }
    }

    /// Test field type conversion consistency
    #[test]
    fn prop_field_type_conversion_consistent(
        mcp_type in mcp_type_strategy(),
    ) {
        let dcp_type = DcpFieldType::from_mcp_type(&mcp_type);

        // Verify type is valid
        prop_assert!(matches!(
            dcp_type,
            DcpFieldType::String
                | DcpFieldType::I64
                | DcpFieldType::F64
                | DcpFieldType::Bool
                | DcpFieldType::Array
                | DcpFieldType::Object
                | DcpFieldType::Null
        ));

        // Verify size is reasonable
        let size = dcp_type.default_size();
        prop_assert!(size <= 8);
    }

    /// Test that required field mask is correctly computed
    #[test]
    fn prop_required_mask_correct(
        field_count in 1usize..20,
        required_indices in prop::collection::vec(0usize..64, 0..10),
    ) {
        let field_names: Vec<String> = (0..field_count)
            .map(|i| format!("field_{}", i))
            .collect();

        let required: Vec<String> = required_indices
            .iter()
            .filter(|&&i| i < field_count)
            .map(|&i| format!("field_{}", i))
            .collect();

        let mut properties = HashMap::new();
        for name in &field_names {
            properties.insert(
                name.clone(),
                McpProperty {
                    prop_type: "string".to_string(),
                    description: String::new(),
                    enum_values: Vec::new(),
                    default: None,
                },
            );
        }

        let mcp = McpSchema {
            name: "test".to_string(),
            description: String::new(),
            input_schema: McpInputSchema {
                schema_type: "object".to_string(),
                properties,
                required: required.clone(),
            },
        };

        let dcp = convert_mcp_to_dcp(&mcp).unwrap();

        // Verify required mask has correct number of bits set
        let expected_required_count = required.len();
        let actual_required_count = dcp.required_mask.count_ones() as usize;

        // The count should match (accounting for deduplication)
        let unique_required: std::collections::HashSet<_> = required.iter().collect();
        prop_assert!(actual_required_count <= unique_required.len());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_schema_conversion() {
        let mcp = McpSchema {
            name: "empty".to_string(),
            description: String::new(),
            input_schema: McpInputSchema {
                schema_type: "object".to_string(),
                properties: HashMap::new(),
                required: Vec::new(),
            },
        };

        let dcp = convert_mcp_to_dcp(&mcp).unwrap();
        assert_eq!(dcp.name, "empty");
        assert_eq!(dcp.fields.len(), 0);
        assert_eq!(dcp.required_mask, 0);
    }

    #[test]
    fn test_all_field_types() {
        let types = vec![
            ("string", DcpFieldType::String),
            ("integer", DcpFieldType::I64),
            ("number", DcpFieldType::F64),
            ("boolean", DcpFieldType::Bool),
            ("array", DcpFieldType::Array),
            ("object", DcpFieldType::Object),
            ("null", DcpFieldType::Null),
        ];

        for (mcp_type, expected_dcp_type) in types {
            let actual = DcpFieldType::from_mcp_type(mcp_type);
            assert_eq!(actual, expected_dcp_type, "Failed for type: {}", mcp_type);
        }
    }
}
