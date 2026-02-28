//! Pydantic Rust Core Compatibility
//!
//! Provides compatibility with Pydantic's Rust core for:
//! - Model validation
//! - Schema generation
//! - Type coercion

use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use thiserror::Error;

/// Errors that can occur during validation
#[derive(Debug, Error, Clone)]
pub enum ValidationError {
    #[error("Field '{field}' is required")]
    MissingField { field: String },

    #[error("Field '{field}' has invalid type: expected {expected}, got {actual}")]
    InvalidType {
        field: String,
        expected: String,
        actual: String,
    },

    #[error("Field '{field}' failed constraint: {constraint}")]
    ConstraintViolation { field: String, constraint: String },

    #[error("Field '{field}' has invalid value: {message}")]
    InvalidValue { field: String, message: String },

    #[error("Multiple validation errors: {0:?}")]
    Multiple(Vec<ValidationError>),
}

/// Field types supported by Pydantic
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FieldType {
    String,
    Integer,
    Float,
    Boolean,
    List(Box<FieldType>),
    Dict(Box<FieldType>, Box<FieldType>),
    Optional(Box<FieldType>),
    Model(String), // Reference to another model by name
    Any,
}

impl FieldType {
    /// Get the JSON schema type name
    pub fn json_type(&self) -> &'static str {
        match self {
            FieldType::String => "string",
            FieldType::Integer => "integer",
            FieldType::Float => "number",
            FieldType::Boolean => "boolean",
            FieldType::List(_) => "array",
            FieldType::Dict(_, _) => "object",
            FieldType::Optional(inner) => inner.json_type(),
            FieldType::Model(_) => "object",
            FieldType::Any => "any",
        }
    }
}

/// Field constraints for validation
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FieldConstraints {
    /// Minimum value (for numbers)
    pub min_value: Option<f64>,
    /// Maximum value (for numbers)
    pub max_value: Option<f64>,
    /// Minimum length (for strings/lists)
    pub min_length: Option<usize>,
    /// Maximum length (for strings/lists)
    pub max_length: Option<usize>,
    /// Regex pattern (for strings)
    pub pattern: Option<String>,
    /// Allowed values (enum)
    pub enum_values: Option<Vec<JsonValue>>,
    /// Custom validator function name
    pub validator: Option<String>,
}

/// A field in a Pydantic model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PydanticField {
    /// Field name
    pub name: String,
    /// Field type
    pub field_type: FieldType,
    /// Whether the field is required
    pub required: bool,
    /// Default value (if any)
    pub default: Option<JsonValue>,
    /// Field description
    pub description: Option<String>,
    /// Field alias (for JSON serialization)
    pub alias: Option<String>,
    /// Validation constraints
    pub constraints: FieldConstraints,
}

impl PydanticField {
    /// Create a new required field
    pub fn new(name: impl Into<String>, field_type: FieldType) -> Self {
        Self {
            name: name.into(),
            field_type,
            required: true,
            default: None,
            description: None,
            alias: None,
            constraints: FieldConstraints::default(),
        }
    }

    /// Make the field optional
    pub fn optional(mut self) -> Self {
        self.required = false;
        self.field_type = FieldType::Optional(Box::new(self.field_type));
        self
    }

    /// Set default value
    pub fn with_default(mut self, default: JsonValue) -> Self {
        self.default = Some(default);
        self.required = false;
        self
    }

    /// Set description
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// Set alias
    pub fn with_alias(mut self, alias: impl Into<String>) -> Self {
        self.alias = Some(alias.into());
        self
    }

    /// Set minimum value constraint
    pub fn with_min(mut self, min: f64) -> Self {
        self.constraints.min_value = Some(min);
        self
    }

    /// Set maximum value constraint
    pub fn with_max(mut self, max: f64) -> Self {
        self.constraints.max_value = Some(max);
        self
    }

    /// Set minimum length constraint
    pub fn with_min_length(mut self, min: usize) -> Self {
        self.constraints.min_length = Some(min);
        self
    }

    /// Set maximum length constraint
    pub fn with_max_length(mut self, max: usize) -> Self {
        self.constraints.max_length = Some(max);
        self
    }

