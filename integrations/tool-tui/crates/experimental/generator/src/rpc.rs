//! AI Agent Protocol - Feature #3
//!
//! JSON-RPC interface for AI agents to invoke code generation.
//! Provides a structured protocol for requesting template generation
//! with intelligent defaults and context-aware parameter inference.
//!
//! ## Protocol
//!
//! The protocol uses JSON-RPC 2.0 format:
//!
//! ```json
//! {
//!   "jsonrpc": "2.0",
//!   "method": "generate",
//!   "params": {
//!     "template": "component",
//!     "parameters": { "name": "Counter" }
//!   },
//!   "id": 1
//! }
//! ```

use crate::params::Parameters;
use crate::registry::{ParameterSchema, TemplateMetadata, TemplateRegistry};
use std::collections::HashMap;
use std::path::PathBuf;

#[cfg(feature = "serde-compat")]
use serde::{Deserialize, Serialize};

// ============================================================================
// Request Types
// ============================================================================

/// A generation request from an AI agent.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde-compat", derive(Serialize, Deserialize))]
pub struct GenerateRequest {
    /// Template identifier (e.g., "component", "rust-crate").
    pub template: String,
    /// Parameters for the template.
    #[cfg_attr(feature = "serde-compat", serde(default))]
    pub parameters: HashMap<String, RequestValue>,
    /// Output path (optional, uses template default if not specified).
    #[cfg_attr(feature = "serde-compat", serde(default))]
    pub output: Option<PathBuf>,
    /// Whether to perform a dry run (preview only).
    #[cfg_attr(feature = "serde-compat", serde(default))]
    pub dry_run: bool,
    /// Context for intelligent defaults.
    #[cfg_attr(feature = "serde-compat", serde(default))]
    pub context: RequestContext,
}

/// A value in a generation request.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde-compat", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde-compat", serde(untagged))]
pub enum RequestValue {
    /// String value.
    String(String),
    /// Integer value.
    Integer(i64),
    /// Boolean value.
    Boolean(bool),
    /// Array of values.
    Array(Vec<RequestValue>),
}

impl RequestValue {
    /// Convert to string representation.
    #[must_use]
    pub fn as_string(&self) -> String {
        match self {
            RequestValue::String(s) => s.clone(),
            RequestValue::Integer(i) => i.to_string(),
            RequestValue::Boolean(b) => b.to_string(),
            RequestValue::Array(arr) => {
                let items: Vec<String> = arr.iter().map(|v| v.as_string()).collect();
                items.join(", ")
            }
        }
    }

    /// Check if this is a truthy value.
    #[must_use]
    pub fn is_truthy(&self) -> bool {
        match self {
            RequestValue::String(s) => !s.is_empty() && s != "false" && s != "0",
            RequestValue::Integer(i) => *i != 0,
            RequestValue::Boolean(b) => *b,
            RequestValue::Array(arr) => !arr.is_empty(),
        }
    }
}

/// Context information for intelligent defaults.
#[derive(Clone, Debug, Default)]
#[cfg_attr(feature = "serde-compat", derive(Serialize, Deserialize))]
pub struct RequestContext {
    /// Current working directory.
    #[cfg_attr(feature = "serde-compat", serde(default))]
    pub cwd: Option<PathBuf>,
    /// Current file being edited.
    #[cfg_attr(feature = "serde-compat", serde(default))]
    pub current_file: Option<PathBuf>,
    /// Project name (from package.json, Cargo.toml, etc.).
    #[cfg_attr(feature = "serde-compat", serde(default))]
    pub project_name: Option<String>,
    /// Additional context hints.
    #[cfg_attr(feature = "serde-compat", serde(default))]
    pub hints: HashMap<String, String>,
}

impl GenerateRequest {
    /// Create a new generation request.
    #[must_use]
    pub fn new(template: impl Into<String>) -> Self {
        Self {
            template: template.into(),
            parameters: HashMap::new(),
            output: None,
            dry_run: false,
            context: RequestContext::default(),
        }
    }

    /// Set a parameter value.
    #[must_use]
    pub fn with_param(mut self, name: impl Into<String>, value: impl Into<RequestValue>) -> Self {
        self.parameters.insert(name.into(), value.into());
        self
    }

