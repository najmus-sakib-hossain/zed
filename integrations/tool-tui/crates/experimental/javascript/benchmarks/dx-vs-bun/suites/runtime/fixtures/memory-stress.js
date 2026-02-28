/**
 * Memory-intensive benchmark
 * Tests memory allocation and garbage collection
 */

function createLargeArray(size) {
    return Array.from({ length: size }, (_, i) => ({
        id: i,
        data: `item-${i}-${'x'.repeat(100)}`,
        nested: { a: i, b: i * 2, c: i * 3 }
    }));
}

const start = Date.now();

// Allocate and process large arrays
let totalItems = 0;
for (let i = 0; i < 50; i++) {
    const arr = createLargeArray(10000);
    totalItems += arr.length;

    // Process the array to prevent optimization
    const sum = arr.reduce((acc, item) => acc + item.id, 0);
}

const elapsed = Date.now() - start;

// Report memory usage if available
let memoryInfo = 'N/A';
if (typeof process !== 'undefined' && process.memoryUsage) {
    const mem = process.memoryUsage();
    memoryInfo = `${(mem.heapUsed / 1024 / 1024).toFixed(1)}MB`;
}

console.log(`Memory stress test`);
console.log(`Total items processed: ${totalItems}`);
console.log(`Heap used: ${memoryInfo}`);
console.log(`Time: ${elapsed}ms`);
