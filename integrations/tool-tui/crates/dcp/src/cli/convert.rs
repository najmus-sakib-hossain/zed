//! MCP to DCP schema conversion utilities.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// MCP JSON schema representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpSchema {
    /// Tool name
    pub name: String,
    /// Tool description
    #[serde(default)]
    pub description: String,
    /// Input schema
    #[serde(rename = "inputSchema", default)]
    pub input_schema: McpInputSchema,
}

/// MCP input schema
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct McpInputSchema {
    /// Schema type (usually "object")
    #[serde(rename = "type", default)]
    pub schema_type: String,
    /// Properties
    #[serde(default)]
    pub properties: HashMap<String, McpProperty>,
    /// Required fields
    #[serde(default)]
    pub required: Vec<String>,
}

/// MCP property definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpProperty {
    /// Property type
    #[serde(rename = "type")]
    pub prop_type: String,
    /// Description
    #[serde(default)]
    pub description: String,
    /// Enum values (if applicable)
    #[serde(rename = "enum", default)]
    pub enum_values: Vec<String>,
    /// Default value
    #[serde(default)]
    pub default: Option<serde_json::Value>,
}

/// DCP binary schema representation
#[derive(Debug, Clone)]
pub struct DcpSchema {
    /// Tool name
    pub name: String,
    /// Tool ID (assigned during conversion)
    pub tool_id: u16,
    /// Description
    pub description: String,
    /// Field definitions
    pub fields: Vec<DcpField>,
    /// Required fields bitmask
    pub required_mask: u64,
}

/// DCP field definition
#[derive(Debug, Clone)]
pub struct DcpField {
    /// Field name
    pub name: String,
    /// Field type
    pub field_type: DcpFieldType,
    /// Offset in binary layout
    pub offset: u16,
    /// Size in bytes
    pub size: u16,
}

/// DCP field types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum DcpFieldType {
    Null = 0,
    Bool = 1,
    I32 = 2,
    I64 = 3,
    F64 = 4,
    String = 5,
    Bytes = 6,
    Array = 7,
    Object = 8,
}

impl DcpFieldType {
    /// Get the default size for this type
    pub fn default_size(&self) -> u16 {
        match self {
            Self::Null => 0,
            Self::Bool => 1,
            Self::I32 => 4,
            Self::I64 => 8,
            Self::F64 => 8,
            Self::String => 8, // offset + length
            Self::Bytes => 8,  // offset + length
            Self::Array => 8,  // offset + length
            Self::Object => 8, // offset + length
        }
    }

    /// Convert from MCP type string
    pub fn from_mcp_type(mcp_type: &str) -> Self {
        match mcp_type {
            "boolean" => Self::Bool,
            "integer" => Self::I64,
            "number" => Self::F64,
            "string" => Self::String,
            "array" => Self::Array,
            "object" => Self::Object,
            "null" => Self::Null,
            _ => Self::String, // Default to string for unknown types
        }
    }
}

impl DcpSchema {
    /// Serialize schema to binary format
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();

        // Header: magic (2) + version (1) + tool_id (2) + name_len (1) + desc_len (2) + field_count (1) + required_mask (8)
        bytes.extend_from_slice(&[0xDC, 0x53]); // Magic "DCS" for DCP Schema
        bytes.push(1); // Version 1
        bytes.extend_from_slice(&self.tool_id.to_le_bytes());

        let name_bytes = self.name.as_bytes();
        bytes.push(name_bytes.len() as u8);

        let desc_bytes = self.description.as_bytes();
        bytes.extend_from_slice(&(desc_bytes.len() as u16).to_le_bytes());

        bytes.push(self.fields.len() as u8);
        bytes.extend_from_slice(&self.required_mask.to_le_bytes());

        // Name
        bytes.extend_from_slice(name_bytes);

        // Description
        bytes.extend_from_slice(desc_bytes);

        // Fields
        for field in &self.fields {
            let field_name = field.name.as_bytes();
            bytes.push(field_name.len() as u8);
            bytes.extend_from_slice(field_name);
            bytes.push(field.field_type as u8);
            bytes.extend_from_slice(&field.offset.to_le_bytes());
            bytes.extend_from_slice(&field.size.to_le_bytes());
        }