    /// Set the output path.
    #[must_use]
    pub fn with_output(mut self, path: impl Into<PathBuf>) -> Self {
        self.output = Some(path.into());
        self
    }

    /// Enable dry run mode.
    #[must_use]
    pub fn dry_run(mut self) -> Self {
        self.dry_run = true;
        self
    }

    /// Set the context.
    #[must_use]
    pub fn with_context(mut self, context: RequestContext) -> Self {
        self.context = context;
        self
    }
}

impl From<String> for RequestValue {
    fn from(s: String) -> Self {
        RequestValue::String(s)
    }
}

impl From<&str> for RequestValue {
    fn from(s: &str) -> Self {
        RequestValue::String(s.to_string())
    }
}

impl From<i64> for RequestValue {
    fn from(i: i64) -> Self {
        RequestValue::Integer(i)
    }
}

impl From<i32> for RequestValue {
    fn from(i: i32) -> Self {
        RequestValue::Integer(i as i64)
    }
}

impl From<bool> for RequestValue {
    fn from(b: bool) -> Self {
        RequestValue::Boolean(b)
    }
}

// ============================================================================
// Response Types
// ============================================================================

/// A generation response to an AI agent.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde-compat", derive(Serialize, Deserialize))]
pub struct GenerateResponse {
    /// Whether the generation was successful.
    pub success: bool,
    /// Generated files (path -> content).
    pub files: Vec<GeneratedFile>,
    /// Error message if generation failed.
    #[cfg_attr(
        feature = "serde-compat",
        serde(skip_serializing_if = "Option::is_none")
    )]
    pub error: Option<String>,
    /// Metrics about the generation.
    pub metrics: GenerationMetrics,
}

/// A generated file.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde-compat", derive(Serialize, Deserialize))]
pub struct GeneratedFile {
    /// Output path.
    pub path: PathBuf,
    /// Generated content.
    pub content: String,
    /// Whether the file was written (false for dry run).
    pub written: bool,
}

/// Metrics about a generation operation.
#[derive(Clone, Debug, Default)]
#[cfg_attr(feature = "serde-compat", derive(Serialize, Deserialize))]
pub struct GenerationMetrics {
    /// Time taken in milliseconds.
    pub duration_ms: f64,
    /// Total bytes generated.
    pub bytes_generated: usize,
    /// Estimated tokens saved.
    pub tokens_saved: usize,
    /// Number of files generated.
    pub files_count: usize,
}

impl GenerateResponse {
    /// Create a successful response.
    #[must_use]
    pub fn success(files: Vec<GeneratedFile>, metrics: GenerationMetrics) -> Self {
        Self {
            success: true,
            files,
            error: None,
            metrics,
        }
    }

    /// Create an error response.
    #[must_use]
    pub fn error(message: impl Into<String>) -> Self {
        Self {
            success: false,
            files: Vec::new(),
            error: Some(message.into()),
            metrics: GenerationMetrics::default(),
        }
    }
}

// ============================================================================
// JSON-RPC Types
// ============================================================================

/// JSON-RPC 2.0 request wrapper.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde-compat", derive(Serialize, Deserialize))]
pub struct JsonRpcRequest {
    /// JSON-RPC version (always "2.0").
    pub jsonrpc: String,
    /// Method name.
    pub method: String,
    /// Request parameters.
    pub params: GenerateRequest,
    /// Request ID.
    pub id: JsonRpcId,
}

/// JSON-RPC 2.0 response wrapper.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde-compat", derive(Serialize, Deserialize))]
pub struct JsonRpcResponse {
    /// JSON-RPC version (always "2.0").
    pub jsonrpc: String,
    /// Result (if successful).
    #[cfg_attr(
        feature = "serde-compat",
        serde(skip_serializing_if = "Option::is_none")
    )]
    pub result: Option<GenerateResponse>,
    /// Error (if failed).
    #[cfg_attr(
        feature = "serde-compat",
        serde(skip_serializing_if = "Option::is_none")
    )]
    pub error: Option<JsonRpcError>,
    /// Request ID.
    pub id: JsonRpcId,
}

/// JSON-RPC ID (can be string or number).
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde-compat", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde-compat", serde(untagged))]
pub enum JsonRpcId {
    /// Numeric ID.
    Number(i64),
    /// String ID.
    String(String),
}

