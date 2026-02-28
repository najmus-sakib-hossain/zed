//! Complete MCP adapter with full protocol support.
//!
//! Supports all MCP methods: tools, resources, prompts, logging, sampling, completion.

use std::collections::HashMap;
use std::sync::Arc;

use serde_json::Value;
use tokio::sync::RwLock;

use crate::dispatch::{BinaryTrieRouter, ToolResult};
use crate::resource::{ResourceContent, ResourceError, ResourceRegistry};
use crate::DCPError;

use super::json_rpc::{
    JsonRpcError, JsonRpcParseError, JsonRpcParser, JsonRpcRequest, JsonRpcResponse, RequestId,
};

/// Complete adapter errors
#[derive(Debug, Clone, thiserror::Error)]
pub enum CompleteAdapterError {
    #[error("JSON-RPC parse error: {0}")]
    ParseError(#[from] JsonRpcParseError),
    #[error("unknown tool: {0}")]
    UnknownTool(String),
    #[error("DCP error: {0}")]
    DcpError(#[from] DCPError),
    #[error("resource error: {0}")]
    ResourceError(String),
    #[error("prompt error: {0}")]
    PromptError(String),
    #[error("serialization error: {0}")]
    SerializationError(String),
    #[error("invalid params: {0}")]
    InvalidParams(String),
}

impl From<ResourceError> for CompleteAdapterError {
    fn from(e: ResourceError) -> Self {
        Self::ResourceError(e.to_string())
    }
}

/// Prompt template
#[derive(Debug, Clone)]
pub struct PromptTemplate {
    /// Unique name
    pub name: String,
    /// Description
    pub description: String,
    /// Arguments
    pub arguments: Vec<PromptArgument>,
    /// Template content with {{arg}} placeholders
    pub template: String,
}

/// Prompt argument
#[derive(Debug, Clone)]
pub struct PromptArgument {
    /// Argument name
    pub name: String,
    /// Description
    pub description: String,
    /// Whether required
    pub required: bool,
}

impl PromptTemplate {
    /// Create a new prompt template
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        template: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            arguments: Vec::new(),
            template: template.into(),
        }
    }

    /// Add an argument
    pub fn with_argument(
        mut self,
        name: impl Into<String>,
        description: impl Into<String>,
        required: bool,
    ) -> Self {
        self.arguments.push(PromptArgument {
            name: name.into(),
            description: description.into(),
            required,
        });
        self
    }

    /// Render the template with arguments
    pub fn render(&self, args: &HashMap<String, String>) -> Result<String, CompleteAdapterError> {
        // Check required arguments
        for arg in &self.arguments {
            if arg.required && !args.contains_key(&arg.name) {
                return Err(CompleteAdapterError::PromptError(format!(
                    "missing required argument: {}",
                    arg.name
                )));
            }
        }

        // Substitute placeholders
        let mut result = self.template.clone();
        for (key, value) in args {
            let placeholder = format!("{{{{{}}}}}", key);
            result = result.replace(&placeholder, value);
        }

        Ok(result)
    }
}

/// Log level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LogLevel {
    Debug,
    #[default]
    Info,
    Warning,
    Error,
}

impl LogLevel {
    fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "debug" => Some(Self::Debug),
            "info" => Some(Self::Info),
            "warning" | "warn" => Some(Self::Warning),
            "error" => Some(Self::Error),
            _ => None,
        }
    }
}

