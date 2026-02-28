//! MCP compatibility layer for DCP protocol.
//!
//! Provides JSON-RPC 2.0 translation for backward compatibility with MCP clients.

pub mod adapter;
pub mod complete_adapter;
pub mod json_rpc;
pub mod mcp2025;
pub mod sse;
pub mod stdio;

pub use adapter::McpAdapter;
pub use complete_adapter::{
    CompleteAdapterError, CompleteMcpAdapter, LogLevel, PromptArgument, PromptTemplate,
};
pub use json_rpc::{JsonRpcError, JsonRpcParser, JsonRpcRequest, JsonRpcResponse};
pub use mcp2025::{
    AnnotatedContent, Annotations, CancellationManager, CancellationNotification,
    CancellationState, CancellationToken, Content, ElicitationAction, ElicitationError,
    ElicitationHandler, ElicitationRequest, ElicitationResponse, ElicitationSchema,
    EnhancedToolResult, ExtendedMcpAdapter, Notification, NotificationManager, PingConfig,
    ProgressNotification, ProgressState, ProgressTracker, PropertySchema, ProtocolVersion,
    ResourceTemplate, ResourceTemplateRegistry, Root, RootsRegistry, SubscriptionTracker,
    TemplateParam, VersionNegotiator,
};
pub use sse::{SseEvent, SseEventType, SseTransport};
pub use stdio::{MessageFramer, StdioTransport};