/// JSON-RPC error.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde-compat", derive(Serialize, Deserialize))]
pub struct JsonRpcError {
    /// Error code.
    pub code: i32,
    /// Error message.
    pub message: String,
    /// Additional data.
    #[cfg_attr(
        feature = "serde-compat",
        serde(skip_serializing_if = "Option::is_none")
    )]
    pub data: Option<String>,
}

impl JsonRpcResponse {
    /// Create a successful response.
    #[must_use]
    pub fn success(id: JsonRpcId, result: GenerateResponse) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            result: Some(result),
            error: None,
            id,
        }
    }

    /// Create an error response.
    #[must_use]
    pub fn error(id: JsonRpcId, code: i32, message: impl Into<String>) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            result: None,
            error: Some(JsonRpcError {
                code,
                message: message.into(),
                data: None,
            }),
            id,
        }
    }
}

// Standard JSON-RPC error codes
/// Parse error.
pub const PARSE_ERROR: i32 = -32700;
/// Invalid request.
pub const INVALID_REQUEST: i32 = -32600;
/// Method not found.
pub const METHOD_NOT_FOUND: i32 = -32601;
/// Invalid params.
pub const INVALID_PARAMS: i32 = -32602;
/// Internal error.
pub const INTERNAL_ERROR: i32 = -32603;
/// Template not found.
pub const TEMPLATE_NOT_FOUND: i32 = -32000;
/// Generation failed.
pub const GENERATION_FAILED: i32 = -32001;

// ============================================================================
// Default Value Inference
// ============================================================================

/// Infers default values for missing parameters based on context.
#[derive(Clone, Debug, Default)]
pub struct DefaultInferrer {
    /// Whether to infer from current directory name.
    pub infer_from_cwd: bool,
    /// Whether to infer from current file name.
    pub infer_from_file: bool,
    /// Whether to use template defaults.
    pub use_template_defaults: bool,
}

impl DefaultInferrer {
    /// Create a new inferrer with all inference enabled.
    #[must_use]
    pub fn new() -> Self {
        Self {
            infer_from_cwd: true,
            infer_from_file: true,
            use_template_defaults: true,
        }
    }

    /// Infer default values for a request.
    pub fn infer(&self, request: &mut GenerateRequest, template: &TemplateMetadata) {
        for param in &template.parameters {
            // Skip if already provided
            if request.parameters.contains_key(&param.name) {
                continue;
            }

            // Try to infer value
            if let Some(value) = self.infer_param(param, &request.context) {
                request.parameters.insert(param.name.clone(), value);
            }
        }
    }

    /// Infer a single parameter value.
    fn infer_param(
        &self,
        param: &ParameterSchema,
        context: &RequestContext,
    ) -> Option<RequestValue> {
        // Try template default first
        if self.use_template_defaults {
            if let Some(ref default) = param.default {
                return Some(RequestValue::String(default.clone()));
            }
        }

        // Try context-based inference
        match param.name.as_str() {
            "name" | "component_name" | "module_name" => {
                // Infer from current file or directory
                if self.infer_from_file {
                    if let Some(ref file) = context.current_file {
                        if let Some(stem) = file.file_stem() {
                            return Some(RequestValue::String(stem.to_string_lossy().to_string()));
                        }
                    }
                }
                if self.infer_from_cwd {
                    if let Some(ref cwd) = context.cwd {
                        if let Some(name) = cwd.file_name() {
                            return Some(RequestValue::String(name.to_string_lossy().to_string()));
                        }
                    }
                }
            }
            "project" | "project_name" => {
                if let Some(ref name) = context.project_name {
                    return Some(RequestValue::String(name.clone()));
                }
            }
            _ => {
                // Check hints
                if let Some(hint) = context.hints.get(&param.name) {
                    return Some(RequestValue::String(hint.clone()));
                }
            }
        }

        None
    }
}

// ============================================================================
// RPC Handler
// ============================================================================

/// Handler for JSON-RPC generation requests.
pub struct RpcHandler {
    /// Template registry.
    registry: TemplateRegistry,
    /// Default value inferrer.
    inferrer: DefaultInferrer,
}

