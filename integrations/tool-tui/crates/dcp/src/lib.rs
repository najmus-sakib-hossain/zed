//! DCP (Development Context Protocol)
//!
//! A binary-first protocol designed to replace MCP (Model Context Protocol)
//! with 10-1000x performance improvements while maintaining backward compatibility.

pub mod bench;
pub mod binary;
pub mod capability;
pub mod cli;
pub mod compat;
pub mod context;
pub mod dispatch;
pub mod error;
pub mod multiplex;
pub mod observability;
pub mod protocol;
pub mod reactor;
pub mod resource;
pub mod security;
pub mod server;
pub mod shutdown;
pub mod stream;
pub mod sync;
pub mod transport;

pub use bench::{
    compare_sizes_auto, estimate_binary_size, measure_dcp_size, measure_json_rpc_size,
    SizeComparison,
};
pub use capability::CapabilityManifest;
pub use cli::{convert_mcp_to_dcp, DcpSchema, McpSchema};
pub use compat::{
    JsonRpcError, JsonRpcParser, JsonRpcRequest, JsonRpcResponse, McpAdapter, SseTransport,
    StdioTransport,
};
pub use context::{ContextLayout, DcpContext, ToolState};
pub use dispatch::ServerCapabilities;
pub use error::{DCPError, SecurityError};
pub use multiplex::{
    MultiplexError, MultiplexedConnection, PipelinedClient, RequestPipeline, StreamFlags,
    StreamHeader, StreamState, StreamStatus, MAX_STREAMS, STREAM_HEADER_SIZE,
};
pub use observability::{
    create_span, init_tracing, LogConfig, LogEntry, LogFormat, LogLevel, MetricsConfig,
    PrometheusMetrics, RequestMetrics, RequestSpan, Span, SpanKind, SpanStatus, StructuredLogger,
    Tracer, TracingConfig,
};
pub use reactor::{
    create_default_reactor, create_reactor, Completion, EpollReactor, Event, Interest,
    IoUringReactor, IocpReactor, KqueueReactor, RawFd, Reactor, ReactorConfig, Token,
};
pub use resource::{
    ResourceContent, ResourceError, ResourceHandler, ResourceInfo, ResourceList, ResourceRegistry,
    SubscriptionId,
};
pub use server::{
    DcpServer, Metrics, MetricsSnapshot, ProtocolVersion, ServerConfig, ServerInfo, Session,
};
pub use shutdown::{
    setup_default_handlers, wait_for_ctrl_c, RequestGuard, ShutdownCoordinator, ShutdownLogger,
    ShutdownProgress, Signal, SignalHandler,
};
pub use transport::{
    Connection, FrameCodec, FrameError, FrameHeader, ProtocolMode, TcpConfig, TcpServer, TlsConfig,
    TlsVersion, MAX_MESSAGE_SIZE, PROTOCOL_VERSION,
};
