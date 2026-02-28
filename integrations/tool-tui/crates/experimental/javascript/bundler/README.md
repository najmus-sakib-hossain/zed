
# DX Bundler

⚠️ Early Development (v0.0.1) - Many features are experimental. APIs may change. JavaScript/TypeScript bundler with tree shaking and source maps.

## Installation

```bash
cargo build --release -p dx-bundle-cli ```


## Usage


```bash

# Basic bundle

dx-bundle bundle src/index.ts -o dist/bundle.js

# With minification

dx-bundle bundle src/index.ts -o dist/bundle.js --minify

# Different output format

dx-bundle bundle src/index.ts -o dist/bundle.js --format cjs

# Watch mode

dx-bundle bundle src/index.ts -o dist/bundle.js --watch

# With source maps

dx-bundle bundle src/index.ts -o dist/bundle.js --sourcemap

# Cache management

dx-bundle cache dx-bundle clear ```

## CLI Options

t:0(Option,Default,Description)[]

## Features

### Module Support

- ES modules (import/export)
- CommonJS (require/module.exports)
- Dynamic import() with code splitting
- JSON imports
- CSS imports

### Transformations

- TypeScript (types stripped)
- JSX transformation
- Tree shaking (dead code elimination)
- Minification

### Output

- Multiple formats (ESM, CJS, IIFE, UMD)
- Source map generation
- Code splitting for dynamic imports

### CSS Bundling

- CSS file imports
- CSS modules (scoped class names)
- Asset URL rewriting

## Examples

### Basic Bundle

```javascript
// src/index.js import { add } from './math.js';
console.log(add(1, 2));
// src/math.js export const add = (a, b) => a + b;
export const unused = () => {}; // Tree shaken ```
```bash
dx-bundle bundle src/index.js -o dist/bundle.js ```

### TypeScript

```typescript
// src/index.ts interface User { name: string;
}
const greet = (user: User): string => `Hello, ${user.name}`;
console.log(greet({ name: 'World' }));
```
```bash
dx-bundle bundle src/index.ts -o dist/bundle.js ```


### Code Splitting


```javascript
// src/index.js const loadModule = async () => { const { heavy } = await import('./heavy.js');
heavy();
};
```
Generates separate chunks for dynamic imports.


### CSS


```javascript
// src/index.js import './styles.css';
import styles from './component.module.css';
element.className = styles.container;
```


## Architecture


- SIMD-accelerated import/export scanning
- Parallel module processing
- Binary cache format
- Arena allocator for fast memory management


## License


MIT OR Apache-2.0
