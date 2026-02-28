import { describe, it, expect } from 'vitest';
import { parseNextConfigValue } from '../src/frameworks/next-config-parser';

describe('parseNextConfigValue', () => {
  it('should extract string literal from export default object', () => {
    const content = `export default { assetPrefix: "/marketing" }`;
    expect(parseNextConfigValue(content, 'assetPrefix')).toBe('/marketing');
  });

  it('should extract single-quoted string literal', () => {
    const content = `export default { basePath: '/docs' }`;
    expect(parseNextConfigValue(content, 'basePath')).toBe('/docs');
  });

  it('should extract from module.exports', () => {
    const content = `module.exports = { basePath: '/docs' }`;
    expect(parseNextConfigValue(content, 'basePath')).toBe('/docs');
  });

  it('should resolve variable reference with export default', () => {
    const content = `
const PREFIX = '/marketing';
export default { assetPrefix: PREFIX }
`;
    expect(parseNextConfigValue(content, 'assetPrefix')).toBe('/marketing');
  });

  it('should resolve variable reference with module.exports', () => {
    const content = `
const bp = '/docs';
module.exports = { basePath: bp }
`;
    expect(parseNextConfigValue(content, 'basePath')).toBe('/docs');
  });

  it('should resolve named variable then export default', () => {
    const content = `
const config = { assetPrefix: "/cdn" };
export default config
`;
    expect(parseNextConfigValue(content, 'assetPrefix')).toBe('/cdn');
  });

  it('should resolve named variable then module.exports', () => {
    const content = `
const nextConfig = { basePath: "/app" };
module.exports = nextConfig
`;
    expect(parseNextConfigValue(content, 'basePath')).toBe('/app');
  });

  it('should handle TypeScript with import type and type annotation', () => {
    const content = `
import type { NextConfig } from "next"

const config: NextConfig = {
  assetPrefix: "/marketing",
  reactStrictMode: true,
}

export default config
`;
    expect(parseNextConfigValue(content, 'assetPrefix', true)).toBe('/marketing');
  });

  it('should handle TypeScript with variable reference', () => {
    const content = `
import type { NextConfig } from "next"

const PREFIX = '/static';
const config: NextConfig = {
  assetPrefix: PREFIX,
  basePath: '/docs',
}

export default config
`;
    expect(parseNextConfigValue(content, 'assetPrefix', true)).toBe('/static');
    expect(parseNextConfigValue(content, 'basePath', true)).toBe('/docs');
  });

  it('should extract template literal without expressions', () => {
    const content = 'export default { assetPrefix: `/marketing` }';
    expect(parseNextConfigValue(content, 'assetPrefix')).toBe('/marketing');
  });

  it('should return null when key is not found', () => {
    const content = `export default { reactStrictMode: true }`;
    expect(parseNextConfigValue(content, 'assetPrefix')).toBeNull();
  });

  it('should return null for empty content', () => {
    expect(parseNextConfigValue('', 'assetPrefix')).toBeNull();
  });

  it('should extract correct key when multiple keys present', () => {
    const content = `
export default {
  assetPrefix: "/cdn",
  basePath: "/docs",
  reactStrictMode: true,
}
`;
    expect(parseNextConfigValue(content, 'assetPrefix')).toBe('/cdn');
    expect(parseNextConfigValue(content, 'basePath')).toBe('/docs');
  });

  it('should handle wrapper function like defineConfig', () => {
    const content = `export default defineConfig({ assetPrefix: "/prefix" })`;
    expect(parseNextConfigValue(content, 'assetPrefix')).toBe('/prefix');
  });

  it('should fall back to regex for unparseable content', () => {
    // Broken syntax that acorn can't parse but regex can still find
    const content = `
some broken {{{ code
export default { assetPrefix: "/marketing" }
`;
    expect(parseNextConfigValue(content, 'assetPrefix')).toBe('/marketing');
  });

  it('should return null for non-string values like booleans', () => {
    const content = `export default { reactStrictMode: true }`;
    expect(parseNextConfigValue(content, 'reactStrictMode')).toBeNull();
  });

  it('should return null for dynamic values like process.env', () => {
    const content = `export default { assetPrefix: process.env.PREFIX }`;
    expect(parseNextConfigValue(content, 'assetPrefix')).toBeNull();
  });
});
