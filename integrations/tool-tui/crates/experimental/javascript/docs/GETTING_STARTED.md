
# Getting Started with DX-JS

⚠️ Early Development Notice (v0.0.1) DX-JS is in early development. Many features are experimental or incomplete. APIs may change without notice. Please test thoroughly before using in production environments. DX-JS is a high-performance JavaScript/TypeScript toolchain written in Rust. It includes four main tools: -dx - A fast npm-compatible package manager -dx-js - A JIT-compiled JavaScript/TypeScript runtime -dx-bundle - An ES module bundler -dx-test - A parallel test runner

## Prerequisites

- Rust 1.70+
- Install from rustup.rs
- Git
- For cloning the repository

## Installation

### From Source

```bash


# Clone the repository


git clone https://github.com/example/dx-javascript cd dx-javascript


# Build release binaries


cargo build --release


# Binaries are in target/release/



# - dx (package manager)



# - dx-js (runtime)



# - dx-bundle (bundler)



# - dx-test (test runner)


```

### Add to PATH (Optional)

```bash


# Linux/macOS


export PATH="10.59xATH:$(pwd)/target/release"


# Windows (PowerShell)


$env:PATH += ";$(Get-Location)\target\release"
```

## Quick Start: Your First Project

Let's create a simple project that demonstrates all four tools.

### 1. Initialize a Project

```bash
mkdir my-dx-project && cd my-dx-project dx init ```
This creates a `package.json` with default settings.


### 2. Create Source Files


Create `src/index.ts`:
```typescript
// src/index.ts interface User { name: string;
age: number;
}
function greet(user: User): string { return `Hello, ${user.name}! You are ${user.age} years old.`;
}
const user: User = { name: "Alice", age: 30 };
console.log(greet(user));
// Async example async function fetchData(): Promise<void> { console.log("Fetching data...");
// Simulated async operation await new Promise(resolve => setTimeout(resolve, 100));
console.log("Data fetched!");
}
fetchData();
```


### 3. Run with the Runtime


```bash
dx-js src/index.ts ```
Output:
```
Hello, Alice! You are 30 years old.
Fetching data...
Data fetched!
```

### 4. Create Tests

Create `tests/index.test.ts`:
```typescript
// tests/index.test.ts describe('User greeting', () => { test('greets user correctly', () => { const greet = (name: string) => `Hello, ${name}!`;
expect(greet('Alice')).toBe('Hello, Alice!');
});
test('handles empty name', () => { const greet = (name: string) => name ? `Hello, ${name}!` : 'Hello, stranger!';
expect(greet('')).toBe('Hello, stranger!');
});
});
describe('Math operations', () => { test('adds numbers correctly', () => { expect(1 + 1).toBe(2);
expect(10 + 20).toBe(30);
});
test('handles negative numbers', () => { expect(-5 + 3).toBe(-2);
});
});
```

### 5. Run Tests

```bash
dx-test ```
Output:
```
Running tests...
User greeting ✓ greets user correctly (1ms)
✓ handles empty name (0ms)
Math operations ✓ adds numbers correctly (0ms)
✓ handles negative numbers (0ms)
Tests: 4 passed, 0 failed Time: 15ms ```

### 6. Bundle for Production

```bash
dx-bundle bundle src/index.ts -o dist/bundle.js --minify ```
This creates an optimized bundle in `dist/bundle.js`.


## Tool Reference



### Package Manager (dx)


The `dx` command manages dependencies and runs scripts.
```bash

# Initialize a new project

dx init

# Add dependencies

dx add lodash # Production dependency dx add typescript --dev # Development dependency dx add react@18.2.0 # Specific version

# Install all dependencies

dx install

# Remove a dependency

dx remove lodash

# Run package.json scripts

dx run build dx run test

# List installed packages

dx list dx list -g # Global packages

# Global installation

dx install -g typescript ```

### Runtime (dx-js)

The `dx-js` command runs JavaScript and TypeScript files.
```bash


# Run a file


dx-js script.js dx-js app.ts


# Interactive REPL


dx-js


# With memory limit


dx-js --max-heap-size=256 script.js


# Show version


dx-js --version ```
Supported Features: -ES2022 JavaScript -TypeScript (type-stripped, no type checking) -CommonJS and ES Modules -Async/await -Classes and inheritance -Destructuring -Template literals -Arrow functions


### Bundler (dx-bundle)


The `dx-bundle` command bundles ES modules for browsers or Node.js.
```bash

# Basic bundle

dx-bundle bundle src/index.ts -o dist/bundle.js

# With options

dx-bundle bundle src/index.ts \
- output dist/bundle.js \
- format esm \
- minify \
- sourcemap

# Watch mode (rebuilds on changes)

dx-bundle bundle src/index.ts -o dist/bundle.js --watch ```
Output Formats: -`esm` - ES Modules (default) -`cjs` - CommonJS -`iife` - Immediately Invoked Function Expression

### Test Runner (dx-test)

The `dx-test` command runs tests in parallel.
```bash


# Run all tests


dx-test


# Run specific file


dx-test tests/math.test.ts


# With coverage


dx-test --coverage


# Watch mode


dx-test --watch


# Filter by test name


dx-test --filter "adds numbers"
```
Test API:
```javascript
describe('Suite name', () => { beforeEach(() => { /* setup */ });
afterEach(() => { /* cleanup */ });
test('test name', () => { expect(value).toBe(expected);
expect(value).toEqual(expected);
expect(value).toBeTruthy();
expect(value).toBeFalsy();
expect(fn).toThrow();
});
test.skip('skipped test', () => { });
test.only('only this test', () => { });
});
```

## Configuration

### dx.config.json

Create a `dx.config.json` file in your project root for custom settings:
```json
{ "runtime": { "maxHeapSize": 512, "moduleResolution": "node"
}, "bundler": { "target": "es2022", "minify": true, "sourceMaps": true }, "testRunner": { "parallel": true, "coverage": false, "timeout": 5000 }
}
```

### Environment Variables

```bash


# Set maximum heap size (MB)


DX_MAX_HEAP_SIZE=1024 dx-js script.js


# Enable verbose logging


DX_VERBOSE=1 dx install ```


## Common Patterns



### Using npm Packages


```bash

# Install a package

dx add chalk

# Use in your code

import chalk from 'chalk';
console.log(chalk.blue('Hello, world!'));
```


### TypeScript Configuration


DX-JS supports TypeScript out of the box. For type checking, create a `tsconfig.json`:
```json
{ "compilerOptions": { "target": "ES2022", "module": "ESNext", "strict": true, "esModuleInterop": true }, "include": ["src/**/*"], "exclude": ["node_modules"]
}
```
Note: DX-JS strips types at runtime but doesn't perform type checking. Use `tsc --noEmit` for type checking.


### Monorepo Setup


```bash

# Root package.json

{ "workspaces": ["packages/*"]
}

# Install all workspace dependencies

dx install ```

## Troubleshooting

### "Module not found" Error

- Check that the package is installed: `dx list`
- Verify the import path is correct
- Try reinstalling: `dx install`

### Out of Memory Error

Increase the heap size:
```bash
dx-js --max-heap-size=1024 script.js ```


### TypeScript Errors


DX-JS doesn't type-check. Run `tsc --noEmit` separately for type checking.


## Next Steps


- API Reference (./API_REFERENCE.md)
- Detailed API documentation
- Benchmarks (./BENCHMARKS.md)
- Performance comparisons
- Compatibility Matrix (./COMPATIBILITY.md)
- Node.js API support status


## Getting Help


- Check the FAQ (./FAQ.md) for common questions
- Report issues on GitHub
- Join our Discord community
