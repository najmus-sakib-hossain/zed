/**
 * Snapshot tests - Part of large test suite (300 tests)
 * Tests snapshot functionality
 */

describe('Snapshot Tests', () => {
    describe('Object Snapshots', () => {
        for (let i = 0; i < 15; i++) {
            test(`object snapshot ${i}`, () => {
                const obj = {
                    id: i,
                    name: `item-${i}`,
                    value: i * 10,
                    nested: {
                        a: i,
                        b: i + 1,
                        c: { deep: i * 2 }
                    }
                };
                expect(obj).toMatchSnapshot();
            });
        }
    });

    describe('Array Snapshots', () => {
        for (let i = 0; i < 15; i++) {
            test(`array snapshot ${i}`, () => {
                const arr = Array.from({ length: i + 1 }, (_, j) => ({
                    index: j,
                    value: j * i,
                    label: `item-${j}`
                }));
                expect(arr).toMatchSnapshot();
            });
        }
    });

    describe('String Snapshots', () => {
        for (let i = 0; i < 10; i++) {
            test(`string snapshot ${i}`, () => {
                const str = `Line 1: ${i}\nLine 2: ${i * 2}\nLine 3: ${i * 3}`;
                expect(str).toMatchSnapshot();
            });
        }
    });
});
