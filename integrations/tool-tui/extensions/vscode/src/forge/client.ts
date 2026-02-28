/**
 * Forge Client - WebSocket connection to Forge daemon
 */

import * as vscode from 'vscode';
import WebSocket from 'ws';

// Types for Forge communication
export interface ForgeStatus {
    state: string;
    uptime_seconds: number;
    files_changed: number;
    tools_executed: number;
    cache_hits: number;
    errors: number;
}

export interface ToolInfo {
    id: string;
    name: string;
    version: string;
    status: string;
    is_dummy: boolean;
    run_count: number;
    error_count: number;
}

export interface DiffHunk {
    old_start: number;
    old_lines: number;
    new_start: number;
    new_lines: number;
    content: string;
}

export interface FileDiff {
    additions: number;
    deletions: number;
    hunks: DiffHunk[];
}

export interface TrackedFileChange {
    path: string;
    change_type: string;
    timestamp: string;
    diff?: FileDiff;
}

export interface GitStatusResponse {
    is_clean: boolean;
    branch: string;
    staged: GitFileStatus[];
    unstaged: GitFileStatus[];
    untracked: string[];
}

export interface GitFileStatus {
    path: string;
    status: string;
    diff?: FileDiff;
}

export interface ForgeEvent {
    type: 'file_changed' | 'tool_started' | 'tool_completed' | 'tool_failed' | 'error';
    data: any;
}

type EventHandler = (event: ForgeEvent) => void;

/**
 * Client for communicating with the Forge daemon
 */
export class ForgeClient {
    private ws: WebSocket | null = null;
    private reconnectAttempts = 0;
    private maxReconnectAttempts = 5;
    private reconnectDelay = 1000;
    private eventHandlers: EventHandler[] = [];
    private connected = false;
    private connecting = false;
    private port: number;

    constructor() {
        const config = vscode.workspace.getConfiguration('dx.forge');
        this.port = config.get('port', 9876);
    }

    /**
     * Check if connected to daemon
     */
    isConnected(): boolean {
        return this.connected && this.ws !== null && this.ws.readyState === WebSocket.OPEN;
    }

    /**
     * Connect to the Forge daemon
     */
    async connect(): Promise<boolean> {
        // Prevent multiple simultaneous connection attempts
        if (this.connecting) {
            console.log('[Forge] Connection already in progress');
            return false;
        }

        if (this.isConnected()) {
            return true;
        }

        this.connecting = true;

        return new Promise((resolve) => {
            try {
                // Connect directly to the WebSocket port (no path needed)
                const url = `ws://127.0.0.1:${this.port}`;
                console.log(`[Forge] Connecting to ${url}...`);
                this.ws = new WebSocket(url);

                // Set a connection timeout
                const timeout = setTimeout(() => {
                    console.log('[Forge] Connection timeout');
                    this.connecting = false;
                    if (this.ws) {
                        this.ws.close();
                        this.ws = null;
                    }
                    resolve(false);
                }, 5000);

                this.ws.on('open', () => {
                    clearTimeout(timeout);
                    this.connected = true;
                    this.connecting = false;
                    this.reconnectAttempts = 0;
                    console.log('[Forge] Connected to daemon');
                    resolve(true);
                });

                this.ws.on('close', () => {
                    clearTimeout(timeout);
                    this.connected = false;
                    this.connecting = false;
                    console.log('[Forge] Disconnected from daemon');
                    // Don't auto-reconnect - let the status bar handle showing disconnected state
                    // User can manually start the daemon if needed
                });

                this.ws.on('error', (error) => {
                    clearTimeout(timeout);
                    console.error('[Forge] WebSocket error:', error.message);
                    this.connected = false;
                    this.connecting = false;
                    resolve(false);
                });

                this.ws.on('message', (data) => {
                    try {
                        const parsed = JSON.parse(data.toString());
                        this.handleMessage(parsed);
                    } catch (e) {
                        console.error('[Forge] Failed to parse message:', e);
                    }
                });
            } catch (error) {
                console.error('[Forge] Failed to connect:', error);
                resolve(false);
            }
        });
    }

    /**
     * Disconnect from the daemon
     */
    async disconnect(): Promise<void> {
        if (this.ws) {
            this.ws.close();
            this.ws = null;
        }
        this.connected = false;
    }

    /**
     * Attempt to reconnect with exponential backoff
     */
    private attemptReconnect(): void {
        if (this.reconnectAttempts >= this.maxReconnectAttempts) {
            console.log('[Forge] Max reconnect attempts reached');
            return;
        }

        this.reconnectAttempts++;
        const delay = this.reconnectDelay * Math.pow(2, this.reconnectAttempts - 1);

        console.log(`[Forge] Reconnecting in ${delay}ms (attempt ${this.reconnectAttempts})`);

        setTimeout(() => {
            this.connect();
        }, delay);
    }

    /**
     * Handle incoming messages
     */
    private handleMessage(data: any): void {
        const event: ForgeEvent = {
            type: data.type || 'error',
            data: data.data || data
        };

        for (const handler of this.eventHandlers) {
            try {
                handler(event);
            } catch (e) {
                console.error('[Forge] Event handler error:', e);
            }
        }
    }

