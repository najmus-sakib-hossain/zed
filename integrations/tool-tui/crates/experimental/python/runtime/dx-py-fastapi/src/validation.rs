//! Type Validation for FastAPI Endpoints
//!
//! Provides type hint validation for FastAPI endpoint parameters
//! and return types.

use crate::pydantic::{FieldType, ModelValidator, PydanticModel, ValidationError};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use thiserror::Error;

/// Errors that can occur during endpoint validation
#[derive(Debug, Error)]
pub enum EndpointValidationError {
    #[error("Parameter validation failed: {0}")]
    ParameterError(String),

    #[error("Body validation failed: {0}")]
    BodyError(String),

    #[error("Response validation failed: {0}")]
    ResponseError(String),

    #[error("Type mismatch: expected {expected}, got {actual}")]
    TypeMismatch { expected: String, actual: String },
}

/// Result of validation
#[derive(Debug, Clone)]
pub struct ValidationResult {
    /// Whether validation passed
    pub valid: bool,
    /// Validated and coerced values
    pub values: HashMap<String, JsonValue>,
    /// Validation errors (if any)
    pub errors: Vec<ValidationError>,
}

impl ValidationResult {
    /// Create a successful result
    pub fn success(values: HashMap<String, JsonValue>) -> Self {
        Self {
            valid: true,
            values,
            errors: Vec::new(),
        }
    }

    /// Create a failed result
    pub fn failure(errors: Vec<ValidationError>) -> Self {
        Self {
            valid: false,
            values: HashMap::new(),
            errors,
        }
    }
}

/// Type validator for Python type hints
pub struct TypeValidator {
    model_validator: ModelValidator,
}

impl TypeValidator {
    /// Create a new type validator
    pub fn new() -> Self {
        Self {
            model_validator: ModelValidator::new(),
        }
    }

    /// Register a Pydantic model
    pub fn register_model(&mut self, model: PydanticModel) {
        self.model_validator.register(model);
    }

    /// Validate a value against a type
    pub fn validate(
        &self,
        value: &JsonValue,
        field_type: &FieldType,
    ) -> Result<JsonValue, ValidationError> {
        match field_type {
            FieldType::String => self.validate_string(value),
            FieldType::Integer => self.validate_integer(value),
            FieldType::Float => self.validate_float(value),
            FieldType::Boolean => self.validate_boolean(value),
            FieldType::List(inner) => self.validate_list(value, inner),
            FieldType::Dict(key_type, value_type) => {
                self.validate_dict(value, key_type, value_type)
            }
            FieldType::Optional(inner) => self.validate_optional(value, inner),
            FieldType::Model(name) => self.model_validator.validate(name, value),
            FieldType::Any => Ok(value.clone()),
        }
    }

    fn validate_string(&self, value: &JsonValue) -> Result<JsonValue, ValidationError> {
        if value.is_string() {
            Ok(value.clone())
        } else {
            Ok(JsonValue::String(value.to_string()))
        }
    }

    fn validate_integer(&self, value: &JsonValue) -> Result<JsonValue, ValidationError> {
        if let Some(n) = value.as_i64() {
            Ok(JsonValue::Number(n.into()))
        } else if let Some(n) = value.as_f64() {
            Ok(JsonValue::Number((n as i64).into()))
        } else if let Some(s) = value.as_str() {
            s.parse::<i64>().map(|n| JsonValue::Number(n.into())).map_err(|_| {
                ValidationError::InvalidType {
                    field: "value".to_string(),
                    expected: "integer".to_string(),
                    actual: "string".to_string(),
                }
            })
        } else {
            Err(ValidationError::InvalidType {
                field: "value".to_string(),
                expected: "integer".to_string(),
                actual: json_type_name(value).to_string(),
            })
        }
    }

    fn validate_float(&self, value: &JsonValue) -> Result<JsonValue, ValidationError> {
        if let Some(n) = value.as_f64() {
            Ok(JsonValue::Number(serde_json::Number::from_f64(n).unwrap_or_else(|| 0.into())))
        } else if let Some(s) = value.as_str() {
            s.parse::<f64>()
                .map(|n| {
                    JsonValue::Number(serde_json::Number::from_f64(n).unwrap_or_else(|| 0.into()))
                })
                .map_err(|_| ValidationError::InvalidType {
                    field: "value".to_string(),
                    expected: "float".to_string(),
                    actual: "string".to_string(),
                })
        } else {
            Err(ValidationError::InvalidType {
                field: "value".to_string(),
                expected: "float".to_string(),
                actual: json_type_name(value).to_string(),
            })
        }
    }

