// String manipulation tests
test('string concatenation', () => {
  expect('hello' + ' ' + 'world').toBe('hello world');
});

test('string length', () => {
  expect('test'.length).toBe(4);
});

test('empty string', () => {
  expect('').toBe('');
});

test('string comparison', () => {
  expect('abc').toBe('abc');
});

test('string includes', () => {
  expect('hello world'.includes('world')).toBe(true);
});

test('string uppercase', () => {
  expect('test'.toUpperCase()).toBe('TEST');
});

test('string lowercase', () => {
  expect('TEST'.toLowerCase()).toBe('test');
});

test('string slice', () => {
  expect('hello'.slice(0, 2)).toBe('he');
});

test('string trim', () => {
  expect('  test  '.trim()).toBe('test');
});

test('string repeat', () => {
  expect('a'.repeat(3)).toBe('aaa');
});
