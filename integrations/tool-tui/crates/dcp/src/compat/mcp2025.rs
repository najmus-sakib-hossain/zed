//! MCP 2025 Compatibility Layer
//!
//! Implements support for MCP specification features from 2025-03-26 and 2025-06-18:
//! - Protocol version negotiation
//! - Roots support
//! - Elicitation
//! - Resource templates
//! - Enhanced structured output
//! - List changed notifications
//! - Cancellation support
//! - Progress notifications
//! - Ping/pong keep-alive

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::{broadcast, oneshot, RwLock};

use super::json_rpc::RequestId;

/// Simple ID generator for elicitation requests
static ELICITATION_ID_COUNTER: AtomicU64 = AtomicU64::new(0);

fn generate_elicitation_id() -> String {
    let id = ELICITATION_ID_COUNTER.fetch_add(1, Ordering::SeqCst);
    format!("elicit-{}", id)
}

// ============================================================================
// Protocol Version Negotiation
// ============================================================================

/// Supported MCP protocol versions
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum ProtocolVersion {
    /// Original MCP specification (2024-11-05)
    #[default]
    V2024_11_05,
    /// Added roots support (2025-03-26)
    V2025_03_26,
    /// Added elicitation, structured output (2025-06-18)
    V2025_06_18,
}

impl ProtocolVersion {
    /// Parse from version string
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "2024-11-05" => Some(Self::V2024_11_05),
            "2025-03-26" => Some(Self::V2025_03_26),
            "2025-06-18" => Some(Self::V2025_06_18),
            _ => None,
        }
    }

    /// Get version string
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::V2024_11_05 => "2024-11-05",
            Self::V2025_03_26 => "2025-03-26",
            Self::V2025_06_18 => "2025-06-18",
        }
    }

    /// Check if roots feature is available
    pub fn supports_roots(&self) -> bool {
        *self >= Self::V2025_03_26
    }

    /// Check if elicitation feature is available
    pub fn supports_elicitation(&self) -> bool {
        *self >= Self::V2025_06_18
    }

    /// Check if progress notifications are available
    pub fn supports_progress(&self) -> bool {
        *self >= Self::V2025_03_26
    }

    /// Check if enhanced structured output is available
    pub fn supports_structured_output(&self) -> bool {
        *self >= Self::V2025_06_18
    }

    /// Get all supported versions
    pub fn all_versions() -> &'static [ProtocolVersion] {
        &[Self::V2024_11_05, Self::V2025_03_26, Self::V2025_06_18]
    }

    /// Get the latest supported version
    pub fn latest() -> Self {
        Self::V2025_06_18
    }
}

impl std::fmt::Display for ProtocolVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Protocol version negotiator
#[derive(Debug, Clone)]
pub struct VersionNegotiator {
    /// Latest supported version
    latest: ProtocolVersion,
}

impl VersionNegotiator {
    /// Create a new version negotiator
    pub fn new() -> Self {
        Self {
            latest: ProtocolVersion::latest(),
        }
    }

    /// Create with a specific latest version (for testing)
    pub fn with_latest(latest: ProtocolVersion) -> Self {
        Self { latest }
    }

    /// Negotiate version with client
    ///
    /// If the requested version is supported, returns it.
    /// Otherwise, returns the latest supported version.
    pub fn negotiate(&self, requested: &str) -> ProtocolVersion {
        ProtocolVersion::from_str(requested)
            .filter(|v| *v <= self.latest)
            .unwrap_or(self.latest)
    }

    /// Get the latest supported version
    pub fn latest(&self) -> ProtocolVersion {
        self.latest
    }
}

impl Default for VersionNegotiator {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Roots Support
// ============================================================================

/// Root definition - filesystem boundary for server operations
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Root {
    /// File URI (e.g., "file:///home/user/project")
    pub uri: String,
    /// Optional display name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

impl Root {
    /// Create a new root with just a URI
    pub fn new(uri: impl Into<String>) -> Self {
        Self {
            uri: uri.into(),
            name: None,
        }
    }

    /// Create a new root with URI and name
    pub fn with_name(uri: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            uri: uri.into(),
            name: Some(name.into()),
        }
    }
}

/// Roots registry - manages filesystem root boundaries
pub struct RootsRegistry {
    /// Configured roots
    roots: RwLock<Vec<Root>>,
    /// Change notification sender
    change_tx: broadcast::Sender<()>,
}

impl RootsRegistry {
    /// Create a new roots registry
    pub fn new() -> Self {
        let (change_tx, _) = broadcast::channel(16);
        Self {
            roots: RwLock::new(Vec::new()),
            change_tx,
        }
    }

    /// Add a root
    pub async fn add_root(&self, root: Root) {
        self.roots.write().await.push(root);
        let _ = self.change_tx.send(());
    }

    /// Remove a root by URI
    pub async fn remove_root(&self, uri: &str) -> bool {
        let mut roots = self.roots.write().await;
        let len_before = roots.len();
        roots.retain(|r| r.uri != uri);
        let removed = roots.len() < len_before;
        if removed {
            let _ = self.change_tx.send(());
        }
        removed
    }

    /// List all roots
    pub async fn list(&self) -> Vec<Root> {
        self.roots.read().await.clone()
    }

    /// Clear all roots
    pub async fn clear(&self) {
        let mut roots = self.roots.write().await;
        if !roots.is_empty() {
            roots.clear();
            let _ = self.change_tx.send(());
        }
    }

    /// Get the number of roots
    pub async fn len(&self) -> usize {
        self.roots.read().await.len()
    }

    /// Check if empty
    pub async fn is_empty(&self) -> bool {
        self.roots.read().await.is_empty()
    }

    /// Subscribe to change notifications
    pub fn subscribe(&self) -> broadcast::Receiver<()> {
        self.change_tx.subscribe()
    }

    /// Get the number of subscribers
    pub fn subscriber_count(&self) -> usize {
        self.change_tx.receiver_count()
    }
}

impl Default for RootsRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Elicitation Support
// ============================================================================

/// Elicitation request - server-initiated user input request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElicitationRequest {
    /// Message to display to user
    pub message: String,
    /// Optional JSON schema for structured input
    #[serde(rename = "requestedSchema", skip_serializing_if = "Option::is_none")]
    pub requested_schema: Option<ElicitationSchema>,
}

