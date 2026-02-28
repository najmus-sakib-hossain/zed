// Array operation tests
test('array creation', () => {
  expect([1, 2, 3].length).toBe(3);
});

test('array push', () => {
  const arr = [1, 2];
  arr.push(3);
  expect(arr.length).toBe(3);
});

test('array pop', () => {
  const arr = [1, 2, 3];
  arr.pop();
  expect(arr.length).toBe(2);
});

test('array map', () => {
  const arr = [1, 2, 3];
  const doubled = arr.map(x => x * 2);
  expect(doubled).toEqual([2, 4, 6]);
});

test('array filter', () => {
  const arr = [1, 2, 3, 4, 5];
  const evens = arr.filter(x => x % 2 === 0);
  expect(evens).toEqual([2, 4]);
});

test('array reduce', () => {
  const arr = [1, 2, 3, 4];
  const sum = arr.reduce((a, b) => a + b, 0);
  expect(sum).toBe(10);
});

test('array find', () => {
  const arr = [1, 2, 3, 4];
  const found = arr.find(x => x > 2);
  expect(found).toBe(3);
});

test('array includes', () => {
  expect([1, 2, 3].includes(2)).toBe(true);
});

test('array concat', () => {
  const arr1 = [1, 2];
  const arr2 = [3, 4];
  expect(arr1.concat(arr2)).toEqual([1, 2, 3, 4]);
});

test('array slice', () => {
  const arr = [1, 2, 3, 4, 5];
  expect(arr.slice(1, 3)).toEqual([2, 3]);
});
