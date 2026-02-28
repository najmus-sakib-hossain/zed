/**
 * Buffer Operations Benchmark
 * Tests Buffer performance: allocation, read/write, conversion
 */

const ITERATIONS = 100000;

// Benchmark: Buffer.alloc
function benchAlloc() {
    const start = performance.now();
    for (let i = 0; i < ITERATIONS; i++) {
        Buffer.alloc(1024);
    }
    return performance.now() - start;
}

// Benchmark: Buffer.allocUnsafe
function benchAllocUnsafe() {
    const start = performance.now();
    for (let i = 0; i < ITERATIONS; i++) {
        Buffer.allocUnsafe(1024);
    }
    return performance.now() - start;
}

// Benchmark: Buffer.from (string)
function benchFromString() {
    const testString = 'Hello World! This is a test string for buffer conversion.';

    const start = performance.now();
    for (let i = 0; i < ITERATIONS; i++) {
        Buffer.from(testString, 'utf8');
    }
    return performance.now() - start;
}

// Benchmark: Buffer.from (array)
function benchFromArray() {
    const testArray = Array.from({ length: 100 }, (_, i) => i % 256);

    const start = performance.now();
    for (let i = 0; i < ITERATIONS; i++) {
        Buffer.from(testArray);
    }
    return performance.now() - start;
}

// Benchmark: Buffer.toString
function benchToString() {
    const buffer = Buffer.from('Hello World! This is a test string for buffer conversion.');

    const start = performance.now();
    for (let i = 0; i < ITERATIONS; i++) {
        buffer.toString('utf8');
    }
    return performance.now() - start;
}

// Benchmark: Buffer.concat
function benchConcat() {
    const buffers = [
        Buffer.from('Hello '),
        Buffer.from('World '),
        Buffer.from('from '),
        Buffer.from('Buffer!')
    ];

    const start = performance.now();
    for (let i = 0; i < ITERATIONS; i++) {
        Buffer.concat(buffers);
    }
    return performance.now() - start;
}

// Benchmark: Buffer read/write operations
function benchReadWrite() {
    const buffer = Buffer.alloc(1024);

    const start = performance.now();
    for (let i = 0; i < ITERATIONS; i++) {
        buffer.writeUInt32LE(i, 0);
        buffer.writeUInt32BE(i, 4);
        buffer.writeFloatLE(i * 1.5, 8);
        buffer.readUInt32LE(0);
        buffer.readUInt32BE(4);
        buffer.readFloatLE(8);
    }
    return performance.now() - start;
}

// Benchmark: Buffer.copy
function benchCopy() {
    const source = Buffer.from('Hello World! This is source data for copy benchmark.');
    const target = Buffer.alloc(100);

    const start = performance.now();
    for (let i = 0; i < ITERATIONS; i++) {
        source.copy(target);
    }
    return performance.now() - start;
}

// Benchmark: Buffer.slice
function benchSlice() {
    const buffer = Buffer.from('Hello World! This is a test string for slice benchmark.');

    const start = performance.now();
    for (let i = 0; i < ITERATIONS; i++) {
        buffer.slice(0, 10);
        buffer.slice(10, 20);
        buffer.slice(20);
    }
    return performance.now() - start;
}

// Benchmark: Buffer.compare
function benchCompare() {
    const buf1 = Buffer.from('ABC');
    const buf2 = Buffer.from('ABD');
    const buf3 = Buffer.from('ABC');

    const start = performance.now();
    for (let i = 0; i < ITERATIONS; i++) {
        Buffer.compare(buf1, buf2);
        Buffer.compare(buf1, buf3);
        buf1.equals(buf3);
    }
    return performance.now() - start;
}

// Run benchmarks
const results = {
    alloc: benchAlloc(),
    allocUnsafe: benchAllocUnsafe(),
    fromString: benchFromString(),
    fromArray: benchFromArray(),
    toString: benchToString(),
    concat: benchConcat(),
    readWrite: benchReadWrite(),
    copy: benchCopy(),
    slice: benchSlice(),
    compare: benchCompare()
};

console.log(JSON.stringify(results));