    /// Set pattern constraint
    pub fn with_pattern(mut self, pattern: impl Into<String>) -> Self {
        self.constraints.pattern = Some(pattern.into());
        self
    }

    /// Get the field name to use for JSON (alias or name)
    pub fn json_name(&self) -> &str {
        self.alias.as_deref().unwrap_or(&self.name)
    }
}

/// A Pydantic model definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PydanticModel {
    /// Model name
    pub name: String,
    /// Model fields
    pub fields: Vec<PydanticField>,
    /// Model description
    pub description: Option<String>,
    /// Whether to allow extra fields
    pub extra: ExtraFieldsBehavior,
}

/// How to handle extra fields not defined in the model
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ExtraFieldsBehavior {
    /// Ignore extra fields
    #[default]
    Ignore,
    /// Allow extra fields
    Allow,
    /// Forbid extra fields (validation error)
    Forbid,
}

impl PydanticModel {
    /// Create a new model
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            fields: Vec::new(),
            description: None,
            extra: ExtraFieldsBehavior::default(),
        }
    }

    /// Add a field to the model
    pub fn field(mut self, field: PydanticField) -> Self {
        self.fields.push(field);
        self
    }

    /// Set description
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// Set extra fields behavior
    pub fn with_extra(mut self, extra: ExtraFieldsBehavior) -> Self {
        self.extra = extra;
        self
    }

    /// Get a field by name
    pub fn get_field(&self, name: &str) -> Option<&PydanticField> {
        self.fields.iter().find(|f| f.name == name || f.alias.as_deref() == Some(name))
    }

    /// Get required fields
    pub fn required_fields(&self) -> impl Iterator<Item = &PydanticField> {
        self.fields.iter().filter(|f| f.required)
    }
}

/// Model validator
pub struct ModelValidator {
    /// Registered models
    models: HashMap<String, PydanticModel>,
}

impl ModelValidator {
    /// Create a new validator
    pub fn new() -> Self {
        Self {
            models: HashMap::new(),
        }
    }

    /// Register a model
    pub fn register(&mut self, model: PydanticModel) {
        self.models.insert(model.name.clone(), model);
    }

    /// Get a registered model
    pub fn get_model(&self, name: &str) -> Option<&PydanticModel> {
        self.models.get(name)
    }

    /// Validate data against a model
    pub fn validate(
        &self,
        model_name: &str,
        data: &JsonValue,
    ) -> Result<JsonValue, ValidationError> {
        let model = self.models.get(model_name).ok_or_else(|| ValidationError::InvalidValue {
            field: "model".to_string(),
            message: format!("Unknown model: {}", model_name),
        })?;

        self.validate_model(model, data)
    }

    /// Validate data against a model definition
    fn validate_model(
        &self,
        model: &PydanticModel,
        data: &JsonValue,
    ) -> Result<JsonValue, ValidationError> {
        let obj = data.as_object().ok_or_else(|| ValidationError::InvalidType {
            field: model.name.clone(),
            expected: "object".to_string(),
            actual: json_type_name(data).to_string(),
        })?;

        let mut errors = Vec::new();
        let mut result = serde_json::Map::new();

        // Validate each field
        for field in &model.fields {
            let json_name = field.json_name();
            let value = obj.get(json_name).or_else(|| obj.get(&field.name));

            match value {
                Some(v) => match self.validate_field(field, v) {
                    Ok(validated) => {
                        result.insert(field.name.clone(), validated);
                    }
                    Err(e) => errors.push(e),
                },
                None => {
                    if field.required {
                        if let Some(ref default) = field.default {
                            result.insert(field.name.clone(), default.clone());
                        } else {
                            errors.push(ValidationError::MissingField {
                                field: field.name.clone(),
                            });
                        }
                    } else if let Some(ref default) = field.default {
                        result.insert(field.name.clone(), default.clone());
                    }
                }
            }
        }

        // Check for extra fields
        if model.extra == ExtraFieldsBehavior::Forbid {
            for key in obj.keys() {
                if model.get_field(key).is_none() {
                    errors.push(ValidationError::InvalidValue {
                        field: key.clone(),
                        message: "Extra field not allowed".to_string(),
                    });
                }
            }
        } else if model.extra == ExtraFieldsBehavior::Allow {
            for (key, value) in obj {
                if model.get_field(key).is_none() {
                    result.insert(key.clone(), value.clone());
                }
            }
        }

        if errors.is_empty() {
            Ok(JsonValue::Object(result))
        } else if errors.len() == 1 {
            Err(errors.remove(0))
        } else {
            Err(ValidationError::Multiple(errors))
        }
    }
}

