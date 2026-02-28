/**
 * Macaly Demo - Load the REAL macaly-web repository into almostnode
 * This tests almostnode's ability to run a real-world Next.js app
 */

import { VirtualFS } from './virtual-fs';
import { createRuntime } from './create-runtime';
import type { IRuntime } from './runtime-interface';
import { NextDevServer } from './frameworks/next-dev-server';
import { getServerBridge } from './server-bridge';

/**
 * Files to load from the real macaly-web repository
 * We'll populate this dynamically, but here's the structure we need
 */
export interface MacalyFiles {
  [path: string]: string;
}

/**
 * Load the real macaly-web project into VirtualFS
 */
export function loadMacalyProject(vfs: VirtualFS, files: MacalyFiles): void {
  for (const [path, content] of Object.entries(files)) {
    // Ensure directory exists
    const dir = path.substring(0, path.lastIndexOf('/'));
    if (dir) {
      vfs.mkdirSync(dir, { recursive: true });
    }

    // Handle base64-encoded binary files
    if (content.startsWith('base64:')) {
      const base64Data = content.slice(7); // Remove 'base64:' prefix
      const binaryData = atob(base64Data);
      const bytes = new Uint8Array(binaryData.length);
      for (let i = 0; i < binaryData.length; i++) {
        bytes[i] = binaryData.charCodeAt(i);
      }
      vfs.writeFileSync(path, Buffer.from(bytes));
    } else {
      vfs.writeFileSync(path, content);
    }
  }
}

/**
 * Initialize the Macaly demo with real files
 */
export async function initMacalyDemo(
  outputElement: HTMLElement,
  files: MacalyFiles,
  options: { useWorker?: boolean } = {}
): Promise<{ vfs: VirtualFS; runtime: IRuntime }> {
  const { useWorker = false } = options;

  const log = (message: string) => {
    const line = document.createElement('div');
    line.textContent = `[${new Date().toLocaleTimeString()}] ${message}`;
    outputElement.appendChild(line);
    outputElement.scrollTop = outputElement.scrollHeight;
  };

  log('Creating virtual file system...');
  const vfs = new VirtualFS();

  log(`Loading ${Object.keys(files).length} files from macaly-web...`);
  loadMacalyProject(vfs, files);

  log(`Initializing runtime (${useWorker ? 'Web Worker mode' : 'main thread'})...`);
  const runtime = await createRuntime(vfs, {
    dangerouslyAllowSameOrigin: true,
    useWorker,
    cwd: '/',
    env: {
      NODE_ENV: 'development',
    },
    onConsole: (method, args) => {
      const prefix = method === 'error' ? '[ERROR]' : method === 'warn' ? '[WARN]' : '';
      log(`${prefix} ${args.map((a) => String(a)).join(' ')}`);
    },
  });

  if (useWorker) {
    log('Runtime is running in a Web Worker for better UI responsiveness');
  }

  log('Setting up file watcher...');
  vfs.watch('/app', { recursive: true }, (eventType, filename) => {
    log(`File ${eventType}: ${filename}`);
  });

  log('Macaly demo initialized!');

  return { vfs, runtime };
}

/**
 * Start the Macaly dev server
 */
export async function startMacalyDevServer(
  vfs: VirtualFS,
  options: {
    port?: number;
    log?: (message: string) => void;
  } = {}
): Promise<{
  server: NextDevServer;
  url: string;
  stop: () => void;
}> {
  const port = options.port || 3001;
  const log = options.log || console.log;

  log('Starting Macaly dev server...');

  const server = new NextDevServer(vfs, { port, root: '/' });
  const bridge = getServerBridge();

  try {
    log('Initializing Service Worker...');
    await bridge.initServiceWorker();
    log('Service Worker ready');
  } catch (error) {
    log(`Warning: Service Worker failed to initialize: ${error}`);
  }

  bridge.on('server-ready', (p: unknown, u: unknown) => {
    log(`Server ready at ${u}`);
  });

  const httpServer = {
    listening: true,
    address: () => ({ port: server.getPort(), address: '0.0.0.0', family: 'IPv4' }),
    async handleRequest(
      method: string,
      url: string,
      headers: Record<string, string>,
      body?: string | ArrayBuffer
    ) {
      const bodyBuffer = body
        ? typeof body === 'string'
          ? Buffer.from(body)
          : Buffer.from(body)
        : undefined;
      return server.handleRequest(method, url, headers, bodyBuffer);
    },
  };

  bridge.registerServer(httpServer as any, port);
  server.start();
  log('File watcher started');

  server.on('hmr-update', (update: unknown) => {
    log(`HMR update: ${JSON.stringify(update)}`);
  });

  const url = bridge.getServerUrl(port);
  log(`Macaly dev server running at: ${url}/`);

  return {
    server,
    url: url + '/',
    stop: () => {
      server.stop();
      bridge.unregisterServer(port);
    },
  };
}

// Export for use in the demo page
export { VirtualFS, NextDevServer, createRuntime };
export type { IRuntime };