impl ElicitationRequest {
    /// Create a simple elicitation request with just a message
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            requested_schema: None,
        }
    }

    /// Create an elicitation request with a schema
    pub fn with_schema(message: impl Into<String>, schema: ElicitationSchema) -> Self {
        Self {
            message: message.into(),
            requested_schema: Some(schema),
        }
    }
}

/// Restricted JSON schema for elicitation (primitives only)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ElicitationSchema {
    /// Schema type (should be "object")
    #[serde(rename = "type")]
    pub schema_type: String,
    /// Property definitions
    #[serde(skip_serializing_if = "Option::is_none")]
    pub properties: Option<HashMap<String, PropertySchema>>,
    /// Required field names
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<Vec<String>>,
}

impl ElicitationSchema {
    /// Create a new object schema
    pub fn object() -> Self {
        Self {
            schema_type: "object".to_string(),
            properties: Some(HashMap::new()),
            required: None,
        }
    }

    /// Add a property to the schema
    pub fn with_property(mut self, name: impl Into<String>, schema: PropertySchema) -> Self {
        self.properties.get_or_insert_with(HashMap::new).insert(name.into(), schema);
        self
    }

    /// Add a required field
    pub fn with_required(mut self, name: impl Into<String>) -> Self {
        self.required.get_or_insert_with(Vec::new).push(name.into());
        self
    }
}

/// Property schema for elicitation (primitives only)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PropertySchema {
    /// Property type: string, number, boolean, or enum
    #[serde(rename = "type")]
    pub prop_type: String,
    /// Description of the property
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Format hint (email, uri, date, date-time)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
    /// Minimum value for numbers
    #[serde(skip_serializing_if = "Option::is_none")]
    pub minimum: Option<f64>,
    /// Maximum value for numbers
    #[serde(skip_serializing_if = "Option::is_none")]
    pub maximum: Option<f64>,
    /// Enum values for enum type
    #[serde(rename = "enum", skip_serializing_if = "Option::is_none")]
    pub enum_values: Option<Vec<String>>,
}

impl PropertySchema {
    /// Create a string property
    pub fn string() -> Self {
        Self {
            prop_type: "string".to_string(),
            ..Default::default()
        }
    }

    /// Create a number property
    pub fn number() -> Self {
        Self {
            prop_type: "number".to_string(),
            ..Default::default()
        }
    }

    /// Create a boolean property
    pub fn boolean() -> Self {
        Self {
            prop_type: "boolean".to_string(),
            ..Default::default()
        }
    }

    /// Create an enum property
    pub fn enumeration(values: Vec<String>) -> Self {
        Self {
            prop_type: "string".to_string(),
            enum_values: Some(values),
            ..Default::default()
        }
    }

    /// Add a description
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// Add a format hint
    pub fn with_format(mut self, format: impl Into<String>) -> Self {
        self.format = Some(format.into());
        self
    }

    /// Add minimum value
    pub fn with_minimum(mut self, min: f64) -> Self {
        self.minimum = Some(min);
        self
    }

    /// Add maximum value
    pub fn with_maximum(mut self, max: f64) -> Self {
        self.maximum = Some(max);
        self
    }
}

/// Elicitation response action
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ElicitationAction {
    /// User accepted and provided input
    Accept,
    /// User declined to provide input
    Decline,
    /// User cancelled the request
    Cancel,
}

/// Elicitation response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ElicitationResponse {
    /// The action taken by the user
    pub action: ElicitationAction,
    /// Content provided (only present when action is Accept)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<Value>,
}

impl ElicitationResponse {
    /// Create an accept response with content
    pub fn accept(content: Value) -> Self {
        Self {
            action: ElicitationAction::Accept,
            content: Some(content),
        }
    }

    /// Create a decline response
    pub fn decline() -> Self {
        Self {
            action: ElicitationAction::Decline,
            content: None,
        }
    }

    /// Create a cancel response
    pub fn cancel() -> Self {
        Self {
            action: ElicitationAction::Cancel,
            content: None,
        }
    }
}

/// Elicitation errors
#[derive(Debug, Clone, thiserror::Error)]
pub enum ElicitationError {
    #[error("elicitation cancelled")]
    Cancelled,
    #[error("elicitation timeout")]
    Timeout,
    #[error("validation failed: {0}")]
    ValidationFailed(String),
}

/// Elicitation handler - manages server-initiated user input requests
pub struct ElicitationHandler {
    /// Pending elicitation requests
    pending: RwLock<HashMap<String, oneshot::Sender<ElicitationResponse>>>,
    /// Timeout duration in seconds
    timeout_secs: u64,
}

impl ElicitationHandler {
    /// Create a new elicitation handler with default timeout (300 seconds)
    pub fn new() -> Self {
        Self {
            pending: RwLock::new(HashMap::new()),
            timeout_secs: 300,
        }
    }

    /// Create with custom timeout
    pub fn with_timeout(timeout_secs: u64) -> Self {
        Self {
            pending: RwLock::new(HashMap::new()),
            timeout_secs,
        }
    }

    /// Create an elicitation request and wait for response
    pub async fn create(
        &self,
        request: ElicitationRequest,
    ) -> Result<ElicitationResponse, ElicitationError> {
        let id = generate_elicitation_id();
        let (tx, rx) = oneshot::channel();

        self.pending.write().await.insert(id.clone(), tx);

        // Wait for response with timeout
        let result =
            tokio::time::timeout(std::time::Duration::from_secs(self.timeout_secs), rx).await;

        // Clean up pending request
        self.pending.write().await.remove(&id);

        match result {
            Ok(Ok(response)) => {
                // Validate response against schema if provided
                if let (Some(schema), Some(content)) =
                    (&request.requested_schema, &response.content)
                {
                    self.validate_response(schema, content)?;
                }
                Ok(response)
            }
            Ok(Err(_)) => Err(ElicitationError::Cancelled),
            Err(_) => Err(ElicitationError::Timeout),
        }
    }

    /// Respond to a pending elicitation request
    pub async fn respond(&self, id: &str, response: ElicitationResponse) -> bool {
        if let Some(tx) = self.pending.write().await.remove(id) {
            tx.send(response).is_ok()
        } else {
            false
        }
    }

    /// Validate response against schema
    fn validate_response(
        &self,
        schema: &ElicitationSchema,
        content: &Value,
    ) -> Result<(), ElicitationError> {
        // Validate required fields
        if let Some(required) = &schema.required {
            for field in required {
                if content.get(field).is_none() {
                    return Err(ElicitationError::ValidationFailed(format!(
                        "missing required field: {}",
                        field
                    )));
                }
            }
        }

        // Validate property types
        if let (Some(properties), Some(obj)) = (&schema.properties, content.as_object()) {
            for (name, prop_schema) in properties {
                if let Some(value) = obj.get(name) {
                    self.validate_property_type(name, prop_schema, value)?;
                }
            }
        }

        Ok(())
    }

