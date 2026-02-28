/**
 * Runtime Worker - Runs the Just Node runtime in a Web Worker
 *
 * This worker receives code execution requests from the main thread
 * and runs them in isolation, preventing UI blocking.
 */

import { expose } from 'comlink';
import { VirtualFS } from '../virtual-fs';
import { Runtime } from '../runtime';
import type { VFSSnapshot, IRuntimeOptions, IExecuteResult } from '../runtime-interface';

let runtime: Runtime | null = null;
let vfs: VirtualFS | null = null;
let consoleCallback: ((method: string, args: unknown[]) => void) | null = null;

/**
 * Worker API exposed via Comlink
 */
const workerApi = {
  /**
   * Initialize the worker with a VFS snapshot and runtime options
   */
  init(vfsSnapshot: VFSSnapshot, options: IRuntimeOptions): void {
    console.log('[Worker] Initializing with', vfsSnapshot.files.length, 'files');

    // Restore VFS from snapshot
    vfs = VirtualFS.fromSnapshot(vfsSnapshot);

    // Create runtime with console forwarding
    const runtimeOptions: IRuntimeOptions = {
      ...options,
      onConsole: (method, args) => {
        // Forward console output to main thread
        if (consoleCallback) {
          consoleCallback(method, args);
        }
      },
    };

    runtime = new Runtime(vfs, runtimeOptions);
    console.log('[Worker] Runtime initialized');
  },

  /**
   * Set the console callback for forwarding output to main thread
   */
  setConsoleCallback(callback: ((method: string, args: unknown[]) => void) | null): void {
    consoleCallback = callback;
  },

  /**
   * Sync a file change from the main thread
   */
  syncFile(path: string, content: string | null): void {
    if (!vfs) {
      console.warn('[Worker] VFS not initialized, cannot sync file:', path);
      return;
    }

    if (content === null) {
      // File was deleted
      try {
        vfs.unlinkSync(path);
        console.log('[Worker] Deleted file:', path);
      } catch (err) {
        // File might not exist, that's ok
      }
    } else {
      // File was created or modified
      vfs.writeFileSync(path, content);
      console.log('[Worker] Synced file:', path);
    }

    // Clear module cache for this file to pick up changes
    if (runtime) {
      runtime.clearCache();
    }
  },

  /**
   * Execute code in the worker
   */
  async execute(code: string, filename?: string): Promise<IExecuteResult> {
    if (!runtime) {
      throw new Error('Worker runtime not initialized. Call init() first.');
    }

    console.log('[Worker] Executing code in file:', filename);
    return runtime.execute(code, filename);
  },

  /**
   * Run a file from the VFS
   */
  async runFile(filename: string): Promise<IExecuteResult> {
    if (!runtime) {
      throw new Error('Worker runtime not initialized. Call init() first.');
    }

    console.log('[Worker] Running file:', filename);
    return runtime.runFile(filename);
  },

  /**
   * Clear the module cache
   */
  clearCache(): void {
    if (runtime) {
      runtime.clearCache();
      console.log('[Worker] Cache cleared');
    }
  },

  /**
   * Get current VFS state (for debugging)
   */
  getVFSSnapshot(): VFSSnapshot | null {
    if (!vfs) return null;
    return vfs.toSnapshot();
  },
};

// Expose the API via Comlink
expose(workerApi);

// Log that worker is ready
console.log('[Worker] Runtime worker loaded and ready');
