/**
 * Centralized CDN version pins and URLs.
 * Change versions here to update them across the entire platform.
 */

// ── Version pins ──
export const REACT_VERSION = '18.2.0';
export const ESBUILD_WASM_VERSION = '0.20.0';
export const ROLLUP_BROWSER_VERSION = '4.9.0';
export const REACT_REFRESH_VERSION = '0.14.0';

// ── React CDN URLs ──
export const REACT_CDN = `https://esm.sh/react@${REACT_VERSION}`;
export const REACT_DOM_CDN = `https://esm.sh/react-dom@${REACT_VERSION}`;
export const REACT_REFRESH_CDN = `https://esm.sh/react-refresh@${REACT_REFRESH_VERSION}/runtime`;

// ── Build tool CDN URLs ──
export const ESBUILD_WASM_ESM_CDN = `https://esm.sh/esbuild-wasm@${ESBUILD_WASM_VERSION}`;
export const ESBUILD_WASM_BINARY_CDN = `https://unpkg.com/esbuild-wasm@${ESBUILD_WASM_VERSION}/esbuild.wasm`;
export const ESBUILD_WASM_BROWSER_CDN = `https://unpkg.com/esbuild-wasm@${ESBUILD_WASM_VERSION}/esm/browser.min.js`;
export const ROLLUP_BROWSER_CDN = `https://esm.sh/@rollup/browser@${ROLLUP_BROWSER_VERSION}`;

// ── Styling CDN URLs ──
export const TAILWIND_CDN_URL = 'https://cdn.tailwindcss.com';
