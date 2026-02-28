
# DX Test Runner

⚠️ Early Development (v0.0.1) - Many features are experimental. APIs may change. Parallel test runner with Jest-compatible API.

## Installation

```bash
cargo build --release -p dx-test-cli ```


## Usage


```bash

# Run all tests

dx-test

# Watch mode

dx-test --watch

# Verbose output

dx-test --verbose

# Filter by pattern

dx-test "math"

# With coverage

dx-test --coverage dx-test --coverage --coverage-threshold 80

# Update snapshots

dx-test --updateSnapshot

# Cache management

dx-test cache dx-test clear ```

## Test Format

```javascript
// tests/math.test.js describe('Math operations', () => { test('addition', () => { expect(1 + 1).toBe(2);
});
test('multiplication', () => { expect(2 * 3).toBe(6);
});
});
```

## Assertions

```javascript
// Equality expect(value).toBe(expected) // Strict equality (===)
expect(value).toEqual(expected) // Deep equality expect(value).not.toBe(unexpected) // Negation // Truthiness expect(value).toBeTruthy()
expect(value).toBeFalsy()
expect(value).toBeNull()
expect(value).toBeUndefined()
expect(value).toBeDefined()
// Numbers expect(value).toBeGreaterThan(n)
expect(value).toBeGreaterThanOrEqual(n)
expect(value).toBeLessThan(n)
expect(value).toBeLessThanOrEqual(n)
expect(value).toBeCloseTo(n, digits)
// Strings expect(string).toMatch(/pattern/)
expect(string).toContain(substring)
// Arrays/Objects expect(array).toContain(item)
expect(object).toHaveProperty(key)
expect(object).toHaveProperty(key, value)
// Exceptions expect(fn).toThrow()
expect(fn).toThrow(Error)
expect(fn).toThrow('message')
// Snapshots expect(value).toMatchSnapshot()
expect(value).toMatchInlineSnapshot()
```

## Mocking

```javascript
// Mock functions const mockFn = jest.fn();
mockFn.mockReturnValue(42);
mockFn.mockImplementation((x) => x * 2);
expect(mockFn).toHaveBeenCalled();
expect(mockFn).toHaveBeenCalledWith(arg);
expect(mockFn).toHaveBeenCalledTimes(n);
// Spying const spy = jest.spyOn(object, 'method');
spy.mockReturnValue(value);
// Module mocking jest.mock('./module', () => ({ fn: jest.fn(() => 'mocked'), }));
```

## Timer Mocks

```javascript
jest.useFakeTimers();
setTimeout(() => callback(), 1000);
jest.advanceTimersByTime(1000);
// or jest.runAllTimers();
jest.useRealTimers();
```

## Code Coverage

```bash


# Generate coverage report


dx-test --coverage


# With threshold


dx-test --coverage --coverage-threshold 80 ```
Coverage reports generated in: -`coverage/lcov-report/index.html` (HTML) -`coverage/lcov.info` (LCOV) -`coverage/coverage.json` (JSON)


## Snapshot Testing


```javascript
test('renders correctly', () => { const tree = render(<Component />);
expect(tree).toMatchSnapshot();
});
```
Snapshots stored in `__snapshots__/` directory. Update snapshots:
```bash
dx-test --updateSnapshot ```

## Configuration

Create `dx-test.config.js`:
```javascript
module.exports = { testMatch: ['**/*.test.js', '**/*.spec.js'], testPathIgnorePatterns: ['/node_modules/'], coverageThreshold: { global: { branches: 80, functions: 80, lines: 80, }, }, };
```

## Architecture

- Parallel execution across all CPU cores
- Work-stealing scheduler
- Binary test format for fast loading
- Memory-mapped cache

## License

MIT OR Apache-2.0