impl ModelValidator {
    /// Validate a single field value
    fn validate_field(
        &self,
        field: &PydanticField,
        value: &JsonValue,
    ) -> Result<JsonValue, ValidationError> {
        // Handle null for optional fields
        if value.is_null() {
            if let FieldType::Optional(_) = field.field_type {
                return Ok(JsonValue::Null);
            }
            return Err(ValidationError::InvalidType {
                field: field.name.clone(),
                expected: field.field_type.json_type().to_string(),
                actual: "null".to_string(),
            });
        }

        // Validate type
        let validated = self.validate_type(&field.name, &field.field_type, value)?;

        // Apply constraints
        self.validate_constraints(field, &validated)?;

        Ok(validated)
    }

    /// Validate value against type
    fn validate_type(
        &self,
        field_name: &str,
        field_type: &FieldType,
        value: &JsonValue,
    ) -> Result<JsonValue, ValidationError> {
        match field_type {
            FieldType::String => {
                if value.is_string() {
                    Ok(value.clone())
                } else {
                    // Try to coerce to string
                    Ok(JsonValue::String(value.to_string()))
                }
            }
            FieldType::Integer => {
                if let Some(n) = value.as_i64() {
                    Ok(JsonValue::Number(n.into()))
                } else if let Some(n) = value.as_f64() {
                    // Coerce float to int
                    Ok(JsonValue::Number((n as i64).into()))
                } else if let Some(s) = value.as_str() {
                    // Try to parse string as int
                    s.parse::<i64>().map(|n| JsonValue::Number(n.into())).map_err(|_| {
                        ValidationError::InvalidType {
                            field: field_name.to_string(),
                            expected: "integer".to_string(),
                            actual: json_type_name(value).to_string(),
                        }
                    })
                } else {
                    Err(ValidationError::InvalidType {
                        field: field_name.to_string(),
                        expected: "integer".to_string(),
                        actual: json_type_name(value).to_string(),
                    })
                }
            }
            FieldType::Float => {
                if let Some(n) = value.as_f64() {
                    Ok(JsonValue::Number(
                        serde_json::Number::from_f64(n).unwrap_or_else(|| 0.into()),
                    ))
                } else if let Some(s) = value.as_str() {
                    s.parse::<f64>()
                        .map(|n| {
                            JsonValue::Number(
                                serde_json::Number::from_f64(n).unwrap_or_else(|| 0.into()),
                            )
                        })
                        .map_err(|_| ValidationError::InvalidType {
                            field: field_name.to_string(),
                            expected: "number".to_string(),
                            actual: json_type_name(value).to_string(),
                        })
                } else {
                    Err(ValidationError::InvalidType {
                        field: field_name.to_string(),
                        expected: "number".to_string(),
                        actual: json_type_name(value).to_string(),
                    })
                }
            }
            FieldType::Boolean => {
                if value.is_boolean() {
                    Ok(value.clone())
                } else if let Some(s) = value.as_str() {
                    match s.to_lowercase().as_str() {
                        "true" | "1" | "yes" => Ok(JsonValue::Bool(true)),
                        "false" | "0" | "no" => Ok(JsonValue::Bool(false)),
                        _ => Err(ValidationError::InvalidType {
                            field: field_name.to_string(),
                            expected: "boolean".to_string(),
                            actual: json_type_name(value).to_string(),
                        }),
                    }
                } else if let Some(n) = value.as_i64() {
                    Ok(JsonValue::Bool(n != 0))
                } else {
                    Err(ValidationError::InvalidType {
                        field: field_name.to_string(),
                        expected: "boolean".to_string(),
                        actual: json_type_name(value).to_string(),
                    })
                }
            }
            FieldType::List(inner) => {
                if let Some(arr) = value.as_array() {
                    let validated: Result<Vec<JsonValue>, _> = arr
                        .iter()
                        .enumerate()
                        .map(|(i, v)| {
                            self.validate_type(&format!("{}[{}]", field_name, i), inner, v)
                        })
                        .collect();
                    Ok(JsonValue::Array(validated?))
                } else {
                    Err(ValidationError::InvalidType {
                        field: field_name.to_string(),
                        expected: "array".to_string(),
                        actual: json_type_name(value).to_string(),
                    })
                }
            }
            FieldType::Dict(key_type, value_type) => {
                if let Some(obj) = value.as_object() {
                    let mut result = serde_json::Map::new();
                    for (k, v) in obj {
                        // Validate key (must be string in JSON)
                        if !matches!(key_type.as_ref(), FieldType::String) {
                            return Err(ValidationError::InvalidType {
                                field: field_name.to_string(),
                                expected: "string keys".to_string(),
                                actual: "non-string keys".to_string(),
                            });
                        }
                        let validated_value =
                            self.validate_type(&format!("{}.{}", field_name, k), value_type, v)?;
                        result.insert(k.clone(), validated_value);
                    }
                    Ok(JsonValue::Object(result))
                } else {
                    Err(ValidationError::InvalidType {
                        field: field_name.to_string(),
                        expected: "object".to_string(),
                        actual: json_type_name(value).to_string(),
                    })
                }
            }
            FieldType::Optional(inner) => {
                if value.is_null() {
                    Ok(JsonValue::Null)
                } else {
                    self.validate_type(field_name, inner, value)
                }
            }
            FieldType::Model(model_name) => {
                if let Some(model) = self.models.get(model_name) {
                    self.validate_model(model, value)
                } else {
                    Err(ValidationError::InvalidValue {
                        field: field_name.to_string(),
                        message: format!("Unknown model: {}", model_name),
                    })
                }
            }
            FieldType::Any => Ok(value.clone()),
        }
    }