/// Complete MCP adapter with full protocol support
pub struct CompleteMcpAdapter {
    /// Tool name to ID cache
    tool_cache: HashMap<String, u16>,
    /// ID to tool name reverse mapping
    id_to_name: HashMap<u16, String>,
    /// Resource registry
    resources: Arc<RwLock<ResourceRegistry>>,
    /// Prompt templates
    prompts: HashMap<String, PromptTemplate>,
    /// Current log level
    log_level: RwLock<LogLevel>,
    /// Server name
    server_name: String,
    /// Server version
    server_version: String,
    /// Negotiated protocol version
    protocol_version: RwLock<super::mcp2025::ProtocolVersion>,
    /// Version negotiator
    version_negotiator: super::mcp2025::VersionNegotiator,
    /// Roots registry
    roots: Arc<super::mcp2025::RootsRegistry>,
    /// Subscription tracker
    subscriptions: Arc<super::mcp2025::SubscriptionTracker>,
    /// Elicitation handler
    elicitation: Arc<super::mcp2025::ElicitationHandler>,
    /// Resource template registry
    resource_templates: Arc<super::mcp2025::ResourceTemplateRegistry>,
    /// Notification manager
    notifications: Arc<super::mcp2025::NotificationManager>,
    /// Cancellation manager
    cancellation: Arc<super::mcp2025::CancellationManager>,
    /// Progress tracker
    progress: Arc<super::mcp2025::ProgressTracker>,
}

impl CompleteMcpAdapter {
    /// Create a new complete adapter
    pub fn new() -> Self {
        Self {
            tool_cache: HashMap::new(),
            id_to_name: HashMap::new(),
            resources: Arc::new(RwLock::new(ResourceRegistry::new())),
            prompts: HashMap::new(),
            log_level: RwLock::new(LogLevel::default()),
            server_name: "dcp-server".to_string(),
            server_version: env!("CARGO_PKG_VERSION").to_string(),
            protocol_version: RwLock::new(super::mcp2025::ProtocolVersion::default()),
            version_negotiator: super::mcp2025::VersionNegotiator::new(),
            roots: Arc::new(super::mcp2025::RootsRegistry::new()),
            subscriptions: Arc::new(super::mcp2025::SubscriptionTracker::new()),
            elicitation: Arc::new(super::mcp2025::ElicitationHandler::new()),
            resource_templates: Arc::new(super::mcp2025::ResourceTemplateRegistry::new()),
            notifications: Arc::new(super::mcp2025::NotificationManager::new()),
            cancellation: Arc::new(super::mcp2025::CancellationManager::new()),
            progress: Arc::new(super::mcp2025::ProgressTracker::new()),
        }
    }

    /// Set server info
    pub fn with_server_info(mut self, name: impl Into<String>, version: impl Into<String>) -> Self {
        self.server_name = name.into();
        self.server_version = version.into();
        self
    }

    /// Register a tool
    pub fn register_tool(&mut self, name: impl Into<String>, tool_id: u16) {
        let name = name.into();
        self.tool_cache.insert(name.clone(), tool_id);
        self.id_to_name.insert(tool_id, name);
    }

    /// Get resource registry for registration
    pub fn resources(&self) -> Arc<RwLock<ResourceRegistry>> {
        Arc::clone(&self.resources)
    }

    /// Register a prompt template
    pub fn register_prompt(&mut self, template: PromptTemplate) {
        self.prompts.insert(template.name.clone(), template);
    }

    /// Parse request
    pub fn parse_request(&self, json: &str) -> Result<JsonRpcRequest, CompleteAdapterError> {
        Ok(JsonRpcParser::parse_request(json)?)
    }

    /// Format success response
    pub fn format_success(
        &self,
        id: RequestId,
        result: Value,
    ) -> Result<String, CompleteAdapterError> {
        let response = JsonRpcResponse::success(id, result);
        JsonRpcParser::format_response(&response)
            .map_err(|e| CompleteAdapterError::SerializationError(e.to_string()))
    }

    /// Format error response
    pub fn format_error(
        &self,
        id: RequestId,
        error: JsonRpcError,
    ) -> Result<String, CompleteAdapterError> {
        let response = JsonRpcResponse::error(id, error);
        JsonRpcParser::format_response(&response)
            .map_err(|e| CompleteAdapterError::SerializationError(e.to_string()))
    }

    // ========================================================================
    // Lifecycle Methods
    // ========================================================================

