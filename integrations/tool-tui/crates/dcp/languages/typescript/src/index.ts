/**
 * DCP TypeScript SDK
 * 
 * A Promise-based client for the Development Context Protocol (DCP).
 */

export { DcpClient, DcpClientConfig } from "./client";
export {
    TcpTransport,
    StdioTransport,
    SseTransport,
} from "./transport";
export {
    DcpError,
    TimeoutError,
    ConnectionError,
    ProtocolError,
} from "./errors";
export {
    // Protocol version
    ProtocolVersion,
    ProtocolVersionUtils,
    // JSON-RPC types
    JsonRpcRequest,
    JsonRpcResponse,
    JsonRpcError,
    // MCP types
    Tool,
    ToolCallResult,
    ContentItem,
    Annotations,
    Resource,
    ResourceContent,
    ResourceTemplate,
    Root,
    ElicitationRequest,
    ElicitationSchema,
    PropertySchema,
    ElicitationAction,
    ElicitationResponse,
    Prompt,
    PromptArgument,
    PromptMessage,
    // Initialize types
    InitializeParams,
    InitializeResult,
    ClientCapabilities,
    ServerCapabilities,
    ClientInfo,
    ServerInfo,
    // List results
    ToolsListResult,
    ResourcesListResult,
    RootsListResult,
    PromptsListResult,
    PromptGetResult,
    // Sampling types
    SamplingMessage,
    CreateMessageParams,
    CreateMessageResult,
    ModelPreferences,
    ModelHint,
    // Completion types
    CompleteParams,
    CompleteResult,
    // Transport
    Transport,
    DcpClientOptions,
} from "./types";
