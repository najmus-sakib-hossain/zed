/**
 * CPU-intensive benchmark: Fibonacci calculation
 * Tests raw JavaScript execution speed
 */

function fibonacci(n) {
    if (n <= 1) return n;
    return fibonacci(n - 1) + fibonacci(n - 2);
}

// Calculate fibonacci(35) - takes measurable time without being too slow
const start = Date.now();
const result = fibonacci(35);
const elapsed = Date.now() - start;

console.log(`fibonacci(35) = ${result}`);
console.log(`Time: ${elapsed}ms`);