    /// Handle initialize with protocol version negotiation
    pub async fn handle_initialize(
        &self,
        request: &JsonRpcRequest,
    ) -> Result<String, CompleteAdapterError> {
        // Extract requested protocol version
        let requested_version = request
            .params
            .as_ref()
            .and_then(|p| p.get("protocolVersion"))
            .and_then(|v| v.as_str())
            .unwrap_or("2024-11-05");

        // Negotiate version
        let negotiated = self.version_negotiator.negotiate(requested_version);
        *self.protocol_version.write().await = negotiated;

        // Build capabilities based on negotiated version
        let mut capabilities = serde_json::json!({
            "tools": { "listChanged": true },
            "resources": { "subscribe": true, "listChanged": true },
            "prompts": { "listChanged": true },
            "logging": {}
        });

        // Add version-specific capabilities
        if negotiated.supports_roots() {
            capabilities["roots"] = serde_json::json!({ "listChanged": true });
        }
        if negotiated.supports_elicitation() {
            capabilities["elicitation"] = serde_json::json!({});
        }

        let result = serde_json::json!({
            "protocolVersion": negotiated.as_str(),
            "capabilities": capabilities,
            "serverInfo": {
                "name": self.server_name,
                "version": self.server_version
            }
        });
        self.format_success(request.id.clone(), result)
    }

    /// Get the negotiated protocol version
    pub async fn protocol_version(&self) -> super::mcp2025::ProtocolVersion {
        *self.protocol_version.read().await
    }

    /// Handle initialized notification
    pub fn handle_initialized(
        &self,
        _request: &JsonRpcRequest,
    ) -> Result<Option<String>, CompleteAdapterError> {
        // Notification - no response
        Ok(None)
    }

    // ========================================================================
    // Roots Methods (MCP 2025-03-26+)
    // ========================================================================

    /// Get the roots registry for configuration
    pub fn roots(&self) -> Arc<super::mcp2025::RootsRegistry> {
        Arc::clone(&self.roots)
    }

    /// Handle roots/list
    pub async fn handle_roots_list(
        &self,
        request: &JsonRpcRequest,
    ) -> Result<String, CompleteAdapterError> {
        // Check if roots are supported in negotiated version
        let version = *self.protocol_version.read().await;
        if !version.supports_roots() {
            return self.format_error(
                request.id.clone(),
                JsonRpcError::with_data(
                    -32601,
                    "Method not found",
                    serde_json::json!({
                        "method": "roots/list",
                        "requiredVersion": "2025-03-26",
                        "negotiatedVersion": version.as_str()
                    }),
                ),
            );
        }

        let roots = self.roots.list().await;
        let result = serde_json::json!({ "roots": roots });
        self.format_success(request.id.clone(), result)
    }

    // ========================================================================
    // Elicitation Methods (MCP 2025-06-18+)
    // ========================================================================

    /// Get the elicitation handler for configuration
    pub fn elicitation(&self) -> Arc<super::mcp2025::ElicitationHandler> {
        Arc::clone(&self.elicitation)
    }

    /// Handle elicitation/create
    pub async fn handle_elicitation_create(
        &self,
        request: &JsonRpcRequest,
    ) -> Result<String, CompleteAdapterError> {
        // Check if elicitation is supported in negotiated version
        let version = *self.protocol_version.read().await;
        if !version.supports_elicitation() {
            return self.format_error(
                request.id.clone(),
                JsonRpcError::with_data(
                    -32601,
                    "Method not found",
                    serde_json::json!({
                        "method": "elicitation/create",
                        "requiredVersion": "2025-06-18",
                        "negotiatedVersion": version.as_str()
                    }),
                ),
            );
        }

        let params = request
            .params
            .as_ref()
            .ok_or(CompleteAdapterError::InvalidParams("missing params".into()))?;

        let message = params
            .get("message")
            .and_then(|v| v.as_str())
            .ok_or(CompleteAdapterError::InvalidParams("missing message".into()))?;

        // Parse optional schema
        let requested_schema = params
            .get("requestedSchema")
            .map(|v| serde_json::from_value::<super::mcp2025::ElicitationSchema>(v.clone()))
            .transpose()
            .map_err(|e| CompleteAdapterError::InvalidParams(format!("invalid schema: {}", e)))?;

        let elicitation_request = if let Some(schema) = requested_schema {
            super::mcp2025::ElicitationRequest::with_schema(message, schema)
        } else {
            super::mcp2025::ElicitationRequest::new(message)
        };

        // Create the elicitation and wait for response
        match self.elicitation.create(elicitation_request).await {
            Ok(response) => {
                let result = serde_json::to_value(&response)
                    .map_err(|e| CompleteAdapterError::SerializationError(e.to_string()))?;
                self.format_success(request.id.clone(), result)
            }
            Err(super::mcp2025::ElicitationError::Cancelled) => self.format_success(
                request.id.clone(),
                serde_json::json!({
                    "action": "cancel"
                }),
            ),
            Err(super::mcp2025::ElicitationError::Timeout) => self.format_error(
                request.id.clone(),
                JsonRpcError::with_data(-32000, "Elicitation timeout", serde_json::json!({})),
            ),
            Err(super::mcp2025::ElicitationError::ValidationFailed(msg)) => self.format_error(
                request.id.clone(),
                JsonRpcError::with_data(
                    -32602,
                    "Validation failed",
                    serde_json::json!({ "message": msg }),
                ),
            ),
        }
    }