    /// Validate a single property value against its schema
    fn validate_property_type(
        &self,
        name: &str,
        schema: &PropertySchema,
        value: &Value,
    ) -> Result<(), ElicitationError> {
        match schema.prop_type.as_str() {
            "string" => {
                if !value.is_string() {
                    return Err(ElicitationError::ValidationFailed(format!(
                        "field '{}' must be a string",
                        name
                    )));
                }
                // Check enum values if specified
                if let (Some(enum_values), Some(s)) = (&schema.enum_values, value.as_str()) {
                    if !enum_values.contains(&s.to_string()) {
                        return Err(ElicitationError::ValidationFailed(format!(
                            "field '{}' must be one of: {:?}",
                            name, enum_values
                        )));
                    }
                }
            }
            "number" => {
                if let Some(n) = value.as_f64() {
                    if let Some(min) = schema.minimum {
                        if n < min {
                            return Err(ElicitationError::ValidationFailed(format!(
                                "field '{}' must be >= {}",
                                name, min
                            )));
                        }
                    }
                    if let Some(max) = schema.maximum {
                        if n > max {
                            return Err(ElicitationError::ValidationFailed(format!(
                                "field '{}' must be <= {}",
                                name, max
                            )));
                        }
                    }
                } else {
                    return Err(ElicitationError::ValidationFailed(format!(
                        "field '{}' must be a number",
                        name
                    )));
                }
            }
            "boolean" => {
                if !value.is_boolean() {
                    return Err(ElicitationError::ValidationFailed(format!(
                        "field '{}' must be a boolean",
                        name
                    )));
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// Get the number of pending requests
    pub async fn pending_count(&self) -> usize {
        self.pending.read().await.len()
    }
}

impl Default for ElicitationHandler {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Resource Templates
// ============================================================================

/// Resource template definition
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ResourceTemplate {
    /// URI template with placeholders (e.g., "file:///{path}")
    #[serde(rename = "uriTemplate")]
    pub uri_template: String,
    /// Display name
    pub name: String,
    /// Description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// MIME type
    #[serde(rename = "mimeType", skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
}

impl ResourceTemplate {
    /// Create a new resource template
    pub fn new(uri_template: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            uri_template: uri_template.into(),
            name: name.into(),
            description: None,
            mime_type: None,
        }
    }

    /// Add a description
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// Add a MIME type
    pub fn with_mime_type(mut self, mime: impl Into<String>) -> Self {
        self.mime_type = Some(mime.into());
        self
    }

    /// Extract placeholder names from the template
    pub fn placeholders(&self) -> Vec<String> {
        let mut placeholders = Vec::new();
        let mut chars = self.uri_template.chars().peekable();

        while let Some(c) = chars.next() {
            if c == '{' {
                let mut name = String::new();
                while let Some(&next) = chars.peek() {
                    if next == '}' {
                        chars.next();
                        break;
                    }
                    name.push(chars.next().unwrap());
                }
                if !name.is_empty() {
                    placeholders.push(name);
                }
            }
        }

        placeholders
    }
}

/// Template parameter extracted from a URI
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TemplateParam {
    /// Parameter name
    pub name: String,
    /// Parameter value
    pub value: String,
}

/// Resource template registry
pub struct ResourceTemplateRegistry {
    /// Registered templates
    templates: RwLock<Vec<ResourceTemplate>>,
}

impl ResourceTemplateRegistry {
    /// Create a new template registry
    pub fn new() -> Self {
        Self {
            templates: RwLock::new(Vec::new()),
        }
    }

    /// Register a template
    pub async fn register(&self, template: ResourceTemplate) {
        self.templates.write().await.push(template);
    }

    /// Unregister a template by URI template
    pub async fn unregister(&self, uri_template: &str) -> bool {
        let mut templates = self.templates.write().await;
        let len_before = templates.len();
        templates.retain(|t| t.uri_template != uri_template);
        templates.len() < len_before
    }

    /// List all templates
    pub async fn list(&self) -> Vec<ResourceTemplate> {
        self.templates.read().await.clone()
    }

    /// Match a URI to a template and extract parameters
    pub async fn match_uri(&self, uri: &str) -> Option<(ResourceTemplate, Vec<TemplateParam>)> {
        let templates = self.templates.read().await;
        for template in templates.iter() {
            if let Some(params) = Self::extract_params(uri, &template.uri_template) {
                return Some((template.clone(), params));
            }
        }
        None
    }

    /// Extract parameters from a URI using a template
    fn extract_params(uri: &str, template: &str) -> Option<Vec<TemplateParam>> {
        let mut params = Vec::new();
        let mut uri_chars = uri.chars().peekable();
        let mut template_chars = template.chars().peekable();

        while let Some(tc) = template_chars.next() {
            if tc == '{' {
                // Extract placeholder name
                let mut name = String::new();
                while let Some(&next) = template_chars.peek() {
                    if next == '}' {
                        template_chars.next();
                        break;
                    }
                    name.push(template_chars.next().unwrap());
                }

                // Find the next literal character in template (or end)
                let next_literal = template_chars.peek().copied();

                // Extract value from URI until we hit the next literal or end
                let mut value = String::new();
                while let Some(&uc) = uri_chars.peek() {
                    if Some(uc) == next_literal {
                        break;
                    }
                    value.push(uri_chars.next().unwrap());
                }

                if !name.is_empty() {
                    params.push(TemplateParam { name, value });
                }
            } else {
                // Literal character - must match
                match uri_chars.next() {
                    Some(uc) if uc == tc => continue,
                    _ => return None,
                }
            }
        }

        // Both should be exhausted
        if uri_chars.next().is_some() {
            return None;
        }

        Some(params)
    }

    /// Get the number of registered templates
    pub async fn len(&self) -> usize {
        self.templates.read().await.len()
    }

    /// Check if empty
    pub async fn is_empty(&self) -> bool {
        self.templates.read().await.is_empty()
    }
}

impl Default for ResourceTemplateRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Enhanced Structured Output
// ============================================================================

/// Content annotations for structured output
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct Annotations {
    /// Target audience roles
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audience: Option<Vec<String>>,
    /// Priority (0.0 to 1.0)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<f64>,
}

impl Annotations {
    /// Create empty annotations
    pub fn new() -> Self {
        Self::default()
    }

    /// Set audience
    pub fn with_audience(mut self, audience: Vec<String>) -> Self {
        self.audience = Some(audience);
        self
    }

    /// Set priority
    pub fn with_priority(mut self, priority: f64) -> Self {
        self.priority = Some(priority.clamp(0.0, 1.0));
        self
    }
}

/// Content type for MCP messages
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum Content {
    /// Text content
    #[serde(rename = "text")]
    Text { text: String },
    /// Image content (base64 encoded)
    #[serde(rename = "image")]
    Image {
        data: String,
        #[serde(rename = "mimeType")]
        mime_type: String,
    },
    /// Resource reference
    #[serde(rename = "resource")]
    Resource { uri: String },
}

impl Content {
    /// Create text content
    pub fn text(text: impl Into<String>) -> Self {
        Self::Text { text: text.into() }
    }

    /// Create image content
    pub fn image(data: impl Into<String>, mime_type: impl Into<String>) -> Self {
        Self::Image {
            data: data.into(),
            mime_type: mime_type.into(),
        }
    }

    /// Create resource reference
    pub fn resource(uri: impl Into<String>) -> Self {
        Self::Resource { uri: uri.into() }
    }
}

/// Content with optional annotations
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AnnotatedContent {
    /// The content
    #[serde(flatten)]
    pub content: Content,
    /// Optional annotations
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<Annotations>,
}

impl AnnotatedContent {
    /// Create annotated content
    pub fn new(content: Content) -> Self {
        Self {
            content,
            annotations: None,
        }
    }

    /// Add annotations
    pub fn with_annotations(mut self, annotations: Annotations) -> Self {
        self.annotations = Some(annotations);
        self
    }
}

/// Enhanced tool result with isError support
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnhancedToolResult {
    /// Content items
    pub content: Vec<AnnotatedContent>,
    /// Whether this is an error result
    #[serde(rename = "isError", skip_serializing_if = "Option::is_none")]
    pub is_error: Option<bool>,
}

impl EnhancedToolResult {
    /// Create a success result
    pub fn success(content: Vec<AnnotatedContent>) -> Self {
        Self {
            content,
            is_error: None,
        }
    }

    /// Create an error result
    pub fn error(message: impl Into<String>) -> Self {
        Self {
            content: vec![AnnotatedContent {
                content: Content::text(message),
                annotations: Some(Annotations::new().with_priority(1.0)),
            }],
            is_error: Some(true),
        }
    }

    /// Create from a single text content
    pub fn text(text: impl Into<String>) -> Self {
        Self::success(vec![AnnotatedContent::new(Content::text(text))])
    }
}

// ============================================================================
// Notification Manager
// ============================================================================

/// Progress notification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressNotification {
    /// Progress token from request _meta
    #[serde(rename = "progressToken")]
    pub progress_token: String,
    /// Progress value (0.0 to 1.0)
    pub progress: f64,
    /// Optional total for absolute progress
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total: Option<u64>,
}

/// Cancellation notification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CancellationNotification {
    /// Request ID to cancel
    #[serde(rename = "requestId")]
    pub request_id: RequestId,
    /// Optional reason
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// Notification types
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "method", content = "params")]
pub enum Notification {
    /// Tools list changed
    #[serde(rename = "notifications/tools/list_changed")]
    ToolsListChanged,
    /// Resources list changed
    #[serde(rename = "notifications/resources/list_changed")]
    ResourcesListChanged,
    /// Prompts list changed
    #[serde(rename = "notifications/prompts/list_changed")]
    PromptsListChanged,
    /// Roots list changed
    #[serde(rename = "notifications/roots/list_changed")]
    RootsListChanged,
    /// Progress update
    #[serde(rename = "notifications/progress")]
    Progress(ProgressNotification),
    /// Request cancelled
    #[serde(rename = "notifications/cancelled")]
    Cancelled(CancellationNotification),
}

impl Notification {
    /// Format notification as JSON-RPC message
    pub fn to_json_rpc(&self) -> String {
        let json = match self {
            Notification::ToolsListChanged => serde_json::json!({
                "jsonrpc": "2.0",
                "method": "notifications/tools/list_changed"
            }),
            Notification::ResourcesListChanged => serde_json::json!({
                "jsonrpc": "2.0",
                "method": "notifications/resources/list_changed"
            }),
            Notification::PromptsListChanged => serde_json::json!({
                "jsonrpc": "2.0",
                "method": "notifications/prompts/list_changed"
            }),
            Notification::RootsListChanged => serde_json::json!({
                "jsonrpc": "2.0",
                "method": "notifications/roots/list_changed"
            }),
            Notification::Progress(p) => serde_json::json!({
                "jsonrpc": "2.0",
                "method": "notifications/progress",
                "params": p
            }),
            Notification::Cancelled(c) => serde_json::json!({
                "jsonrpc": "2.0",
                "method": "notifications/cancelled",
                "params": c
            }),
        };
        serde_json::to_string(&json).unwrap_or_default()
    }
}

/// Notification manager - centralized notification handling
pub struct NotificationManager {
    /// Broadcast channel for notifications
    tx: broadcast::Sender<Notification>,
}

impl NotificationManager {
    /// Create a new notification manager
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(256);
        Self { tx }
    }

