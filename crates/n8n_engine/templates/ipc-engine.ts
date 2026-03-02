/**
 * n8n IPC Engine Entry Point
 *
 * This file should be added to your forked n8n project at:
 * packages/cli/src/ipc-engine.ts
 *
 * It exposes n8n's execution engine via IPC (Unix Domain Socket on Unix,
 * TCP on Windows) instead of the normal HTTP server.
 *
 * Usage from Rust:
 * 1. Set environment variables:
 *    - N8N_IPC_SOCKET: Path to Unix socket (Unix) or "host:port" (Windows)
 *    - N8N_EXECUTION_MODE: "ipc"
 *    - DB_TYPE: "sqlite" or "postgres"
 *    - DB_SQLITE_DATABASE: Path to SQLite database file
 *
 * 2. Start with: node dist/ipc-engine.js
 *
 * 3. Connect via Unix socket or TCP and send JSON-delimited messages
 */

import * as net from 'net';

// These imports would come from your n8n fork
// Uncomment and adjust paths as needed for your n8n version:
//
// import { WorkflowExecute } from '@n8n/core';
// import { Workflow } from 'n8n-workflow';
// import type {
//     INodeTypes,
//     IWorkflowExecuteAdditionalData,
//     IRun,
// } from 'n8n-workflow';
// import { Container } from '@n8n/di';
// import { NodeTypes } from '@/node-types';
// import { CredentialsHelper } from '@/credentials-helper';
// import { LoadNodesAndCredentials } from '@/load-nodes-and-credentials';
// import { Logger } from '@/logging/logger.service';

interface IpcMessage {
    id: string;
    type: 'execute' | 'execute_async' | 'get_status' | 'stop' | 'shutdown';
    payload: any;
}

interface IpcResponse {
    id: string;
    type: 'result' | 'error' | 'ack' | 'async_result';
    payload: any;
}

/**
 * The n8n IPC Engine
 *
 * This class manages the n8n execution engine and exposes it via IPC.
 */
class N8nIpcEngine {
    private activeExecutions: Map<string, any> = new Map();
    // private nodeTypes!: INodeTypes;

    constructor() {
        console.log('[n8n IPC Engine] Initializing...');
    }

    /**
     * Initialize the n8n engine - load all nodes and credentials
     */
    async initialize(): Promise<void> {
        // In a real implementation, you would:
        // 1. Load all n8n nodes and credentials
        // 2. Initialize the database connection
        // 3. Set up the credential helper
        //
        // Example (uncomment for real n8n integration):
        //
        // const loader = Container.get(LoadNodesAndCredentials);
        // await loader.init();
        // this.nodeTypes = Container.get(NodeTypes);

        console.log('[n8n IPC Engine] Initialized (mock mode - replace with real n8n)');
    }

    /**
     * Execute a workflow synchronously
     */
    async executeWorkflow(message: IpcMessage): Promise<IpcResponse> {
        const { workflow: workflowData, input_data } = message.payload;
        const startTime = Date.now();

        try {
            // In a real implementation, you would:
            // 1. Create a Workflow instance from the JSON
            // 2. Set up execution context
            // 3. Run the workflow using WorkflowExecute
            //
            // Example (uncomment for real n8n integration):
            //
            // const workflow = new Workflow({
            //     id: workflowData.id || 'ipc-' + message.id,
            //     name: workflowData.name,
            //     nodes: workflowData.nodes,
            //     connections: workflowData.connections,
            //     active: false,
            //     nodeTypes: this.nodeTypes,
            //     settings: workflowData.settings || {},
            // });
            //
            // const additionalData: Partial<IWorkflowExecuteAdditionalData> = {
            //     credentialsHelper: Container.get(CredentialsHelper),
            // };
            //
            // const workflowExecute = new WorkflowExecute(
            //     additionalData as IWorkflowExecuteAdditionalData,
            //     'integrated',
            // );
            //
            // const runData = await workflowExecute.run(workflow);

            // Mock execution for development/testing
            console.log(`[n8n IPC Engine] Executing workflow: ${workflowData.name}`);

            // Simulate some processing time
            await new Promise(resolve => setTimeout(resolve, 50));

            const executionTime = Date.now() - startTime;

            return {
                id: message.id,
                type: 'result',
                payload: {
                    execution_id: message.id,
                    status: 'success',
                    data: {
                        mock: true,
                        workflow_name: workflowData.name,
                        nodes_count: workflowData.nodes?.length || 0,
                        input_data: input_data,
                        executed_at: new Date().toISOString(),
                    },
                    execution_time_ms: executionTime,
                },
            };
        } catch (error: any) {
            return {
                id: message.id,
                type: 'error',
                payload: {
                    execution_id: message.id,
                    status: 'error',
                    data: {
                        error: error.message,
                        stack: error.stack,
                    },
                    execution_time_ms: Date.now() - startTime,
                },
            };
        }
    }

