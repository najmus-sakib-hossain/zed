/**
 * Next.js + Convex Todo App Demo
 *
 * A simple todo list app using Next.js App Router and Convex for real-time data sync,
 * running entirely in the browser.
 */

import { VirtualFS } from './virtual-fs';
import { Runtime } from './runtime';
import { NextDevServer } from './frameworks/next-dev-server';
import { getServerBridge } from './server-bridge';
import { Buffer } from './shims/stream';
import { PackageManager, InstallOptions, InstallResult } from './npm';

/**
 * Package.json for a realistic Next.js + Convex app
 */
const PACKAGE_JSON = {
  name: "convex-todo-app",
  version: "0.1.0",
  private: true,
  scripts: {
    dev: "next dev",
    build: "next build",
    start: "next start",
  },
  dependencies: {
    "next": "^14.0.0",
    "react": "^18.2.0",
    "react-dom": "^18.2.0",
  },
  devDependencies: {
    "@types/node": "^20",
    "@types/react": "^19",
    "@types/react-dom": "^19",
    "typescript": "^5.9.3",
  }
};

/**
 * Minimal packages to install for demo (others loaded from CDN)
 */
const DEMO_PACKAGES: string[] = [];

/**
 * Create the project structure in the virtual filesystem
 */
export function createConvexAppProject(vfs: VirtualFS): void {
  // Create package.json
  vfs.writeFileSync('/package.json', JSON.stringify(PACKAGE_JSON, null, 2));

  // Create directories - App Router structure
  vfs.mkdirSync('/app', { recursive: true });
  vfs.mkdirSync('/components', { recursive: true });
  vfs.mkdirSync('/lib', { recursive: true });
  vfs.mkdirSync('/convex', { recursive: true });
  vfs.mkdirSync('/public', { recursive: true });

  // Create convex.json configuration (required by Convex CLI)
  vfs.writeFileSync('/convex.json', JSON.stringify({
    functions: "convex/"
  }, null, 2));

  // Create TypeScript config
  vfs.writeFileSync('/tsconfig.json', JSON.stringify({
    compilerOptions: {
      target: "es5",
      lib: ["dom", "dom.iterable", "esnext"],
      allowJs: true,
      skipLibCheck: true,
      strict: true,
      noEmit: true,
      esModuleInterop: true,
      module: "esnext",
      moduleResolution: "bundler",
      resolveJsonModule: true,
      isolatedModules: true,
      jsx: "preserve",
      incremental: true,
      paths: {
        "@/*": ["./*"]
      }
    },
    include: ["**/*.ts", "**/*.tsx"],
    exclude: ["node_modules"]
  }, null, 2));

  // Create global CSS — minimal dark theme
  vfs.writeFileSync('/app/globals.css', `*, *::before, *::after {
  box-sizing: border-box;
}

body {
  background-color: hsl(222.2 84% 4.9%);
  color: hsl(210 40% 98%);
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
  margin: 0;
  line-height: 1.5;
}

input[type="text"], input:not([type]) {
  background-color: hsl(222.2 84% 4.9%);
  color: hsl(210 40% 98%);
  border: 1px solid hsl(217.2 32.6% 17.5%);
  border-radius: 0.375rem;
  padding: 0.5rem 0.75rem;
  font-size: 0.875rem;
  height: 2.5rem;
  outline: none;
}
input:focus {
  border-color: hsl(212.7 26.8% 83.9%);
  box-shadow: 0 0 0 2px hsl(212.7 26.8% 83.9% / 0.2);
}
input::placeholder {
  color: hsl(215 20.2% 65.1%);
}

input[type="checkbox"] {
  width: 1rem;
  height: 1rem;
  accent-color: hsl(210 40% 98%);
  cursor: pointer;
}

button {
  background-color: hsl(210 40% 98%);
  color: hsl(222.2 47.4% 11.2%);
  border: none;
  border-radius: 0.375rem;
  padding: 0.5rem 1rem;
  font-size: 0.875rem;
  font-weight: 500;
  cursor: pointer;
  height: 2.5rem;
}
button:hover {
  opacity: 0.9;
}

ul { list-style: none; padding: 0; margin: 0; }
`);

  // Create Convex config (required by CLI bundler)
  // IMPORTANT: CLI needs BOTH .ts and .js versions!
  vfs.writeFileSync('/convex/convex.config.ts', `import { defineApp } from "convex/server";

const app = defineApp();
export default app;
`);
  vfs.writeFileSync('/convex/convex.config.js', `import { defineApp } from "convex/server";

const app = defineApp();
export default app;
`);

  // Create Convex schema
  // priority is optional for backwards-compatibility with existing documents
  vfs.writeFileSync('/convex/schema.ts', `import { defineSchema, defineTable } from "convex/server";
import { v } from "convex/values";

export default defineSchema({
  todos: defineTable({
    title: v.string(),
    completed: v.boolean(),
    priority: v.optional(v.union(v.literal("low"), v.literal("medium"), v.literal("high"))),
  }),
});
`);

  // Create Convex functions for todos
  vfs.writeFileSync('/convex/todos.ts', `import { query, mutation } from "./_generated/server";
import { v } from "convex/values";

export const list = query({
  args: {},
  handler: async (ctx) => {
    return await ctx.db.query("todos").order("desc").collect();
  },
});

export const create = mutation({
  args: { title: v.string() },
  handler: async (ctx, args) => {
    return await ctx.db.insert("todos", {
      title: args.title,
      completed: false,
    });
  },
});

export const toggle = mutation({
  args: { id: v.id("todos") },
  handler: async (ctx, args) => {
    const todo = await ctx.db.get(args.id);
    if (!todo) throw new Error("Todo not found");
    await ctx.db.patch(args.id, { completed: !todo.completed });
  },
});

export const remove = mutation({
  args: { id: v.id("todos") },
  handler: async (ctx, args) => {
    await ctx.db.delete(args.id);
  },
});
`);

  // Create Convex API (normally auto-generated, but we create manually for the demo)
  // This creates function references that Convex's useQuery/useMutation understand
  vfs.writeFileSync('/convex/_generated/api.ts', `// Convex API - manually created for browser demo
// In a real project, this is auto-generated by 'npx convex dev'

// Function references for the Convex client
// These are string identifiers that map to server functions
export const api = {
  todos: {
    list: "todos:list",
    create: "todos:create",
    toggle: "todos:toggle",
    remove: "todos:remove",
  },
} as const;
`);

  // Create server stubs (needed for schema/function imports to work)
  vfs.writeFileSync('/convex/_generated/server.ts', `// Server stubs for browser demo
// In a real project, this is auto-generated by Convex

export function query<Args, Output>(config: {
  args: Args;
  handler: (ctx: any, args: any) => Promise<Output>;
}) {
  return config;
}

export function mutation<Args, Output>(config: {
  args: Args;
  handler: (ctx: any, args: any) => Promise<Output>;
}) {
  return config;
}
`);

  // Create Convex provider using real Convex client from CDN
  vfs.writeFileSync('/lib/convex.tsx', `"use client";

import React, { useState, useEffect } from 'react';
import { ConvexProvider as BaseConvexProvider, ConvexReactClient, useQuery as useConvexQuery, useMutation as useConvexMutation } from 'convex/react';

// Re-export the API
export { api } from '../convex/_generated/api.ts';

// Get Convex URL using standard Next.js env var pattern
// Falls back to window.__CONVEX_URL__ for backwards compatibility
const getConvexUrl = () => {
  // Standard Next.js pattern: process.env.NEXT_PUBLIC_*
  if (typeof process !== 'undefined' && process.env?.NEXT_PUBLIC_CONVEX_URL) {
    return process.env.NEXT_PUBLIC_CONVEX_URL;
  }
  // Fallback for backwards compatibility
  if (typeof window !== 'undefined' && (window as any).__CONVEX_URL__) {
    return (window as any).__CONVEX_URL__;
  }
  return null;
};

// Create client lazily
let client: ConvexReactClient | null = null;

function getClient() {
  const url = getConvexUrl();
  if (!url) return null;
  if (!client || (client as any)._address !== url) {
    client = new ConvexReactClient(url);
  }
  return client;
}

// Wrapper hooks that handle the case when Convex is not connected
export function useQuery(query: any, ...args: any[]) {
  const url = getConvexUrl();
  // When not connected, return undefined
  if (!url) return undefined;
  return useConvexQuery(query, ...args);
}

export function useMutation(mutation: any) {
  const url = getConvexUrl();
  const convexMutation = url ? useConvexMutation(mutation) : null;

  return async (args: any) => {
    if (!convexMutation) {
      console.warn('Convex not connected - mutation ignored');
      return;
    }
    return convexMutation(args);
  };
}

export function ConvexProvider({ children }: { children: React.ReactNode }) {
  const [convexUrl, setConvexUrl] = useState(getConvexUrl());

  // Check for URL changes (after deploy)
  useEffect(() => {
    const checkUrl = () => {
      const url = getConvexUrl();
      if (url !== convexUrl) {
        setConvexUrl(url);
      }
    };

    // Check periodically for URL changes
    const interval = setInterval(checkUrl, 1000);
    return () => clearInterval(interval);
  }, [convexUrl]);

  const convexClient = getClient();

  if (!convexClient) {
    // Show a message when Convex is not configured
    return (
      <div style={{ maxWidth: 480, margin: "0 auto", padding: "4rem 1rem", textAlign: "center" }}>
        <h2 style={{ fontSize: "1.5rem", fontWeight: 700, marginBottom: "1rem" }}>Connect to Convex</h2>
        <p style={{ color: "hsl(215 20.2% 65.1%)", marginBottom: "1.5rem" }}>
          Enter your Convex deploy key in the console panel and click "Deploy Schema" to connect.
        </p>
        <p style={{ color: "hsl(215 20.2% 65.1%)", fontSize: "0.75rem" }}>
          Get a deploy key from your Convex dashboard at convex.dev
        </p>
      </div>
    );
  }

  return (
    <BaseConvexProvider client={convexClient}>
      {children}
    </BaseConvexProvider>
  );
}
`);


  // Create TaskList component (uses real Convex API)
  vfs.writeFileSync('/components/task-list.tsx', `"use client";

import React from 'react';
import { useQuery, useMutation, api } from '../lib/convex.tsx';

type Todo = {
  _id: string;
  _creationTime: number;
  title: string;
  completed: boolean;
};

export function TaskList() {
  const todos = useQuery(api.todos.list) as Todo[] | undefined;
  const createTodo = useMutation(api.todos.create);
  const toggleTodo = useMutation(api.todos.toggle);
  const removeTodo = useMutation(api.todos.remove);

  const [newTitle, setNewTitle] = React.useState("");

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!newTitle.trim()) return;
    await createTodo({ title: newTitle.trim() });
    setNewTitle("");
  };

  return (
    <div style={{ maxWidth: 480, margin: "0 auto", padding: "2rem 1rem" }}>
      <h1 style={{ fontSize: "1.5rem", fontWeight: 700, marginBottom: "1.5rem" }}>
        Todos
      </h1>

      <form onSubmit={handleSubmit} style={{ display: "flex", gap: "0.5rem", marginBottom: "1.5rem" }}>
        <input
          placeholder="What needs to be done?"
          value={newTitle}
          onChange={(e) => setNewTitle(e.target.value)}
          style={{ flex: 1 }}
        />
        <button type="submit">Add</button>
      </form>

      {todos === undefined ? (
        <p style={{ color: "hsl(215 20.2% 65.1%)" }}>Loading...</p>
      ) : todos.length === 0 ? (
        <p style={{ color: "hsl(215 20.2% 65.1%)" }}>No todos yet.</p>
      ) : (
        <ul style={{ listStyle: "none", padding: 0, margin: 0 }}>
          {todos.map((todo) => (
            <li key={todo._id} style={{
              display: "flex",
              alignItems: "center",
              gap: "0.75rem",
              padding: "0.625rem 0",
              borderBottom: "1px solid hsl(217.2 32.6% 17.5%)",
            }}>
              <input
                type="checkbox"
                checked={todo.completed}
                onChange={() => toggleTodo({ id: todo._id })}
              />
              <span style={{
                flex: 1,
                textDecoration: todo.completed ? "line-through" : "none",
                opacity: todo.completed ? 0.5 : 1,
              }}>
                {todo.title}
              </span>
              <button
                onClick={() => removeTodo({ id: todo._id })}
                style={{
                  background: "transparent",
                  color: "hsl(0 62.8% 50%)",
                  padding: "0.25rem 0.5rem",
                  fontSize: "0.75rem",
                }}
              >
                Delete
              </button>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
`);

  // Create root layout (App Router)
  // Note: In browser environment, we don't use <html>/<head>/<body> tags
  // since we're rendering inside an existing HTML document's #__next div
  vfs.writeFileSync('/app/layout.tsx', `import React from 'react';
import './globals.css';
import { ConvexProvider } from '../lib/convex.tsx';

export const metadata = {
  title: 'Convex Todo App',
  description: 'A simple todo app powered by Convex, running in the browser',
};

export default function RootLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <ConvexProvider>
      {children}
    </ConvexProvider>
  );
}
`);

  // Create home page (App Router) - Shows TaskList directly
  vfs.writeFileSync('/app/page.tsx', `"use client";

import React from 'react';
import { TaskList } from '../components/task-list.tsx';

export default function HomePage() {
  return <TaskList />;
}
`);


  // Create public files
  vfs.writeFileSync('/public/favicon.ico', 'favicon placeholder');
  vfs.writeFileSync('/public/robots.txt', 'User-agent: *\nAllow: /');
}