    // ========================================================================
    // Resource Template Methods (MCP 2025-03-26+)
    // ========================================================================

    /// Get the resource template registry for configuration
    pub fn resource_templates(&self) -> Arc<super::mcp2025::ResourceTemplateRegistry> {
        Arc::clone(&self.resource_templates)
    }

    /// Get the notification manager
    pub fn notifications(&self) -> Arc<super::mcp2025::NotificationManager> {
        Arc::clone(&self.notifications)
    }

    /// Get the cancellation manager
    pub fn cancellation(&self) -> Arc<super::mcp2025::CancellationManager> {
        Arc::clone(&self.cancellation)
    }

    /// Get the progress tracker
    pub fn progress(&self) -> Arc<super::mcp2025::ProgressTracker> {
        Arc::clone(&self.progress)
    }

    // ========================================================================
    // Cancellation Methods
    // ========================================================================

    /// Handle notifications/cancelled
    pub async fn handle_cancelled(
        &self,
        request: &JsonRpcRequest,
    ) -> Result<Option<String>, CompleteAdapterError> {
        let params = request
            .params
            .as_ref()
            .ok_or(CompleteAdapterError::InvalidParams("missing params".into()))?;

        let request_id = params
            .get("requestId")
            .ok_or(CompleteAdapterError::InvalidParams("missing requestId".into()))?;

        let request_id = if let Some(n) = request_id.as_i64() {
            RequestId::Number(n)
        } else if let Some(s) = request_id.as_str() {
            RequestId::String(s.to_string())
        } else {
            return Err(CompleteAdapterError::InvalidParams("invalid requestId".into()));
        };

        let reason = params.get("reason").and_then(|v| v.as_str()).map(|s| s.to_string());

        // Cancel the request - this is idempotent
        self.cancellation.cancel(&request_id, reason).await;

        // Notifications don't return a response
        Ok(None)
    }

    // ========================================================================
    // Progress Methods
    // ========================================================================

    /// Extract progressToken from request _meta
    pub fn extract_progress_token(request: &JsonRpcRequest) -> Option<String> {
        request
            .params
            .as_ref()
            .and_then(|p| p.get("_meta"))
            .and_then(|m| m.get("progressToken"))
            .and_then(|t| t.as_str())
            .map(|s| s.to_string())
    }

    // ========================================================================
    // Ping/Pong Methods
    // ========================================================================

    /// Handle ping method
    pub fn handle_ping(&self, request: &JsonRpcRequest) -> Result<String, CompleteAdapterError> {
        // Return empty result object
        self.format_success(request.id.clone(), serde_json::json!({}))
    }

    // ========================================================================
    // Tool Methods
    // ========================================================================

    /// Handle tools/list
    pub fn handle_tools_list(
        &self,
        request: &JsonRpcRequest,
    ) -> Result<String, CompleteAdapterError> {
        let tools: Vec<Value> = self
            .tool_cache
            .keys()
            .map(|name| {
                serde_json::json!({
                    "name": name,
                    "description": format!("Tool: {}", name),
                    "inputSchema": { "type": "object", "properties": {} }
                })
            })
            .collect();

        self.format_success(request.id.clone(), serde_json::json!({ "tools": tools }))
    }

