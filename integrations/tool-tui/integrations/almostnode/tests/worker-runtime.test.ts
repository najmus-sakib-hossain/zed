/**
 * Worker Runtime Tests
 *
 * Tests for the optional Web Worker support in Just Node runtime.
 */

import { describe, it, expect, beforeEach } from 'vitest';
import { VirtualFS } from '../src/virtual-fs';
import { createRuntime } from '../src/create-runtime';

describe('WorkerRuntime', () => {
  let vfs: VirtualFS;

  beforeEach(() => {
    vfs = new VirtualFS();
    vfs.mkdirSync('/project', { recursive: true });
  });

  describe('createRuntime factory', () => {
    it('should create main-thread runtime when useWorker is false', async () => {
      const runtime = await createRuntime(vfs, { dangerouslyAllowSameOrigin: true, useWorker: false });
      // Check it has the expected interface
      expect(typeof runtime.execute).toBe('function');
      expect(typeof runtime.runFile).toBe('function');
      expect(typeof runtime.clearCache).toBe('function');
      expect(typeof runtime.getVFS).toBe('function');
    });

    it('should throw error when neither sandbox nor dangerouslyAllowSameOrigin is provided', async () => {
      await expect(createRuntime(vfs)).rejects.toThrow('almostnode: For security');
    });

    it('should create runtime with dangerouslyAllowSameOrigin', async () => {
      const runtime = await createRuntime(vfs, { dangerouslyAllowSameOrigin: true });
      expect(typeof runtime.execute).toBe('function');
    });

    it('should fallback to main-thread runtime if Worker is unavailable', async () => {
      // Mock Worker as undefined
      const originalWorker = globalThis.Worker;
      // @ts-expect-error - intentionally removing Worker
      delete globalThis.Worker;

      const runtime = await createRuntime(vfs, { dangerouslyAllowSameOrigin: true, useWorker: true });
      expect(typeof runtime.execute).toBe('function');

      // Restore Worker
      globalThis.Worker = originalWorker;
    });
  });

  describe('Runtime.execute() async interface', () => {
    it('should return a Promise from execute()', async () => {
      const runtime = await createRuntime(vfs, { dangerouslyAllowSameOrigin: true, useWorker: false });
      const result = runtime.execute('module.exports = 42;');
      expect(result).toBeInstanceOf(Promise);
      const resolved = await result;
      expect(resolved.exports).toBe(42);
    });

    it('should return a Promise from runFile()', async () => {
      vfs.writeFileSync('/project/test.js', 'module.exports = "hello";');
      const runtime = await createRuntime(vfs, { dangerouslyAllowSameOrigin: true, useWorker: false });
      const result = runtime.runFile('/project/test.js');
      expect(result).toBeInstanceOf(Promise);
      const resolved = await result;
      expect(resolved.exports).toBe('hello');
    });
  });

  describe('execute() parity between main-thread and async', () => {
    const testCases = [
      {
        name: 'simple expression',
        code: 'module.exports = 1 + 1;',
        expected: 2,
      },
      {
        name: 'string export',
        code: 'module.exports = "hello world";',
        expected: 'hello world',
      },
      {
        name: 'object export',
        code: 'module.exports = { foo: "bar", num: 42 };',
        expected: { foo: 'bar', num: 42 },
      },
      {
        name: 'function export',
        code: 'module.exports = function add(a, b) { return a + b; };',
        expectedFn: (fn: unknown) => {
          expect(typeof fn).toBe('function');
          expect((fn as (a: number, b: number) => number)(2, 3)).toBe(5);
        },
      },
      {
        name: 'require builtin (path)',
        code: 'const path = require("path"); module.exports = path.join("a", "b");',
        expected: 'a/b',
      },
      {
        name: 'multiple exports',
        code: 'exports.a = 1; exports.b = 2; exports.c = 3;',
        expected: { a: 1, b: 2, c: 3 },
      },
    ];

    for (const { name, code, expected, expectedFn } of testCases) {
      it(`main-thread: ${name}`, async () => {
        const runtime = await createRuntime(vfs, { dangerouslyAllowSameOrigin: true, useWorker: false });
        const result = await runtime.execute(code);

        if (expectedFn) {
          expectedFn(result.exports);
        } else {
          expect(result.exports).toEqual(expected);
        }
      });
    }
  });

  describe('environment variables', () => {
    it('should pass env to runtime', async () => {
      const runtime = await createRuntime(vfs, {
        dangerouslyAllowSameOrigin: true,
        useWorker: false,
        env: { TEST_VAR: 'hello', NODE_ENV: 'test' },
      });
      const result = await runtime.execute('module.exports = process.env.TEST_VAR;');
      expect(result.exports).toBe('hello');
    });

    it('should handle multiple env vars', async () => {
      const runtime = await createRuntime(vfs, {
        dangerouslyAllowSameOrigin: true,
        useWorker: false,
        env: { VAR1: 'one', VAR2: 'two', VAR3: 'three' },
      });
      const result = await runtime.execute(`
        module.exports = {
          var1: process.env.VAR1,
          var2: process.env.VAR2,
          var3: process.env.VAR3,
        };
      `);
      expect(result.exports).toEqual({ var1: 'one', var2: 'two', var3: 'three' });
    });
  });

  describe('console capture', () => {
    it('should forward console output via callback', async () => {
      const logs: Array<{ method: string; args: unknown[] }> = [];

      const runtime = await createRuntime(vfs, {
        dangerouslyAllowSameOrigin: true,
        useWorker: false,
        onConsole: (method, args) => logs.push({ method, args }),
      });

      await runtime.execute('console.log("hello"); console.warn("warning"); console.error("error");');

      expect(logs).toHaveLength(3);
      expect(logs[0]).toEqual({ method: 'log', args: ['hello'] });
      expect(logs[1]).toEqual({ method: 'warn', args: ['warning'] });
      expect(logs[2]).toEqual({ method: 'error', args: ['error'] });
    });
  });

  describe('error handling', () => {
    it('should propagate syntax errors', async () => {
      const runtime = await createRuntime(vfs, { dangerouslyAllowSameOrigin: true, useWorker: false });

      await expect(runtime.execute('this is not valid javascript {')).rejects.toThrow();
    });

    it('should propagate runtime errors', async () => {
      const runtime = await createRuntime(vfs, { dangerouslyAllowSameOrigin: true, useWorker: false });

      await expect(
        runtime.execute('throw new Error("test error");')
      ).rejects.toThrow('test error');
    });

    it('should propagate require errors for missing modules', async () => {
      const runtime = await createRuntime(vfs, { dangerouslyAllowSameOrigin: true, useWorker: false });

      await expect(
        runtime.execute('require("nonexistent-module");')
      ).rejects.toThrow(/Cannot find module/);
    });
  });

  describe('module caching', () => {
    it('should cache modules', async () => {
      vfs.writeFileSync('/project/counter.js', `
        let count = 0;
        module.exports = {
          increment: () => ++count,
          get: () => count,
        };
      `);

      const runtime = await createRuntime(vfs, { dangerouslyAllowSameOrigin: true, useWorker: false });

      // First call
      const result1 = await runtime.execute(`
        const counter = require('/project/counter.js');
        counter.increment();
        module.exports = counter.get();
      `, '/project/test1.js');
      expect(result1.exports).toBe(1);

      // Second call - should use cached module
      const result2 = await runtime.execute(`
        const counter = require('/project/counter.js');
        counter.increment();
        module.exports = counter.get();
      `, '/project/test2.js');
      expect(result2.exports).toBe(2);
    });

    it('should clear cache when requested', async () => {
      vfs.writeFileSync('/project/value.js', 'module.exports = 1;');

      const runtime = await createRuntime(vfs, { dangerouslyAllowSameOrigin: true, useWorker: false });

      const result1 = await runtime.execute('module.exports = require("/project/value.js");');
      expect(result1.exports).toBe(1);

      // Update file
      vfs.writeFileSync('/project/value.js', 'module.exports = 2;');

      // Without cache clear, should still return old value
      const result2 = await runtime.execute('module.exports = require("/project/value.js");');
      expect(result2.exports).toBe(1);

      // Clear cache
      runtime.clearCache();

      // Now should return new value
      const result3 = await runtime.execute('module.exports = require("/project/value.js");');
      expect(result3.exports).toBe(2);
    });
  });

  describe('working directory', () => {
    it('should use specified cwd', async () => {
      const runtime = await createRuntime(vfs, {
        dangerouslyAllowSameOrigin: true,
        useWorker: false,
        cwd: '/project',
      });

      const result = await runtime.execute('module.exports = process.cwd();');
      expect(result.exports).toBe('/project');
    });

    it('should default to root', async () => {
      const runtime = await createRuntime(vfs, { dangerouslyAllowSameOrigin: true, useWorker: false });

      const result = await runtime.execute('module.exports = process.cwd();');
      expect(result.exports).toBe('/');
    });
  });
});

