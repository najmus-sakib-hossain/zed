/**
 * Transport implementations for DCP client.
 */

import * as net from "net";
import { spawn, ChildProcess } from "child_process";
import { Transport } from "./types";
import { ConnectionError } from "./errors";

/**
 * TCP transport for Node.js.
 */
export class TcpTransport implements Transport {
    private socket: net.Socket | null = null;
    private buffer = "";
    private messageHandler?: (message: string) => void;
    private _isConnected = false;

    constructor(
        private readonly host: string,
        private readonly port: number,
        private readonly timeout: number = 30000
    ) { }

    static async connect(
        host: string,
        port: number,
        timeout?: number
    ): Promise<TcpTransport> {
        const transport = new TcpTransport(host, port, timeout);
        await transport.connect();
        return transport;
    }

    async connect(): Promise<void> {
        return new Promise((resolve, reject) => {
            const timeoutId = setTimeout(() => {
                reject(new ConnectionError(`Connection to ${this.host}:${this.port} timed out`));
            }, this.timeout);

            this.socket = net.createConnection({ host: this.host, port: this.port }, () => {
                clearTimeout(timeoutId);
                this._isConnected = true;
                resolve();
            });

            this.socket.on("error", (err) => {
                clearTimeout(timeoutId);
                reject(new ConnectionError(`Failed to connect: ${err.message}`));
            });

            this.socket.on("data", (data) => {
                this.buffer += data.toString();
                this.processBuffer();
            });

            this.socket.on("close", () => {
                this._isConnected = false;
            });
        });
    }

    private processBuffer(): void {
        const lines = this.buffer.split("\n");
        this.buffer = lines.pop() || "";

        for (const line of lines) {
            if (line.trim() && this.messageHandler) {
                this.messageHandler(line.trim());
            }
        }
    }

    async send(message: string): Promise<void> {
        if (!this._isConnected || !this.socket) {
            throw new ConnectionError("Not connected");
        }

        return new Promise((resolve, reject) => {
            this.socket!.write(message + "\n", (err) => {
                if (err) {
                    reject(new ConnectionError(`Failed to send: ${err.message}`));
                } else {
                    resolve();
                }
            });
        });
    }

    async receive(): Promise<string | null> {
        // For TCP, we use the message handler pattern
        return null;
    }

    async close(): Promise<void> {
        if (this.socket) {
            this.socket.destroy();
            this.socket = null;
        }
        this._isConnected = false;
    }

    get isConnected(): boolean {
        return this._isConnected;
    }

    onMessage(handler: (message: string) => void): void {
        this.messageHandler = handler;
    }
}

/**
 * Stdio transport for Node.js subprocess communication.
 */
export class StdioTransport implements Transport {
    private process: ChildProcess | null = null;
    private buffer = "";
    private messageHandler?: (message: string) => void;
    private _isConnected = false;

    constructor(private readonly command: string[]) { }

    static async spawn(command: string[]): Promise<StdioTransport> {
        const transport = new StdioTransport(command);
        await transport.connect();
        return transport;
    }

    async connect(): Promise<void> {
        const [cmd, ...args] = this.command;
        this.process = spawn(cmd, args, {
            stdio: ["pipe", "pipe", "pipe"],
        });

        this.process.stdout?.on("data", (data) => {
            this.buffer += data.toString();
            this.processBuffer();
        });

        this.process.on("error", (err) => {
            throw new ConnectionError(`Process error: ${err.message}`);
        });

        this.process.on("close", () => {
            this._isConnected = false;
        });

        this._isConnected = true;
    }

    private processBuffer(): void {
        const lines = this.buffer.split("\n");
        this.buffer = lines.pop() || "";

        for (const line of lines) {
            if (line.trim() && this.messageHandler) {
                this.messageHandler(line.trim());
            }
        }
    }

    async send(message: string): Promise<void> {
        if (!this._isConnected || !this.process?.stdin) {
            throw new ConnectionError("Not connected");
        }

        return new Promise((resolve, reject) => {
            this.process!.stdin!.write(message + "\n", (err) => {
                if (err) {
                    reject(new ConnectionError(`Failed to send: ${err.message}`));
                } else {
                    resolve();
                }
            });
        });
    }

    async receive(): Promise<string | null> {
        return null;
    }

    async close(): Promise<void> {
        if (this.process) {
            this.process.kill();
            this.process = null;
        }
        this._isConnected = false;
    }

    get isConnected(): boolean {
        return this._isConnected;
    }

    onMessage(handler: (message: string) => void): void {
        this.messageHandler = handler;
    }
}


/**
 * SSE transport for browser environments.
 */
export class SseTransport implements Transport {
    private eventSource: EventSource | null = null;
    private messageHandler?: (message: string) => void;
    private _isConnected = false;
    private lastEventId?: string;

    constructor(
        private readonly url: string,
        private readonly timeout: number = 30000
    ) { }

    static async connect(url: string, timeout?: number): Promise<SseTransport> {
        const transport = new SseTransport(url, timeout);
        await transport.connect();
        return transport;
    }

    async connect(): Promise<void> {
        return new Promise((resolve, reject) => {
            const eventsUrl = this.url.replace(/\/$/, "") + "/events";

            // Check if we're in a browser environment
            if (typeof EventSource === "undefined") {
                reject(new ConnectionError("SSE transport requires browser environment"));
                return;
            }

            this.eventSource = new EventSource(eventsUrl);

            const timeoutId = setTimeout(() => {
                reject(new ConnectionError("SSE connection timed out"));
            }, this.timeout);

            this.eventSource.onopen = () => {
                clearTimeout(timeoutId);
                this._isConnected = true;
                resolve();
            };

            this.eventSource.onerror = () => {
                clearTimeout(timeoutId);
                this._isConnected = false;
                reject(new ConnectionError("SSE connection failed"));
            };

            this.eventSource.onmessage = (event) => {
                if (event.lastEventId) {
                    this.lastEventId = event.lastEventId;
                }
                if (this.messageHandler) {
                    this.messageHandler(event.data);
                }
            };
        });
    }

    async send(message: string): Promise<void> {
        const postUrl = this.url.replace(/\/$/, "") + "/message";

        const response = await fetch(postUrl, {
            method: "POST",
            headers: {
                "Content-Type": "application/json",
            },
            body: message,
        });

        if (!response.ok) {
            throw new ConnectionError(`POST failed with status ${response.status}`);
        }
    }

    async receive(): Promise<string | null> {
        return null;
    }

    async close(): Promise<void> {
        if (this.eventSource) {
            this.eventSource.close();
            this.eventSource = null;
        }
        this._isConnected = false;
    }

    get isConnected(): boolean {
        return this._isConnected;
    }

    onMessage(handler: (message: string) => void): void {
        this.messageHandler = handler;
    }
}