    /// Send a notification
    pub fn notify(&self, notification: Notification) {
        let _ = self.tx.send(notification);
    }

    /// Subscribe to notifications
    pub fn subscribe(&self) -> broadcast::Receiver<Notification> {
        self.tx.subscribe()
    }

    /// Get the number of subscribers
    pub fn subscriber_count(&self) -> usize {
        self.tx.receiver_count()
    }

    /// Notify tools list changed
    pub fn notify_tools_changed(&self) {
        self.notify(Notification::ToolsListChanged);
    }

    /// Notify resources list changed
    pub fn notify_resources_changed(&self) {
        self.notify(Notification::ResourcesListChanged);
    }

    /// Notify prompts list changed
    pub fn notify_prompts_changed(&self) {
        self.notify(Notification::PromptsListChanged);
    }

    /// Notify roots list changed
    pub fn notify_roots_changed(&self) {
        self.notify(Notification::RootsListChanged);
    }

    /// Send progress notification
    pub fn notify_progress(&self, token: impl Into<String>, progress: f64, total: Option<u64>) {
        self.notify(Notification::Progress(ProgressNotification {
            progress_token: token.into(),
            progress: progress.clamp(0.0, 1.0),
            total,
        }));
    }

    /// Send cancellation notification
    pub fn notify_cancelled(&self, request_id: RequestId, reason: Option<String>) {
        self.notify(Notification::Cancelled(CancellationNotification { request_id, reason }));
    }
}