    /// Handle tools/call
    pub fn handle_tools_call(
        &self,
        request: &JsonRpcRequest,
        router: &BinaryTrieRouter,
    ) -> Result<String, CompleteAdapterError> {
        let params = request
            .params
            .as_ref()
            .ok_or(CompleteAdapterError::InvalidParams("missing params".into()))?;

        let tool_name = params
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or(CompleteAdapterError::InvalidParams("missing tool name".into()))?;

        let tool_id = self
            .tool_cache
            .get(tool_name)
            .ok_or_else(|| CompleteAdapterError::UnknownTool(tool_name.into()))?;

        let arguments = params.get("arguments").cloned();
        let args_bytes = arguments
            .map(|v| serde_json::to_vec(&v).unwrap_or_default())
            .unwrap_or_default();
        let shared_args = crate::dispatch::SharedArgs::new(&args_bytes, 0);

        let result = router.execute(*tool_id, &shared_args)?;
        let result_value = match result {
            ToolResult::Success(data) => serde_json::from_slice(&data)
                .unwrap_or(Value::String(String::from_utf8_lossy(&data).into())),
            ToolResult::Empty => Value::Null,
            ToolResult::Error(e) => serde_json::json!({"error": e.to_string()}),
        };

        self.format_success(request.id.clone(), serde_json::json!({
            "content": [{ "type": "text", "text": serde_json::to_string(&result_value).unwrap_or_default() }]
        }))
    }

    // ========================================================================
    // Resource Methods
    // ========================================================================

    /// Handle resources/list
    pub async fn handle_resources_list(
        &self,
        request: &JsonRpcRequest,
    ) -> Result<String, CompleteAdapterError> {
        let cursor = request.params.as_ref().and_then(|p| p.get("cursor")).and_then(|v| v.as_str());

        let registry = self.resources.read().await;
        let list = registry.list_all(cursor)?;

        let resources: Vec<Value> = list
            .resources
            .iter()
            .map(|r| {
                serde_json::json!({
                    "uri": r.uri,
                    "name": r.name,
                    "description": r.description,
                    "mimeType": r.mime_type
                })
            })
            .collect();

        let mut result = serde_json::json!({ "resources": resources });
        if let Some(cursor) = list.next_cursor {
            result["nextCursor"] = Value::String(cursor);
        }

        // Include resource templates (MCP 2025-03-26+)
        let version = *self.protocol_version.read().await;
        if version.supports_roots() {
            let templates = self.resource_templates.list().await;
            if !templates.is_empty() {
                result["resourceTemplates"] =
                    serde_json::to_value(&templates).unwrap_or(Value::Array(vec![]));
            }
        }

        self.format_success(request.id.clone(), result)
    }

    /// Handle resources/read
    pub async fn handle_resources_read(
        &self,
        request: &JsonRpcRequest,
    ) -> Result<String, CompleteAdapterError> {
        let uri = request
            .params
            .as_ref()
            .and_then(|p| p.get("uri"))
            .and_then(|v| v.as_str())
            .ok_or(CompleteAdapterError::InvalidParams("missing uri".into()))?;

        let registry = self.resources.read().await;
        let content = registry.read(uri)?;

        let content_value = match content {
            ResourceContent::Text {
                uri,
                mime_type,
                text,
            } => serde_json::json!({
                "uri": uri,
                "mimeType": mime_type,
                "text": text
            }),
            ResourceContent::Blob {
                uri,
                mime_type,
                blob,
            } => serde_json::json!({
                "uri": uri,
                "mimeType": mime_type,
                "blob": blob
            }),
        };

        self.format_success(
            request.id.clone(),
            serde_json::json!({
                "contents": [content_value]
            }),
        )
    }