    /// Validate constraints on a field value
    fn validate_constraints(
        &self,
        field: &PydanticField,
        value: &JsonValue,
    ) -> Result<(), ValidationError> {
        let constraints = &field.constraints;

        // Min/max value for numbers
        if let Some(min) = constraints.min_value {
            if let Some(n) = value.as_f64() {
                if n < min {
                    return Err(ValidationError::ConstraintViolation {
                        field: field.name.clone(),
                        constraint: format!("value must be >= {}", min),
                    });
                }
            }
        }

        if let Some(max) = constraints.max_value {
            if let Some(n) = value.as_f64() {
                if n > max {
                    return Err(ValidationError::ConstraintViolation {
                        field: field.name.clone(),
                        constraint: format!("value must be <= {}", max),
                    });
                }
            }
        }

        // Min/max length for strings and arrays
        if let Some(min) = constraints.min_length {
            let len = if let Some(s) = value.as_str() {
                s.len()
            } else if let Some(arr) = value.as_array() {
                arr.len()
            } else {
                return Ok(());
            };

            if len < min {
                return Err(ValidationError::ConstraintViolation {
                    field: field.name.clone(),
                    constraint: format!("length must be >= {}", min),
                });
            }
        }

        if let Some(max) = constraints.max_length {
            let len = if let Some(s) = value.as_str() {
                s.len()
            } else if let Some(arr) = value.as_array() {
                arr.len()
            } else {
                return Ok(());
            };

            if len > max {
                return Err(ValidationError::ConstraintViolation {
                    field: field.name.clone(),
                    constraint: format!("length must be <= {}", max),
                });
            }
        }

        // Pattern matching for strings
        if let Some(ref pattern) = constraints.pattern {
            if let Some(s) = value.as_str() {
                let re = regex::Regex::new(pattern).map_err(|_| ValidationError::InvalidValue {
                    field: field.name.clone(),
                    message: format!("Invalid regex pattern: {}", pattern),
                })?;

                if !re.is_match(s) {
                    return Err(ValidationError::ConstraintViolation {
                        field: field.name.clone(),
                        constraint: format!("value must match pattern: {}", pattern),
                    });
                }
            }
        }

        // Enum validation
        if let Some(ref enum_values) = constraints.enum_values {
            if !enum_values.contains(value) {
                return Err(ValidationError::ConstraintViolation {
                    field: field.name.clone(),
                    constraint: format!("value must be one of: {:?}", enum_values),
                });
            }
        }

        Ok(())
    }
}

