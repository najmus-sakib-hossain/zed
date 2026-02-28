/**
 * Tests for macaly-demo functionality
 */

import { describe, it, expect, beforeEach } from 'vitest';
import { VirtualFS } from '../src/virtual-fs';
import { loadMacalyProject, MacalyFiles } from '../src/macaly-demo';

describe('loadMacalyProject', () => {
  let vfs: VirtualFS;

  beforeEach(() => {
    vfs = new VirtualFS();
  });

  it('should load text files into VFS', () => {
    const files: MacalyFiles = {
      '/app/page.tsx': 'export default function Page() { return <div>Hello</div> }',
      '/package.json': '{"name": "test"}',
    };

    loadMacalyProject(vfs, files);

    expect(vfs.existsSync('/app/page.tsx')).toBe(true);
    expect(vfs.readFileSync('/app/page.tsx', 'utf-8')).toBe(
      'export default function Page() { return <div>Hello</div> }'
    );
    expect(vfs.readFileSync('/package.json', 'utf-8')).toBe('{"name": "test"}');
  });

  it('should load base64-encoded binary files into VFS', () => {
    // "Hello" in base64 is "SGVsbG8="
    const files: MacalyFiles = {
      '/public/test.bin': 'base64:SGVsbG8=',
    };

    loadMacalyProject(vfs, files);

    expect(vfs.existsSync('/public/test.bin')).toBe(true);
    const content = vfs.readFileSync('/public/test.bin');
    expect(content.toString()).toBe('Hello');
  });

  it('should handle PNG image data as base64', () => {
    // PNG header bytes (first 8 bytes of a PNG file)
    // 89 50 4E 47 0D 0A 1A 0A in hex
    const pngHeader = Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]);
    const base64Data = pngHeader.toString('base64');

    const files: MacalyFiles = {
      '/public/images/test.png': `base64:${base64Data}`,
    };

    loadMacalyProject(vfs, files);

    expect(vfs.existsSync('/public/images/test.png')).toBe(true);
    const content = vfs.readFileSync('/public/images/test.png');

    // Verify PNG magic bytes
    expect(content[0]).toBe(0x89);
    expect(content[1]).toBe(0x50); // P
    expect(content[2]).toBe(0x4e); // N
    expect(content[3]).toBe(0x47); // G
  });

  it('should create nested directories automatically', () => {
    const files: MacalyFiles = {
      '/deep/nested/path/file.txt': 'content',
    };

    loadMacalyProject(vfs, files);

    expect(vfs.existsSync('/deep')).toBe(true);
    expect(vfs.existsSync('/deep/nested')).toBe(true);
    expect(vfs.existsSync('/deep/nested/path')).toBe(true);
    expect(vfs.existsSync('/deep/nested/path/file.txt')).toBe(true);
  });

  it('should handle mixed text and binary files', () => {
    const files: MacalyFiles = {
      '/app/page.tsx': 'export default function() {}',
      '/public/logo.png': 'base64:iVBORw0KGgo=', // Partial PNG base64
      '/styles/main.css': 'body { color: red; }',
      '/public/data.json': '{"key": "value"}', // JSON is text, not base64
    };

    loadMacalyProject(vfs, files);

    expect(vfs.readFileSync('/app/page.tsx', 'utf-8')).toBe('export default function() {}');
    expect(vfs.readFileSync('/styles/main.css', 'utf-8')).toBe('body { color: red; }');
    expect(vfs.readFileSync('/public/data.json', 'utf-8')).toBe('{"key": "value"}');

    // Binary file should be decoded
    const logoContent = vfs.readFileSync('/public/logo.png');
    expect(Buffer.isBuffer(logoContent)).toBe(true);
  });

  it('should not treat "base64:" in content as binary marker', () => {
    // If "base64:" appears later in the content, it should be treated as text
    const files: MacalyFiles = {
      '/docs/encoding.md': 'To encode to base64: use btoa() function',
    };

    loadMacalyProject(vfs, files);

    expect(vfs.readFileSync('/docs/encoding.md', 'utf-8')).toBe(
      'To encode to base64: use btoa() function'
    );
  });
});
