// Object tests
test('object creation', () => {
  const obj = { a: 1, b: 2 };
  expect(obj.a).toBe(1);
});

test('object property access', () => {
  const obj = { name: 'test' };
  expect(obj.name).toBe('test');
});

test('object keys', () => {
  const obj = { a: 1, b: 2 };
  expect(Object.keys(obj)).toEqual(['a', 'b']);
});

test('object values', () => {
  const obj = { a: 1, b: 2 };
  expect(Object.values(obj)).toEqual([1, 2]);
});

test('object assign', () => {
  const obj1 = { a: 1 };
  const obj2 = { b: 2 };
  const merged = Object.assign({}, obj1, obj2);
  expect(merged).toEqual({ a: 1, b: 2 });
});

test('object spread', () => {
  const obj1 = { a: 1 };
  const obj2 = { b: 2 };
  const merged = { ...obj1, ...obj2 };
  expect(merged).toEqual({ a: 1, b: 2 });
});

test('object delete', () => {
  const obj = { a: 1, b: 2 };
  delete obj.a;
  expect(obj.a).toBeUndefined();
});

test('object has property', () => {
  const obj = { a: 1 };
  expect(obj.hasOwnProperty('a')).toBe(true);
});

test('object nested access', () => {
  const obj = { a: { b: { c: 1 } } };
  expect(obj.a.b.c).toBe(1);
});

test('object equality', () => {
  const obj1 = { a: 1 };
  const obj2 = { a: 1 };
  expect(obj1).toEqual(obj2);
});
