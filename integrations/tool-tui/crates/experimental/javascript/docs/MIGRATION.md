
# Migration Guide

⚠️ Early Development Notice (v0.0.1) DX-JS is in early development. Migration paths may change as the project matures. Consider running DX alongside your existing tools during evaluation. This guide helps you migrate existing Node.js/npm/Jest projects to DX-JS.

## Table of Contents

- From npm to dx
- From Node.js to dx-js
- From Jest to dx-test
- From webpack/esbuild to dx-bundle
- Common Issues

## From npm to dx

### Basic Migration

+------+---------+-----+---------+
| npm  | Command | dx  | Command |
+======+=========+=====+=========+
| `npm | init`   | `dx | init`   |
+------+---------+-----+---------+



### package.json Compatibility

Your existing `package.json` works with dx without modification:
```json
{ "name": "my-project", "version": "1.0.0", "scripts": { "build": "dx-bundle bundle src/index.ts -o dist/bundle.js", "test": "dx-test", "start": "dx-js src/index.ts"
}, "dependencies": { "lodash": "^4.17.21"
}, "devDependencies": { "typescript": "^5.0.0"
}
}
```

### Lock File Migration

dx uses its own lock file format (`dx-lock.json`). On first `dx install`, it will: -Read your existing `package-lock.json` or `yarn.lock` -Resolve dependencies -Create `dx-lock.json` You can keep both lock files during migration, or remove the old one once you've fully migrated.

### Workspaces

dx supports npm workspaces:
```json
{ "workspaces": ["packages/*"]
}
```
Run `dx install` from the root to install all workspace dependencies.

## From Node.js to dx-js

### Running Scripts

+---------+------------+
| Node.js | dx-js      |
+=========+============+
| `node   | script.js` |
+---------+------------+



### TypeScript Support

dx-js runs TypeScript directly without compilation:
```bash


# No need for ts-node or tsc


dx-js app.ts ```
Note: dx-js strips types but doesn't type-check. For type checking, run `tsc --noEmit` separately.


### Environment Variables


dx-js loads `.env` files automatically:
```bash

# .env

DATABASE_URL=postgres://localhost/mydb API_KEY=secret123 ```
```javascript
// Access in code console.log(process.env.DATABASE_URL);
```

### Module Resolution

dx-js follows Node.js module resolution:
```javascript
// CommonJS const lodash = require('lodash');
const local = require('./local');
// ES Modules import lodash from 'lodash';
import local from './local.js';
```

### API Differences

Most Node.js APIs work identically. Key differences: -Unimplemented APIs throw clear errors:
```javascript
// Instead of undefined behavior fs.watch('./file'); // Throws: "Not implemented: fs.watch"
```
- Memory management:
```javascript
// Check memory usage console.log(process.memoryUsage());
// Set heap limit via CLI // dx-js --max-heap-size=1024 script.js ```
- Feature detection:
```javascript
// Check supported features if (dx.features.es2022) { // Use ES2022 features }
```


### Async/Await


Works identically to Node.js:
```javascript
async function main() { const data = await fetchData();
console.log(data);
}
main();
```
Top-level await is supported in ES modules:
```javascript
// app.mjs const data = await fetchData();
export default data;
```


## From Jest to dx-test



### Test File Migration


dx-test is Jest-compatible. Most tests work without changes:
```javascript
// Jest describe('Math', () => { test('adds numbers', () => { expect(1 + 1).toBe(2);
});
});
// dx-test - identical!
describe('Math', () => { test('adds numbers', () => { expect(1 + 1).toBe(2);
});
});
```


### Configuration


+------------------+------------------+-------------+------------+
| Jest             | Config           | dx-test     | Equivalent |
+==================+==================+=============+============+
| `jest.config.js` | `dx.config.json` | (testRunner | section)   |
+------------------+------------------+-------------+------------+
```javascript
module.exports = { testMatch: ['**/*.test.js'], coverage: true, coverageThreshold: { global: { lines: 80 } }
};
```
dx.config.json:
```json
{ "testRunner": { "coverage": true, "timeout": 5000, "parallel": true }
}
```


### Assertions


All Jest assertions are supported:
```javascript
// Equality expect(value).toBe(expected);
expect(value).toEqual(expected);
expect(value).toStrictEqual(expected);
// Truthiness expect(value).toBeTruthy();
expect(value).toBeFalsy();
expect(value).toBeNull();
expect(value).toBeUndefined();
// Numbers expect(value).toBeGreaterThan(n);
expect(value).toBeLessThan(n);
expect(value).toBeCloseTo(n, precision);
// Strings expect(str).toMatch(/pattern/);
expect(str).toContain('substring');
// Arrays expect(arr).toContain(item);
expect(arr).toHaveLength(n);
// Objects expect(obj).toHaveProperty('key');
expect(obj).toMatchObject(partial);
// Exceptions expect(() => fn()).toThrow();
// Async await expect(promise).resolves.toBe(value);
await expect(promise).rejects.toThrow();
// Snapshots expect(value).toMatchSnapshot();
```


