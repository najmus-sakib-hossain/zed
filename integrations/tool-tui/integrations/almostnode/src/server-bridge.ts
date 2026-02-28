/**
 * Server Bridge
 * Connects Service Worker requests to virtual HTTP servers
 */

import {
  Server,
  ResponseData,
  setServerListenCallback,
  setServerCloseCallback,
  getServer,
} from './shims/http';
import { EventEmitter } from './shims/events';
import { Buffer } from './shims/stream';
import { uint8ToBase64 } from './utils/binary-encoding';

const _encoder = new TextEncoder();

/**
 * Interface for virtual servers that can be registered with the bridge
 */
export interface IVirtualServer {
  listening: boolean;
  address(): { port: number; address: string; family: string } | null;
  handleRequest(
    method: string,
    url: string,
    headers: Record<string, string>,
    body?: Buffer | string
  ): Promise<ResponseData>;
}

export interface VirtualServer {
  server: Server | IVirtualServer;
  port: number;
  hostname: string;
}

export interface BridgeOptions {
  baseUrl?: string;
  onServerReady?: (port: number, url: string) => void;
}

export interface InitServiceWorkerOptions {
  /**
   * The URL path to the service worker file
   * @default '/__sw__.js'
   */
  swUrl?: string;
}

/**
 * Server Bridge manages virtual HTTP servers and routes requests
 */
export class ServerBridge extends EventEmitter {
  static DEBUG = false;
  private servers: Map<number, VirtualServer> = new Map();
  private baseUrl: string;
  private options: BridgeOptions;
  private messageChannel: MessageChannel | null = null;
  private serviceWorkerReady: boolean = false;
  private keepaliveInterval: ReturnType<typeof setInterval> | null = null;

  constructor(options: BridgeOptions = {}) {
    super();
    this.options = options;

    // Handle browser vs Node.js environment
    if (typeof location !== 'undefined') {
      this.baseUrl = options.baseUrl || `${location.protocol}//${location.host}`;
    } else {
      this.baseUrl = options.baseUrl || 'http://localhost';
    }

    // Set up auto-registration from http module
    setServerListenCallback((port, server) => {
      this.registerServer(server, port);
    });

    setServerCloseCallback((port) => {
      this.unregisterServer(port);
    });
  }

  /**
   * Register a server on a port
   */
  registerServer(server: Server | IVirtualServer, port: number, hostname: string = '0.0.0.0'): void {
    this.servers.set(port, { server, port, hostname });

    // Emit server-ready event
    const url = this.getServerUrl(port);
    this.emit('server-ready', port, url);

    if (this.options.onServerReady) {
      this.options.onServerReady(port, url);
    }

    // Notify service worker if connected
    this.notifyServiceWorker('server-registered', { port, hostname });
  }

  /**
   * Unregister a server
   */
  unregisterServer(port: number): void {
    this.servers.delete(port);
    this.notifyServiceWorker('server-unregistered', { port });
  }

  /**
   * Get server URL for a port
   */
  getServerUrl(port: number): string {
    return `${this.baseUrl}/__virtual__/${port}`;
  }

  /**
   * Get all registered server ports
   */
  getServerPorts(): number[] {
    return [...this.servers.keys()];
  }

  /**
   * Handle an incoming request from Service Worker
   */
  async handleRequest(
    port: number,
    method: string,
    url: string,
    headers: Record<string, string>,
    body?: ArrayBuffer
  ): Promise<ResponseData> {
    const virtualServer = this.servers.get(port);

    if (!virtualServer) {
      return {
        statusCode: 503,
        statusMessage: 'Service Unavailable',
        headers: { 'Content-Type': 'text/plain' },
        body: Buffer.from(`No server listening on port ${port}`),
      };
    }

    try {
      const bodyBuffer = body ? Buffer.from(new Uint8Array(body)) : undefined;
      return await virtualServer.server.handleRequest(method, url, headers, bodyBuffer);
    } catch (error) {
      const message = error instanceof Error ? error.message : 'Internal Server Error';
      return {
        statusCode: 500,
        statusMessage: 'Internal Server Error',
        headers: { 'Content-Type': 'text/plain' },
        body: Buffer.from(message),
      };
    }
  }