        bytes
    }

    /// Deserialize schema from binary format
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() < 17 {
            return Err("Insufficient data for schema header".to_string());
        }

        // Check magic
        if bytes[0] != 0xDC || bytes[1] != 0x53 {
            return Err("Invalid schema magic number".to_string());
        }

        // Check version
        if bytes[2] != 1 {
            return Err(format!("Unsupported schema version: {}", bytes[2]));
        }

        let tool_id = u16::from_le_bytes([bytes[3], bytes[4]]);
        let name_len = bytes[5] as usize;
        let desc_len = u16::from_le_bytes([bytes[6], bytes[7]]) as usize;
        let field_count = bytes[8] as usize;
        let required_mask = u64::from_le_bytes([
            bytes[9], bytes[10], bytes[11], bytes[12], bytes[13], bytes[14], bytes[15], bytes[16],
        ]);

        let mut offset = 17;

        // Read name
        if offset + name_len > bytes.len() {
            return Err("Insufficient data for name".to_string());
        }
        let name = String::from_utf8(bytes[offset..offset + name_len].to_vec())
            .map_err(|e| format!("Invalid name encoding: {}", e))?;
        offset += name_len;

        // Read description
        if offset + desc_len > bytes.len() {
            return Err("Insufficient data for description".to_string());
        }
        let description = String::from_utf8(bytes[offset..offset + desc_len].to_vec())
            .map_err(|e| format!("Invalid description encoding: {}", e))?;
        offset += desc_len;

        // Read fields
        let mut fields = Vec::with_capacity(field_count);
        for _ in 0..field_count {
            if offset >= bytes.len() {
                return Err("Insufficient data for field".to_string());
            }

            let field_name_len = bytes[offset] as usize;
            offset += 1;

            if offset + field_name_len + 5 > bytes.len() {
                return Err("Insufficient data for field data".to_string());
            }

            let field_name = String::from_utf8(bytes[offset..offset + field_name_len].to_vec())
                .map_err(|e| format!("Invalid field name encoding: {}", e))?;
            offset += field_name_len;

            let field_type = match bytes[offset] {
                0 => DcpFieldType::Null,
                1 => DcpFieldType::Bool,
                2 => DcpFieldType::I32,
                3 => DcpFieldType::I64,
                4 => DcpFieldType::F64,
                5 => DcpFieldType::String,
                6 => DcpFieldType::Bytes,
                7 => DcpFieldType::Array,
                8 => DcpFieldType::Object,
                _ => return Err(format!("Invalid field type: {}", bytes[offset])),
            };
            offset += 1;

            let field_offset = u16::from_le_bytes([bytes[offset], bytes[offset + 1]]);
            offset += 2;

            let field_size = u16::from_le_bytes([bytes[offset], bytes[offset + 1]]);
            offset += 2;

            fields.push(DcpField {
                name: field_name,
                field_type,
                offset: field_offset,
                size: field_size,
            });
        }

        Ok(Self {
            name,
            tool_id,
            description,
            fields,
            required_mask,
        })
    }
}

/// Convert MCP schema to DCP schema
pub fn convert_mcp_to_dcp(mcp: &McpSchema) -> Result<DcpSchema, String> {
    // Generate tool ID from name hash
    let tool_id = {
        let hash = blake3::hash(mcp.name.as_bytes());
        let bytes = hash.as_bytes();
        u16::from_le_bytes([bytes[0], bytes[1]])
    };

    // Convert fields
    let mut fields = Vec::new();
    let mut current_offset: u16 = 0;
    let mut required_mask: u64 = 0;

    // Sort properties by name for consistent ordering
    let mut prop_names: Vec<_> = mcp.input_schema.properties.keys().collect();
    prop_names.sort();

    for (idx, name) in prop_names.iter().enumerate() {
        if idx >= 64 {
            return Err("Too many fields (max 64)".to_string());
        }

        let prop = &mcp.input_schema.properties[*name];
        let field_type = DcpFieldType::from_mcp_type(&prop.prop_type);
        let size = field_type.default_size();

        fields.push(DcpField {
            name: (*name).clone(),
            field_type,
            offset: current_offset,
            size,
        });

        // Check if required
        if mcp.input_schema.required.contains(name) {
            required_mask |= 1 << idx;
        }

        current_offset += size;
    }

    Ok(DcpSchema {
        name: mcp.name.clone(),
        tool_id,
        description: mcp.description.clone(),
        fields,
        required_mask,
    })
}

