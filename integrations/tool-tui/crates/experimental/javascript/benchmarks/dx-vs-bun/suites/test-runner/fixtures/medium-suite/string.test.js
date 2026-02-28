/**
 * String operations test suite - Part of medium test suite (150 tests)
 * Tests string manipulation
 */

describe('String Operations', () => {
    describe('Concatenation', () => {
        for (let i = 0; i < 10; i++) {
            test(`concat ${i}: joins strings`, () => {
                expect(`a${i}` + `b${i}`).toBe(`a${i}b${i}`);
            });
        }
    });

    describe('Length', () => {
        for (let i = 1; i <= 10; i++) {
            test(`length ${i}: correct length`, () => {
                expect('x'.repeat(i).length).toBe(i);
            });
        }
    });

    describe('Uppercase', () => {
        for (let i = 0; i < 10; i++) {
            test(`uppercase ${i}: converts correctly`, () => {
                expect(`test${i}`.toUpperCase()).toBe(`TEST${i}`);
            });
        }
    });

    describe('Lowercase', () => {
        for (let i = 0; i < 10; i++) {
            test(`lowercase ${i}: converts correctly`, () => {
                expect(`TEST${i}`.toLowerCase()).toBe(`test${i}`);
            });
        }
    });

    describe('Trim', () => {
        for (let i = 0; i < 10; i++) {
            test(`trim ${i}: removes whitespace`, () => {
                expect(`  test${i}  `.trim()).toBe(`test${i}`);
            });
        }
    });
});
