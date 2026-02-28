/**
 * Async/await concurrency benchmark
 * Tests Promise handling and async execution
 */

async function delay(ms) {
    return new Promise(resolve => setTimeout(resolve, ms));
}

async function asyncTask(id) {
    await delay(1);
    return id * 2;
}

async function runConcurrent(count) {
    const promises = [];
    for (let i = 0; i < count; i++) {
        promises.push(asyncTask(i));
    }
    return Promise.all(promises);
}

async function main() {
    const start = Date.now();

    // Run 1000 concurrent async tasks
    const results = await runConcurrent(1000);

    const elapsed = Date.now() - start;

    console.log(`Concurrent async tasks: 1000`);
    console.log(`Results sum: ${results.reduce((a, b) => a + b, 0)}`);
    console.log(`Time: ${elapsed}ms`);
}

main();