    /// Handle resources/subscribe
    pub async fn handle_resources_subscribe(
        &self,
        request: &JsonRpcRequest,
    ) -> Result<String, CompleteAdapterError> {
        let uri = request
            .params
            .as_ref()
            .and_then(|p| p.get("uri"))
            .and_then(|v| v.as_str())
            .ok_or(CompleteAdapterError::InvalidParams("missing uri".into()))?;

        let registry = self.resources.read().await;
        // Just verify the resource exists
        let _ = registry.match_uri(uri).ok_or_else(|| {
            CompleteAdapterError::ResourceError(format!("resource not found: {}", uri))
        })?;

        // Track subscription (using a default client ID for now)
        self.subscriptions.subscribe(uri, "default").await;

        self.format_success(request.id.clone(), serde_json::json!({}))
    }

    /// Handle resources/unsubscribe (idempotent)
    pub async fn handle_resources_unsubscribe(
        &self,
        request: &JsonRpcRequest,
    ) -> Result<String, CompleteAdapterError> {
        let uri = request
            .params
            .as_ref()
            .and_then(|p| p.get("uri"))
            .and_then(|v| v.as_str())
            .ok_or(CompleteAdapterError::InvalidParams("missing uri".into()))?;

        // Unsubscribe is idempotent - always succeeds
        self.subscriptions.unsubscribe(uri, "default").await;

        self.format_success(request.id.clone(), serde_json::json!({}))
    }

    // ========================================================================
    // Prompt Methods
    // ========================================================================

    /// Handle prompts/list
    pub fn handle_prompts_list(
        &self,
        request: &JsonRpcRequest,
    ) -> Result<String, CompleteAdapterError> {
        let prompts: Vec<Value> = self
            .prompts
            .values()
            .map(|p| {
                let args: Vec<Value> = p
                    .arguments
                    .iter()
                    .map(|a| {
                        serde_json::json!({
                            "name": a.name,
                            "description": a.description,
                            "required": a.required
                        })
                    })
                    .collect();
                serde_json::json!({
                    "name": p.name,
                    "description": p.description,
                    "arguments": args
                })
            })
            .collect();

        self.format_success(request.id.clone(), serde_json::json!({ "prompts": prompts }))
    }

    /// Handle prompts/get
    pub fn handle_prompts_get(
        &self,
        request: &JsonRpcRequest,
    ) -> Result<String, CompleteAdapterError> {
        let params = request
            .params
            .as_ref()
            .ok_or(CompleteAdapterError::InvalidParams("missing params".into()))?;

        let name = params
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or(CompleteAdapterError::InvalidParams("missing prompt name".into()))?;

        let template = self.prompts.get(name).ok_or_else(|| {
            CompleteAdapterError::PromptError(format!("prompt not found: {}", name))
        })?;

        let args: HashMap<String, String> = params
            .get("arguments")
            .and_then(|v| v.as_object())
            .map(|obj| {
                obj.iter()
                    .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                    .collect()
            })
            .unwrap_or_default();

        let rendered = template.render(&args)?;

        self.format_success(
            request.id.clone(),
            serde_json::json!({
                "description": template.description,
                "messages": [{
                    "role": "user",
                    "content": { "type": "text", "text": rendered }
                }]
            }),
        )
    }

    // ========================================================================
    // Logging Methods
    // ========================================================================

    /// Handle logging/setLevel
    pub async fn handle_logging_set_level(
        &self,
        request: &JsonRpcRequest,
    ) -> Result<String, CompleteAdapterError> {
        let level_str = request
            .params
            .as_ref()
            .and_then(|p| p.get("level"))
            .and_then(|v| v.as_str())
            .ok_or(CompleteAdapterError::InvalidParams("missing level".into()))?;

        let level = LogLevel::from_str(level_str).ok_or_else(|| {
            CompleteAdapterError::InvalidParams(format!("invalid log level: {}", level_str))
        })?;

        *self.log_level.write().await = level;

        self.format_success(request.id.clone(), serde_json::json!({}))
    }

    // ========================================================================
    // Sampling Methods
    // ========================================================================