    fn validate_boolean(&self, value: &JsonValue) -> Result<JsonValue, ValidationError> {
        if value.is_boolean() {
            Ok(value.clone())
        } else if let Some(s) = value.as_str() {
            match s.to_lowercase().as_str() {
                "true" | "1" | "yes" => Ok(JsonValue::Bool(true)),
                "false" | "0" | "no" => Ok(JsonValue::Bool(false)),
                _ => Err(ValidationError::InvalidType {
                    field: "value".to_string(),
                    expected: "boolean".to_string(),
                    actual: "string".to_string(),
                }),
            }
        } else if let Some(n) = value.as_i64() {
            Ok(JsonValue::Bool(n != 0))
        } else {
            Err(ValidationError::InvalidType {
                field: "value".to_string(),
                expected: "boolean".to_string(),
                actual: json_type_name(value).to_string(),
            })
        }
    }

    fn validate_list(
        &self,
        value: &JsonValue,
        inner: &FieldType,
    ) -> Result<JsonValue, ValidationError> {
        if let Some(arr) = value.as_array() {
            let validated: Result<Vec<JsonValue>, _> =
                arr.iter().map(|v| self.validate(v, inner)).collect();
            Ok(JsonValue::Array(validated?))
        } else {
            Err(ValidationError::InvalidType {
                field: "value".to_string(),
                expected: "array".to_string(),
                actual: json_type_name(value).to_string(),
            })
        }
    }

    fn validate_dict(
        &self,
        value: &JsonValue,
        _key_type: &FieldType,
        value_type: &FieldType,
    ) -> Result<JsonValue, ValidationError> {
        if let Some(obj) = value.as_object() {
            let mut result = serde_json::Map::new();
            for (k, v) in obj {
                let validated_value = self.validate(v, value_type)?;
                result.insert(k.clone(), validated_value);
            }
            Ok(JsonValue::Object(result))
        } else {
            Err(ValidationError::InvalidType {
                field: "value".to_string(),
                expected: "object".to_string(),
                actual: json_type_name(value).to_string(),
            })
        }
    }

    fn validate_optional(
        &self,
        value: &JsonValue,
        inner: &FieldType,
    ) -> Result<JsonValue, ValidationError> {
        if value.is_null() {
            Ok(JsonValue::Null)
        } else {
            self.validate(value, inner)
        }
    }
}

impl Default for TypeValidator {
    fn default() -> Self {
        Self::new()
    }
}

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

/// Endpoint parameter definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndpointParameter {
    /// Parameter name
    pub name: String,
    /// Parameter type
    pub param_type: FieldType,
    /// Parameter source (path, query, body, header)
    pub source: ParameterSource,
    /// Whether the parameter is required
    pub required: bool,
    /// Default value
    pub default: Option<JsonValue>,
    /// Description
    pub description: Option<String>,
}

/// Source of a parameter value
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ParameterSource {
    Path,
    Query,
    Body,
    Header,
    Cookie,
}

impl EndpointParameter {
    /// Create a path parameter
    pub fn path(name: impl Into<String>, param_type: FieldType) -> Self {
        Self {
            name: name.into(),
            param_type,
            source: ParameterSource::Path,
            required: true,
            default: None,
            description: None,
        }
    }

    /// Create a query parameter
    pub fn query(name: impl Into<String>, param_type: FieldType) -> Self {
        Self {
            name: name.into(),
            param_type,
            source: ParameterSource::Query,
            required: false,
            default: None,
            description: None,
        }
    }

    /// Create a body parameter
    pub fn body(name: impl Into<String>, param_type: FieldType) -> Self {
        Self {
            name: name.into(),
            param_type,
            source: ParameterSource::Body,
            required: true,
            default: None,
            description: None,
        }
    }

    /// Create a header parameter
    pub fn header(name: impl Into<String>, param_type: FieldType) -> Self {
        Self {
            name: name.into(),
            param_type,
            source: ParameterSource::Header,
            required: false,
            default: None,
            description: None,
        }
    }

    /// Make the parameter required
    pub fn required(mut self) -> Self {
        self.required = true;
        self
    }

    /// Make the parameter optional with a default
    pub fn with_default(mut self, default: JsonValue) -> Self {
        self.required = false;
        self.default = Some(default);
        self
    }

    /// Set description
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }
}

/// Endpoint validator
pub struct EndpointValidator {
    type_validator: TypeValidator,
    parameters: Vec<EndpointParameter>,
    response_type: Option<FieldType>,
}

impl EndpointValidator {
    /// Create a new endpoint validator
    pub fn new() -> Self {
        Self {
            type_validator: TypeValidator::new(),
            parameters: Vec::new(),
            response_type: None,
        }
    }

    /// Register a Pydantic model
    pub fn register_model(&mut self, model: PydanticModel) {
        self.type_validator.register_model(model);
    }

    /// Add a parameter
    pub fn add_parameter(&mut self, param: EndpointParameter) {
        self.parameters.push(param);
    }

    /// Set response type
    pub fn set_response_type(&mut self, response_type: FieldType) {
        self.response_type = Some(response_type);
    }

