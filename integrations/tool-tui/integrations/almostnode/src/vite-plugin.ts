/**
 * Vite Plugin for almostnode
 *
 * Serves the service worker file from the package's dist directory,
 * enabling seamless integration when almostnode is installed as an npm package.
 */

import type { Plugin, ViteDevServer } from 'vite';
import * as fs from 'fs';
import * as path from 'path';
import { fileURLToPath } from 'url';

// eslint-disable-next-line @typescript-eslint/ban-ts-comment
// @ts-ignore - import.meta.url is available in ESM
const __dirname = path.dirname(fileURLToPath(import.meta.url));

export interface AlmostnodePluginOptions {
  /**
   * The URL path where the service worker will be served
   * @default '/__sw__.js'
   */
  swPath?: string;
}

/**
 * Vite plugin that serves the almostnode service worker file.
 *
 * When almostnode is installed as an npm package, the service worker file
 * is located at node_modules/almostnode/dist/__sw__.js but the browser
 * tries to load it from the root URL (/__sw__.js). This plugin intercepts
 * requests to the service worker path and serves the file from the correct location.
 *
 * @example
 * ```typescript
 * // vite.config.ts
 * import { defineConfig } from 'vite';
 * import { almostnodePlugin } from 'almostnode/vite';
 *
 * export default defineConfig({
 *   plugins: [almostnodePlugin()]
 * });
 * ```
 */
export function almostnodePlugin(options: AlmostnodePluginOptions = {}): Plugin {
  const swPath = options.swPath || '/__sw__.js';

  return {
    name: 'almostnode',

    configureServer(server: ViteDevServer) {
      server.middlewares.use(swPath, (_req, res) => {
        // The service worker file is in the dist directory relative to this file
        // In src: ../dist/__sw__.js
        // In dist: ./__sw__.js
        let swFilePath = path.join(__dirname, '__sw__.js');

        // If running from src directory during development, look in dist
        if (!fs.existsSync(swFilePath)) {
          swFilePath = path.join(__dirname, '../dist/__sw__.js');
        }

        if (!fs.existsSync(swFilePath)) {
          res.statusCode = 404;
          res.end('Service worker file not found. Make sure almostnode is built.');
          return;
        }

        res.setHeader('Content-Type', 'application/javascript');
        res.setHeader('Cache-Control', 'no-cache');
        res.end(fs.readFileSync(swFilePath));
      });
    },
  };
}

export default almostnodePlugin;