impl RpcHandler {
    /// Create a new RPC handler.
    #[must_use]
    pub fn new(registry: TemplateRegistry) -> Self {
        Self {
            registry,
            inferrer: DefaultInferrer::new(),
        }
    }

    /// Handle a JSON-RPC request.
    #[cfg(feature = "serde-compat")]
    pub fn handle_json(&mut self, json: &str) -> String {
        match serde_json::from_str::<JsonRpcRequest>(json) {
            Ok(request) => {
                let response = self.handle_request(request);
                serde_json::to_string(&response).unwrap_or_else(|e| {
                    format!(r#"{{"jsonrpc":"2.0","error":{{"code":{},"message":"Serialization error: {}"}},"id":null}}"#, INTERNAL_ERROR, e)
                })
            }
            Err(e) => {
                format!(
                    r#"{{"jsonrpc":"2.0","error":{{"code":{},"message":"Parse error: {}"}},"id":null}}"#,
                    PARSE_ERROR, e
                )
            }
        }
    }

    /// Handle a parsed JSON-RPC request.
    pub fn handle_request(&mut self, request: JsonRpcRequest) -> JsonRpcResponse {
        match request.method.as_str() {
            "generate" => self.handle_generate(request.id, request.params),
            "list" => self.handle_list(request.id),
            "info" => self.handle_info(request.id, &request.params.template),
            _ => JsonRpcResponse::error(
                request.id,
                METHOD_NOT_FOUND,
                format!("Method not found: {}", request.method),
            ),
        }
    }

    /// Handle a generate request.
    fn handle_generate(&mut self, id: JsonRpcId, mut request: GenerateRequest) -> JsonRpcResponse {
        let start = std::time::Instant::now();

        // Find template
        let template = match self.registry.get(&request.template) {
            Some(t) => t.clone(),
            None => {
                return JsonRpcResponse::error(
                    id,
                    TEMPLATE_NOT_FOUND,
                    format!("Template not found: {}", request.template),
                );
            }
        };

        // Infer defaults
        self.inferrer.infer(&mut request, &template);

        // Validate required parameters
        for param in &template.parameters {
            if param.required && !request.parameters.contains_key(&param.name) {
                return JsonRpcResponse::error(
                    id,
                    INVALID_PARAMS,
                    format!("Missing required parameter: {}", param.name),
                );
            }
        }

        // Build parameters
        let mut params = Parameters::new();
        for (key, value) in &request.parameters {
            params = params.set(key.clone(), value.as_string());
        }

        // For now, return a placeholder response
        // In a full implementation, this would call the generator
        let elapsed = start.elapsed();
        let content = format!(
            "// Generated from template: {}\n// Parameters: {:?}\n",
            request.template, request.parameters
        );
        let bytes = content.len();

        let files = vec![GeneratedFile {
            path: request.output.unwrap_or_else(|| PathBuf::from("output.rs")),
            content,
            written: !request.dry_run,
        }];

        let metrics = GenerationMetrics {
            duration_ms: elapsed.as_secs_f64() * 1000.0,
            bytes_generated: bytes,
            tokens_saved: bytes / 4, // Rough estimate
            files_count: files.len(),
        };

        JsonRpcResponse::success(id, GenerateResponse::success(files, metrics))
    }

    /// Handle a list request.
    fn handle_list(&self, id: JsonRpcId) -> JsonRpcResponse {
        // Return list of available templates
        let _templates: Vec<String> = self.registry.list().iter().map(|t| t.id.clone()).collect();

        // Create a simple response with template list
        let metrics = GenerationMetrics::default();
        let response = GenerateResponse {
            success: true,
            files: Vec::new(),
            error: None,
            metrics,
        };

        JsonRpcResponse::success(id, response)
    }

    /// Handle an info request.
    fn handle_info(&self, id: JsonRpcId, template_id: &str) -> JsonRpcResponse {
        match self.registry.get(template_id) {
            Some(_template) => {
                // Return template info
                let metrics = GenerationMetrics::default();
                let response = GenerateResponse {
                    success: true,
                    files: Vec::new(),
                    error: None,
                    metrics,
                };
                JsonRpcResponse::success(id, response)
            }
            None => JsonRpcResponse::error(
                id,
                TEMPLATE_NOT_FOUND,
                format!("Template not found: {}", template_id),
            ),
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_request_builder() {
        let request = GenerateRequest::new("component")
            .with_param("name", "Counter")
            .with_param("with_state", true)
            .with_output("src/Counter.tsx")
            .dry_run();

        assert_eq!(request.template, "component");
        assert!(request.parameters.contains_key("name"));
        assert!(request.parameters.contains_key("with_state"));
        assert!(request.output.is_some());
        assert!(request.dry_run);
    }

    #[test]
    fn test_request_value_conversions() {
        let s: RequestValue = "hello".into();
        assert_eq!(s.as_string(), "hello");

        let i: RequestValue = 42i64.into();
        assert_eq!(i.as_string(), "42");

        let b: RequestValue = true.into();
        assert_eq!(b.as_string(), "true");
    }

    #[test]
    fn test_request_value_truthy() {
        assert!(RequestValue::String("hello".to_string()).is_truthy());
        assert!(!RequestValue::String("".to_string()).is_truthy());
        assert!(!RequestValue::String("false".to_string()).is_truthy());
        assert!(!RequestValue::String("0".to_string()).is_truthy());

        assert!(RequestValue::Integer(1).is_truthy());
        assert!(!RequestValue::Integer(0).is_truthy());

        assert!(RequestValue::Boolean(true).is_truthy());
        assert!(!RequestValue::Boolean(false).is_truthy());
    }

    #[test]
    fn test_generate_response_success() {
        let files = vec![GeneratedFile {
            path: PathBuf::from("test.rs"),
            content: "fn main() {}".to_string(),
            written: true,
        }];
        let metrics = GenerationMetrics {
            duration_ms: 1.5,
            bytes_generated: 12,
            tokens_saved: 3,
            files_count: 1,
        };

        let response = GenerateResponse::success(files, metrics);
        assert!(response.success);
        assert!(response.error.is_none());
        assert_eq!(response.files.len(), 1);
    }

    #[test]
    fn test_generate_response_error() {
        let response = GenerateResponse::error("Template not found");
        assert!(!response.success);
        assert!(response.error.is_some());
        assert!(response.files.is_empty());
    }

    #[test]
    fn test_json_rpc_response() {
        let id = JsonRpcId::Number(1);
        let response = JsonRpcResponse::success(
            id.clone(),
            GenerateResponse::success(Vec::new(), GenerationMetrics::default()),
        );

        assert_eq!(response.jsonrpc, "2.0");
        assert!(response.result.is_some());
        assert!(response.error.is_none());
    }

    #[test]
    fn test_json_rpc_error_response() {
        let id = JsonRpcId::String("test".to_string());
        let response = JsonRpcResponse::error(id, TEMPLATE_NOT_FOUND, "Not found");

        assert_eq!(response.jsonrpc, "2.0");
        assert!(response.result.is_none());
        assert!(response.error.is_some());
        assert_eq!(response.error.as_ref().unwrap().code, TEMPLATE_NOT_FOUND);
    }

    #[test]
    fn test_default_inferrer() {
        let inferrer = DefaultInferrer::new();
        let mut request = GenerateRequest::new("component");
        request.context.cwd = Some(PathBuf::from("/projects/my-app"));

        let template = TemplateMetadata::new("component", "Component", "path")
            .with_parameter(crate::registry::ParameterSchema::new("name", "string"));

        inferrer.infer(&mut request, &template);

        // Should infer name from cwd
        assert!(request.parameters.contains_key("name"));
    }

    #[test]
    fn test_rpc_handler_template_not_found() {
        let registry = TemplateRegistry::new(".dx/templates");
        let mut handler = RpcHandler::new(registry);

        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "generate".to_string(),
            params: GenerateRequest::new("nonexistent"),
            id: JsonRpcId::Number(1),
        };

        let response = handler.handle_request(request);
        assert!(response.error.is_some());
        assert_eq!(response.error.as_ref().unwrap().code, TEMPLATE_NOT_FOUND);
    }

    #[test]
    fn test_rpc_handler_method_not_found() {
        let registry = TemplateRegistry::new(".dx/templates");
        let mut handler = RpcHandler::new(registry);

        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: "unknown".to_string(),
            params: GenerateRequest::new("test"),
            id: JsonRpcId::Number(1),
        };

        let response = handler.handle_request(request);
        assert!(response.error.is_some());
        assert_eq!(response.error.as_ref().unwrap().code, METHOD_NOT_FOUND);
    }
}

// ============================================================================
// Property-Based Tests
// ============================================================================

#[cfg(test)]
mod property_tests {
    use super::*;
    use proptest::prelude::*;