/**
 * Initialize the Convex App demo
 */
export async function initConvexAppDemo(
  outputElement: HTMLElement,
  options: {
    installPackages?: boolean;
  } = {}
): Promise<{ vfs: VirtualFS; runtime: Runtime }> {
  const log = (message: string) => {
    const line = document.createElement('div');
    line.textContent = `[${new Date().toLocaleTimeString()}] ${message}`;
    outputElement.appendChild(line);
    outputElement.scrollTop = outputElement.scrollHeight;
  };

  log('Creating virtual file system...');
  const vfs = new VirtualFS();

  log('Creating Convex App project structure...');
  createConvexAppProject(vfs);

  // Optionally install npm packages
  if (options.installPackages) {
    log('Installing npm packages (this may take a while)...');
    const npm = new PackageManager(vfs);

    for (const pkg of DEMO_PACKAGES) {
      try {
        log(`Installing ${pkg}...`);
        await npm.install(pkg, {
          onProgress: (msg) => log(`  ${msg}`),
        });
      } catch (error) {
        log(`Warning: Failed to install ${pkg}: ${error}`);
      }
    }
  }

  log('Initializing runtime...');
  const runtime = new Runtime(vfs, {
    cwd: '/',
    env: {
      NODE_ENV: 'development',
    },
    onConsole: (method, args) => {
      const prefix = method === 'error' ? '[ERROR]' : method === 'warn' ? '[WARN]' : '';
      log(`${prefix} ${args.map((a) => String(a)).join(' ')}`);
    },
  });

  log('Setting up file watcher...');
  vfs.watch('/app', { recursive: true }, (eventType, filename) => {
    log(`File ${eventType}: ${filename}`);
  });

  log('Convex App demo initialized!');
  log('');
  log('Project structure:');
  listFiles(vfs, '/', log, '  ');

  return { vfs, runtime };
}

