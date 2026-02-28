/**
 * EventEmitter Benchmark
 * Tests EventEmitter performance: emit, on, off operations
 */

const EventEmitter = require('events');

const ITERATIONS = 100000;

// Benchmark: emit with single listener
function benchEmitSingle() {
    const emitter = new EventEmitter();
    let count = 0;

    emitter.on('event', (data) => {
        count += data;
    });

    const start = performance.now();
    for (let i = 0; i < ITERATIONS; i++) {
        emitter.emit('event', 1);
    }
    return performance.now() - start;
}

// Benchmark: emit with multiple listeners
function benchEmitMultiple() {
    const emitter = new EventEmitter();
    emitter.setMaxListeners(100);
    let count = 0;

    // Add 10 listeners
    for (let i = 0; i < 10; i++) {
        emitter.on('event', (data) => {
            count += data;
        });
    }

    const start = performance.now();
    for (let i = 0; i < ITERATIONS; i++) {
        emitter.emit('event', 1);
    }
    return performance.now() - start;
}

// Benchmark: on/off (add/remove listeners)
function benchOnOff() {
    const emitter = new EventEmitter();
    emitter.setMaxListeners(ITERATIONS);

    const listeners = [];
    for (let i = 0; i < 1000; i++) {
        listeners.push(() => { });
    }

    const start = performance.now();
    for (let i = 0; i < 10000; i++) {
        const listener = listeners[i % listeners.length];
        emitter.on('event', listener);
        emitter.off('event', listener);
    }
    return performance.now() - start;
}

// Benchmark: once
function benchOnce() {
    const emitter = new EventEmitter();
    emitter.setMaxListeners(ITERATIONS);

    const start = performance.now();
    for (let i = 0; i < 10000; i++) {
        emitter.once('event', () => { });
        emitter.emit('event');
    }
    return performance.now() - start;
}

// Benchmark: emit with data
function benchEmitWithData() {
    const emitter = new EventEmitter();
    let result = null;

    emitter.on('data', (a, b, c) => {
        result = { a, b, c };
    });

    const start = performance.now();
    for (let i = 0; i < ITERATIONS; i++) {
        emitter.emit('data', i, 'string', { nested: true });
    }
    return performance.now() - start;
}

// Benchmark: listenerCount
function benchListenerCount() {
    const emitter = new EventEmitter();
    emitter.setMaxListeners(100);

    for (let i = 0; i < 50; i++) {
        emitter.on('event', () => { });
    }

    const start = performance.now();
    for (let i = 0; i < ITERATIONS; i++) {
        emitter.listenerCount('event');
    }
    return performance.now() - start;
}

// Run benchmarks
const results = {
    emitSingle: benchEmitSingle(),
    emitMultiple: benchEmitMultiple(),
    onOff: benchOnOff(),
    once: benchOnce(),
    emitWithData: benchEmitWithData(),
    listenerCount: benchListenerCount()
};

console.log(JSON.stringify(results));