    /// Strategy for generating valid template IDs
    fn template_id_strategy() -> impl Strategy<Value = String> {
        "[a-z][a-z0-9-]{0,15}".prop_map(|s| s.to_string())
    }

    /// Strategy for generating parameter names
    fn param_name_strategy() -> impl Strategy<Value = String> {
        "[a-z][a-z0-9_]{0,15}".prop_map(|s| s.to_string())
    }

    /// Strategy for generating parameter values
    fn param_value_strategy() -> impl Strategy<Value = RequestValue> {
        prop_oneof![
            "[a-zA-Z0-9_]{1,20}".prop_map(RequestValue::String),
            any::<i64>().prop_map(RequestValue::Integer),
            any::<bool>().prop_map(RequestValue::Boolean),
        ]
    }

    // **Feature: dx-generator-production, Property 8: AI Protocol Request-Response Consistency**
    // **Validates: Requirements 3.1, 3.4**
    //
    // *For any* valid generation request, the response SHALL contain
    // consistent information about the generation result.
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Property 8.1: Request values are correctly converted to strings
        /// For any request value, as_string() SHALL produce a valid string representation.
        #[test]
        fn prop_request_value_string_conversion(
            value in param_value_strategy()
        ) {
            let string_repr = value.as_string();

            // Property: string representation should not be empty for non-empty values
            match &value {
                RequestValue::String(s) => {
                    prop_assert_eq!(string_repr, s.clone());
                }
                RequestValue::Integer(i) => {
                    prop_assert_eq!(string_repr, i.to_string());
                }
                RequestValue::Boolean(b) => {
                    prop_assert_eq!(string_repr, b.to_string());
                }
                RequestValue::Array(_) => {
                    // Array conversion is implementation-defined
                }
            }
        }