impl Default for NotificationManager {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Resource Subscription Tracking
// ============================================================================

/// Resource subscription tracker for unsubscribe support
pub struct SubscriptionTracker {
    /// Active subscriptions: URI -> set of subscriber IDs
    subscriptions: RwLock<HashMap<String, Vec<String>>>,
}

impl SubscriptionTracker {
    /// Create a new subscription tracker
    pub fn new() -> Self {
        Self {
            subscriptions: RwLock::new(HashMap::new()),
        }
    }

    /// Subscribe to a resource
    pub async fn subscribe(&self, uri: &str, subscriber_id: &str) {
        let mut subs = self.subscriptions.write().await;
        subs.entry(uri.to_string())
            .or_insert_with(Vec::new)
            .push(subscriber_id.to_string());
    }

    /// Unsubscribe from a resource (idempotent)
    pub async fn unsubscribe(&self, uri: &str, subscriber_id: &str) -> bool {
        let mut subs = self.subscriptions.write().await;
        let should_remove;
        let removed;

        if let Some(subscribers) = subs.get_mut(uri) {
            let len_before = subscribers.len();
            subscribers.retain(|id| id != subscriber_id);
            removed = subscribers.len() < len_before;
            should_remove = subscribers.is_empty();
        } else {
            // Return true even if not found (idempotent)
            return true;
        }

        if should_remove {
            subs.remove(uri);
        }
        removed
    }

    /// Check if a subscriber is subscribed to a resource
    pub async fn is_subscribed(&self, uri: &str, subscriber_id: &str) -> bool {
        let subs = self.subscriptions.read().await;
        subs.get(uri).map(|s| s.contains(&subscriber_id.to_string())).unwrap_or(false)
    }

    /// Get all subscribers for a resource
    pub async fn get_subscribers(&self, uri: &str) -> Vec<String> {
        let subs = self.subscriptions.read().await;
        subs.get(uri).cloned().unwrap_or_default()
    }

    /// Get the number of subscriptions for a resource
    pub async fn subscription_count(&self, uri: &str) -> usize {
        let subs = self.subscriptions.read().await;
        subs.get(uri).map(|s| s.len()).unwrap_or(0)
    }

    /// Clear all subscriptions
    pub async fn clear(&self) {
        self.subscriptions.write().await.clear();
    }
}

impl Default for SubscriptionTracker {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Cancellation Support
// ============================================================================

/// Cancellation state for a request
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CancellationState {
    /// Request is active
    Active,
    /// Request was cancelled
    Cancelled,
    /// Request completed before cancellation
    Completed,
}

/// Cancellation token for tracking request cancellation
#[derive(Debug)]
pub struct CancellationToken {
    /// Request ID
    request_id: RequestId,
    /// Cancellation state
    state: std::sync::atomic::AtomicU8,
    /// Optional reason for cancellation
    reason: RwLock<Option<String>>,
}

impl CancellationToken {
    /// Create a new cancellation token
    pub fn new(request_id: RequestId) -> Self {
        Self {
            request_id,
            state: std::sync::atomic::AtomicU8::new(0), // Active
            reason: RwLock::new(None),
        }
    }

    /// Get the request ID
    pub fn request_id(&self) -> &RequestId {
        &self.request_id
    }

    /// Check if cancelled
    pub fn is_cancelled(&self) -> bool {
        self.state.load(Ordering::SeqCst) == 1
    }

    /// Check if completed
    pub fn is_completed(&self) -> bool {
        self.state.load(Ordering::SeqCst) == 2
    }

    /// Get the current state
    pub fn state(&self) -> CancellationState {
        match self.state.load(Ordering::SeqCst) {
            1 => CancellationState::Cancelled,
            2 => CancellationState::Completed,
            _ => CancellationState::Active,
        }
    }

    /// Cancel the request
    pub async fn cancel(&self, reason: Option<String>) -> bool {
        // Only cancel if still active
        let result = self.state.compare_exchange(
            0, // Active
            1, // Cancelled
            Ordering::SeqCst,
            Ordering::SeqCst,
        );
        if result.is_ok() {
            *self.reason.write().await = reason;
            true
        } else {
            false
        }
    }

    /// Mark the request as completed
    pub fn complete(&self) -> bool {
        // Only complete if still active
        self.state
            .compare_exchange(
                0, // Active
                2, // Completed
                Ordering::SeqCst,
                Ordering::SeqCst,
            )
            .is_ok()
    }

    /// Get the cancellation reason
    pub async fn reason(&self) -> Option<String> {
        self.reason.read().await.clone()
    }
}

/// Cancellation manager - tracks cancellation state for requests
pub struct CancellationManager {
    /// Active tokens: request ID -> token
    tokens: RwLock<HashMap<String, Arc<CancellationToken>>>,
}

impl CancellationManager {
    /// Create a new cancellation manager
    pub fn new() -> Self {
        Self {
            tokens: RwLock::new(HashMap::new()),
        }
    }

    /// Create a cancellation token for a request
    pub async fn create_token(&self, request_id: RequestId) -> Arc<CancellationToken> {
        let token = Arc::new(CancellationToken::new(request_id.clone()));
        let key = Self::request_id_to_key(&request_id);
        self.tokens.write().await.insert(key, Arc::clone(&token));
        token
    }

    /// Get a token by request ID
    pub async fn get_token(&self, request_id: &RequestId) -> Option<Arc<CancellationToken>> {
        let key = Self::request_id_to_key(request_id);
        self.tokens.read().await.get(&key).cloned()
    }

    /// Cancel a request by ID
    pub async fn cancel(&self, request_id: &RequestId, reason: Option<String>) -> bool {
        if let Some(token) = self.get_token(request_id).await {
            token.cancel(reason).await
        } else {
            // Request not found - might have already completed
            false
        }
    }

    /// Remove a token (called when request completes)
    pub async fn remove_token(&self, request_id: &RequestId) {
        let key = Self::request_id_to_key(request_id);
        self.tokens.write().await.remove(&key);
    }

    /// Check if a request is cancelled
    pub async fn is_cancelled(&self, request_id: &RequestId) -> bool {
        if let Some(token) = self.get_token(request_id).await {
            token.is_cancelled()
        } else {
            false
        }
    }

