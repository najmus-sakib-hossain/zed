/**
 * DCP Client implementation with Promise-based API.
 */

// Declare Node.js timer types for cross-environment compatibility
declare function setTimeout(callback: () => void, ms: number): number;
declare function clearTimeout(id: number): void;

import {
    Transport,
    DcpClientOptions,
    JsonRpcRequest,
    JsonRpcResponse,
    Tool,
    ToolCallResult,
    Resource,
    ResourceContent,
    ResourceTemplate,
    Prompt,
    PromptGetResult,
    InitializeResult,
    ResourcesListResult,
    RootsListResult,
    Root,
    ElicitationRequest,
    ElicitationResponse,
    CreateMessageParams,
    CreateMessageResult,
    CompleteParams,
    CompleteResult,
    ProtocolVersion,
    ProtocolVersionUtils,
    ServerCapabilities,
} from "./types";
import { DcpError, TimeoutError, ConnectionError } from "./errors";
import { TcpTransport, StdioTransport, SseTransport } from "./transport";

export interface DcpClientConfig {
    transport: Transport;
    timeout?: number;
    protocolVersion?: ProtocolVersion;
}

export class DcpClient {
    private transport: Transport;
    private timeout: number;
    private preferredVersion: ProtocolVersion;
    private negotiatedVersion?: ProtocolVersion;
    private requestId = 0;
    private pending = new Map<number | string, {
        resolve: (value: unknown) => void;
        reject: (error: Error) => void;
    }>();
    private notificationHandlers = new Map<string, (params: unknown) => void>();
    private initialized = false;
    private serverCapabilities: ServerCapabilities = {};
    private rootsChangedCallback?: (roots: Root[]) => void;

    constructor(options: DcpClientOptions | DcpClientConfig) {
        this.transport = options.transport;
        this.timeout = options.timeout ?? 30000;
        this.preferredVersion = (options as DcpClientConfig).protocolVersion ?? ProtocolVersion.V2025_06_18;
        this.setupMessageHandler();
    }

    private setupMessageHandler(): void {
        if (this.transport.onMessage) {
            this.transport.onMessage((message) => {
                this.handleMessage(message);
            });
        }
    }

    /**
     * Connect via TCP (Node.js).
     */
    static async connectTcp(
        host: string,
        port: number,
        timeout?: number,
        protocolVersion?: ProtocolVersion
    ): Promise<DcpClient> {
        const transport = await TcpTransport.connect(host, port, timeout);
        return new DcpClient({ transport, timeout, protocolVersion });
    }

    /**
     * Connect via stdio to a subprocess (Node.js).
     */
    static async connectStdio(
        command: string[],
        timeout?: number,
        protocolVersion?: ProtocolVersion
    ): Promise<DcpClient> {
        const transport = await StdioTransport.spawn(command);
        return new DcpClient({ transport, timeout, protocolVersion });
    }

    /**
     * Connect via SSE (browser).
     */
    static async connectSse(
        url: string,
        timeout?: number,
        protocolVersion?: ProtocolVersion
    ): Promise<DcpClient> {
        const transport = await SseTransport.connect(url, timeout);
        return new DcpClient({ transport, timeout, protocolVersion });
    }

    private handleMessage(message: string): void {
        let data: JsonRpcResponse;
        try {
            data = JSON.parse(message);
        } catch {
            return;
        }

        // Check if it's a response (has id)
        if (data.id !== undefined && data.id !== null) {
            const pending = this.pending.get(data.id);
            if (pending) {
                this.pending.delete(data.id);
                if (data.error) {
                    pending.reject(new DcpError(
                        data.error.message,
                        data.error.code,
                        data.error.data
                    ));
                } else {
                    pending.resolve(data.result);
                }
            }
        }

        // Check if it's a notification (no id, has method)
        const notification = data as unknown as JsonRpcRequest;
        if (notification.method && !("id" in data)) {
            const handler = this.notificationHandlers.get(notification.method);
            if (handler) {
                handler(notification.params);
            }
        }
    }

