/**
 * JSON parsing/serialization benchmark
 * Tests JSON operations performance
 */

// Generate a large JSON structure
const data = JSON.stringify(Array.from({ length: 10000 }, (_, i) => ({
    id: i,
    name: `Item ${i}`,
    value: Math.random(),
    nested: {
        a: i * 2,
        b: `nested-${i}`,
        c: [1, 2, 3, 4, 5]
    }
})));

const start = Date.now();

// Parse and stringify 100 times
for (let i = 0; i < 100; i++) {
    const parsed = JSON.parse(data);
    JSON.stringify(parsed);
}

const elapsed = Date.now() - start;

console.log(`JSON parse/stringify x100`);
console.log(`Data size: ${(data.length / 1024).toFixed(1)}KB`);
console.log(`Time: ${elapsed}ms`);