describe('VirtualFS snapshot', () => {
  it('should serialize and deserialize empty file tree', () => {
    const vfs1 = new VirtualFS();
    const snapshot = vfs1.toSnapshot();
    const vfs2 = VirtualFS.fromSnapshot(snapshot);

    expect(vfs2.existsSync('/')).toBe(true);
    expect(vfs2.readdirSync('/')).toEqual([]);
  });

  it('should serialize and deserialize files', () => {
    const vfs1 = new VirtualFS();
    vfs1.writeFileSync('/test.txt', 'hello world');

    const snapshot = vfs1.toSnapshot();
    const vfs2 = VirtualFS.fromSnapshot(snapshot);

    expect(vfs2.existsSync('/test.txt')).toBe(true);
    expect(vfs2.readFileSync('/test.txt', 'utf8')).toBe('hello world');
  });

  it('should serialize and deserialize directories', () => {
    const vfs1 = new VirtualFS();
    vfs1.mkdirSync('/project/src', { recursive: true });
    vfs1.writeFileSync('/project/src/index.ts', 'export const x = 1;');
    vfs1.writeFileSync('/project/package.json', '{"name": "test"}');

    const snapshot = vfs1.toSnapshot();
    const vfs2 = VirtualFS.fromSnapshot(snapshot);

    expect(vfs2.existsSync('/project')).toBe(true);
    expect(vfs2.existsSync('/project/src')).toBe(true);
    expect(vfs2.readFileSync('/project/src/index.ts', 'utf8')).toBe('export const x = 1;');
    expect(vfs2.readFileSync('/project/package.json', 'utf8')).toBe('{"name": "test"}');
  });

  it('should serialize and deserialize binary content', () => {
    const vfs1 = new VirtualFS();
    const binaryData = new Uint8Array([0, 1, 2, 255, 254, 253]);
    vfs1.writeFileSync('/binary.bin', binaryData);

    const snapshot = vfs1.toSnapshot();
    const vfs2 = VirtualFS.fromSnapshot(snapshot);

    const result = vfs2.readFileSync('/binary.bin');
    expect(result).toEqual(binaryData);
  });

  it('should emit change events on write', () => {
    const vfs = new VirtualFS();
    const changes: Array<{ path: string; content: string }> = [];

    vfs.on('change', (path: string, content: string) => {
      changes.push({ path, content });
    });

    vfs.writeFileSync('/test.txt', 'hello');
    vfs.writeFileSync('/test.txt', 'world');

    expect(changes).toHaveLength(2);
    expect(changes[0]).toEqual({ path: '/test.txt', content: 'hello' });
    expect(changes[1]).toEqual({ path: '/test.txt', content: 'world' });
  });

  it('should emit delete events on unlink', () => {
    const vfs = new VirtualFS();
    const deletions: string[] = [];

    vfs.on('delete', (path: string) => {
      deletions.push(path);
    });

    vfs.writeFileSync('/test.txt', 'hello');
    vfs.unlinkSync('/test.txt');

    expect(deletions).toHaveLength(1);
    expect(deletions[0]).toBe('/test.txt');
  });

  it('should allow removing event listeners', () => {
    const vfs = new VirtualFS();
    const changes: string[] = [];

    const listener = (path: string) => {
      changes.push(path);
    };

    vfs.on('change', listener);
    vfs.writeFileSync('/test1.txt', 'hello');
    expect(changes).toHaveLength(1);

    vfs.off('change', listener);
    vfs.writeFileSync('/test2.txt', 'world');
    expect(changes).toHaveLength(1); // Should not have changed
  });
});
