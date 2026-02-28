/**
 * Type declarations for CDN-loaded modules
 *
 * Dynamic imports use variable URLs from src/config/cdn.ts, so we declare
 * types on the base module names. TypeScript resolves them via the
 * `esbuild-wasm` and `@rollup/browser` module declarations below.
 */

// Type declarations for esbuild-wasm to support dynamic import from CDN
interface EsbuildTransformOptions {
  loader?: string;
  jsx?: string;
  jsxFactory?: string;
  jsxFragment?: string;
  jsxImportSource?: string;
  sourcemap?: boolean | 'inline' | 'external' | 'both';
  sourcefile?: string;
  target?: string | string[];
  format?: 'iife' | 'cjs' | 'esm';
  minify?: boolean;
  tsconfigRaw?: string | object;
  platform?: 'browser' | 'node' | 'neutral';
  define?: Record<string, string>;
}

interface EsbuildTransformResult {
  code: string;
  map: string;
  warnings: unknown[];
}

// Declare the esbuild-wasm module for type-only imports
declare module 'esbuild-wasm' {
  export function initialize(options?: { wasmURL?: string; worker?: boolean }): Promise<void>;
  export function transform(input: string, options?: EsbuildTransformOptions): Promise<EsbuildTransformResult>;
  export function build(options: unknown): Promise<unknown>;
  export function formatMessages(messages: unknown[], options: unknown): Promise<string[]>;
  export const version: string;
}

// Centralized Window interface augmentation for esbuild
interface Window {
  __esbuild?: typeof import('esbuild-wasm');
  __esbuildInitPromise?: Promise<void>;
}