    /// Get the number of active tokens
    pub async fn active_count(&self) -> usize {
        self.tokens.read().await.len()
    }

    fn request_id_to_key(request_id: &RequestId) -> String {
        match request_id {
            RequestId::Number(n) => format!("n:{}", n),
            RequestId::String(s) => format!("s:{}", s),
            RequestId::Null => "null".to_string(),
        }
    }
}

impl Default for CancellationManager {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Progress Tracking
// ============================================================================

/// Progress state for a request
#[derive(Debug, Clone)]
pub struct ProgressState {
    /// Progress token
    pub token: String,
    /// Current progress (0.0 to 1.0)
    pub progress: f64,
    /// Optional total for absolute progress
    pub total: Option<u64>,
    /// Whether progress tracking is complete
    pub completed: bool,
}

/// Progress tracker - tracks progress for requests with progressToken
pub struct ProgressTracker {
    /// Active progress states: token -> state
    states: RwLock<HashMap<String, ProgressState>>,
    /// Notification sender
    notification_tx: broadcast::Sender<ProgressNotification>,
}

impl ProgressTracker {
    /// Create a new progress tracker
    pub fn new() -> Self {
        let (notification_tx, _) = broadcast::channel(256);
        Self {
            states: RwLock::new(HashMap::new()),
            notification_tx,
        }
    }

    /// Start tracking progress for a token
    pub async fn start(&self, token: impl Into<String>, total: Option<u64>) {
        let token = token.into();
        self.states.write().await.insert(
            token.clone(),
            ProgressState {
                token,
                progress: 0.0,
                total,
                completed: false,
            },
        );
    }

    /// Update progress for a token
    pub async fn update(&self, token: &str, progress: f64) -> bool {
        let mut states = self.states.write().await;
        if let Some(state) = states.get_mut(token) {
            if !state.completed {
                state.progress = progress.clamp(0.0, 1.0);
                let _ = self.notification_tx.send(ProgressNotification {
                    progress_token: token.to_string(),
                    progress: state.progress,
                    total: state.total,
                });
                return true;
            }
        }
        false
    }

    /// Complete progress tracking for a token
    pub async fn complete(&self, token: &str) -> bool {
        let mut states = self.states.write().await;
        if let Some(state) = states.get_mut(token) {
            if !state.completed {
                state.progress = 1.0;
                state.completed = true;
                let _ = self.notification_tx.send(ProgressNotification {
                    progress_token: token.to_string(),
                    progress: 1.0,
                    total: state.total,
                });
                return true;
            }
        }
        false
    }

    /// Get progress state for a token
    pub async fn get(&self, token: &str) -> Option<ProgressState> {
        self.states.read().await.get(token).cloned()
    }

    /// Remove progress tracking for a token
    pub async fn remove(&self, token: &str) {
        self.states.write().await.remove(token);
    }

    /// Subscribe to progress notifications
    pub fn subscribe(&self) -> broadcast::Receiver<ProgressNotification> {
        self.notification_tx.subscribe()
    }

    /// Check if a token is being tracked
    pub async fn is_tracking(&self, token: &str) -> bool {
        self.states.read().await.contains_key(token)
    }

    /// Get the number of active progress trackers
    pub async fn active_count(&self) -> usize {
        self.states.read().await.len()
    }
}

impl Default for ProgressTracker {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Extended MCP Adapter
// ============================================================================

/// Configuration for ping timeout
#[derive(Debug, Clone)]
pub struct PingConfig {
    /// Timeout in seconds for ping response
    pub timeout_secs: u64,
}

impl Default for PingConfig {
    fn default() -> Self {
        Self { timeout_secs: 30 }
    }
}

/// Extended MCP adapter integrating all MCP 2025 features.
///
/// This adapter composes all the new feature handlers and routes methods
/// based on the negotiated protocol version.
pub struct ExtendedMcpAdapter {
    /// Protocol version negotiator
    version_negotiator: VersionNegotiator,
    /// Negotiated protocol version
    negotiated_version: RwLock<ProtocolVersion>,
    /// Roots registry
    roots: Arc<RootsRegistry>,
    /// Elicitation handler
    elicitation: Arc<ElicitationHandler>,
    /// Resource template registry
    resource_templates: Arc<ResourceTemplateRegistry>,
    /// Subscription tracker
    subscriptions: Arc<SubscriptionTracker>,
    /// Notification manager
    notifications: Arc<NotificationManager>,
    /// Cancellation manager
    cancellation: Arc<CancellationManager>,
    /// Progress tracker
    progress: Arc<ProgressTracker>,
    /// Ping configuration
    ping_config: PingConfig,
}

impl ExtendedMcpAdapter {
    /// Create a new extended adapter with default configuration
    pub fn new() -> Self {
        Self {
            version_negotiator: VersionNegotiator::new(),
            negotiated_version: RwLock::new(ProtocolVersion::default()),
            roots: Arc::new(RootsRegistry::new()),
            elicitation: Arc::new(ElicitationHandler::new()),
            resource_templates: Arc::new(ResourceTemplateRegistry::new()),
            subscriptions: Arc::new(SubscriptionTracker::new()),
            notifications: Arc::new(NotificationManager::new()),
            cancellation: Arc::new(CancellationManager::new()),
            progress: Arc::new(ProgressTracker::new()),
            ping_config: PingConfig::default(),
        }
    }

    /// Create with custom ping timeout
    pub fn with_ping_timeout(mut self, timeout_secs: u64) -> Self {
        self.ping_config.timeout_secs = timeout_secs;
        self
    }

    /// Create with custom elicitation timeout
    pub fn with_elicitation_timeout(mut self, timeout_secs: u64) -> Self {
        self.elicitation = Arc::new(ElicitationHandler::with_timeout(timeout_secs));
        self
    }

    // ========================================================================
    // Accessors
    // ========================================================================

    /// Get the version negotiator
    pub fn version_negotiator(&self) -> &VersionNegotiator {
        &self.version_negotiator
    }

    /// Get the negotiated protocol version
    pub async fn negotiated_version(&self) -> ProtocolVersion {
        *self.negotiated_version.read().await
    }

    /// Set the negotiated protocol version
    pub async fn set_negotiated_version(&self, version: ProtocolVersion) {
        *self.negotiated_version.write().await = version;
    }

    /// Get the roots registry
    pub fn roots(&self) -> Arc<RootsRegistry> {
        Arc::clone(&self.roots)
    }