  /**
   * Initialize Service Worker communication
   * @param options - Configuration options for the service worker
   * @param options.swUrl - Custom URL path to the service worker file (default: '/__sw__.js')
   */
  async initServiceWorker(options?: InitServiceWorkerOptions): Promise<void> {
    if (!('serviceWorker' in navigator)) {
      throw new Error('Service Workers not supported');
    }

    const swUrl = options?.swUrl ?? '/__sw__.js';

    // Set up controllerchange listener BEFORE registration so we don't miss the event.
    // clients.claim() in the SW's activate handler fires controllerchange, and it can
    // happen before our activation wait completes.
    const controllerReady = navigator.serviceWorker.controller
      ? Promise.resolve()
      : new Promise<void>((resolve) => {
          navigator.serviceWorker.addEventListener('controllerchange', () => resolve(), { once: true });
        });

    // Register service worker
    const registration = await navigator.serviceWorker.register(swUrl, {
      scope: '/',
    });

    // Wait for service worker to be active
    const sw = registration.active || registration.waiting || registration.installing;

    if (!sw) {
      throw new Error('Service Worker registration failed');
    }

    await new Promise<void>((resolve) => {
      if (sw.state === 'activated') {
        resolve();
      } else {
        const handler = () => {
          if (sw.state === 'activated') {
            sw.removeEventListener('statechange', handler);
            resolve();
          }
        };
        sw.addEventListener('statechange', handler);
      }
    });

    // Set up message channel for communication
    this.messageChannel = new MessageChannel();
    this.messageChannel.port1.onmessage = this.handleServiceWorkerMessage.bind(this);

    // Send port to service worker
    sw.postMessage({ type: 'init', port: this.messageChannel.port2 }, [
      this.messageChannel.port2,
    ]);

    // Wait for SW to actually control this page (clients.claim() in SW activate handler)
    // Without this, fetch requests bypass the SW and go directly to the server
    await controllerReady;

    // Re-establish communication when the SW loses its port (idle termination)
    // or when the SW is replaced (new deployment). The SW sends 'sw-needs-init'
    // to all clients when a request arrives but mainPort is null.
    const reinit = () => {
      if (navigator.serviceWorker.controller) {
        this.messageChannel = new MessageChannel();
        this.messageChannel.port1.onmessage = this.handleServiceWorkerMessage.bind(this);
        navigator.serviceWorker.controller.postMessage(
          { type: 'init', port: this.messageChannel.port2 },
          [this.messageChannel.port2]
        );
      }
    };
    navigator.serviceWorker.addEventListener('controllerchange', reinit);
    navigator.serviceWorker.addEventListener('message', (event) => {
      if (event.data?.type === 'sw-needs-init') {
        reinit();
      }
    });

    // Keep the SW alive with periodic pings. Browsers terminate idle SWs
    // after ~30s, losing the MessageChannel port and all in-memory state.
    this.keepaliveInterval = setInterval(() => {
      this.messageChannel?.port1.postMessage({ type: 'keepalive' });
    }, 20_000);

    this.serviceWorkerReady = true;
    this.emit('sw-ready');
  }

  /**
   * Handle messages from Service Worker
   */
  private async handleServiceWorkerMessage(event: MessageEvent): Promise<void> {
    const { type, id, data } = event.data;

    ServerBridge.DEBUG && console.log('[ServerBridge] SW message:', type, id, data?.url);

    if (type === 'request') {
      const { port, method, url, headers, body, streaming } = data;

      ServerBridge.DEBUG && console.log('[ServerBridge] Handling request:', port, method, url, 'streaming:', streaming);
      if (streaming) {
        ServerBridge.DEBUG && console.log('[ServerBridge] 🔴 Will use streaming handler');
      }

      try {
        if (streaming) {
          // Handle streaming request
          await this.handleStreamingRequest(id, port, method, url, headers, body);
        } else {
          // Handle regular request
          const response = await this.handleRequest(port, method, url, headers, body);
          ServerBridge.DEBUG && console.log('[ServerBridge] Response:', response.statusCode, 'body length:', response.body?.length);

          // Convert body to base64 string to avoid structured cloning issues with Uint8Array
          let bodyBase64 = '';
          if (response.body && response.body.length > 0) {
            const bytes = response.body instanceof Uint8Array ? response.body : new Uint8Array(0);
            bodyBase64 = uint8ToBase64(bytes);
          }

          ServerBridge.DEBUG && console.log('[ServerBridge] Sending response to SW, body base64 length:', bodyBase64.length);

          this.messageChannel?.port1.postMessage({
            type: 'response',
            id,
            data: {
              statusCode: response.statusCode,
              statusMessage: response.statusMessage,
              headers: response.headers,
              bodyBase64: bodyBase64,
            },
          });
        }
      } catch (error) {
        this.messageChannel?.port1.postMessage({
          type: 'response',
          id,
          error: error instanceof Error ? error.message : 'Unknown error',
        });
      }
    }
  }

