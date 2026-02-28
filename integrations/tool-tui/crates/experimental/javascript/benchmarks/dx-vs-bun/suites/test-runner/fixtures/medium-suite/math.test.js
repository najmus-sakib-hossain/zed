/**
 * Math operations test suite - Part of medium test suite (150 tests)
 * Tests arithmetic operations
 */

describe('Math Operations', () => {
    describe('Addition', () => {
        for (let i = 0; i < 15; i++) {
            test(`addition ${i}: ${i} + ${i + 1} = ${i + i + 1}`, () => {
                expect(i + (i + 1)).toBe(i + i + 1);
            });
        }
    });

    describe('Subtraction', () => {
        for (let i = 0; i < 15; i++) {
            test(`subtraction ${i}: ${i * 3} - ${i} = ${i * 2}`, () => {
                expect(i * 3 - i).toBe(i * 2);
            });
        }
    });

    describe('Multiplication', () => {
        for (let i = 1; i <= 15; i++) {
            test(`multiplication ${i}: ${i} * ${i} = ${i * i}`, () => {
                expect(i * i).toBe(i * i);
            });
        }
    });

    describe('Division', () => {
        for (let i = 1; i <= 15; i++) {
            test(`division ${i}: ${i * 2} / 2 = ${i}`, () => {
                expect((i * 2) / 2).toBe(i);
            });
        }
    });
});
