
# Test Suites

Benchmark test suites for DX Test Runner.

## Test Files

+----------------+-------+-------------+
| File           | Tests | Description |
+================+=======+=============+
| `math.test.js` | 10    | Arithmetic  |
+----------------+-------+-------------+



## Running

```bash


# DX Test Runner


./target/release/dx-test


# With verbose output


./target/release/dx-test --verbose


# Show cache stats


./target/release/dx-test cache ```


## Adding Tests


Create a file matching `*.test.js`:
```javascript
test('description', () => { expect(actual).toBe(expected);
});
```