  /**
   * Handle a streaming request - sends chunks as they arrive
   */
  private async handleStreamingRequest(
    id: number,
    port: number,
    method: string,
    url: string,
    headers: Record<string, string>,
    body?: ArrayBuffer
  ): Promise<void> {
    const virtualServer = this.servers.get(port);

    if (!virtualServer) {
      this.messageChannel?.port1.postMessage({
        type: 'stream-start',
        id,
        data: { statusCode: 503, statusMessage: 'Service Unavailable', headers: {} },
      });
      this.messageChannel?.port1.postMessage({ type: 'stream-end', id });
      return;
    }

    // Check if the server supports streaming (has handleStreamingRequest method)
    const server = virtualServer.server as any;
    if (typeof server.handleStreamingRequest === 'function') {
      ServerBridge.DEBUG && console.log('[ServerBridge] 🟢 Server has streaming support, calling handleStreamingRequest');
      // Use streaming handler
      const bodyBuffer = body ? Buffer.from(new Uint8Array(body)) : undefined;

      await server.handleStreamingRequest(
        method,
        url,
        headers,
        bodyBuffer,
        // onStart - called with headers
        (statusCode: number, statusMessage: string, respHeaders: Record<string, string>) => {
          ServerBridge.DEBUG && console.log('[ServerBridge] 🟢 onStart called, sending stream-start');
          this.messageChannel?.port1.postMessage({
            type: 'stream-start',
            id,
            data: { statusCode, statusMessage, headers: respHeaders },
          });
        },
        // onChunk - called for each chunk
        (chunk: string | Uint8Array) => {
          const bytes = typeof chunk === 'string' ? _encoder.encode(chunk) : chunk;
          const chunkBase64 = uint8ToBase64(bytes);
          ServerBridge.DEBUG && console.log('[ServerBridge] 🟡 onChunk called, sending stream-chunk, size:', chunkBase64.length);
          this.messageChannel?.port1.postMessage({
            type: 'stream-chunk',
            id,
            data: { chunkBase64 },
          });
        },
        // onEnd - called when response is complete
        () => {
          ServerBridge.DEBUG && console.log('[ServerBridge] 🟢 onEnd called, sending stream-end');
          this.messageChannel?.port1.postMessage({ type: 'stream-end', id });
        }
      );
    } else {
      // Fall back to regular request handling
      const bodyBuffer = body ? Buffer.from(new Uint8Array(body)) : undefined;
      const response = await virtualServer.server.handleRequest(method, url, headers, bodyBuffer);

      // Send as a single stream
      this.messageChannel?.port1.postMessage({
        type: 'stream-start',
        id,
        data: {
          statusCode: response.statusCode,
          statusMessage: response.statusMessage,
          headers: response.headers,
        },
      });

      if (response.body && response.body.length > 0) {
        const bytes = response.body instanceof Uint8Array ? response.body : new Uint8Array(0);
        this.messageChannel?.port1.postMessage({
          type: 'stream-chunk',
          id,
          data: { chunkBase64: uint8ToBase64(bytes) },
        });
      }

      this.messageChannel?.port1.postMessage({ type: 'stream-end', id });
    }
  }

  /**
   * Send message to Service Worker
   */
  private notifyServiceWorker(type: string, data: unknown): void {
    if (this.serviceWorkerReady && this.messageChannel) {
      this.messageChannel.port1.postMessage({ type, data });
    }
  }

  /**
   * Create a mock request handler for testing without Service Worker
   */
  createFetchHandler(): (request: Request) => Promise<Response> {
    return async (request: Request): Promise<Response> => {
      const url = new URL(request.url);

      // Check if this is a virtual server request
      const match = url.pathname.match(/^\/__virtual__\/(\d+)(\/.*)?$/);
      if (!match) {
        throw new Error('Not a virtual server request');
      }

      const port = parseInt(match[1], 10);
      const path = match[2] || '/';

      // Build headers object
      const headers: Record<string, string> = {};
      request.headers.forEach((value, key) => {
        headers[key] = value;
      });

      // Get body if present
      let body: ArrayBuffer | undefined;
      if (request.method !== 'GET' && request.method !== 'HEAD') {
        body = await request.arrayBuffer();
      }

      // Handle request
      const response = await this.handleRequest(
        port,
        request.method,
        path + url.search,
        headers,
        body
      );

      // Convert to fetch Response
      return new Response(response.body, {
        status: response.statusCode,
        statusText: response.statusMessage,
        headers: response.headers,
      });
    };
  }
}

// Global bridge instance
let globalBridge: ServerBridge | null = null;

/**
 * Get or create the global server bridge
 */
export function getServerBridge(options?: BridgeOptions): ServerBridge {
  if (!globalBridge) {
    globalBridge = new ServerBridge(options);
  }
  return globalBridge;
}

/**
 * Reset the global bridge (for testing)
 */
export function resetServerBridge(): void {
  globalBridge = null;
}

export default ServerBridge;
