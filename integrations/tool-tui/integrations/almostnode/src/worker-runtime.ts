/**
 * WorkerRuntime - Runs code in a Web Worker for non-blocking execution
 *
 * This class provides the same IRuntime interface as Runtime,
 * but executes code in a separate Web Worker thread.
 */

import { wrap, proxy, Remote } from 'comlink';
import type { VirtualFS } from './virtual-fs';
import type { IRuntime, IExecuteResult, IRuntimeOptions, VFSSnapshot } from './runtime-interface';

/**
 * Type for the worker API
 */
interface WorkerApi {
  init(vfsSnapshot: VFSSnapshot, options: IRuntimeOptions): void;
  setConsoleCallback(callback: ((method: string, args: unknown[]) => void) | null): void;
  syncFile(path: string, content: string | null): void;
  execute(code: string, filename?: string): Promise<IExecuteResult>;
  runFile(filename: string): Promise<IExecuteResult>;
  clearCache(): void;
  getVFSSnapshot(): VFSSnapshot | null;
}

/**
 * WorkerRuntime - Executes code in a Web Worker
 */
export class WorkerRuntime implements IRuntime {
  private worker: Worker;
  private workerApi: Remote<WorkerApi>;
  private vfs: VirtualFS;
  private options: IRuntimeOptions;
  private initialized: Promise<void>;
  private changeListener: ((path: string, content: string) => void) | null = null;
  private deleteListener: ((path: string) => void) | null = null;

  constructor(vfs: VirtualFS, options: IRuntimeOptions = {}) {
    this.vfs = vfs;
    this.options = options;

    // Create the worker
    // Using Vite's worker import syntax
    this.worker = new Worker(
      new URL('./worker/runtime-worker.ts', import.meta.url),
      { type: 'module' }
    );

    // Wrap with Comlink
    this.workerApi = wrap<WorkerApi>(this.worker);

    // Initialize the worker
    this.initialized = this.initWorker();

    // Set up VFS change listeners
    this.setupVFSListeners();
  }

  /**
   * Initialize the worker with VFS snapshot and options
   */
  private async initWorker(): Promise<void> {
    const snapshot = this.vfs.toSnapshot();

    // Create options without the onConsole callback (we'll set it separately via proxy)
    const workerOptions: IRuntimeOptions = {
      cwd: this.options.cwd,
      env: this.options.env,
    };

    await this.workerApi.init(snapshot, workerOptions);

    // Set up console forwarding if callback provided
    if (this.options.onConsole) {
      await this.workerApi.setConsoleCallback(
        proxy(this.options.onConsole)
      );
    }

    console.log('[WorkerRuntime] Worker initialized');
  }

  /**
   * Set up listeners for VFS changes to sync to worker
   */
  private setupVFSListeners(): void {
    // Listen for file changes
    this.changeListener = (path: string, content: string) => {
      this.workerApi.syncFile(path, content);
    };
    this.vfs.on('change', this.changeListener);

    // Listen for file deletions
    this.deleteListener = (path: string) => {
      this.workerApi.syncFile(path, null);
    };
    this.vfs.on('delete', this.deleteListener);
  }

  /**
   * Execute code in the worker
   */
  async execute(code: string, filename?: string): Promise<IExecuteResult> {
    await this.initialized;
    return this.workerApi.execute(code, filename);
  }

  /**
   * Run a file from the VFS in the worker
   */
  async runFile(filename: string): Promise<IExecuteResult> {
    await this.initialized;
    return this.workerApi.runFile(filename);
  }

  /**
   * Clear the module cache in the worker
   */
  clearCache(): void {
    this.workerApi.clearCache();
  }

  /**
   * Get the VFS (main thread instance)
   */
  getVFS(): VirtualFS {
    return this.vfs;
  }

  /**
   * Terminate the worker
   */
  terminate(): void {
    // Remove VFS listeners
    if (this.changeListener) {
      this.vfs.off('change', this.changeListener);
    }
    if (this.deleteListener) {
      this.vfs.off('delete', this.deleteListener);
    }

    // Terminate the worker
    this.worker.terminate();
    console.log('[WorkerRuntime] Worker terminated');
  }
}
