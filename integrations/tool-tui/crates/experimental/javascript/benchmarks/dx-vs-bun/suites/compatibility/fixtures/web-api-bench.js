/**
 * Web API Benchmark
 * Tests Web API performance: fetch, TextEncoder/Decoder, URL
 */

const ITERATIONS = 10000;

// Benchmark: TextEncoder
function benchTextEncoder() {
    const encoder = new TextEncoder();
    const testStrings = [
        'Hello World!',
        'The quick brown fox jumps over the lazy dog.',
        'Unicode: 你好世界 🌍 مرحبا',
        'Lorem ipsum dolor sit amet, consectetur adipiscing elit.'
    ];

    const start = performance.now();
    for (let i = 0; i < ITERATIONS; i++) {
        encoder.encode(testStrings[i % testStrings.length]);
    }
    return performance.now() - start;
}

// Benchmark: TextDecoder
function benchTextDecoder() {
    const encoder = new TextEncoder();
    const decoder = new TextDecoder();
    const testBuffers = [
        encoder.encode('Hello World!'),
        encoder.encode('The quick brown fox jumps over the lazy dog.'),
        encoder.encode('Unicode: 你好世界 🌍 مرحبا'),
        encoder.encode('Lorem ipsum dolor sit amet, consectetur adipiscing elit.')
    ];

    const start = performance.now();
    for (let i = 0; i < ITERATIONS; i++) {
        decoder.decode(testBuffers[i % testBuffers.length]);
    }
    return performance.now() - start;
}

// Benchmark: URL parsing
function benchURLParse() {
    const testUrls = [
        'https://example.com/path/to/resource?query=value&foo=bar#section',
        'http://user:pass@localhost:8080/api/v1/users',
        'https://subdomain.domain.tld/path?a=1&b=2&c=3',
        'file:///home/user/documents/file.txt'
    ];

    const start = performance.now();
    for (let i = 0; i < ITERATIONS; i++) {
        new URL(testUrls[i % testUrls.length]);
    }
    return performance.now() - start;
}

// Benchmark: URL manipulation
function benchURLManipulation() {
    const baseUrl = new URL('https://example.com/path');

    const start = performance.now();
    for (let i = 0; i < ITERATIONS; i++) {
        const url = new URL(baseUrl);
        url.searchParams.set('page', String(i));
        url.searchParams.set('limit', '10');
        url.pathname = `/api/v${i % 3}/resource`;
        url.toString();
    }
    return performance.now() - start;
}

// Benchmark: URLSearchParams
function benchURLSearchParams() {
    const start = performance.now();
    for (let i = 0; i < ITERATIONS; i++) {
        const params = new URLSearchParams();
        params.set('key1', 'value1');
        params.set('key2', 'value2');
        params.append('key3', 'value3');
        params.get('key1');
        params.has('key2');
        params.toString();
    }
    return performance.now() - start;
}

// Benchmark: atob/btoa (Base64)
function benchBase64() {
    const testStrings = [
        'Hello World!',
        'The quick brown fox jumps over the lazy dog.',
        'Base64 encoding and decoding benchmark test string.'
    ];

    const start = performance.now();
    for (let i = 0; i < ITERATIONS; i++) {
        const str = testStrings[i % testStrings.length];
        const encoded = btoa(str);
        atob(encoded);
    }
    return performance.now() - start;
}

// Benchmark: JSON (Web API style)
function benchJSON() {
    const testObjects = [
        { name: 'John', age: 30, city: 'New York' },
        { users: [1, 2, 3, 4, 5], active: true },
        { nested: { deep: { value: 'test' } } }
    ];

    const start = performance.now();
    for (let i = 0; i < ITERATIONS; i++) {
        const obj = testObjects[i % testObjects.length];
        const str = JSON.stringify(obj);
        JSON.parse(str);
    }
    return performance.now() - start;
}

// Benchmark: Blob (if available)
function benchBlob() {
    if (typeof Blob === 'undefined') {
        return -1; // Not available
    }

    const testData = 'Hello World! '.repeat(100);

    const start = performance.now();
    for (let i = 0; i < ITERATIONS / 10; i++) {
        const blob = new Blob([testData], { type: 'text/plain' });
        blob.size;
        blob.type;
    }
    return performance.now() - start;
}

// Run benchmarks
const results = {
    textEncoder: benchTextEncoder(),
    textDecoder: benchTextDecoder(),
    urlParse: benchURLParse(),
    urlManipulation: benchURLManipulation(),
    urlSearchParams: benchURLSearchParams(),
    base64: benchBase64(),
    json: benchJSON(),
    blob: benchBlob()
};

console.log(JSON.stringify(results));