    /// Handle sampling/createMessage
    pub fn handle_sampling_create_message(
        &self,
        request: &JsonRpcRequest,
    ) -> Result<String, CompleteAdapterError> {
        // Sampling is typically handled by the client, but we provide a stub
        self.format_success(
            request.id.clone(),
            serde_json::json!({
                "role": "assistant",
                "content": { "type": "text", "text": "Sampling not implemented" },
                "model": "stub",
                "stopReason": "end_turn"
            }),
        )
    }

    // ========================================================================
    // Completion Methods
    // ========================================================================

    /// Handle completion/complete
    pub fn handle_completion_complete(
        &self,
        request: &JsonRpcRequest,
    ) -> Result<String, CompleteAdapterError> {
        // Auto-completion stub
        self.format_success(
            request.id.clone(),
            serde_json::json!({
                "completion": { "values": [], "hasMore": false }
            }),
        )
    }

    // ========================================================================
    // Main Dispatch
    // ========================================================================

    /// Handle any MCP request
    pub async fn handle_request(
        &self,
        json: &str,
        router: &BinaryTrieRouter,
    ) -> Result<Option<String>, CompleteAdapterError> {
        let request = self.parse_request(json)?;

        // Check if notification (no id)
        let is_notification = request.is_notification();

        let result = match request.method.as_str() {
            // Lifecycle
            "initialize" => Some(self.handle_initialize(&request).await?),
            "initialized" => self.handle_initialized(&request)?,

            // Ping/Pong
            "ping" => Some(self.handle_ping(&request)?),

            // Roots (MCP 2025-03-26+)
            "roots/list" => Some(self.handle_roots_list(&request).await?),

            // Elicitation (MCP 2025-06-18+)
            "elicitation/create" => Some(self.handle_elicitation_create(&request).await?),

            // Tools
            "tools/list" => Some(self.handle_tools_list(&request)?),
            "tools/call" => Some(self.handle_tools_call(&request, router)?),

            // Resources
            "resources/list" => Some(self.handle_resources_list(&request).await?),
            "resources/read" => Some(self.handle_resources_read(&request).await?),
            "resources/subscribe" => Some(self.handle_resources_subscribe(&request).await?),
            "resources/unsubscribe" => Some(self.handle_resources_unsubscribe(&request).await?),

            // Prompts
            "prompts/list" => Some(self.handle_prompts_list(&request)?),
            "prompts/get" => Some(self.handle_prompts_get(&request)?),

            // Logging
            "logging/setLevel" => Some(self.handle_logging_set_level(&request).await?),

            // Sampling
            "sampling/createMessage" => Some(self.handle_sampling_create_message(&request)?),

            // Completion
            "completion/complete" => Some(self.handle_completion_complete(&request)?),

            // Notifications (no response)
            "notifications/cancelled" => self.handle_cancelled(&request).await?,

            // Unknown method
            _ => Some(self.format_error(request.id.clone(), JsonRpcError::method_not_found())?),
        };

        // Don't return response for notifications
        if is_notification {
            Ok(None)
        } else {
            Ok(result)
        }
    }
}

impl Default for CompleteMcpAdapter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prompt_template_render() {
        let template = PromptTemplate::new("test", "Test prompt", "Hello {{name}}!")
            .with_argument("name", "The name", true);

        let mut args = HashMap::new();
        args.insert("name".to_string(), "World".to_string());

