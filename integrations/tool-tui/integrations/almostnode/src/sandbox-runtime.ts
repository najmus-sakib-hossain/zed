/**
 * SandboxRuntime - Runs code in a cross-origin iframe for secure execution
 *
 * This provides browser-enforced isolation from cookies, localStorage,
 * sessionStorage, and IndexedDB by running code on a different origin.
 */

import type { VirtualFS } from './virtual-fs';
import type { IRuntime, IExecuteResult, IRuntimeOptions, VFSSnapshot } from './runtime-interface';

interface SandboxMessage {
  type: 'init' | 'execute' | 'runFile' | 'clearCache' | 'syncFile' | 'ready' | 'result' | 'error' | 'console';
  id?: string;
  code?: string;
  filename?: string;
  vfsSnapshot?: VFSSnapshot;
  options?: IRuntimeOptions;
  result?: IExecuteResult;
  error?: string;
  path?: string;
  content?: string | null;
  consoleMethod?: string;
  consoleArgs?: unknown[];
}

/**
 * SandboxRuntime - Executes code in a cross-origin iframe
 */
export class SandboxRuntime implements IRuntime {
  private iframe: HTMLIFrameElement;
  private sandboxOrigin: string;
  private vfs: VirtualFS;
  private options: IRuntimeOptions;
  private initialized: Promise<void>;
  private pending = new Map<string, { resolve: (result: IExecuteResult) => void; reject: (error: Error) => void }>();
  private messageId = 0;
  private changeListener: ((path: string, content: string) => void) | null = null;
  private deleteListener: ((path: string) => void) | null = null;
  private messageHandler: ((event: MessageEvent) => void) | null = null;

  constructor(sandboxUrl: string, vfs: VirtualFS, options: IRuntimeOptions = {}) {
    this.sandboxOrigin = new URL(sandboxUrl).origin;
    this.vfs = vfs;
    this.options = options;

    // Create hidden iframe
    // NOTE: Security comes from the DIFFERENT ORIGIN (e.g., localhost:3002 vs localhost:5173)
    // The browser's same-origin policy prevents the sandbox from accessing the parent's
    // cookies, localStorage, sessionStorage, and IndexedDB.
    // We use the 'credentialless' attribute for compatibility with pages that have
    // Cross-Origin-Embedder-Policy set (like Vite dev server).
    this.iframe = document.createElement('iframe');
    this.iframe.src = sandboxUrl;
    this.iframe.style.display = 'none';
    // @ts-expect-error - credentialless is a newer attribute not in all TypeScript definitions
    this.iframe.credentialless = true;
    this.iframe.setAttribute('credentialless', '');
    document.body.appendChild(this.iframe);

    // Set up message handler
    this.setupMessageHandler();

    // Wait for iframe to be ready, then initialize
    this.initialized = this.waitForReady().then(() => this.initSandbox());

    // Set up VFS change listeners
    this.setupVFSListeners();
  }

  /**
   * Set up the message event handler
   */
  private setupMessageHandler(): void {
    this.messageHandler = (event: MessageEvent) => {
      // Only accept messages from our sandbox origin
      if (event.origin !== this.sandboxOrigin) return;

      const message = event.data as SandboxMessage;

      if (message.type === 'result' && message.id) {
        const pending = this.pending.get(message.id);
        if (pending && message.result) {
          pending.resolve(message.result);
          this.pending.delete(message.id);
        }
      } else if (message.type === 'error' && message.id) {
        const pending = this.pending.get(message.id);
        if (pending) {
          pending.reject(new Error(message.error || 'Unknown sandbox error'));
          this.pending.delete(message.id);
        }
      } else if (message.type === 'console' && this.options.onConsole) {
        this.options.onConsole(message.consoleMethod || 'log', message.consoleArgs || []);
      }
    };

    window.addEventListener('message', this.messageHandler);
  }

  /**
   * Wait for the sandbox iframe to signal it's ready
   */
  private waitForReady(): Promise<void> {
    return new Promise((resolve) => {
      const handler = (event: MessageEvent) => {
        if (event.origin !== this.sandboxOrigin) return;
        const message = event.data as SandboxMessage;
        if (message.type === 'ready') {
          window.removeEventListener('message', handler);
          resolve();
        }
      };
      window.addEventListener('message', handler);
    });
  }

  /**
   * Initialize the sandbox with VFS snapshot and options
   */
  private async initSandbox(): Promise<void> {
    const snapshot = this.vfs.toSnapshot();

    const message: SandboxMessage = {
      type: 'init',
      vfsSnapshot: snapshot,
      options: {
        cwd: this.options.cwd,
        env: this.options.env,
        // Note: onConsole callback can't be sent cross-origin,
        // sandbox will send console messages back via postMessage
      },
    };

    this.iframe.contentWindow?.postMessage(message, this.sandboxOrigin);
    console.log('[SandboxRuntime] Sandbox initialized');
  }

  /**
   * Set up listeners for VFS changes to sync to sandbox
   */
  private setupVFSListeners(): void {
    this.changeListener = (path: string, content: string) => {
      const message: SandboxMessage = {
        type: 'syncFile',
        path,
        content,
      };
      this.iframe.contentWindow?.postMessage(message, this.sandboxOrigin);
    };
    this.vfs.on('change', this.changeListener);

    this.deleteListener = (path: string) => {
      const message: SandboxMessage = {
        type: 'syncFile',
        path,
        content: null,
      };
      this.iframe.contentWindow?.postMessage(message, this.sandboxOrigin);
    };
    this.vfs.on('delete', this.deleteListener);
  }

  /**
   * Send a message and wait for response
   */
  private sendAndWait(message: SandboxMessage): Promise<IExecuteResult> {
    return new Promise((resolve, reject) => {
      const id = String(this.messageId++);
      this.pending.set(id, { resolve, reject });

      this.iframe.contentWindow?.postMessage(
        { ...message, id },
        this.sandboxOrigin
      );

      // Timeout after 60 seconds
      setTimeout(() => {
        if (this.pending.has(id)) {
          this.pending.delete(id);
          reject(new Error('Sandbox execution timeout'));
        }
      }, 60000);
    });
  }

  /**
   * Execute code in the sandbox
   */
  async execute(code: string, filename?: string): Promise<IExecuteResult> {
    await this.initialized;
    return this.sendAndWait({
      type: 'execute',
      code,
      filename,
    });
  }

  /**
   * Run a file from the VFS in the sandbox
   */
  async runFile(filename: string): Promise<IExecuteResult> {
    await this.initialized;
    return this.sendAndWait({
      type: 'runFile',
      filename,
    });
  }

  /**
   * Clear the module cache in the sandbox
   */
  clearCache(): void {
    const message: SandboxMessage = { type: 'clearCache' };
    this.iframe.contentWindow?.postMessage(message, this.sandboxOrigin);
  }

  /**
   * Get the VFS (main thread instance)
   */
  getVFS(): VirtualFS {
    return this.vfs;
  }

  /**
   * Terminate the sandbox
   */
  terminate(): void {
    // Remove VFS listeners
    if (this.changeListener) {
      this.vfs.off('change', this.changeListener);
    }
    if (this.deleteListener) {
      this.vfs.off('delete', this.deleteListener);
    }

    // Remove message handler
    if (this.messageHandler) {
      window.removeEventListener('message', this.messageHandler);
    }

    // Remove iframe
    this.iframe.remove();

    // Reject any pending promises
    for (const [id, { reject }] of this.pending) {
      reject(new Error('Sandbox terminated'));
      this.pending.delete(id);
    }

    console.log('[SandboxRuntime] Sandbox terminated');
  }
}