        /// Property 8.2: Request builder produces consistent state
        /// For any sequence of builder calls, the resulting request SHALL
        /// contain all specified values.
        #[test]
        fn prop_request_builder_consistency(
            template in template_id_strategy(),
            param_name in param_name_strategy(),
            param_value in param_value_strategy(),
            dry_run in any::<bool>()
        ) {
            let mut request = GenerateRequest::new(&template)
                .with_param(&param_name, param_value.clone());

            if dry_run {
                request = request.dry_run();
            }

            // Property: template should match
            prop_assert_eq!(request.template, template);

            // Property: parameter should be present
            prop_assert!(
                request.parameters.contains_key(&param_name),
                "Parameter {} should be present",
                param_name
            );

            // Property: dry_run should match
            prop_assert_eq!(request.dry_run, dry_run);
        }

        /// Property 8.3: Response success/error are mutually exclusive
        /// For any response, success and error states SHALL be mutually exclusive.
        #[test]
        fn prop_response_success_error_exclusive(
            is_success in any::<bool>(),
            message in "[a-zA-Z ]{1,50}".prop_map(|s| s.to_string())
        ) {
            let response = if is_success {
                GenerateResponse::success(Vec::new(), GenerationMetrics::default())
            } else {
                GenerateResponse::error(&message)
            };

            // Property: success and error are mutually exclusive
            if response.success {
                prop_assert!(
                    response.error.is_none(),
                    "Successful response should not have error"
                );
            } else {
                prop_assert!(
                    response.error.is_some(),
                    "Failed response should have error"
                );
            }
        }

        /// Property 8.4: JSON-RPC response has correct version
        /// For any JSON-RPC response, the version SHALL be "2.0".
        #[test]
        fn prop_jsonrpc_version(
            id in any::<i64>(),
            is_success in any::<bool>()
        ) {
            let rpc_id = JsonRpcId::Number(id);
            let response = if is_success {
                JsonRpcResponse::success(
                    rpc_id,
                    GenerateResponse::success(Vec::new(), GenerationMetrics::default()),
                )
            } else {
                JsonRpcResponse::error(rpc_id, INTERNAL_ERROR, "Error")
            };

            // Property: version should always be "2.0"
            prop_assert_eq!(response.jsonrpc, "2.0");
        }
    }
}

