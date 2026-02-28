/**
 * DX DCP Types
 * 
 * Type definitions for the VS Code DCP integration.
 * Requirements: 11.1-11.10
 */

/**
 * DCP Server status
 */
export interface DcpServerStatus {
    /** Server name/identifier */
    name: string;
    /** Server port */
    port: number;
    /** Whether server is running */
    running: boolean;
    /** Server mode */
    mode: DcpServerMode;
    /** Uptime in seconds */
    uptime?: number;
    /** Last error if any */
    error?: string;
}

/**
 * DCP server modes
 */
export type DcpServerMode = 'dcp' | 'mcp' | 'hybrid';

/**
 * DCP Tool definition
 */
export interface DcpTool {
    /** Tool ID */
    id: string;
    /** Tool name */
    name: string;
    /** Tool description */
    description: string;
    /** Input schema */
    inputSchema: DcpSchema;
    /** Output schema */
    outputSchema?: DcpSchema;
    /** Capabilities bitset */
    capabilities: number;
    /** Whether tool is signed */
    signed: boolean;
    /** Tool version */
    version?: string;
}

/**
 * DCP Schema definition
 */
export interface DcpSchema {
    /** Schema type */
    type: 'object' | 'array' | 'string' | 'number' | 'boolean' | 'null';
    /** Properties for object type */
    properties?: Record<string, DcpSchemaProperty>;
    /** Required properties */
    required?: string[];
    /** Items schema for array type */
    items?: DcpSchema;
    /** Description */
    description?: string;
}

/**
 * DCP Schema property
 */
export interface DcpSchemaProperty {
    /** Property type */
    type: string;
    /** Property description */
    description?: string;
    /** Default value */
    default?: any;
    /** Enum values */
    enum?: any[];
}

/**
 * DCP Resource definition
 */
export interface DcpResource {
    /** Resource URI */
    uri: string;
    /** Resource name */
    name: string;
    /** Resource description */
    description?: string;
    /** MIME type */
    mimeType?: string;
    /** Access level */
    access: DcpAccessLevel;
}

/**
 * DCP access levels
 */
export type DcpAccessLevel = 'read' | 'write' | 'execute' | 'admin';

/**
 * DCP Performance metrics
 */
export interface DcpMetrics {
    /** Average latency in microseconds */
    avgLatencyUs: number;
    /** P99 latency in microseconds */
    p99LatencyUs: number;
    /** Messages per second */
    messagesPerSecond: number;
    /** Average message size in bytes */
    avgMessageSize: number;
    /** Total messages processed */
    totalMessages: number;
    /** Error count */
    errorCount: number;
}

/**
 * DCP Tool invocation result
 */
export interface DcpInvocationResult {
    /** Whether invocation succeeded */
    success: boolean;
    /** Result data */
    result?: any;
    /** Error message if failed */
    error?: string;
    /** Execution time in microseconds */
    timeUs?: number;
}

/**
 * MCP compatibility status
 */
export interface McpCompatibilityStatus {
    /** Whether MCP mode is available */
    available: boolean;
    /** MCP version supported */
    version?: string;
    /** Migration suggestions */
    suggestions: string[];
}

/**
 * DCP Configuration
 */
export interface DcpConfig {
    /** Default server port */
    port: number;
    /** Server mode */
    mode: DcpServerMode;
    /** Enable MCP compatibility */
    mcpCompat: boolean;
    /** Tool definitions path */
    toolsPath?: string;
    /** Enable metrics collection */
    metricsEnabled: boolean;
}
