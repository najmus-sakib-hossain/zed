/**
 * Tests for dev iteration project
 */

const { processData, validateInput } = require('../src/utils');

describe('Utils', () => {
    test('processData adds processed flag', () => {
        const input = [{ id: 1 }, { id: 2 }];
        const result = processData(input);
        expect(result[0].processed).toBe(true);
        expect(result[1].processed).toBe(true);
    });

    test('validateInput throws on invalid input', () => {
        expect(() => validateInput(null)).toThrow('Invalid input');
    });

    test('validateInput returns true for valid input', () => {
        expect(validateInput({ data: 'test' })).toBe(true);
    });
});