impl Default for ModelValidator {
    fn default() -> Self {
        Self::new()
    }
}

/// Get the JSON type name for a value
fn json_type_name(value: &JsonValue) -> &'static str {
    match value {
        JsonValue::Null => "null",
        JsonValue::Bool(_) => "boolean",
        JsonValue::Number(_) => "number",
        JsonValue::String(_) => "string",
        JsonValue::Array(_) => "array",
        JsonValue::Object(_) => "object",
    }
}

/// JSON Schema generator for Pydantic models
pub struct SchemaGenerator {
    /// Registered models
    models: HashMap<String, PydanticModel>,
}

/// JSON Schema representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonSchema {
    #[serde(rename = "$schema", skip_serializing_if = "Option::is_none")]
    pub schema: Option<String>,
    #[serde(rename = "type")]
    pub schema_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub properties: Option<HashMap<String, JsonSchema>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub items: Option<Box<JsonSchema>>,
    #[serde(
        rename = "additionalProperties",
        skip_serializing_if = "Option::is_none"
    )]
    pub additional_properties: Option<Box<JsonSchema>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub minimum: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub maximum: Option<f64>,
    #[serde(rename = "minLength", skip_serializing_if = "Option::is_none")]
    pub min_length: Option<usize>,
    #[serde(rename = "maxLength", skip_serializing_if = "Option::is_none")]
    pub max_length: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pattern: Option<String>,
    #[serde(rename = "enum", skip_serializing_if = "Option::is_none")]
    pub enum_values: Option<Vec<JsonValue>>,
    #[serde(rename = "$ref", skip_serializing_if = "Option::is_none")]
    pub reference: Option<String>,
}

impl JsonSchema {
    /// Create a new schema with a type
    pub fn new(schema_type: impl Into<String>) -> Self {
        Self {
            schema: None,
            schema_type: schema_type.into(),
            title: None,
            description: None,
            properties: None,
            required: None,
            items: None,
            additional_properties: None,
            minimum: None,
            maximum: None,
            min_length: None,
            max_length: None,
            pattern: None,
            enum_values: None,
            reference: None,
        }
    }

    /// Create a reference schema
    pub fn reference(ref_path: impl Into<String>) -> Self {
        Self {
            schema: None,
            schema_type: String::new(),
            title: None,
            description: None,
            properties: None,
            required: None,
            items: None,
            additional_properties: None,
            minimum: None,
            maximum: None,
            min_length: None,
            max_length: None,
            pattern: None,
            enum_values: None,
            reference: Some(ref_path.into()),
        }
    }
}

impl SchemaGenerator {
    /// Create a new schema generator
    pub fn new() -> Self {
        Self {
            models: HashMap::new(),
        }
    }

    /// Register a model
    pub fn register(&mut self, model: PydanticModel) {
        self.models.insert(model.name.clone(), model);
    }

    /// Generate JSON schema for a model
    pub fn generate(&self, model_name: &str) -> Option<JsonSchema> {
        let model = self.models.get(model_name)?;
        Some(self.model_to_schema(model))
    }

