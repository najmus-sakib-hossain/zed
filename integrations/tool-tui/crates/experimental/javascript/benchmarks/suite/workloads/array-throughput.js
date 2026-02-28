// Array operations throughput benchmark
// Requirements: 9.5

const ARRAY_SIZE = 10000;
const ITERATIONS = 1000;

// Create test array
const createArray = () => Array.from({ length: ARRAY_SIZE }, (_, i) => ({
  id: i,
  value: Math.random(),
  name: `item-${i}`
}));

// Benchmark map
const mapStart = performance.now();
for (let i = 0; i < ITERATIONS; i++) {
  const arr = createArray();
  arr.map(x => x.value * 2);
}
const mapEnd = performance.now();
const mapTime = mapEnd - mapStart;

// Benchmark filter
const filterStart = performance.now();
for (let i = 0; i < ITERATIONS; i++) {
  const arr = createArray();
  arr.filter(x => x.value > 0.5);
}
const filterEnd = performance.now();
const filterTime = filterEnd - filterStart;

// Benchmark reduce
const reduceStart = performance.now();
for (let i = 0; i < ITERATIONS; i++) {
  const arr = createArray();
  arr.reduce((acc, x) => acc + x.value, 0);
}
const reduceEnd = performance.now();
const reduceTime = reduceEnd - reduceStart;

// Benchmark sort
const sortStart = performance.now();
for (let i = 0; i < ITERATIONS; i++) {
  const arr = createArray();
  arr.sort((a, b) => a.value - b.value);
}
const sortEnd = performance.now();
const sortTime = sortEnd - sortStart;

// Benchmark find
const findStart = performance.now();
for (let i = 0; i < ITERATIONS; i++) {
  const arr = createArray();
  arr.find(x => x.id === ARRAY_SIZE / 2);
}
const findEnd = performance.now();
const findTime = findEnd - findStart;

console.log(JSON.stringify({
  benchmark: 'array-throughput',
  arraySize: ARRAY_SIZE,
  iterations: ITERATIONS,
  map: { totalMs: mapTime.toFixed(2), opsPerSec: Math.round(ITERATIONS / (mapTime / 1000)) },
  filter: { totalMs: filterTime.toFixed(2), opsPerSec: Math.round(ITERATIONS / (filterTime / 1000)) },
  reduce: { totalMs: reduceTime.toFixed(2), opsPerSec: Math.round(ITERATIONS / (reduceTime / 1000)) },
  sort: { totalMs: sortTime.toFixed(2), opsPerSec: Math.round(ITERATIONS / (sortTime / 1000)) },
  find: { totalMs: findTime.toFixed(2), opsPerSec: Math.round(ITERATIONS / (findTime / 1000)) }
}));
