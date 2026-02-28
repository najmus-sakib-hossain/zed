// Async operations throughput benchmark
// Requirements: 9.5

const ITERATIONS = 10000;

// Benchmark Promise.resolve chain
async function benchmarkPromiseResolve() {
  const start = performance.now();
  for (let i = 0; i < ITERATIONS; i++) {
    await Promise.resolve(i);
  }
  const end = performance.now();
  return end - start;
}

// Benchmark Promise.all
async function benchmarkPromiseAll() {
  const start = performance.now();
  for (let i = 0; i < ITERATIONS / 100; i++) {
    const promises = Array.from({ length: 100 }, (_, j) => Promise.resolve(j));
    await Promise.all(promises);
  }
  const end = performance.now();
  return end - start;
}

// Benchmark async/await overhead
async function benchmarkAsyncAwait() {
  async function asyncAdd(a, b) {
    return a + b;
  }
  
  const start = performance.now();
  let sum = 0;
  for (let i = 0; i < ITERATIONS; i++) {
    sum = await asyncAdd(sum, 1);
  }
  const end = performance.now();
  return end - start;
}

// Benchmark setTimeout(0) scheduling
async function benchmarkSetTimeout() {
  const start = performance.now();
  let count = 0;
  const target = 1000;
  
  await new Promise(resolve => {
    function tick() {
      count++;
      if (count < target) {
        setTimeout(tick, 0);
      } else {
        resolve();
      }
    }
    tick();
  });
  
  const end = performance.now();
  return end - start;
}

// Run all benchmarks
async function main() {
  const promiseResolveTime = await benchmarkPromiseResolve();
  const promiseAllTime = await benchmarkPromiseAll();
  const asyncAwaitTime = await benchmarkAsyncAwait();
  const setTimeoutTime = await benchmarkSetTimeout();
  
  console.log(JSON.stringify({
    benchmark: 'async-throughput',
    iterations: ITERATIONS,
    promiseResolve: {
      totalMs: promiseResolveTime.toFixed(2),
      opsPerSec: Math.round(ITERATIONS / (promiseResolveTime / 1000))
    },
    promiseAll: {
      totalMs: promiseAllTime.toFixed(2),
      opsPerSec: Math.round(ITERATIONS / (promiseAllTime / 1000))
    },
    asyncAwait: {
      totalMs: asyncAwaitTime.toFixed(2),
      opsPerSec: Math.round(ITERATIONS / (asyncAwaitTime / 1000))
    },
    setTimeout: {
      totalMs: setTimeoutTime.toFixed(2),
      opsPerSec: Math.round(1000 / (setTimeoutTime / 1000))
    }
  }));
}

main();