    /// Get the elicitation handler
    pub fn elicitation(&self) -> Arc<ElicitationHandler> {
        Arc::clone(&self.elicitation)
    }

    /// Get the resource template registry
    pub fn resource_templates(&self) -> Arc<ResourceTemplateRegistry> {
        Arc::clone(&self.resource_templates)
    }

    /// Get the subscription tracker
    pub fn subscriptions(&self) -> Arc<SubscriptionTracker> {
        Arc::clone(&self.subscriptions)
    }

    /// Get the notification manager
    pub fn notifications(&self) -> Arc<NotificationManager> {
        Arc::clone(&self.notifications)
    }

    /// Get the cancellation manager
    pub fn cancellation(&self) -> Arc<CancellationManager> {
        Arc::clone(&self.cancellation)
    }

    /// Get the progress tracker
    pub fn progress(&self) -> Arc<ProgressTracker> {
        Arc::clone(&self.progress)
    }

    /// Get the ping configuration
    pub fn ping_config(&self) -> &PingConfig {
        &self.ping_config
    }

    // ========================================================================
    // Version Negotiation
    // ========================================================================

    /// Negotiate protocol version from client request
    pub async fn negotiate_version(&self, requested: &str) -> ProtocolVersion {
        let version = self.version_negotiator.negotiate(requested);
        *self.negotiated_version.write().await = version;
        version
    }

    /// Check if a feature is available in the negotiated version
    pub async fn supports_roots(&self) -> bool {
        self.negotiated_version.read().await.supports_roots()
    }

    /// Check if elicitation is available
    pub async fn supports_elicitation(&self) -> bool {
        self.negotiated_version.read().await.supports_elicitation()
    }

    /// Check if progress notifications are available
    pub async fn supports_progress(&self) -> bool {
        self.negotiated_version.read().await.supports_progress()
    }

    /// Check if structured output is available
    pub async fn supports_structured_output(&self) -> bool {
        self.negotiated_version.read().await.supports_structured_output()
    }

    // ========================================================================
    // Capability Building
    // ========================================================================

    /// Build capabilities object based on negotiated version
    pub async fn build_capabilities(&self) -> serde_json::Value {
        let version = *self.negotiated_version.read().await;

        let mut capabilities = serde_json::json!({
            "tools": { "listChanged": true },
            "resources": { "subscribe": true, "listChanged": true },
            "prompts": { "listChanged": true },
            "logging": {}
        });

        if version.supports_roots() {
            capabilities["roots"] = serde_json::json!({ "listChanged": true });
        }

        if version.supports_elicitation() {
            capabilities["elicitation"] = serde_json::json!({});
        }

        capabilities
    }

    // ========================================================================
    // Request Handling Helpers
    // ========================================================================

    /// Check if a method is available in the current version
    pub async fn is_method_available(&self, method: &str) -> bool {
        let version = *self.negotiated_version.read().await;

        match method {
            "roots/list" => version.supports_roots(),
            "elicitation/create" => version.supports_elicitation(),
            // All other methods are available in all versions
            _ => true,
        }
    }

    /// Create a cancellation token for a request
    pub async fn create_cancellation_token(&self, request_id: RequestId) -> Arc<CancellationToken> {
        self.cancellation.create_token(request_id).await
    }

    /// Start progress tracking for a request
    pub async fn start_progress(&self, token: &str, total: Option<u64>) {
        self.progress.start(token, total).await;
    }

    /// Update progress for a request
    pub async fn update_progress(&self, token: &str, progress: f64) -> bool {
        self.progress.update(token, progress).await
    }

    /// Complete progress tracking for a request
    pub async fn complete_progress(&self, token: &str) -> bool {
        self.progress.complete(token).await
    }

    // ========================================================================
    // Notification Helpers
    // ========================================================================

    /// Notify that tools list has changed
    pub fn notify_tools_changed(&self) {
        self.notifications.notify_tools_changed();
    }

    /// Notify that resources list has changed
    pub fn notify_resources_changed(&self) {
        self.notifications.notify_resources_changed();
    }

    /// Notify that prompts list has changed
    pub fn notify_prompts_changed(&self) {
        self.notifications.notify_prompts_changed();
    }

    /// Notify that roots list has changed
    pub fn notify_roots_changed(&self) {
        self.notifications.notify_roots_changed();
    }

    /// Subscribe to all notifications
    pub fn subscribe_notifications(&self) -> broadcast::Receiver<Notification> {
        self.notifications.subscribe()
    }
}

impl Default for ExtendedMcpAdapter {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // Protocol Version Tests

    #[test]
    fn test_protocol_version_parsing() {
        assert_eq!(ProtocolVersion::from_str("2024-11-05"), Some(ProtocolVersion::V2024_11_05));
        assert_eq!(ProtocolVersion::from_str("2025-03-26"), Some(ProtocolVersion::V2025_03_26));
        assert_eq!(ProtocolVersion::from_str("2025-06-18"), Some(ProtocolVersion::V2025_06_18));
        assert_eq!(ProtocolVersion::from_str("invalid"), None);
    }

    #[test]
    fn test_protocol_version_as_str() {
        assert_eq!(ProtocolVersion::V2024_11_05.as_str(), "2024-11-05");
        assert_eq!(ProtocolVersion::V2025_03_26.as_str(), "2025-03-26");
        assert_eq!(ProtocolVersion::V2025_06_18.as_str(), "2025-06-18");
    }

    #[test]
    fn test_protocol_version_ordering() {
        assert!(ProtocolVersion::V2024_11_05 < ProtocolVersion::V2025_03_26);
        assert!(ProtocolVersion::V2025_03_26 < ProtocolVersion::V2025_06_18);
    }

    #[test]
    fn test_protocol_version_features() {
        let v1 = ProtocolVersion::V2024_11_05;
        assert!(!v1.supports_roots());
        assert!(!v1.supports_elicitation());
        assert!(!v1.supports_progress());

        let v2 = ProtocolVersion::V2025_03_26;
        assert!(v2.supports_roots());
        assert!(!v2.supports_elicitation());
        assert!(v2.supports_progress());

        let v3 = ProtocolVersion::V2025_06_18;
        assert!(v3.supports_roots());
        assert!(v3.supports_elicitation());
        assert!(v3.supports_progress());
    }

