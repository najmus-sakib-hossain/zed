/**
 * Tests for fresh project
 */

const { main } = require('../src/index');
const { add, multiply } = require('../src/utils');

describe('Main', () => {
    test('main returns doubled array', () => {
        const result = main();
        expect(result).toEqual([2, 4, 6, 8, 10]);
    });
});

describe('Utils', () => {
    test('add works correctly', () => {
        expect(add(2, 3)).toBe(5);
    });

    test('multiply works correctly', () => {
        expect(multiply(2, 3)).toBe(6);
    });
});