    private async request<T>(method: string, params?: unknown): Promise<T> {
        const id = ++this.requestId;

        const request: JsonRpcRequest = {
            jsonrpc: "2.0",
            id,
            method,
        };
        if (params !== undefined) {
            request.params = params;
        }

        return new Promise<T>((resolve, reject) => {
            const timeoutId = setTimeout(() => {
                this.pending.delete(id);
                reject(new TimeoutError(`Request ${method} timed out`));
            }, this.timeout);

            this.pending.set(id, {
                resolve: (value) => {
                    clearTimeout(timeoutId);
                    resolve(value as T);
                },
                reject: (error) => {
                    clearTimeout(timeoutId);
                    reject(error);
                },
            });

            this.transport.send(JSON.stringify(request)).catch((err) => {
                clearTimeout(timeoutId);
                this.pending.delete(id);
                reject(err);
            });
        });
    }

    private async notify(method: string, params?: unknown): Promise<void> {
        const notification: JsonRpcRequest = {
            jsonrpc: "2.0",
            method,
        };
        if (params !== undefined) {
            notification.params = params;
        }

        await this.transport.send(JSON.stringify(notification));
    }

    /**
     * Register a handler for notifications.
     */
    onNotification(method: string, handler: (params: unknown) => void): void {
        this.notificationHandlers.set(method, handler);
    }

    // ===========================================================================
    // Lifecycle Methods
    // ===========================================================================

    /**
     * Initialize connection and negotiate capabilities.
     */
    async initialize(): Promise<InitializeResult> {
        // Build capabilities based on preferred version
        const capabilities: Record<string, unknown> = {};
        if (ProtocolVersionUtils.supportsRoots(this.preferredVersion)) {
            capabilities.roots = { listChanged: true };
        }

        const result = await this.request<InitializeResult>("initialize", {
            protocolVersion: this.preferredVersion,
            capabilities,
            clientInfo: {
                name: "dcp-typescript",
                version: "0.1.0",
            },
        });

        // Parse negotiated version
        const negotiatedStr = result.protocolVersion || "2024-11-05";
        this.negotiatedVersion = ProtocolVersionUtils.fromString(negotiatedStr) ?? ProtocolVersion.V2024_11_05;

        this.serverCapabilities = result.capabilities;
        this.initialized = true;

        // Register internal notification handler for roots changes
        if (ProtocolVersionUtils.supportsRoots(this.negotiatedVersion)) {
            this.onNotification("notifications/roots/list_changed", async () => {
                if (this.rootsChangedCallback) {
                    const roots = await this.listRoots();
                    this.rootsChangedCallback(roots);
                }
            });
        }

        // Send initialized notification
        await this.notify("notifications/initialized");

        return result;
    }

    /**
     * Get the negotiated protocol version.
     */
    getNegotiatedVersion(): ProtocolVersion | undefined {
        return this.negotiatedVersion;
    }

    /**
     * Get server capabilities from initialization.
     */
    getServerCapabilities(): ServerCapabilities {
        return this.serverCapabilities;
    }

    // ===========================================================================
    // Tool Methods
    // ===========================================================================

    /**
     * List available tools.
     */
    async listTools(): Promise<Tool[]> {
        const result = await this.request<{ tools: Tool[] }>("tools/list", {});
        return result.tools;
    }

    /**
     * Call a tool by name.
     */
    async callTool(name: string, args?: Record<string, unknown>): Promise<ToolCallResult> {
        const params: { name: string; arguments?: Record<string, unknown> } = { name };
        if (args !== undefined) {
            params.arguments = args;
        }
        return this.request<ToolCallResult>("tools/call", params);
    }

    // ===========================================================================
    // Resource Methods
    // ===========================================================================

    /**
     * List available resources.
     */
    async listResources(cursor?: string): Promise<ResourcesListResult> {
        const params: { cursor?: string } = {};
        if (cursor !== undefined) {
            params.cursor = cursor;
        }
        return this.request<ResourcesListResult>("resources/list", params);
    }

    /**
     * Read a resource by URI.
     */
    async readResource(uri: string): Promise<{ contents: ResourceContent[] }> {
        return this.request<{ contents: ResourceContent[] }>("resources/read", { uri });
    }

    /**
     * Subscribe to resource changes.
     */
    async subscribeResource(uri: string): Promise<void> {
        await this.request("resources/subscribe", { uri });
    }

    /**
     * Unsubscribe from resource changes.
     */
    async unsubscribeResource(uri: string): Promise<void> {
        await this.request("resources/unsubscribe", { uri });
    }

    // ===========================================================================
    // Prompt Methods
    // ===========================================================================

    /**
     * List available prompts.
     */
    async listPrompts(): Promise<Prompt[]> {
        const result = await this.request<{ prompts: Prompt[] }>("prompts/list", {});
        return result.prompts;
    }

