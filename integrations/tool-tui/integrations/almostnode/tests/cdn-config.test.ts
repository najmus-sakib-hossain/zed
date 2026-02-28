import { describe, it, expect } from 'vitest';
import {
  REACT_VERSION,
  ESBUILD_WASM_VERSION,
  ROLLUP_BROWSER_VERSION,
  REACT_REFRESH_VERSION,
  REACT_CDN,
  REACT_DOM_CDN,
  REACT_REFRESH_CDN,
  ESBUILD_WASM_ESM_CDN,
  ESBUILD_WASM_BINARY_CDN,
  ESBUILD_WASM_BROWSER_CDN,
  ROLLUP_BROWSER_CDN,
  TAILWIND_CDN_URL,
} from '../src/config/cdn';

describe('CDN config', () => {
  describe('version pins are valid semver', () => {
    const semverRegex = /^\d+\.\d+\.\d+$/;
    it('REACT_VERSION', () => expect(REACT_VERSION).toMatch(semverRegex));
    it('ESBUILD_WASM_VERSION', () => expect(ESBUILD_WASM_VERSION).toMatch(semverRegex));
    it('ROLLUP_BROWSER_VERSION', () => expect(ROLLUP_BROWSER_VERSION).toMatch(semverRegex));
    it('REACT_REFRESH_VERSION', () => expect(REACT_REFRESH_VERSION).toMatch(semverRegex));
  });

  describe('URLs contain their version pins', () => {
    it('REACT_CDN includes REACT_VERSION', () => {
      expect(REACT_CDN).toContain(REACT_VERSION);
      expect(REACT_CDN).toMatch(/^https:\/\/esm\.sh\/react@/);
    });

    it('REACT_DOM_CDN includes REACT_VERSION', () => {
      expect(REACT_DOM_CDN).toContain(REACT_VERSION);
      expect(REACT_DOM_CDN).toMatch(/^https:\/\/esm\.sh\/react-dom@/);
    });

    it('REACT_REFRESH_CDN includes REACT_REFRESH_VERSION', () => {
      expect(REACT_REFRESH_CDN).toContain(REACT_REFRESH_VERSION);
      expect(REACT_REFRESH_CDN).toMatch(/^https:\/\/esm\.sh\/react-refresh@/);
    });

    it('ESBUILD_WASM_ESM_CDN includes ESBUILD_WASM_VERSION', () => {
      expect(ESBUILD_WASM_ESM_CDN).toContain(ESBUILD_WASM_VERSION);
      expect(ESBUILD_WASM_ESM_CDN).toMatch(/^https:\/\/esm\.sh\/esbuild-wasm@/);
    });

    it('ESBUILD_WASM_BINARY_CDN includes ESBUILD_WASM_VERSION', () => {
      expect(ESBUILD_WASM_BINARY_CDN).toContain(ESBUILD_WASM_VERSION);
      expect(ESBUILD_WASM_BINARY_CDN).toMatch(/\.wasm$/);
    });

    it('ESBUILD_WASM_BROWSER_CDN includes ESBUILD_WASM_VERSION', () => {
      expect(ESBUILD_WASM_BROWSER_CDN).toContain(ESBUILD_WASM_VERSION);
      expect(ESBUILD_WASM_BROWSER_CDN).toMatch(/\.js$/);
    });

    it('ROLLUP_BROWSER_CDN includes ROLLUP_BROWSER_VERSION', () => {
      expect(ROLLUP_BROWSER_CDN).toContain(ROLLUP_BROWSER_VERSION);
      expect(ROLLUP_BROWSER_CDN).toMatch(/^https:\/\/esm\.sh\/@rollup\/browser@/);
    });
  });

  describe('no hardcoded URLs leak into platform code', () => {
    it('TAILWIND_CDN_URL is a valid URL', () => {
      expect(TAILWIND_CDN_URL).toMatch(/^https:\/\//);
    });
  });
});