/**
 * Start the dev server for Convex App demo
 */
export async function startConvexAppDevServer(
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
  const port = options.port || 3002;
  const log = options.log || console.log;

  log('Starting Convex App dev server...');

  // Create NextDevServer with App Router preference
  const server = new NextDevServer(vfs, {
    port,
    root: '/',
    preferAppRouter: true,
  });

  // Get the server bridge
  const bridge = getServerBridge();

  // Initialize Service Worker
  try {
    log('Initializing Service Worker...');
    await bridge.initServiceWorker();
    log('Service Worker ready');
  } catch (error) {
    log(`Warning: Service Worker failed to initialize: ${error}`);
    log('Falling back to direct request handling...');
  }

  // Register event handlers
  bridge.on('server-ready', (p: unknown, u: unknown) => {
    log(`Server ready at ${u}`);
  });

  // Wire up the NextDevServer to handle requests through the bridge
  const httpServer = createHttpServerWrapper(server);
  bridge.registerServer(httpServer, port);

  // Start watching for file changes
  server.start();
  log('File watcher started');

  // Set up HMR event forwarding
  server.on('hmr-update', (update: unknown) => {
    log(`HMR update: ${JSON.stringify(update)}`);
  });

  const url = bridge.getServerUrl(port);
  log(`Convex App dev server running at: ${url}/`);

  return {
    server,
    url: url + '/',
    stop: () => {
      server.stop();
      bridge.unregisterServer(port);
    },
  };
}