    /**
     * Get a prompt with arguments.
     */
    async getPrompt(name: string, args?: Record<string, string>): Promise<PromptGetResult> {
        const params: { name: string; arguments?: Record<string, string> } = { name };
        if (args !== undefined) {
            params.arguments = args;
        }
        return this.request<PromptGetResult>("prompts/get", params);
    }

    // ===========================================================================
    // Roots Methods (MCP 2025-03-26+)
    // ===========================================================================

    /**
     * List configured roots (filesystem boundaries).
     * Requires protocol version 2025-03-26 or later.
     */
    async listRoots(): Promise<Root[]> {
        if (this.negotiatedVersion && !ProtocolVersionUtils.supportsRoots(this.negotiatedVersion)) {
            throw new DcpError(
                "Roots not supported in negotiated protocol version",
                -32601,
                {
                    requiredVersion: "2025-03-26",
                    negotiatedVersion: this.negotiatedVersion
                }
            );
        }

        const result = await this.request<RootsListResult>("roots/list", {});
        return result.roots;
    }

    /**
     * Register a callback for roots list changes.
     * The callback will be invoked when the server emits a
     * notifications/roots/list_changed notification.
     */
    onRootsChanged(callback: (roots: Root[]) => void): void {
        this.rootsChangedCallback = callback;
    }

    // ===========================================================================
    // Elicitation Methods (MCP 2025-06-18+)
    // ===========================================================================

    /**
     * Register a handler for elicitation requests from the server.
     * Requires protocol version 2025-06-18 or later.
     */
    handleElicitation(
        handler: (request: ElicitationRequest) => ElicitationResponse | Promise<ElicitationResponse>
    ): void {
        if (this.negotiatedVersion && !ProtocolVersionUtils.supportsElicitation(this.negotiatedVersion)) {
            throw new DcpError(
                "Elicitation not supported in negotiated protocol version",
                -32601,
                {
                    requiredVersion: "2025-06-18",
                    negotiatedVersion: this.negotiatedVersion
                }
            );
        }

        this.onNotification("elicitation/create", async (params) => {
            const request = params as ElicitationRequest;
            const response = await handler(request);
            await this.notify("elicitation/respond", {
                action: response.action,
                content: response.content,
            });
        });
    }

    // ===========================================================================
    // Resource Template Methods (MCP 2025-03-26+)
    // ===========================================================================

    /**
     * List available resource templates.
     * Resource templates are included in the resources/list response
     * for protocol versions 2025-03-26 and later.
     */
    async listResourceTemplates(): Promise<ResourceTemplate[]> {
        const result = await this.request<ResourcesListResult>("resources/list", {});
        return result.resourceTemplates ?? [];
    }

    /**
     * Read a resource by substituting template parameters.
     */
    async readResourceTemplate(
        template: ResourceTemplate,
        params: Record<string, string>
    ): Promise<{ contents: ResourceContent[] }> {
        let uri = template.uriTemplate;
        for (const [key, value] of Object.entries(params)) {
            uri = uri.replace(`{${key}}`, value);
        }
        return this.readResource(uri);
    }

    // ===========================================================================
    // Logging Methods
    // ===========================================================================

    /**
     * Set the server log level.
     */
    async setLogLevel(level: string): Promise<void> {
        await this.request("logging/setLevel", { level });
    }

    // ===========================================================================
    // Sampling Methods
    // ===========================================================================

    /**
     * Create a message using LLM sampling.
     */
    async createMessage(params: CreateMessageParams): Promise<CreateMessageResult> {
        return this.request<CreateMessageResult>("sampling/createMessage", params);
    }

    // ===========================================================================
    // Completion Methods
    // ===========================================================================

    /**
     * Get completions for an argument.
     */
    async complete(params: CompleteParams): Promise<CompleteResult> {
        return this.request<CompleteResult>("completion/complete", params);
    }

    // ===========================================================================
    // Connection Management
    // ===========================================================================

    /**
     * Close the connection.
     */
    async close(): Promise<void> {
        await this.transport.close();
        this.initialized = false;
    }

    /**
     * Reconnect to the server.
     */
    async reconnect(): Promise<void> {
        await this.transport.close();
        await this.transport.connect();
        this.setupMessageHandler();

        if (this.initialized) {
            await this.initialize();
        }
    }

    /**
     * Check if client is connected.
     */
    get isConnected(): boolean {
        return this.transport.isConnected;
    }
}
