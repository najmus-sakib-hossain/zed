// Memory baseline benchmark
// Requirements: 9.4

// Force GC if available (V8/Bun)
if (typeof gc === 'function') {
  gc();
}

// Get memory usage
function getMemoryUsage() {
  if (typeof process !== 'undefined' && process.memoryUsage) {
    const mem = process.memoryUsage();
    return {
      heapUsed: mem.heapUsed,
      heapTotal: mem.heapTotal,
      rss: mem.rss,
      external: mem.external || 0
    };
  }
  // Fallback for environments without process.memoryUsage
  return {
    heapUsed: 0,
    heapTotal: 0,
    rss: 0,
    external: 0
  };
}

const baseline = getMemoryUsage();

// Allocate some objects to measure memory growth
const objects = [];
for (let i = 0; i < 10000; i++) {
  objects.push({
    id: i,
    data: 'x'.repeat(100),
    nested: { a: 1, b: 2, c: 3 }
  });
}

const afterAllocation = getMemoryUsage();

// Clear references
objects.length = 0;

// Force GC if available
if (typeof gc === 'function') {
  gc();
}

const afterGC = getMemoryUsage();

console.log(JSON.stringify({
  benchmark: 'memory-baseline',
  baseline: {
    heapUsedMB: (baseline.heapUsed / 1024 / 1024).toFixed(2),
    heapTotalMB: (baseline.heapTotal / 1024 / 1024).toFixed(2),
    rssMB: (baseline.rss / 1024 / 1024).toFixed(2)
  },
  afterAllocation: {
    heapUsedMB: (afterAllocation.heapUsed / 1024 / 1024).toFixed(2),
    heapTotalMB: (afterAllocation.heapTotal / 1024 / 1024).toFixed(2),
    rssMB: (afterAllocation.rss / 1024 / 1024).toFixed(2)
  },
  afterGC: {
    heapUsedMB: (afterGC.heapUsed / 1024 / 1024).toFixed(2),
    heapTotalMB: (afterGC.heapTotal / 1024 / 1024).toFixed(2),
    rssMB: (afterGC.rss / 1024 / 1024).toFixed(2)
  },
  growth: {
    heapUsedMB: ((afterAllocation.heapUsed - baseline.heapUsed) / 1024 / 1024).toFixed(2),
    rssMB: ((afterAllocation.rss - baseline.rss) / 1024 / 1024).toFixed(2)
  }
}));
