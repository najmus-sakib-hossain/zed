/**
 * DCP Client type definitions.
 */

// Protocol version enum
export enum ProtocolVersion {
    V2024_11_05 = "2024-11-05",
    V2025_03_26 = "2025-03-26",
    V2025_06_18 = "2025-06-18",
}

// Protocol version utilities
export const ProtocolVersionUtils = {
    fromString(version: string): ProtocolVersion | undefined {
        const versions: Record<string, ProtocolVersion> = {
            "2024-11-05": ProtocolVersion.V2024_11_05,
            "2025-03-26": ProtocolVersion.V2025_03_26,
            "2025-06-18": ProtocolVersion.V2025_06_18,
        };
        return versions[version];
    },

    supportsRoots(version: ProtocolVersion): boolean {
        return version === ProtocolVersion.V2025_03_26 ||
            version === ProtocolVersion.V2025_06_18;
    },

    supportsElicitation(version: ProtocolVersion): boolean {
        return version === ProtocolVersion.V2025_06_18;
    },
};

// JSON-RPC types
export interface JsonRpcRequest {
    jsonrpc: "2.0";
    id?: number | string;
    method: string;
    params?: unknown;
}

export interface JsonRpcResponse {
    jsonrpc: "2.0";
    id: number | string | null;
    result?: unknown;
    error?: JsonRpcError;
}

export interface JsonRpcError {
    code: number;
    message: string;
    data?: unknown;
}

// MCP types
export interface Tool {
    name: string;
    description?: string;
    inputSchema?: Record<string, unknown>;
}

export interface ToolCallResult {
    content: ContentItem[];
    isError?: boolean;
}

export interface ContentItem {
    type: "text" | "image" | "resource";
    text?: string;
    data?: string;
    mimeType?: string;
    uri?: string;
    annotations?: Annotations;
}

// Annotations for enhanced structured output (MCP 2025-06-18+)
export interface Annotations {
    audience?: string[];
    priority?: number;
}

export interface Resource {
    uri: string;
    name: string;
    description?: string;
    mimeType?: string;
}

export interface ResourceContent {
    uri: string;
    mimeType?: string;
    text?: string;
    blob?: string;
}

// Resource Template (MCP 2025-03-26+)
export interface ResourceTemplate {
    uriTemplate: string;
    name: string;
    description?: string;
    mimeType?: string;
}

// Root definition (MCP 2025-03-26+)
export interface Root {
    uri: string;
    name?: string;
}

// Elicitation types (MCP 2025-06-18+)
export interface ElicitationRequest {
    message: string;
    requestedSchema?: ElicitationSchema;
}

export interface ElicitationSchema {
    type: string;
    properties?: Record<string, PropertySchema>;
    required?: string[];
}

export interface PropertySchema {
    type: string;
    description?: string;
    format?: string;
    minimum?: number;
    maximum?: number;
    enum?: string[];
}

export type ElicitationAction = "accept" | "decline" | "cancel";

export interface ElicitationResponse {
    action: ElicitationAction;
    content?: Record<string, unknown>;
}

export interface Prompt {
    name: string;
    description?: string;
    arguments?: PromptArgument[];
}

export interface PromptArgument {
    name: string;
    description?: string;
    required?: boolean;
}

export interface PromptMessage {
    role: "user" | "assistant";
    content: ContentItem;
}


// Initialize types
export interface InitializeParams {
    protocolVersion: string;
    capabilities: ClientCapabilities;
    clientInfo: ClientInfo;
}

export interface InitializeResult {
    protocolVersion: string;
    capabilities: ServerCapabilities;
    serverInfo: ServerInfo;
}

export interface ClientCapabilities {
    roots?: { listChanged?: boolean };
    sampling?: Record<string, unknown>;
    elicitation?: Record<string, unknown>;
}

export interface ServerCapabilities {
    tools?: { listChanged?: boolean };
    resources?: { subscribe?: boolean; listChanged?: boolean };
    prompts?: { listChanged?: boolean };
    logging?: Record<string, unknown>;
    roots?: { listChanged?: boolean };
    elicitation?: Record<string, unknown>;
}

export interface ClientInfo {
    name: string;
    version: string;
}

export interface ServerInfo {
    name: string;
    version: string;
}

// List results
export interface ToolsListResult {
    tools: Tool[];
}

export interface ResourcesListResult {
    resources: Resource[];
    resourceTemplates?: ResourceTemplate[];
    nextCursor?: string;
}

export interface RootsListResult {
    roots: Root[];
}

export interface PromptsListResult {
    prompts: Prompt[];
}

export interface PromptGetResult {
    description?: string;
    messages: PromptMessage[];
}

// Sampling types
export interface SamplingMessage {
    role: "user" | "assistant";
    content: ContentItem;
}

export interface CreateMessageParams {
    messages: SamplingMessage[];
    modelPreferences?: ModelPreferences;
    systemPrompt?: string;
    maxTokens: number;
}

export interface ModelPreferences {
    hints?: ModelHint[];
    costPriority?: number;
    speedPriority?: number;
    intelligencePriority?: number;
}

export interface ModelHint {
    name?: string;
}

export interface CreateMessageResult {
    role: "assistant";
    content: ContentItem;
    model: string;
    stopReason?: string;
}

// Completion types
export interface CompleteParams {
    ref: { type: string; name: string };
    argument: { name: string; value: string };
}

export interface CompleteResult {
    completion: {
        values: string[];
        total?: number;
        hasMore?: boolean;
    };
}

// Transport interface
export interface Transport {
    connect(): Promise<void>;
    send(message: string): Promise<void>;
    receive(): Promise<string | null>;
    close(): Promise<void>;
    readonly isConnected: boolean;
    onMessage?(handler: (message: string) => void): void;
}

// Client options
export interface DcpClientOptions {
    transport: Transport;
    timeout?: number;
}
