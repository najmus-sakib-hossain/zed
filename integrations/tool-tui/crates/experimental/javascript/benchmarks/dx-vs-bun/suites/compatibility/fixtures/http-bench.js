/**
 * HTTP Module Benchmark
 * Tests http module performance: server creation, request handling
 */

const http = require('http');

const PORT = 9876 + Math.floor(Math.random() * 1000);
const REQUESTS = 1000;

// Benchmark: Server creation time
function benchServerCreation() {
    const times = [];

    for (let i = 0; i < 100; i++) {
        const start = performance.now();
        const server = http.createServer((req, res) => {
            res.writeHead(200);
            res.end('OK');
        });
        times.push(performance.now() - start);
        server.close();
    }

    return times.reduce((a, b) => a + b, 0);
}

// Benchmark: Request handling throughput
async function benchRequestHandling() {
    return new Promise((resolve) => {
        const server = http.createServer((req, res) => {
            res.writeHead(200, { 'Content-Type': 'text/plain' });
            res.end('Hello World');
        });

        server.listen(PORT, async () => {
            const start = performance.now();
            let completed = 0;

            const makeRequest = () => {
                return new Promise((res, rej) => {
                    const req = http.request({
                        hostname: 'localhost',
                        port: PORT,
                        path: '/',
                        method: 'GET'
                    }, (response) => {
                        let data = '';
                        response.on('data', chunk => data += chunk);
                        response.on('end', () => res(data));
                    });
                    req.on('error', rej);
                    req.end();
                });
            };

            // Run requests in batches
            const batchSize = 50;
            for (let i = 0; i < REQUESTS; i += batchSize) {
                const batch = [];
                for (let j = 0; j < batchSize && i + j < REQUESTS; j++) {
                    batch.push(makeRequest());
                }
                await Promise.all(batch);
                completed += batch.length;
            }

            const elapsed = performance.now() - start;
            server.close();

            resolve({
                totalTime: elapsed,
                requestsPerSecond: (REQUESTS / elapsed) * 1000
            });
        });
    });
}

// Benchmark: JSON response handling
async function benchJsonResponse() {
    return new Promise((resolve) => {
        const jsonData = JSON.stringify({
            users: Array.from({ length: 100 }, (_, i) => ({
                id: i,
                name: `User ${i}`,
                email: `user${i}@example.com`
            }))
        });

        const server = http.createServer((req, res) => {
            res.writeHead(200, { 'Content-Type': 'application/json' });
            res.end(jsonData);
        });

        server.listen(PORT + 1, async () => {
            const start = performance.now();

            const makeRequest = () => {
                return new Promise((res, rej) => {
                    const req = http.request({
                        hostname: 'localhost',
                        port: PORT + 1,
                        path: '/',
                        method: 'GET'
                    }, (response) => {
                        let data = '';
                        response.on('data', chunk => data += chunk);
                        response.on('end', () => res(JSON.parse(data)));
                    });
                    req.on('error', rej);
                    req.end();
                });
            };

            const requests = 500;
            for (let i = 0; i < requests; i += 25) {
                const batch = [];
                for (let j = 0; j < 25 && i + j < requests; j++) {
                    batch.push(makeRequest());
                }
                await Promise.all(batch);
            }

            const elapsed = performance.now() - start;
            server.close();

            resolve({
                totalTime: elapsed,
                requestsPerSecond: (requests / elapsed) * 1000
            });
        });
    });
}

// Run benchmarks
async function main() {
    const results = {
        serverCreation: benchServerCreation(),
        requestHandling: await benchRequestHandling(),
        jsonResponse: await benchJsonResponse()
    };

    console.log(JSON.stringify(results));
}

main().catch(console.error);