/// Convert DCP schema back to MCP schema
pub fn convert_dcp_to_mcp(dcp: &DcpSchema) -> McpSchema {
    let mut properties = HashMap::new();

    for field in dcp.fields.iter() {
        let prop_type = match field.field_type {
            DcpFieldType::Null => "null",
            DcpFieldType::Bool => "boolean",
            DcpFieldType::I32 | DcpFieldType::I64 => "integer",
            DcpFieldType::F64 => "number",
            DcpFieldType::String => "string",
            DcpFieldType::Bytes => "string", // Base64 encoded
            DcpFieldType::Array => "array",
            DcpFieldType::Object => "object",
        };

        properties.insert(
            field.name.clone(),
            McpProperty {
                prop_type: prop_type.to_string(),
                description: String::new(),
                enum_values: Vec::new(),
                default: None,
            },
        );
    }

    // Build required list
    let required: Vec<String> = dcp
        .fields
        .iter()
        .enumerate()
        .filter(|(idx, _)| (dcp.required_mask & (1 << idx)) != 0)
        .map(|(_, field)| field.name.clone())
        .collect();

    McpSchema {
        name: dcp.name.clone(),
        description: dcp.description.clone(),
        input_schema: McpInputSchema {
            schema_type: "object".to_string(),
            properties,
            required,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_convert_simple_schema() {
        let mcp = McpSchema {
            name: "test_tool".to_string(),
            description: "A test tool".to_string(),
            input_schema: McpInputSchema {
                schema_type: "object".to_string(),
                properties: {
                    let mut props = HashMap::new();
                    props.insert(
                        "name".to_string(),
                        McpProperty {
                            prop_type: "string".to_string(),
                            description: "The name".to_string(),
                            enum_values: Vec::new(),
                            default: None,
                        },
                    );
                    props.insert(
                        "count".to_string(),
                        McpProperty {
                            prop_type: "integer".to_string(),
                            description: "The count".to_string(),
                            enum_values: Vec::new(),
                            default: None,
                        },
                    );
                    props
                },
                required: vec!["name".to_string()],
            },
        };

        let dcp = convert_mcp_to_dcp(&mcp).unwrap();

        assert_eq!(dcp.name, "test_tool");
        assert_eq!(dcp.description, "A test tool");
        assert_eq!(dcp.fields.len(), 2);
    }

    #[test]
    fn test_schema_round_trip() {
        let mcp = McpSchema {
            name: "round_trip".to_string(),
            description: "Test round trip".to_string(),
            input_schema: McpInputSchema {
                schema_type: "object".to_string(),
                properties: {
                    let mut props = HashMap::new();
                    props.insert(
                        "field1".to_string(),
                        McpProperty {
                            prop_type: "string".to_string(),
                            description: String::new(),
                            enum_values: Vec::new(),
                            default: None,
                        },
                    );
                    props
                },
                required: vec!["field1".to_string()],
            },
        };

        let dcp = convert_mcp_to_dcp(&mcp).unwrap();
        let bytes = dcp.to_bytes();
        let restored = DcpSchema::from_bytes(&bytes).unwrap();

        assert_eq!(restored.name, dcp.name);
        assert_eq!(restored.tool_id, dcp.tool_id);
        assert_eq!(restored.fields.len(), dcp.fields.len());
        assert_eq!(restored.required_mask, dcp.required_mask);
    }

    #[test]
    fn test_field_type_conversion() {
        assert_eq!(DcpFieldType::from_mcp_type("boolean"), DcpFieldType::Bool);
        assert_eq!(DcpFieldType::from_mcp_type("integer"), DcpFieldType::I64);
        assert_eq!(DcpFieldType::from_mcp_type("number"), DcpFieldType::F64);
        assert_eq!(DcpFieldType::from_mcp_type("string"), DcpFieldType::String);
        assert_eq!(DcpFieldType::from_mcp_type("array"), DcpFieldType::Array);
        assert_eq!(DcpFieldType::from_mcp_type("object"), DcpFieldType::Object);
    }

    #[test]
    fn test_dcp_to_mcp_conversion() {
        let dcp = DcpSchema {
            name: "test".to_string(),
            tool_id: 1,
            description: "Test".to_string(),
            fields: vec![DcpField {
                name: "field1".to_string(),
                field_type: DcpFieldType::String,
                offset: 0,
                size: 8,
            }],
            required_mask: 1,
        };

        let mcp = convert_dcp_to_mcp(&dcp);

        assert_eq!(mcp.name, "test");
        assert!(mcp.input_schema.properties.contains_key("field1"));
        assert!(mcp.input_schema.required.contains(&"field1".to_string()));
    }
}