// ============================================================================
// Property-Based Tests for Default Value Application
// ============================================================================

#[cfg(test)]
mod default_value_tests {
    use super::*;
    use crate::registry::ParameterSchema;
    use proptest::prelude::*;

    /// Strategy for generating default values
    fn default_value_strategy() -> impl Strategy<Value = String> {
        "[a-zA-Z0-9_]{1,20}".prop_map(|s| s.to_string())
    }

    // **Feature: dx-generator-production, Property 12: Default Value Application**
    // **Validates: Requirements 4.3, 3.3**
    //
    // *For any* template with default values, missing parameters SHALL
    // be filled with the specified defaults.
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Property 12.1: Template defaults are applied
        /// For any parameter with a default value, if not provided in the request,
        /// the default SHALL be used.
        #[test]
        fn prop_template_defaults_applied(
            param_name in "[a-z][a-z0-9_]{0,15}".prop_map(|s| s.to_string()),
            default_value in default_value_strategy()
        ) {
            let inferrer = DefaultInferrer::new();
            let mut request = GenerateRequest::new("test");

            // Create template with default value
            let template = TemplateMetadata::new("test", "Test", "path")
                .with_parameter(
                    ParameterSchema::new(&param_name, "string")
                        .with_default(&default_value)
                );

            // Infer defaults
            inferrer.infer(&mut request, &template);

            // Property: parameter should be filled with default
            prop_assert!(
                request.parameters.contains_key(&param_name),
                "Parameter {} should be present after inference",
                param_name
            );

            let value = request.parameters.get(&param_name).unwrap();
            prop_assert_eq!(
                value.as_string(),
                default_value,
                "Parameter should have default value"
            );
        }

        /// Property 12.2: Explicit values override defaults
        /// For any parameter with both explicit value and default,
        /// the explicit value SHALL be used.
        #[test]
        fn prop_explicit_overrides_default(
            param_name in "[a-z][a-z0-9_]{0,15}".prop_map(|s| s.to_string()),
            explicit_value in "[a-zA-Z0-9_]{1,20}".prop_map(|s| format!("explicit_{}", s)),
            default_value in "[a-zA-Z0-9_]{1,20}".prop_map(|s| format!("default_{}", s))
        ) {
            // Ensure values are different
            prop_assume!(explicit_value != default_value);

            let inferrer = DefaultInferrer::new();
            let mut request = GenerateRequest::new("test")
                .with_param(&param_name, explicit_value.clone());

            // Create template with default value
            let template = TemplateMetadata::new("test", "Test", "path")
                .with_parameter(
                    ParameterSchema::new(&param_name, "string")
                        .with_default(&default_value)
                );

            // Infer defaults (should not override explicit)
            inferrer.infer(&mut request, &template);

            // Property: explicit value should be preserved
            let value = request.parameters.get(&param_name).unwrap();
            prop_assert_eq!(
                value.as_string(),
                explicit_value,
                "Explicit value should not be overridden"
            );
        }

        /// Property 12.3: Context-based inference for name parameter
        /// For the "name" parameter, if not provided and cwd is set,
        /// the directory name SHALL be used.
        #[test]
        fn prop_name_inferred_from_cwd(
            dir_name in "[a-z][a-z0-9-]{0,15}".prop_map(|s| s.to_string())
        ) {
            let inferrer = DefaultInferrer::new();
            let mut request = GenerateRequest::new("test");
            request.context.cwd = Some(PathBuf::from(format!("/projects/{}", dir_name)));

            // Create template with name parameter (no default)
            let template = TemplateMetadata::new("test", "Test", "path")
                .with_parameter(ParameterSchema::new("name", "string"));

            // Infer defaults
            inferrer.infer(&mut request, &template);

            // Property: name should be inferred from cwd
            prop_assert!(
                request.parameters.contains_key("name"),
                "Name parameter should be inferred"
            );

            let value = request.parameters.get("name").unwrap();
            prop_assert_eq!(
                value.as_string(),
                dir_name,
                "Name should match directory name"
            );
        }
    }
}