    /// Convert a model to JSON schema
    fn model_to_schema(&self, model: &PydanticModel) -> JsonSchema {
        let mut properties = HashMap::new();
        let mut required = Vec::new();

        for field in &model.fields {
            let field_schema = self.field_to_schema(field);
            properties.insert(field.name.clone(), field_schema);

            if field.required {
                required.push(field.name.clone());
            }
        }

        let additional_properties = match model.extra {
            ExtraFieldsBehavior::Allow => Some(Box::new(JsonSchema::new("any"))),
            ExtraFieldsBehavior::Forbid => None,
            ExtraFieldsBehavior::Ignore => None,
        };

        JsonSchema {
            schema: Some("http://json-schema.org/draft-07/schema#".to_string()),
            schema_type: "object".to_string(),
            title: Some(model.name.clone()),
            description: model.description.clone(),
            properties: Some(properties),
            required: if required.is_empty() {
                None
            } else {
                Some(required)
            },
            items: None,
            additional_properties,
            minimum: None,
            maximum: None,
            min_length: None,
            max_length: None,
            pattern: None,
            enum_values: None,
            reference: None,
        }
    }

    /// Convert a field to JSON schema
    fn field_to_schema(&self, field: &PydanticField) -> JsonSchema {
        let mut schema = self.type_to_schema(&field.field_type);

        schema.description = field.description.clone();

        // Apply constraints
        schema.minimum = field.constraints.min_value;
        schema.maximum = field.constraints.max_value;
        schema.min_length = field.constraints.min_length;
        schema.max_length = field.constraints.max_length;
        schema.pattern = field.constraints.pattern.clone();
        schema.enum_values = field.constraints.enum_values.clone();

        schema
    }

    /// Convert a field type to JSON schema
    fn type_to_schema(&self, field_type: &FieldType) -> JsonSchema {
        let _ = self; // Silence unused self warning - method may use self in future
        match field_type {
            FieldType::String => JsonSchema::new("string"),
            FieldType::Integer => JsonSchema::new("integer"),
            FieldType::Float => JsonSchema::new("number"),
            FieldType::Boolean => JsonSchema::new("boolean"),
            FieldType::List(inner) => {
                let mut schema = JsonSchema::new("array");
                schema.items = Some(Box::new(self.type_to_schema(inner)));
                schema
            }
            FieldType::Dict(_, value_type) => {
                let mut schema = JsonSchema::new("object");
                schema.additional_properties = Some(Box::new(self.type_to_schema(value_type)));
                schema
            }
            FieldType::Optional(inner) => {
                // In JSON Schema, optional is handled at the required level
                self.type_to_schema(inner)
            }
            FieldType::Model(name) => JsonSchema::reference(format!("#/definitions/{}", name)),
            FieldType::Any => JsonSchema::new("any"),
        }
    }
}

