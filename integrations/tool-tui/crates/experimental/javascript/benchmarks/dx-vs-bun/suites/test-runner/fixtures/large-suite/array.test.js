/**
 * Array operations test suite - Part of large test suite (300 tests)
 * Tests array manipulation
 */

describe('Array Operations', () => {
    describe('Push', () => {
        for (let i = 0; i < 20; i++) {
            test(`push ${i}: adds element`, () => {
                const arr = [1, 2, 3];
                arr.push(i);
                expect(arr.length).toBe(4);
                expect(arr[3]).toBe(i);
            });
        }
    });

    describe('Pop', () => {
        for (let i = 0; i < 20; i++) {
            test(`pop ${i}: removes last element`, () => {
                const arr = [1, 2, 3, i];
                const popped = arr.pop();
                expect(popped).toBe(i);
                expect(arr.length).toBe(3);
            });
        }
    });

    describe('Map', () => {
        for (let i = 1; i <= 20; i++) {
            test(`map ${i}: transforms elements`, () => {
                const arr = [1, 2, 3];
                const mapped = arr.map(x => x * i);
                expect(mapped).toEqual([i, 2 * i, 3 * i]);
            });
        }
    });

    describe('Filter', () => {
        for (let i = 0; i < 20; i++) {
            test(`filter ${i}: filters elements`, () => {
                const arr = [1, 2, 3, 4, 5];
                const filtered = arr.filter(x => x > i % 5);
                expect(filtered.every(x => x > i % 5)).toBe(true);
            });
        }
    });

    describe('Reduce', () => {
        for (let i = 0; i < 20; i++) {
            test(`reduce ${i}: accumulates values`, () => {
                const arr = [1, 2, 3, 4, 5];
                const sum = arr.reduce((a, b) => a + b, i);
                expect(sum).toBe(15 + i);
            });
        }
    });
});
