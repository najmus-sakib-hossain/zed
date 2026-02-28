/**
 * String operations test suite - Part of small test suite (50 tests)
 * Tests string manipulation operations
 */

describe('String Operations', () => {
    describe('Concatenation', () => {
        for (let i = 0; i < 5; i++) {
            test(`concat case ${i}: joins strings correctly`, () => {
                const a = `hello${i}`;
                const b = `world${i}`;
                expect(a + b).toBe(`hello${i}world${i}`);
            });
        }
    });

    describe('Length', () => {
        for (let i = 1; i <= 5; i++) {
            test(`length case ${i}: string of ${i} chars has length ${i}`, () => {
                const str = 'x'.repeat(i);
                expect(str.length).toBe(i);
            });
        }
    });

    describe('Uppercase', () => {
        const words = ['hello', 'world', 'test', 'bench', 'mark'];
        words.forEach((word, i) => {
            test(`uppercase case ${i}: ${word} -> ${word.toUpperCase()}`, () => {
                expect(word.toUpperCase()).toBe(word.toUpperCase());
            });
        });
    });

    describe('Includes', () => {
        for (let i = 0; i < 5; i++) {
            test(`includes case ${i}: finds substring`, () => {
                const str = `test${i}string`;
                expect(str.includes(`${i}`)).toBe(true);
            });
        }
    });
});
