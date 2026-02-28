// Simple arithmetic tests
test('addition works', () => {
  expect(1 + 1).toBe(2);
});

test('subtraction works', () => {
  expect(5 - 3).toBe(2);
});

test('multiplication works', () => {
  expect(3 * 4).toBe(12);
});

test('division works', () => {
  expect(10 / 2).toBe(5);
});

test('negative numbers', () => {
  expect(-5 + 3).toBe(-2);
});

test('zero handling', () => {
  expect(0 + 0).toBe(0);
});

test('large numbers', () => {
  expect(1000000 + 1).toBe(1000001);
});

test('decimal arithmetic', () => {
  expect(0.1 + 0.2).toBeCloseTo(0.3);
});

test('negative results', () => {
  expect(5 - 10).toBe(-5);
});

test('order of operations', () => {
  expect(2 + 3 * 4).toBe(14);
});