    #[test]
    fn test_version_negotiator() {
        let negotiator = VersionNegotiator::new();

        // Supported versions return as-is
        assert_eq!(negotiator.negotiate("2024-11-05"), ProtocolVersion::V2024_11_05);
        assert_eq!(negotiator.negotiate("2025-03-26"), ProtocolVersion::V2025_03_26);
        assert_eq!(negotiator.negotiate("2025-06-18"), ProtocolVersion::V2025_06_18);

        // Unsupported versions fall back to latest
        assert_eq!(negotiator.negotiate("invalid"), ProtocolVersion::V2025_06_18);
        assert_eq!(negotiator.negotiate("2099-01-01"), ProtocolVersion::V2025_06_18);
    }

    // Roots Tests

    #[tokio::test]
    async fn test_roots_registry() {
        let registry = RootsRegistry::new();

        assert!(registry.is_empty().await);

        registry.add_root(Root::new("file:///home/user")).await;
        assert_eq!(registry.len().await, 1);

        registry.add_root(Root::with_name("file:///project", "Project")).await;
        assert_eq!(registry.len().await, 2);

        let roots = registry.list().await;
        assert_eq!(roots.len(), 2);
        assert_eq!(roots[0].uri, "file:///home/user");
        assert_eq!(roots[1].name, Some("Project".to_string()));
    }

    #[tokio::test]
    async fn test_roots_remove() {
        let registry = RootsRegistry::new();
        registry.add_root(Root::new("file:///a")).await;
        registry.add_root(Root::new("file:///b")).await;

        assert!(registry.remove_root("file:///a").await);
        assert_eq!(registry.len().await, 1);

        // Removing non-existent returns false
        assert!(!registry.remove_root("file:///nonexistent").await);
    }

    // Elicitation Tests

    #[test]
    fn test_elicitation_schema() {
        let schema = ElicitationSchema::object()
            .with_property("name", PropertySchema::string().with_description("User name"))
            .with_property("age", PropertySchema::number().with_minimum(0.0).with_maximum(150.0))
            .with_required("name");

        assert_eq!(schema.schema_type, "object");
        assert!(schema.properties.as_ref().unwrap().contains_key("name"));
        assert!(schema.required.as_ref().unwrap().contains(&"name".to_string()));
    }

    #[test]
    fn test_elicitation_response() {
        let accept = ElicitationResponse::accept(serde_json::json!({"name": "test"}));
        assert_eq!(accept.action, ElicitationAction::Accept);
        assert!(accept.content.is_some());

        let decline = ElicitationResponse::decline();
        assert_eq!(decline.action, ElicitationAction::Decline);
        assert!(decline.content.is_none());

        let cancel = ElicitationResponse::cancel();
        assert_eq!(cancel.action, ElicitationAction::Cancel);
    }

    // Resource Template Tests

    #[test]
    fn test_resource_template_placeholders() {
        let template = ResourceTemplate::new("file:///{path}", "File");
        assert_eq!(template.placeholders(), vec!["path"]);

        let template2 = ResourceTemplate::new("db:///{table}/{id}", "Database");
        assert_eq!(template2.placeholders(), vec!["table", "id"]);
    }

    #[tokio::test]
    async fn test_resource_template_matching() {
        let registry = ResourceTemplateRegistry::new();
        registry.register(ResourceTemplate::new("file:///{path}", "File")).await;

        let result = registry.match_uri("file:///test.txt").await;
        assert!(result.is_some());

        let (template, params) = result.unwrap();
        assert_eq!(template.name, "File");
        assert_eq!(params.len(), 1);
        assert_eq!(params[0].name, "path");
        assert_eq!(params[0].value, "test.txt");
    }

    #[test]
    fn test_extract_params() {
        let params = ResourceTemplateRegistry::extract_params("file:///test.txt", "file:///{path}");
        assert!(params.is_some());
        let params = params.unwrap();
        assert_eq!(params.len(), 1);
        assert_eq!(params[0].value, "test.txt");

        // Non-matching
        let params =
            ResourceTemplateRegistry::extract_params("http://example.com", "file:///{path}");
        assert!(params.is_none());
    }

    // Annotation Tests

    #[test]
    fn test_annotations() {
        let ann = Annotations::new()
            .with_audience(vec!["user".to_string(), "admin".to_string()])
            .with_priority(0.8);

        assert_eq!(ann.audience, Some(vec!["user".to_string(), "admin".to_string()]));
        assert_eq!(ann.priority, Some(0.8));
    }

    #[test]
    fn test_annotations_priority_clamping() {
        let ann = Annotations::new().with_priority(1.5);
        assert_eq!(ann.priority, Some(1.0));

        let ann = Annotations::new().with_priority(-0.5);
        assert_eq!(ann.priority, Some(0.0));
    }

    #[test]
    fn test_enhanced_tool_result() {
        let success = EnhancedToolResult::text("Hello");
        assert!(success.is_error.is_none());

        let error = EnhancedToolResult::error("Something went wrong");
        assert_eq!(error.is_error, Some(true));
    }

    // Notification Tests

    #[test]
    fn test_notification_json_rpc() {
        let notif = Notification::ToolsListChanged;
        let json = notif.to_json_rpc();
        assert!(json.contains("notifications/tools/list_changed"));

        let progress = Notification::Progress(ProgressNotification {
            progress_token: "token-1".to_string(),
            progress: 0.5,
            total: Some(100),
        });
        let json = progress.to_json_rpc();
        assert!(json.contains("notifications/progress"));
        assert!(json.contains("token-1"));
    }

    // Subscription Tracker Tests

    #[tokio::test]
    async fn test_subscription_tracker() {
        let tracker = SubscriptionTracker::new();

        tracker.subscribe("file:///test.txt", "client-1").await;
        assert!(tracker.is_subscribed("file:///test.txt", "client-1").await);
        assert!(!tracker.is_subscribed("file:///test.txt", "client-2").await);

        tracker.subscribe("file:///test.txt", "client-2").await;
        assert_eq!(tracker.subscription_count("file:///test.txt").await, 2);

        // Unsubscribe
        assert!(tracker.unsubscribe("file:///test.txt", "client-1").await);
        assert!(!tracker.is_subscribed("file:///test.txt", "client-1").await);
        assert_eq!(tracker.subscription_count("file:///test.txt").await, 1);
    }

    #[tokio::test]
    async fn test_subscription_tracker_idempotent() {
        let tracker = SubscriptionTracker::new();

        // Unsubscribing from non-existent subscription returns true (idempotent)
        assert!(tracker.unsubscribe("file:///nonexistent", "client-1").await);
    }
}
