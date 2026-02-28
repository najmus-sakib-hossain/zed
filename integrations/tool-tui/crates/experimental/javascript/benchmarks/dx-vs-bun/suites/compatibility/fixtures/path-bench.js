/**
 * Path Module Benchmark
 * Tests path module performance: join, resolve, parse
 */

const path = require('path');

const ITERATIONS = 100000;

// Test data
const segments = ['home', 'user', 'documents', 'projects', 'my-app', 'src', 'components'];
const absolutePaths = [
    '/home/user/documents',
    '/var/log/app',
    '/usr/local/bin',
    'C:\\Users\\Admin\\Documents',
    'D:\\Projects\\MyApp\\src'
];
const relativePaths = [
    '../parent/child',
    './sibling',
    '../../grandparent',
    'child/grandchild'
];

// Benchmark: path.join
function benchJoin() {
    const start = performance.now();
    for (let i = 0; i < ITERATIONS; i++) {
        path.join(...segments);
        path.join('base', segments[i % segments.length], 'file.js');
        path.join('/root', 'sub', 'deep', 'file.txt');
    }
    return performance.now() - start;
}

// Benchmark: path.resolve
function benchResolve() {
    const start = performance.now();
    for (let i = 0; i < ITERATIONS; i++) {
        path.resolve(...segments);
        path.resolve(absolutePaths[i % absolutePaths.length], relativePaths[i % relativePaths.length]);
        path.resolve('/base', '../sibling', './child');
    }
    return performance.now() - start;
}

// Benchmark: path.parse
function benchParse() {
    const testPaths = [
        '/home/user/file.txt',
        '/var/log/app.log',
        'C:\\Users\\Admin\\document.pdf',
        './relative/path/script.js',
        '../parent/config.json'
    ];

    const start = performance.now();
    for (let i = 0; i < ITERATIONS; i++) {
        path.parse(testPaths[i % testPaths.length]);
    }
    return performance.now() - start;
}

// Benchmark: path.basename
function benchBasename() {
    const testPaths = [
        '/home/user/file.txt',
        '/var/log/app.log',
        'C:\\Users\\Admin\\document.pdf'
    ];

    const start = performance.now();
    for (let i = 0; i < ITERATIONS; i++) {
        path.basename(testPaths[i % testPaths.length]);
        path.basename(testPaths[i % testPaths.length], '.txt');
    }
    return performance.now() - start;
}

// Benchmark: path.dirname
function benchDirname() {
    const testPaths = [
        '/home/user/file.txt',
        '/var/log/app.log',
        'relative/path/file.js'
    ];

    const start = performance.now();
    for (let i = 0; i < ITERATIONS; i++) {
        path.dirname(testPaths[i % testPaths.length]);
    }
    return performance.now() - start;
}

// Benchmark: path.normalize
function benchNormalize() {
    const testPaths = [
        '/home/user/../admin/./documents',
        'a/b/../c/./d',
        '/foo/bar//baz/asdf/quux/..'
    ];

    const start = performance.now();
    for (let i = 0; i < ITERATIONS; i++) {
        path.normalize(testPaths[i % testPaths.length]);
    }
    return performance.now() - start;
}

// Run benchmarks
const results = {
    join: benchJoin(),
    resolve: benchResolve(),
    parse: benchParse(),
    basename: benchBasename(),
    dirname: benchDirname(),
    normalize: benchNormalize()
};

console.log(JSON.stringify(results));