    /// Validate request parameters
    pub fn validate_request(
        &self,
        path_params: &HashMap<String, String>,
        query_params: &HashMap<String, String>,
        headers: &HashMap<String, String>,
        body: Option<&JsonValue>,
    ) -> ValidationResult {
        let mut values = HashMap::new();
        let mut errors = Vec::new();

        for param in &self.parameters {
            let raw_value = match param.source {
                ParameterSource::Path => {
                    path_params.get(&param.name).map(|s| JsonValue::String(s.clone()))
                }
                ParameterSource::Query => {
                    query_params.get(&param.name).map(|s| JsonValue::String(s.clone()))
                }
                ParameterSource::Header => {
                    headers.get(&param.name.to_lowercase()).map(|s| JsonValue::String(s.clone()))
                }
                ParameterSource::Body => body.cloned(),
                ParameterSource::Cookie => None, // Not implemented yet
            };

            match raw_value {
                Some(value) => match self.type_validator.validate(&value, &param.param_type) {
                    Ok(validated) => {
                        values.insert(param.name.clone(), validated);
                    }
                    Err(e) => {
                        errors.push(e);
                    }
                },
                None => {
                    if param.required {
                        if let Some(ref default) = param.default {
                            values.insert(param.name.clone(), default.clone());
                        } else {
                            errors.push(ValidationError::MissingField {
                                field: param.name.clone(),
                            });
                        }
                    } else if let Some(ref default) = param.default {
                        values.insert(param.name.clone(), default.clone());
                    }
                }
            }
        }

        if errors.is_empty() {
            ValidationResult::success(values)
        } else {
            ValidationResult::failure(errors)
        }
    }

    /// Validate response
    pub fn validate_response(&self, response: &JsonValue) -> Result<JsonValue, ValidationError> {
        if let Some(ref response_type) = self.response_type {
            self.type_validator.validate(response, response_type)
        } else {
            Ok(response.clone())
        }
    }
}

impl Default for EndpointValidator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_type_validator_string() {
        let validator = TypeValidator::new();

        let result =
            validator.validate(&JsonValue::String("hello".to_string()), &FieldType::String);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), JsonValue::String("hello".to_string()));
    }

    #[test]
    fn test_type_validator_integer() {
        let validator = TypeValidator::new();

        let result = validator.validate(&JsonValue::Number(42.into()), &FieldType::Integer);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), JsonValue::Number(42.into()));

        // String coercion
        let result = validator.validate(&JsonValue::String("123".to_string()), &FieldType::Integer);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), JsonValue::Number(123.into()));
    }

    #[test]
    fn test_type_validator_boolean() {
        let validator = TypeValidator::new();

        let result = validator.validate(&JsonValue::Bool(true), &FieldType::Boolean);
        assert!(result.is_ok());

        // String coercion
        let result =
            validator.validate(&JsonValue::String("true".to_string()), &FieldType::Boolean);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), JsonValue::Bool(true));
    }

    #[test]
    fn test_type_validator_list() {
        let validator = TypeValidator::new();

        let value = serde_json::json!([1, 2, 3]);
        let result = validator.validate(&value, &FieldType::List(Box::new(FieldType::Integer)));
        assert!(result.is_ok());
    }

    #[test]
    fn test_endpoint_parameter_path() {
        let param = EndpointParameter::path("id", FieldType::Integer).with_description("User ID");

        assert_eq!(param.name, "id");
        assert_eq!(param.source, ParameterSource::Path);
        assert!(param.required);
    }

    #[test]
    fn test_endpoint_parameter_query() {
        let param = EndpointParameter::query("page", FieldType::Integer)
            .with_default(JsonValue::Number(1.into()));

        assert_eq!(param.name, "page");
        assert_eq!(param.source, ParameterSource::Query);
        assert!(!param.required);
        assert_eq!(param.default, Some(JsonValue::Number(1.into())));
    }

    #[test]
    fn test_endpoint_validator() {
        let mut validator = EndpointValidator::new();
        validator.add_parameter(EndpointParameter::path("id", FieldType::Integer));
        validator.add_parameter(
            EndpointParameter::query("page", FieldType::Integer)
                .with_default(JsonValue::Number(1.into())),
        );

        let mut path_params = HashMap::new();
        path_params.insert("id".to_string(), "42".to_string());

        let query_params = HashMap::new();
        let headers = HashMap::new();

        let result = validator.validate_request(&path_params, &query_params, &headers, None);

        assert!(result.valid);
        assert_eq!(result.values.get("id"), Some(&JsonValue::Number(42.into())));
        assert_eq!(result.values.get("page"), Some(&JsonValue::Number(1.into())));
    }

    #[test]
    fn test_endpoint_validator_missing_required() {
        let mut validator = EndpointValidator::new();
        validator.add_parameter(EndpointParameter::path("id", FieldType::Integer));

        let path_params = HashMap::new();
        let query_params = HashMap::new();
        let headers = HashMap::new();

        let result = validator.validate_request(&path_params, &query_params, &headers, None);

        assert!(!result.valid);
        assert!(!result.errors.is_empty());
    }
}
