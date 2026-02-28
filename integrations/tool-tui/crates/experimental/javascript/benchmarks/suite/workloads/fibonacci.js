// CPU-intensive benchmark - Fibonacci calculation
// Requirements: 9.5

function fibonacci(n) {
  if (n <= 1) return n;
  return fibonacci(n - 1) + fibonacci(n - 2);
}

const N = 35;
const ITERATIONS = 5;

const times = [];
for (let i = 0; i < ITERATIONS; i++) {
  const start = performance.now();
  const result = fibonacci(N);
  const end = performance.now();
  times.push(end - start);
}

// Calculate statistics
times.sort((a, b) => a - b);
const min = times[0];
const max = times[times.length - 1];
const median = times[Math.floor(times.length / 2)];
const mean = times.reduce((a, b) => a + b, 0) / times.length;
const variance = times.reduce((acc, t) => acc + Math.pow(t - mean, 2), 0) / times.length;
const stdDev = Math.sqrt(variance);

console.log(JSON.stringify({
  benchmark: 'fibonacci',
  n: N,
  iterations: ITERATIONS,
  result: fibonacci(N),
  stats: {
    minMs: min.toFixed(2),
    maxMs: max.toFixed(2),
    medianMs: median.toFixed(2),
    meanMs: mean.toFixed(2),
    stdDevMs: stdDev.toFixed(2)
  }
}));