    /**
     * Subscribe to events
     */
    onEvent(handler: EventHandler): void {
        this.eventHandlers.push(handler);
    }

    /**
     * Send a command to the daemon and wait for response
     */
    private send(command: any): Promise<any> {
        return new Promise((resolve, reject) => {
            if (!this.isConnected() || !this.ws) {
                reject(new Error('Not connected to Forge daemon'));
                return;
            }

            // Set up a one-time message handler for the response
            const timeout = setTimeout(() => {
                this.ws?.off('message', responseHandler);
                reject(new Error('Request timeout'));
            }, 5000);

            const responseHandler = (data: WebSocket.Data) => {
                clearTimeout(timeout);
                this.ws?.off('message', responseHandler);
                try {
                    const parsed = JSON.parse(data.toString());
                    resolve(parsed);
                } catch (e) {
                    reject(e);
                }
            };

            this.ws.on('message', responseHandler);

            try {
                this.ws.send(JSON.stringify(command));
            } catch (error) {
                clearTimeout(timeout);
                this.ws.off('message', responseHandler);
                reject(error);
            }
        });
    }

    /**
     * Get daemon status
     */
    async getStatus(): Promise<ForgeStatus | null> {
        try {
            const response = await this.send({ command: 'GetStatus' });
            if (response.type === 'Status') {
                return {
                    state: response.state,
                    uptime_seconds: response.uptime_seconds,
                    files_changed: response.files_changed,
                    tools_executed: response.tools_executed,
                    cache_hits: response.cache_hits,
                    errors: response.errors,
                };
            }
            return null;
        } catch (e) {
            console.error('[Forge] Failed to get status:', e);
            return null;
        }
    }

    /**
     * List all tools
     */
    async listTools(): Promise<ToolInfo[]> {
        try {
            const response = await this.send({ command: 'ListTools' });
            if (response.type === 'ToolList' && response.tools) {
                return response.tools.map((t: any) => ({
                    id: t.id || t.name,
                    name: t.name,
                    version: t.version,
                    status: t.status,
                    is_dummy: t.is_dummy,
                    run_count: t.run_count,
                    error_count: t.error_count,
                }));
            }
            return [];
        } catch (e) {
            console.error('[Forge] Failed to list tools:', e);
            return [];
        }
    }

    /**
     * Run a specific tool
     */
    async runTool(name: string): Promise<boolean> {
        try {
            const response = await this.send({ command: 'RunTool', name, args: [] });
            return response.type === 'ToolResult' && response.success;
        } catch {
            return false;
        }
    }

    /**
     * Notify daemon of file change
     */
    async notifyFileChange(filePath: string, changeType: 'created' | 'modified' | 'deleted'): Promise<TrackedFileChange | null> {
        try {
            const response = await this.send({
                command: 'FileChanged',
                path: filePath,
                change_type: changeType
            });
            if (response.type === 'FileChangeEvent') {
                return response as TrackedFileChange;
            }
            return null;
        } catch (e) {
            console.error('[Forge] Failed to notify file change:', e);
            return null;
        }
    }

    /**
     * Get recent file changes with diffs
     */
    async getFileChanges(limit?: number): Promise<TrackedFileChange[]> {
        try {
            const response = await this.send({
                command: 'GetFileChanges',
                limit: limit || 50
            });
            if (response.type === 'FileChanges' && response.changes) {
                return response.changes;
            }
            return [];
        } catch (e) {
            console.error('[Forge] Failed to get file changes:', e);
            return [];
        }
    }

    /**
     * Clear tracked file changes
     */
    async clearFileChanges(): Promise<boolean> {
        try {
            const response = await this.send({ command: 'ClearFileChanges' });
            return response.type === 'Success';
        } catch {
            return false;
        }
    }

    /**
     * Get Git status (actual uncommitted changes)
     */
    async getGitStatus(): Promise<GitStatusResponse | null> {
        try {
            const workspacePath = vscode.workspace.workspaceFolders?.[0]?.uri.fsPath;
            const response = await this.send({
                command: 'GetGitStatus',
                workspace_path: workspacePath
            });
            if (response.type === 'GitStatus') {
                return response as GitStatusResponse;
            }
            console.error('[Forge] Unexpected response type:', response.type, response);
            return null;
        } catch (e) {
            console.error('[Forge] Failed to get git status:', e);
            return null;
        }
    }

    /**
     * Sync Forge with Git status (reset counters to match Git)
     */
    async syncWithGit(): Promise<GitStatusResponse | null> {
        try {
            const workspacePath = vscode.workspace.workspaceFolders?.[0]?.uri.fsPath;
            const response = await this.send({
                command: 'SyncWithGit',
                workspace_path: workspacePath
            });
            if (response.type === 'GitStatus') {
                return response as GitStatusResponse;
            }
            console.error('[Forge] Unexpected response type:', response.type, response);
            return null;
        } catch (e) {
            console.error('[Forge] Failed to sync with git:', e);
            return null;
        }
    }
}

/**
 * Singleton instance
 */
let forgeClientInstance: ForgeClient | null = null;

export function getForgeClient(): ForgeClient {
    if (!forgeClientInstance) {
        forgeClientInstance = new ForgeClient();
    }
    return forgeClientInstance;
}
