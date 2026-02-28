// Boolean and truthiness tests
test('true is truthy', () => {
  expect(true).toBeTruthy();
});

test('false is falsy', () => {
  expect(false).toBeFalsy();
});

test('null is falsy', () => {
  expect(null).toBeFalsy();
});

test('undefined is falsy', () => {
  expect(undefined).toBeFalsy();
});

test('0 is falsy', () => {
  expect(0).toBeFalsy();
});

test('empty string is falsy', () => {
  expect('').toBeFalsy();
});

test('non-zero number is truthy', () => {
  expect(1).toBeTruthy();
});

test('non-empty string is truthy', () => {
  expect('test').toBeTruthy();
});

test('array is truthy', () => {
  expect([]).toBeTruthy();
});

test('object is truthy', () => {
  expect({}).toBeTruthy();
});
