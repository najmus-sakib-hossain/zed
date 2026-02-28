/**
 * Next.js Plugin for almostnode
 *
 * Provides utilities for serving the service worker file in Next.js applications.
 */

import * as fs from 'fs';
import * as path from 'path';
import { fileURLToPath } from 'url';

// eslint-disable-next-line @typescript-eslint/ban-ts-comment
// @ts-ignore - import.meta.url is available in ESM
const __dirname = path.dirname(fileURLToPath(import.meta.url));

/**
 * Get the contents of the almostnode service worker file.
 * Use this in a Next.js API route or middleware to serve the service worker.
 *
 * @example
 * ```typescript
 * // app/api/__sw__/route.ts (App Router)
 * import { getServiceWorkerContent } from 'almostnode/next';
 *
 * export async function GET() {
 *   const content = getServiceWorkerContent();
 *   return new Response(content, {
 *     headers: {
 *       'Content-Type': 'application/javascript',
 *       'Cache-Control': 'no-cache',
 *     },
 *   });
 * }
 * ```
 *
 * @example
 * ```typescript
 * // pages/api/__sw__.ts (Pages Router)
 * import { getServiceWorkerContent } from 'almostnode/next';
 * import type { NextApiRequest, NextApiResponse } from 'next';
 *
 * export default function handler(req: NextApiRequest, res: NextApiResponse) {
 *   const content = getServiceWorkerContent();
 *   res.setHeader('Content-Type', 'application/javascript');
 *   res.setHeader('Cache-Control', 'no-cache');
 *   res.send(content);
 * }
 * ```
 */
export function getServiceWorkerContent(): string {
  // The service worker file is in the dist directory relative to this file
  // In src: ../dist/__sw__.js
  // In dist: ./__sw__.js
  let swFilePath = path.join(__dirname, '__sw__.js');

  // If running from src directory during development, look in dist
  if (!fs.existsSync(swFilePath)) {
    swFilePath = path.join(__dirname, '../dist/__sw__.js');
  }

  if (!fs.existsSync(swFilePath)) {
    throw new Error('Service worker file not found. Make sure almostnode is built.');
  }

  return fs.readFileSync(swFilePath, 'utf-8');
}

/**
 * Get the path to the almostnode service worker file.
 * Useful if you want to copy it to your public directory.
 *
 * @example
 * ```javascript
 * // scripts/copy-sw.js
 * const { getServiceWorkerPath } = require('almostnode/next');
 * const fs = require('fs');
 * const path = require('path');
 *
 * const swPath = getServiceWorkerPath();
 * fs.copyFileSync(swPath, path.join(__dirname, '../public/__sw__.js'));
 * ```
 */
export function getServiceWorkerPath(): string {
  let swFilePath = path.join(__dirname, '__sw__.js');

  if (!fs.existsSync(swFilePath)) {
    swFilePath = path.join(__dirname, '../dist/__sw__.js');
  }

  if (!fs.existsSync(swFilePath)) {
    throw new Error('Service worker file not found. Make sure almostnode is built.');
  }

  return swFilePath;
}

export default { getServiceWorkerContent, getServiceWorkerPath };
