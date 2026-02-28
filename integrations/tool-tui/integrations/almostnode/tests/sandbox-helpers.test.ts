/**
 * Sandbox Helpers Tests
 */

import { describe, it, expect } from 'vitest';
import {
  getSandboxHtml,
  getSandboxVercelConfig,
  generateSandboxFiles,
  SANDBOX_SETUP_INSTRUCTIONS,
} from '../src/sandbox-helpers';

describe('Sandbox Helpers', () => {
  describe('getSandboxHtml', () => {
    it('should generate valid HTML', () => {
      const html = getSandboxHtml();
      expect(html).toContain('<!DOCTYPE html>');
      expect(html).toContain('<html>');
      expect(html).toContain('</html>');
    });

    it('should include almostnode import from unpkg by default', () => {
      const html = getSandboxHtml();
      expect(html).toContain('https://unpkg.com/almostnode/dist/index.js');
    });

    it('should use custom URL when provided as string (legacy)', () => {
      const customUrl = 'https://cdn.example.com/almostnode.js';
      const html = getSandboxHtml(customUrl);
      expect(html).toContain(customUrl);
      expect(html).not.toContain('unpkg.com');
    });

    it('should use custom URL when provided in options', () => {
      const customUrl = 'https://cdn.example.com/almostnode.js';
      const html = getSandboxHtml({ almostnodeUrl: customUrl });
      expect(html).toContain(customUrl);
      expect(html).not.toContain('unpkg.com');
    });

    it('should include service worker registration by default', () => {
      const html = getSandboxHtml();
      expect(html).toContain('serviceWorker');
      expect(html).toContain("register('/__sw__.js'");
    });

    it('should exclude service worker registration when disabled', () => {
      const html = getSandboxHtml({ includeServiceWorker: false });
      expect(html).not.toContain("register('/__sw__.js'");
    });

    it('should include VirtualFS and Runtime imports', () => {
      const html = getSandboxHtml();
      expect(html).toContain('VirtualFS');
      expect(html).toContain('Runtime');
    });

    it('should include message handler for postMessage communication', () => {
      const html = getSandboxHtml();
      expect(html).toContain("addEventListener('message'");
      expect(html).toContain("type: 'ready'");
      expect(html).toContain("case 'init'");
      expect(html).toContain("case 'execute'");
      expect(html).toContain("case 'runFile'");
    });

    it('should signal ready to parent on load', () => {
      const html = getSandboxHtml();
      expect(html).toContain("parent.postMessage({ type: 'ready' }");
    });
  });

  describe('getSandboxVercelConfig', () => {
    it('should return valid config object', () => {
      const config = getSandboxVercelConfig();
      expect(config).toHaveProperty('headers');
    });

    it('should include CORS headers', () => {
      const config = getSandboxVercelConfig() as { headers: Array<{ headers: Array<{ key: string; value: string }> }> };
      const headers = config.headers[0].headers;

      const corsHeader = headers.find(h => h.key === 'Access-Control-Allow-Origin');
      expect(corsHeader).toBeDefined();
      expect(corsHeader?.value).toBe('*');
    });

    it('should include Cross-Origin-Resource-Policy header', () => {
      const config = getSandboxVercelConfig() as { headers: Array<{ headers: Array<{ key: string; value: string }> }> };
      const headers = config.headers[0].headers;

      const corpHeader = headers.find(h => h.key === 'Cross-Origin-Resource-Policy');
      expect(corpHeader).toBeDefined();
      expect(corpHeader?.value).toBe('cross-origin');
    });
  });

  describe('generateSandboxFiles', () => {
    it('should generate index.html, vercel.json, and __sw__.js', () => {
      const files = generateSandboxFiles();
      expect(files).toHaveProperty('index.html');
      expect(files).toHaveProperty('vercel.json');
      expect(files).toHaveProperty('__sw__.js');
    });

    it('should generate valid HTML in index.html', () => {
      const files = generateSandboxFiles();
      expect(files['index.html']).toContain('<!DOCTYPE html>');
    });

    it('should generate valid JSON in vercel.json', () => {
      const files = generateSandboxFiles();
      expect(() => JSON.parse(files['vercel.json'])).not.toThrow();
    });

    it('should use custom URL in generated HTML (legacy string)', () => {
      const customUrl = 'https://my-cdn.com/almostnode.js';
      const files = generateSandboxFiles(customUrl);
      expect(files['index.html']).toContain(customUrl);
    });

    it('should use custom URL in generated HTML (options)', () => {
      const customUrl = 'https://my-cdn.com/almostnode.js';
      const files = generateSandboxFiles({ almostnodeUrl: customUrl });
      expect(files['index.html']).toContain(customUrl);
    });

    it('should include service worker file with valid JS content', () => {
      const files = generateSandboxFiles();
      expect(files['__sw__.js']).toBeDefined();
      expect(files['__sw__.js']).toContain('self');
    });

    it('should not include service worker when disabled', () => {
      const files = generateSandboxFiles({ includeServiceWorker: false });
      expect(files['__sw__.js']).toBeUndefined();
      expect(files['index.html']).not.toContain("register('/__sw__.js'");
    });
  });

  describe('SANDBOX_SETUP_INSTRUCTIONS', () => {
    it('should contain setup steps', () => {
      expect(SANDBOX_SETUP_INSTRUCTIONS).toContain('mkdir');
      expect(SANDBOX_SETUP_INSTRUCTIONS).toContain('vercel');
      expect(SANDBOX_SETUP_INSTRUCTIONS).toContain('createRuntime');
    });
  });
});