### Mocking


```javascript
// Mock functions const mockFn = jest.fn();
mockFn.mockReturnValue(42);
mockFn.mockImplementation(x => x * 2);
// Spies const spy = jest.spyOn(object, 'method');
// Module mocks jest.mock('./module');
// Timer mocks jest.useFakeTimers();
jest.advanceTimersByTime(1000);
jest.useRealTimers();
```


### Lifecycle Hooks


```javascript
describe('Suite', () => { beforeAll(() => { /* once before all */ });
afterAll(() => { /* once after all */ });
beforeEach(() => { /* before each test */ });
afterEach(() => { /* after each test */ });
test('test', () => { });
});
```


### Running Tests


+--------+-----------+
| Jest   | dx-test   |
+========+===========+
| `jest` | `dx-test` |
+--------+-----------+


## From webpack/esbuild to dx-bundle



### Basic Migration


+-----------------+-----------+
| webpack/esbuild | dx-bundle |
+=================+===========+
| Entry           | point     |
+-----------------+-----------+


### webpack.config.js to dx-bundle


webpack.config.js:
```javascript
module.exports = { entry: './src/index.ts', output: { filename: 'bundle.js', path: path.resolve(__dirname, 'dist'), }, mode: 'production', devtool: 'source-map', };
```
dx-bundle command:
```bash
dx-bundle bundle src/index.ts \
- output dist/bundle.js \
- minify \
- sourcemap
```


### esbuild to dx-bundle


esbuild:
```bash
esbuild src/index.ts --bundle --minify --sourcemap --outfile=dist/bundle.js ```
dx-bundle:
```bash
dx-bundle bundle src/index.ts -o dist/bundle.js --minify --sourcemap ```


### Output Formats


+--------+---------+-----------------------+-----------+
| Format | webpack | esbuild               | dx-bundle |
+========+=========+=======================+===========+
| ES     | Modules | `output.library.type: | 'module'` |
+--------+---------+-----------------------+-----------+


### External Packages


```bash

# Don't bundle react

dx-bundle bundle src/index.ts --external react --external react-dom ```

### Define Constants

```bash
dx-bundle bundle src/index.ts \
- define "process.env.NODE_ENV='production'" \
- define "DEBUG=false"
```

## Common Issues

### "Module not found" Errors

- Check installation: Run `dx list` to verify the package is installed
- Check import path: Ensure the path is correct
- Reinstall: Try `dx install` to refresh dependencies

### TypeScript Errors at Runtime

dx-js doesn't type-check. If you see type-related runtime errors: -Run `tsc --noEmit` to check types -Fix any TypeScript errors -Re-run with dx-js

### Memory Issues

If you see "JavaScript heap out of memory":
```bash


# Increase heap size


dx-js --max-heap-size=2048 script.js ```


### Unsupported Features


If you see "Unsupported feature" errors: -Check the Compatibility Matrix (./COMPATIBILITY.md) -Look for workarounds in the error message -Consider using a polyfill or alternative approach


### Test Failures After Migration


- Check async handling: Ensure async tests use `async/await` or return promises
- Check mocks: Verify mock implementations are correct
- Check snapshots: Run `dx-test
- u` to update snapshots


### Bundle Size Differences


dx-bundle may produce different bundle sizes than webpack/esbuild: -Tree shaking: dx-bundle has different tree-shaking behavior -Minification: Different minification algorithms -Dead code elimination: May differ in edge cases


### Performance Differences


If you notice performance differences: -JIT warmup: dx-js uses JIT compilation; first run may be slower -Memory usage: Check `process.memoryUsage()` for insights -Async operations: May have different scheduling behavior


## Getting Help


- Check the FAQ (./FAQ.md) for common questions
- Review the Compatibility Matrix (./COMPATIBILITY.md) for API support
- Report issues on GitHub with reproduction steps
- Join our Discord community for real-time help


## Rollback Plan


If migration issues arise, you can run both systems in parallel:
```json
{ "scripts": { "start:node": "node src/index.js", "start:dx": "dx-js src/index.js", "test:jest": "jest", "test:dx": "dx-test"
}
}
```
This allows gradual migration and easy comparison.
