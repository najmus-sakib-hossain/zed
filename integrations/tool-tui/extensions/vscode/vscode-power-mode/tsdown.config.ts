import { defineConfig } from 'tsdown'

export default defineConfig({
  entry: ['src/index.ts'],
  format: ['cjs'],
  external: ['vscode'],
  fixedExtension: false,
  shims: false,
  dts: false,
  sourcemap: true,
})
