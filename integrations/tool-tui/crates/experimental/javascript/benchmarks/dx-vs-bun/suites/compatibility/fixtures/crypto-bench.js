/**
 * Crypto Module Benchmark
 * Tests crypto module performance: hash, randomBytes, randomUUID
 */

const crypto = require('crypto');

const ITERATIONS = 10000;

// Test data
const testData = 'The quick brown fox jumps over the lazy dog'.repeat(100);
const testBuffer = Buffer.from(testData);

// Benchmark: SHA-256 hash
function benchSha256() {
    const start = performance.now();
    for (let i = 0; i < ITERATIONS; i++) {
        crypto.createHash('sha256').update(testData).digest('hex');
    }
    return performance.now() - start;
}

// Benchmark: SHA-512 hash
function benchSha512() {
    const start = performance.now();
    for (let i = 0; i < ITERATIONS; i++) {
        crypto.createHash('sha512').update(testData).digest('hex');
    }
    return performance.now() - start;
}

// Benchmark: MD5 hash
function benchMd5() {
    const start = performance.now();
    for (let i = 0; i < ITERATIONS; i++) {
        crypto.createHash('md5').update(testData).digest('hex');
    }
    return performance.now() - start;
}

// Benchmark: randomBytes
function benchRandomBytes() {
    const start = performance.now();
    for (let i = 0; i < ITERATIONS; i++) {
        crypto.randomBytes(32);
    }
    return performance.now() - start;
}

// Benchmark: randomUUID
function benchRandomUUID() {
    const start = performance.now();
    for (let i = 0; i < ITERATIONS; i++) {
        crypto.randomUUID();
    }
    return performance.now() - start;
}

// Benchmark: HMAC
function benchHmac() {
    const key = crypto.randomBytes(32);
    const start = performance.now();
    for (let i = 0; i < ITERATIONS; i++) {
        crypto.createHmac('sha256', key).update(testData).digest('hex');
    }
    return performance.now() - start;
}

// Benchmark: pbkdf2 (fewer iterations due to computational cost)
function benchPbkdf2() {
    const password = 'password123';
    const salt = crypto.randomBytes(16);
    const start = performance.now();
    for (let i = 0; i < 100; i++) {
        crypto.pbkdf2Sync(password, salt, 1000, 32, 'sha256');
    }
    return performance.now() - start;
}

// Run benchmarks
const results = {
    sha256: benchSha256(),
    sha512: benchSha512(),
    md5: benchMd5(),
    randomBytes: benchRandomBytes(),
    randomUUID: benchRandomUUID(),
    hmac: benchHmac(),
    pbkdf2: benchPbkdf2()
};

console.log(JSON.stringify(results));