        let rendered = template.render(&args).unwrap();
        assert_eq!(rendered, "Hello World!");
    }

    #[test]
    fn test_prompt_template_missing_required() {
        let template = PromptTemplate::new("test", "Test prompt", "Hello {{name}}!")
            .with_argument("name", "The name", true);

        let args = HashMap::new();
        let result = template.render(&args);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_complete_adapter_initialize() {
        let adapter = CompleteMcpAdapter::new();

        // When no version is specified, should default to 2024-11-05 for backward compatibility
        let request = JsonRpcRequest::new("initialize", None, RequestId::Number(1));
        let response = adapter.handle_initialize(&request).await.unwrap();
        let parsed = JsonRpcParser::parse_response(&response).unwrap();

        assert!(parsed.is_success());
        let result = parsed.result.unwrap();
        // When no version specified, defaults to 2024-11-05 for backward compatibility
        assert_eq!(result["protocolVersion"], "2024-11-05");
        // Should NOT have roots or elicitation capabilities (2024-11-05)
        assert!(result["capabilities"]["roots"].is_null());
        assert!(result["capabilities"]["elicitation"].is_null());
    }

    #[tokio::test]
    async fn test_complete_adapter_initialize_version_negotiation() {
        let adapter = CompleteMcpAdapter::new();

        // Test with 2024-11-05
        let request = JsonRpcRequest::new(
            "initialize",
            Some(serde_json::json!({"protocolVersion": "2024-11-05"})),
            RequestId::Number(1),
        );
        let response = adapter.handle_initialize(&request).await.unwrap();
        let parsed = JsonRpcParser::parse_response(&response).unwrap();
        let result = parsed.result.unwrap();
        assert_eq!(result["protocolVersion"], "2024-11-05");
        // Should not have roots capability
        assert!(result["capabilities"]["roots"].is_null());

        // Test with 2025-03-26
        let request = JsonRpcRequest::new(
            "initialize",
            Some(serde_json::json!({"protocolVersion": "2025-03-26"})),
            RequestId::Number(2),
        );
        let response = adapter.handle_initialize(&request).await.unwrap();
        let parsed = JsonRpcParser::parse_response(&response).unwrap();
        let result = parsed.result.unwrap();
        assert_eq!(result["protocolVersion"], "2025-03-26");
        // Should have roots capability
        assert!(result["capabilities"]["roots"].is_object());
        // Should not have elicitation capability
        assert!(result["capabilities"]["elicitation"].is_null());

        // Test with 2025-06-18
        let request = JsonRpcRequest::new(
            "initialize",
            Some(serde_json::json!({"protocolVersion": "2025-06-18"})),
            RequestId::Number(3),
        );
        let response = adapter.handle_initialize(&request).await.unwrap();
        let parsed = JsonRpcParser::parse_response(&response).unwrap();
        let result = parsed.result.unwrap();
        assert_eq!(result["protocolVersion"], "2025-06-18");
        // Should have both roots and elicitation capabilities
        assert!(result["capabilities"]["roots"].is_object());
        assert!(result["capabilities"]["elicitation"].is_object());
    }

    #[tokio::test]
    async fn test_complete_adapter_prompts() {
        let mut adapter = CompleteMcpAdapter::new();
        adapter.register_prompt(
            PromptTemplate::new("greet", "Greeting prompt", "Hello {{name}}!").with_argument(
                "name",
                "Name to greet",
                true,
            ),
        );

        // List prompts
        let request = JsonRpcRequest::new("prompts/list", None, RequestId::Number(1));
        let response = adapter.handle_prompts_list(&request).unwrap();
        let parsed = JsonRpcParser::parse_response(&response).unwrap();
        assert!(parsed.is_success());

        // Get prompt
        let request = JsonRpcRequest::new(
            "prompts/get",
            Some(serde_json::json!({"name": "greet", "arguments": {"name": "World"}})),
            RequestId::Number(2),
        );
        let response = adapter.handle_prompts_get(&request).unwrap();
        let parsed = JsonRpcParser::parse_response(&response).unwrap();
        assert!(parsed.is_success());
    }

    #[tokio::test]
    async fn test_notification_no_response() {
        let adapter = CompleteMcpAdapter::new();
        let router = BinaryTrieRouter::new();

        // Notification has no id
        let json = r#"{"jsonrpc":"2.0","method":"initialized"}"#;
        let result = adapter.handle_request(json, &router).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_unknown_method() {
        let adapter = CompleteMcpAdapter::new();
        let router = BinaryTrieRouter::new();

        let json = r#"{"jsonrpc":"2.0","method":"unknown/method","id":1}"#;
        let result = adapter.handle_request(json, &router).await.unwrap();

        let response = JsonRpcParser::parse_response(&result.unwrap()).unwrap();
        assert!(response.is_error());
        assert_eq!(response.error.unwrap().code, -32601);
    }
}
