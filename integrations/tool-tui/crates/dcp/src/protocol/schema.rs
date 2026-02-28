//! Schema validation for DCP protocol.

use crate::binary::ArgType;
use crate::DCPError;

/// Compile-time tool schema
#[derive(Debug, Clone)]
pub struct ToolSchema {
    /// Tool name (for MCP compatibility)
    pub name: &'static str,
    /// Tool ID (for DCP native)
    pub id: u16,
    /// Description
    pub description: &'static str,
    /// Input schema
    pub input: InputSchema,
}

/// Input parameter schema
#[derive(Debug, Clone)]
pub struct InputSchema {
    /// Required fields bitmask (bit N = field N is required)
    pub required: u64,
    /// Field definitions
    pub fields: Vec<FieldDef>,
}

/// Field definition
#[derive(Debug, Clone)]
pub struct FieldDef {
    /// Field name
    pub name: &'static str,
    /// Field type
    pub field_type: ArgType,
    /// Offset in binary layout
    pub offset: u16,
    /// Size in bytes
    pub size: u16,
    /// For enum types: valid range (min, max)
    pub enum_range: Option<(u8, u8)>,
}

/// Schema validator
pub struct SchemaValidator;

impl SchemaValidator {
    /// Validate that all required fields are present
    /// `present_mask` is a bitmask where bit N indicates field N is present
    pub fn validate_required(schema: &InputSchema, present_mask: u64) -> Result<(), DCPError> {
        let missing = schema.required & !present_mask;
        if missing != 0 {
            return Err(DCPError::ValidationFailed);
        }
        Ok(())
    }

    /// Validate an enum value is within range
    pub fn validate_enum(field: &FieldDef, value: u8) -> Result<(), DCPError> {
        if let Some((min, max)) = field.enum_range {
            if value < min || value > max {
                return Err(DCPError::ValidationFailed);
            }
        }
        Ok(())
    }

    /// Validate a complete input against schema
    pub fn validate_input(
        schema: &InputSchema,
        present_mask: u64,
        field_values: &[(usize, u8)], // (field_index, enum_value) for enum fields
    ) -> Result<(), DCPError> {
        // Check required fields
        Self::validate_required(schema, present_mask)?;

        // Check enum ranges
        for &(field_idx, value) in field_values {
            if let Some(field) = schema.fields.get(field_idx) {
                Self::validate_enum(field, value)?;
            }
        }

        Ok(())
    }
}

impl InputSchema {
    /// Create a new input schema
    pub fn new() -> Self {
        Self {
            required: 0,
            fields: Vec::new(),
        }
    }

    /// Add a field to the schema
    pub fn add_field(&mut self, field: FieldDef) -> &mut Self {
        self.fields.push(field);
        self
    }

    /// Mark a field as required by index
    pub fn set_required(&mut self, field_index: usize) -> &mut Self {
        if field_index < 64 {
            self.required |= 1 << field_index;
        }
        self
    }

    /// Check if a field is required
    pub fn is_required(&self, field_index: usize) -> bool {
        if field_index >= 64 {
            return false;
        }
        self.required & (1 << field_index) != 0
    }

    /// Get the number of required fields
    pub fn required_count(&self) -> u32 {
        self.required.count_ones()
    }
}

impl Default for InputSchema {
    fn default() -> Self {
        Self::new()
    }
}

impl FieldDef {
    /// Create a new field definition
    pub fn new(name: &'static str, field_type: ArgType, offset: u16, size: u16) -> Self {
        Self {
            name,
            field_type,
            offset,
            size,
            enum_range: None,
        }
    }

    /// Create an enum field with valid range
    pub fn new_enum(name: &'static str, offset: u16, size: u16, min: u8, max: u8) -> Self {
        Self {
            name,
            field_type: ArgType::I32, // Enums are represented as integers
            offset,
            size,
            enum_range: Some((min, max)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_required_validation() {
        let mut schema = InputSchema::new();
        schema.set_required(0);
        schema.set_required(2);

        // All required present
        assert!(SchemaValidator::validate_required(&schema, 0b101).is_ok());
        assert!(SchemaValidator::validate_required(&schema, 0b111).is_ok());

        // Missing field 0
        assert_eq!(
            SchemaValidator::validate_required(&schema, 0b100),
            Err(DCPError::ValidationFailed)
        );

        // Missing field 2
        assert_eq!(
            SchemaValidator::validate_required(&schema, 0b001),
            Err(DCPError::ValidationFailed)
        );
    }

    #[test]
    fn test_enum_validation() {
        let field = FieldDef::new_enum("status", 0, 1, 1, 5);

        assert!(SchemaValidator::validate_enum(&field, 1).is_ok());
        assert!(SchemaValidator::validate_enum(&field, 3).is_ok());
        assert!(SchemaValidator::validate_enum(&field, 5).is_ok());

        assert_eq!(SchemaValidator::validate_enum(&field, 0), Err(DCPError::ValidationFailed));
        assert_eq!(SchemaValidator::validate_enum(&field, 6), Err(DCPError::ValidationFailed));
    }

    #[test]
    fn test_complete_validation() {
        let mut schema = InputSchema::new();
        schema.add_field(FieldDef::new("name", ArgType::String, 0, 32));
        schema.add_field(FieldDef::new_enum("type", 32, 1, 1, 3));
        schema.set_required(0);
        schema.set_required(1);

        // Valid input
        assert!(SchemaValidator::validate_input(&schema, 0b11, &[(1, 2)]).is_ok());

        // Missing required field
        assert_eq!(
            SchemaValidator::validate_input(&schema, 0b01, &[(1, 2)]),
            Err(DCPError::ValidationFailed)
        );

        // Invalid enum value
        assert_eq!(
            SchemaValidator::validate_input(&schema, 0b11, &[(1, 5)]),
            Err(DCPError::ValidationFailed)
        );
    }

    #[test]
    fn test_schema_helpers() {
        let mut schema = InputSchema::new();
        schema.set_required(0);
        schema.set_required(3);

        assert!(schema.is_required(0));
        assert!(!schema.is_required(1));
        assert!(!schema.is_required(2));
        assert!(schema.is_required(3));
        assert_eq!(schema.required_count(), 2);
    }
}