    /**
     * Execute a workflow asynchronously (fire-and-forget)
     */
    async executeWorkflowAsync(message: IpcMessage, socket: net.Socket): Promise<void> {
        // Send immediate acknowledgment
        const ackResponse: IpcResponse = {
            id: message.id,
            type: 'ack',
            payload: {
                execution_id: message.id,
                status: 'queued',
            },
        };
        socket.write(JSON.stringify(ackResponse) + '\n');

        // Execute in background
        this.executeWorkflow(message).then(result => {
            const asyncResult: IpcResponse = {
                ...result,
                type: 'async_result',
            };
            socket.write(JSON.stringify(asyncResult) + '\n');
        });
    }

    /**
     * Get the status of an execution
     */
    getExecutionStatus(executionId: string): IpcResponse {
        const execution = this.activeExecutions.get(executionId);

        if (!execution) {
            return {
                id: executionId,
                type: 'result',
                payload: {
                    execution_id: executionId,
                    status: 'not_found',
                    data: null,
                    execution_time_ms: 0,
                },
            };
        }

        return {
            id: executionId,
            type: 'result',
            payload: execution,
        };
    }

    /**
     * Stop a running execution
     */
    stopExecution(executionId: string): IpcResponse {
        this.activeExecutions.delete(executionId);

        return {
            id: executionId,
            type: 'result',
            payload: {
                execution_id: executionId,
                status: 'stopped',
            },
        };
    }

    /**
     * Start the IPC server
     */
    async startIpcServer(): Promise<void> {
        const socketPath = process.env.N8N_IPC_SOCKET || '/tmp/n8n-engine.sock';
        const isWindows = process.platform === 'win32';

        let server: net.Server;

        if (isWindows) {
            // On Windows, use TCP
            const [host, portStr] = socketPath.includes(':')
                ? socketPath.split(':')
                : ['127.0.0.1', socketPath];
            const port = parseInt(portStr, 10) || 58765;

            server = net.createServer(socket => this.handleConnection(socket));
            server.listen(port, host, () => {
                console.log(`[n8n IPC Engine] Listening on ${host}:${port}`);
            });
        } else {
            // On Unix, use Unix Domain Socket
            // Clean up old socket file if it exists
            try {
                const fs = await import('fs');
                if (fs.existsSync(socketPath)) {
                    fs.unlinkSync(socketPath);
                }
            } catch {
                // Ignore errors
            }

            server = net.createServer(socket => this.handleConnection(socket));
            server.listen(socketPath, () => {
                console.log(`[n8n IPC Engine] Listening on ${socketPath}`);
            });
        }

        // Handle server errors
        server.on('error', (err) => {
            console.error('[n8n IPC Engine] Server error:', err);
            process.exit(1);
        });
    }

    /**
     * Handle a new client connection
     */
    private handleConnection(socket: net.Socket): void {
        console.log('[n8n IPC Engine] Client connected');

        let buffer = '';

        socket.on('data', async (data) => {
            buffer += data.toString();

            // Process complete JSON messages (newline-delimited)
            const lines = buffer.split('\n');
            buffer = lines.pop() || '';

            for (const line of lines) {
                if (!line.trim()) continue;

                try {
                    const message: IpcMessage = JSON.parse(line);
                    await this.handleMessage(message, socket);
                } catch (err: any) {
                    const errorResponse: IpcResponse = {
                        id: 'error',
                        type: 'error',
                        payload: { error: err.message },
                    };
                    socket.write(JSON.stringify(errorResponse) + '\n');
                }
            }
        });

        socket.on('end', () => {
            console.log('[n8n IPC Engine] Client disconnected');
        });

        socket.on('error', (err) => {
            console.error('[n8n IPC Engine] Socket error:', err);
        });
    }

    /**
     * Handle an incoming IPC message
     */
    private async handleMessage(message: IpcMessage, socket: net.Socket): Promise<void> {
        console.log(`[n8n IPC Engine] Received message: ${message.type}`);

        switch (message.type) {
            case 'execute': {
                const result = await this.executeWorkflow(message);
                socket.write(JSON.stringify(result) + '\n');
                break;
            }

            case 'execute_async': {
                await this.executeWorkflowAsync(message, socket);
                break;
            }

            case 'get_status': {
                const status = this.getExecutionStatus(message.payload.execution_id);
                socket.write(JSON.stringify(status) + '\n');
                break;
            }

            case 'stop': {
                const stopped = this.stopExecution(message.payload.execution_id);
                socket.write(JSON.stringify(stopped) + '\n');
                break;
            }

            case 'shutdown': {
                console.log('[n8n IPC Engine] Shutting down...');
                socket.end();
                process.exit(0);
                break;
            }

            default: {
                const errorResponse: IpcResponse = {
                    id: message.id,
                    type: 'error',
                    payload: { error: `Unknown message type: ${message.type}` },
                };
                socket.write(JSON.stringify(errorResponse) + '\n');
            }
        }
    }
}

// Boot the engine
(async () => {
    try {
        const engine = new N8nIpcEngine();
        await engine.initialize();
        await engine.startIpcServer();
    } catch (error) {
        console.error('[n8n IPC Engine] Failed to start:', error);
        process.exit(1);
    }
})();
