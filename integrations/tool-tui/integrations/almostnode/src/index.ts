/**
 * Mini WebContainers MVP - Main Entry Point
 *
 * Provides a browser-based Node.js-like environment
 * with virtual file system and CommonJS module support
 */

export { VirtualFS } from './virtual-fs';
export type { FSNode, Stats, FSWatcher, WatchListener, WatchEventType } from './virtual-fs';
export { Runtime, execute } from './runtime';
export type { Module, RuntimeOptions, RequireFunction } from './runtime';
export { createRuntime, WorkerRuntime, SandboxRuntime } from './create-runtime';
export type { IRuntime, IExecuteResult, CreateRuntimeOptions, IRuntimeOptions, VFSSnapshot } from './runtime-interface';
export { generateSandboxFiles, getSandboxHtml, getSandboxVercelConfig, SANDBOX_SETUP_INSTRUCTIONS } from './sandbox-helpers';
export { createFsShim } from './shims/fs';
export type { FsShim } from './shims/fs';
export { createProcess } from './shims/process';
export type { Process, ProcessEnv } from './shims/process';
export * as path from './shims/path';
export * as http from './shims/http';
export * as net from './shims/net';
export * as events from './shims/events';
export * as stream from './shims/stream';
export * as url from './shims/url';
export * as querystring from './shims/querystring';
export * as util from './shims/util';
export * as npm from './npm';
export { PackageManager, install } from './npm';
export { ServerBridge, getServerBridge, resetServerBridge } from './server-bridge';
export type { InitServiceWorkerOptions } from './server-bridge';
// Dev servers
export { DevServer } from './dev-server';
export type { DevServerOptions, ResponseData, HMRUpdate } from './dev-server';
export { ViteDevServer } from './frameworks/vite-dev-server';
export type { ViteDevServerOptions } from './frameworks/vite-dev-server';
export { NextDevServer } from './frameworks/next-dev-server';
export type { NextDevServerOptions } from './frameworks/next-dev-server';
// New shims for Vite support
export * as chokidar from './shims/chokidar';
export * as ws from './shims/ws';
export * as fsevents from './shims/fsevents';
export * as readdirp from './shims/readdirp';
export * as module from './shims/module';
export * as perf_hooks from './shims/perf_hooks';
export * as worker_threads from './shims/worker_threads';
export * as esbuild from './shims/esbuild';
export * as rollup from './shims/rollup';
export * as assert from './shims/assert';

// Demo exports
export {
  createConvexAppProject,
  initConvexAppDemo,
  startConvexAppDevServer,
  PACKAGE_JSON as CONVEX_APP_PACKAGE_JSON,
  DEMO_PACKAGES as CONVEX_APP_DEMO_PACKAGES,
} from './convex-app-demo';

import { VirtualFS } from './virtual-fs';
import { Runtime, RuntimeOptions } from './runtime';
import { PackageManager } from './npm';
import { ServerBridge, getServerBridge } from './server-bridge';
import { exec as cpExec, setStreamingCallbacks, clearStreamingCallbacks, sendStdin } from './shims/child_process';

export interface RunResult {
  stdout: string;
  stderr: string;
  exitCode: number;
}

export interface RunOptions {
  cwd?: string;
  /** Callback for streaming stdout chunks as they arrive (for long-running commands like vitest watch) */
  onStdout?: (data: string) => void;
  /** Callback for streaming stderr chunks as they arrive */
  onStderr?: (data: string) => void;
  /** AbortSignal to cancel long-running commands */
  signal?: AbortSignal;
}

export interface ContainerOptions extends RuntimeOptions {
  baseUrl?: string;
  onServerReady?: (port: number, url: string) => void;
}

/**
 * Create a new WebContainer-like environment
 */
export function createContainer(options?: ContainerOptions): {
  vfs: VirtualFS;
  runtime: Runtime;
  npm: PackageManager;
  serverBridge: ServerBridge;
  execute: (code: string, filename?: string) => { exports: unknown };
  runFile: (filename: string) => { exports: unknown };
  run: (command: string, options?: RunOptions) => Promise<RunResult>;
  sendInput: (data: string) => void;
  createREPL: () => { eval: (code: string) => unknown };
  on: (event: string, listener: (...args: unknown[]) => void) => void;
} {
  const vfs = new VirtualFS();
  const runtime = new Runtime(vfs, options);
  const npmManager = new PackageManager(vfs);
  const serverBridge = getServerBridge({
    baseUrl: options?.baseUrl,
    onServerReady: options?.onServerReady,
  });

  return {
    vfs,
    runtime,
    npm: npmManager,
    serverBridge,
    execute: (code: string, filename?: string) => runtime.execute(code, filename),
    runFile: (filename: string) => runtime.runFile(filename),
    run: (command: string, runOptions?: RunOptions): Promise<RunResult> => {
      // If signal is already aborted, resolve immediately
      if (runOptions?.signal?.aborted) {
        return Promise.resolve({ stdout: '', stderr: '', exitCode: 130 });
      }

      // Set streaming callbacks for long-running commands (e.g. vitest watch)
      const hasStreaming = runOptions?.onStdout || runOptions?.onStderr || runOptions?.signal;
      if (hasStreaming) {
        setStreamingCallbacks({
          onStdout: runOptions?.onStdout,
          onStderr: runOptions?.onStderr,
          signal: runOptions?.signal,
        });
      }

      return new Promise((resolve) => {
        cpExec(command, { cwd: runOptions?.cwd }, (error, stdout, stderr) => {
          if (hasStreaming) clearStreamingCallbacks();
          resolve({
            stdout: String(stdout),
            stderr: String(stderr),
            exitCode: error ? ((error as any).code ?? 1) : 0,
          });
        });
      });
    },
    sendInput: (data: string) => sendStdin(data),
    createREPL: () => runtime.createREPL(),
    on: (event: string, listener: (...args: unknown[]) => void) => {
      serverBridge.on(event, listener);
    },
  };
}

export default createContainer;
