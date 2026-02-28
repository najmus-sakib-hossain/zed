/**
 * Snapshot tests - Part of medium test suite (150 tests)
 * Tests snapshot functionality
 */

describe('Snapshot Tests', () => {
    describe('Object Snapshots', () => {
        for (let i = 0; i < 10; i++) {
            test(`object snapshot ${i}`, () => {
                const obj = {
                    id: i,
                    name: `item-${i}`,
                    value: i * 10,
                    nested: {
                        a: i,
                        b: i + 1
                    }
                };
                expect(obj).toMatchSnapshot();
            });
        }
    });

    describe('Array Snapshots', () => {
        for (let i = 0; i < 10; i++) {
            test(`array snapshot ${i}`, () => {
                const arr = Array.from({ length: i + 1 }, (_, j) => ({
                    index: j,
                    value: j * i
                }));
                expect(arr).toMatchSnapshot();
            });
        }
    });
});
