/**
 * File System Benchmark
 * Tests fs module performance: readFile, writeFile, readdir, stat
 */

const fs = require('fs');
const path = require('path');
const os = require('os');

const ITERATIONS = 1000;
const TEMP_DIR = path.join(os.tmpdir(), 'dx-bench-fs');

// Ensure temp directory exists
if (!fs.existsSync(TEMP_DIR)) {
    fs.mkdirSync(TEMP_DIR, { recursive: true });
}

// Create test files
const testFiles = [];
for (let i = 0; i < 100; i++) {
    const filePath = path.join(TEMP_DIR, `test-file-${i}.txt`);
    fs.writeFileSync(filePath, `Test content for file ${i}\n`.repeat(100));
    testFiles.push(filePath);
}

// Benchmark: readFile (sync)
function benchReadFileSync() {
    const start = performance.now();
    for (let i = 0; i < ITERATIONS; i++) {
        const file = testFiles[i % testFiles.length];
        fs.readFileSync(file, 'utf8');
    }
    return performance.now() - start;
}

// Benchmark: readFile (async)
async function benchReadFileAsync() {
    const start = performance.now();
    const promises = [];
    for (let i = 0; i < ITERATIONS; i++) {
        const file = testFiles[i % testFiles.length];
        promises.push(fs.promises.readFile(file, 'utf8'));
    }
    await Promise.all(promises);
    return performance.now() - start;
}

// Benchmark: writeFile (sync)
function benchWriteFileSync() {
    const content = 'Benchmark write content\n'.repeat(50);
    const start = performance.now();
    for (let i = 0; i < ITERATIONS; i++) {
        const file = path.join(TEMP_DIR, `write-test-${i}.txt`);
        fs.writeFileSync(file, content);
    }
    return performance.now() - start;
}

// Benchmark: readdir
function benchReaddir() {
    const start = performance.now();
    for (let i = 0; i < ITERATIONS; i++) {
        fs.readdirSync(TEMP_DIR);
    }
    return performance.now() - start;
}

// Benchmark: stat
function benchStat() {
    const start = performance.now();
    for (let i = 0; i < ITERATIONS; i++) {
        const file = testFiles[i % testFiles.length];
        fs.statSync(file);
    }
    return performance.now() - start;
}

// Run benchmarks
async function main() {
    const results = {
        readFileSync: benchReadFileSync(),
        readFileAsync: await benchReadFileAsync(),
        writeFileSync: benchWriteFileSync(),
        readdir: benchReaddir(),
        stat: benchStat()
    };

    // Cleanup
    fs.rmSync(TEMP_DIR, { recursive: true, force: true });

    console.log(JSON.stringify(results));
}

main().catch(console.error);