impl Default for SchemaGenerator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_field_type_json_type() {
        assert_eq!(FieldType::String.json_type(), "string");
        assert_eq!(FieldType::Integer.json_type(), "integer");
        assert_eq!(FieldType::Float.json_type(), "number");
        assert_eq!(FieldType::Boolean.json_type(), "boolean");
        assert_eq!(FieldType::List(Box::new(FieldType::String)).json_type(), "array");
    }

    #[test]
    fn test_pydantic_field_builder() {
        let field = PydanticField::new("name", FieldType::String)
            .with_description("User name")
            .with_min_length(1)
            .with_max_length(100);

        assert_eq!(field.name, "name");
        assert!(field.required);
        assert_eq!(field.description, Some("User name".to_string()));
        assert_eq!(field.constraints.min_length, Some(1));
        assert_eq!(field.constraints.max_length, Some(100));
    }

    #[test]
    fn test_pydantic_field_optional() {
        let field = PydanticField::new("age", FieldType::Integer).optional();

        assert!(!field.required);
        assert!(matches!(field.field_type, FieldType::Optional(_)));
    }

    #[test]
    fn test_pydantic_model_builder() {
        let model = PydanticModel::new("User")
            .field(PydanticField::new("name", FieldType::String))
            .field(PydanticField::new("age", FieldType::Integer).optional())
            .with_description("A user model");

        assert_eq!(model.name, "User");
        assert_eq!(model.fields.len(), 2);
        assert_eq!(model.description, Some("A user model".to_string()));
    }

    #[test]
    fn test_model_validator_simple() {
        let mut validator = ModelValidator::new();

        let model = PydanticModel::new("User")
            .field(PydanticField::new("name", FieldType::String))
            .field(PydanticField::new("age", FieldType::Integer));

        validator.register(model);

        let data = serde_json::json!({
            "name": "Alice",
            "age": 30
        });

        let result = validator.validate("User", &data);
        assert!(result.is_ok());
    }

    #[test]
    fn test_model_validator_missing_required() {
        let mut validator = ModelValidator::new();

        let model = PydanticModel::new("User")
            .field(PydanticField::new("name", FieldType::String))
            .field(PydanticField::new("age", FieldType::Integer));

        validator.register(model);

        let data = serde_json::json!({
            "name": "Alice"
        });

        let result = validator.validate("User", &data);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ValidationError::MissingField { .. }));
    }

    #[test]
    fn test_model_validator_type_coercion() {
        let mut validator = ModelValidator::new();

        let model = PydanticModel::new("User").field(PydanticField::new("age", FieldType::Integer));

        validator.register(model);

        // String should be coerced to integer
        let data = serde_json::json!({
            "age": "30"
        });

        let result = validator.validate("User", &data);
        assert!(result.is_ok());
        let validated = result.unwrap();
        assert_eq!(validated["age"], 30);
    }

    #[test]
    fn test_model_validator_constraints() {
        let mut validator = ModelValidator::new();

        let model = PydanticModel::new("User")
            .field(PydanticField::new("age", FieldType::Integer).with_min(0.0).with_max(150.0));

        validator.register(model);

        // Valid age
        let data = serde_json::json!({ "age": 30 });
        assert!(validator.validate("User", &data).is_ok());

        // Invalid age (negative)
        let data = serde_json::json!({ "age": -5 });
        assert!(validator.validate("User", &data).is_err());

        // Invalid age (too high)
        let data = serde_json::json!({ "age": 200 });
        assert!(validator.validate("User", &data).is_err());
    }

    #[test]
    fn test_model_validator_nested() {
        let mut validator = ModelValidator::new();

        let address_model = PydanticModel::new("Address")
            .field(PydanticField::new("city", FieldType::String))
            .field(PydanticField::new("country", FieldType::String));

        let user_model = PydanticModel::new("User")
            .field(PydanticField::new("name", FieldType::String))
            .field(PydanticField::new("address", FieldType::Model("Address".to_string())));

        validator.register(address_model);
        validator.register(user_model);

        let data = serde_json::json!({
            "name": "Alice",
            "address": {
                "city": "New York",
                "country": "USA"
            }
        });

        let result = validator.validate("User", &data);
        assert!(result.is_ok());
    }

    #[test]
    fn test_schema_generator() {
        let mut generator = SchemaGenerator::new();

        let model = PydanticModel::new("User")
            .field(PydanticField::new("name", FieldType::String).with_description("User name"))
            .field(PydanticField::new("age", FieldType::Integer).optional())
            .with_description("A user model");

        generator.register(model);

        let schema = generator.generate("User").unwrap();
        assert_eq!(schema.title, Some("User".to_string()));
        assert_eq!(schema.schema_type, "object");
        assert!(schema.properties.is_some());

        let props = schema.properties.unwrap();
        assert!(props.contains_key("name"));
        assert!(props.contains_key("age"));
    }

    #[test]
    fn test_extra_fields_forbid() {
        let mut validator = ModelValidator::new();

        let model = PydanticModel::new("User")
            .field(PydanticField::new("name", FieldType::String))
            .with_extra(ExtraFieldsBehavior::Forbid);

        validator.register(model);

        let data = serde_json::json!({
            "name": "Alice",
            "extra_field": "not allowed"
        });

        let result = validator.validate("User", &data);
        assert!(result.is_err());
    }

    #[test]
    fn test_extra_fields_allow() {
        let mut validator = ModelValidator::new();

        let model = PydanticModel::new("User")
            .field(PydanticField::new("name", FieldType::String))
            .with_extra(ExtraFieldsBehavior::Allow);

        validator.register(model);

        let data = serde_json::json!({
            "name": "Alice",
            "extra_field": "allowed"
        });

        let result = validator.validate("User", &data);
        assert!(result.is_ok());
        let validated = result.unwrap();
        assert_eq!(validated["extra_field"], "allowed");
    }
}
