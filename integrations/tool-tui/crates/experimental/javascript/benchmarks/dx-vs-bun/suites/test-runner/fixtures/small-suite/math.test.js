/**
 * Math operations test suite - Part of small test suite (50 tests)
 * Tests basic arithmetic operations
 */

describe('Math Operations', () => {
    describe('Addition', () => {
        for (let i = 0; i < 10; i++) {
            test(`addition case ${i}: ${i} + ${i} = ${i * 2}`, () => {
                expect(i + i).toBe(i * 2);
            });
        }
    });

    describe('Subtraction', () => {
        for (let i = 0; i < 10; i++) {
            test(`subtraction case ${i}: ${i * 2} - ${i} = ${i}`, () => {
                expect(i * 2 - i).toBe(i);
            });
        }
    });

    describe('Multiplication', () => {
        for (let i = 1; i <= 10; i++) {
            test(`multiplication case ${i}: ${i} * ${i} = ${i * i}`, () => {
                expect(i * i).toBe(i * i);
            });
        }
    });
});