/**
 * Create an http.Server-compatible wrapper
 */
function createHttpServerWrapper(devServer: NextDevServer) {
  return {
    listening: true,
    address: () => ({ port: devServer.getPort(), address: '0.0.0.0', family: 'IPv4' }),
    async handleRequest(
      method: string,
      url: string,
      headers: Record<string, string>,
      body?: string | Buffer
    ) {
      const bodyBuffer = body
        ? typeof body === 'string'
          ? Buffer.from(body)
          : body
        : undefined;
      return devServer.handleRequest(method, url, headers, bodyBuffer);
    },
  };
}

function listFiles(
  vfs: VirtualFS,
  path: string,
  log: (msg: string) => void,
  indent: string
): void {
  try {
    const entries = vfs.readdirSync(path);
    for (const entry of entries) {
      if (entry === 'node_modules') {
        log(`${indent}${entry}/ (skipped)`);
        continue;
      }
      const fullPath = path === '/' ? `/${entry}` : `${path}/${entry}`;
      try {
        const stat = vfs.statSync(fullPath);
        if (stat.isDirectory()) {
          log(`${indent}${entry}/`);
          listFiles(vfs, fullPath, log, indent + '  ');
        } else {
          log(`${indent}${entry}`);
        }
      } catch {
        log(`${indent}${entry}`);
      }
    }
  } catch {
    // Directory doesn't exist or can't be read
  }
}

// Export for use in HTML demos
export { PACKAGE_JSON, DEMO_PACKAGES };
