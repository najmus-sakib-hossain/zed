// JSON parsing/serialization throughput benchmark
// Requirements: 9.5

const ITERATIONS = 100000;

// Sample data structure
const sampleData = {
  id: 12345,
  name: 'Test User',
  email: 'test@example.com',
  active: true,
  roles: ['admin', 'user', 'moderator'],
  metadata: {
    created: '2024-01-01T00:00:00Z',
    updated: '2024-12-30T00:00:00Z',
    version: 42
  },
  scores: [98.5, 87.3, 92.1, 88.9, 95.0]
};

const jsonString = JSON.stringify(sampleData);

// Benchmark JSON.parse
const parseStart = performance.now();
for (let i = 0; i < ITERATIONS; i++) {
  JSON.parse(jsonString);
}
const parseEnd = performance.now();
const parseTime = parseEnd - parseStart;
const parseOps = ITERATIONS / (parseTime / 1000);

// Benchmark JSON.stringify
const stringifyStart = performance.now();
for (let i = 0; i < ITERATIONS; i++) {
  JSON.stringify(sampleData);
}
const stringifyEnd = performance.now();
const stringifyTime = stringifyEnd - stringifyStart;
const stringifyOps = ITERATIONS / (stringifyTime / 1000);

console.log(JSON.stringify({
  benchmark: 'json-throughput',
  iterations: ITERATIONS,
  parse: {
    totalMs: parseTime.toFixed(2),
    opsPerSec: Math.round(parseOps)
  },
  stringify: {
    totalMs: stringifyTime.toFixed(2),
    opsPerSec: Math.round(stringifyOps)
  }
}));
