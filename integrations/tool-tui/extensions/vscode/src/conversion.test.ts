/**
 * Tests for LLM to Human format conversion
 * Run with: npx ts-node conversion.test.ts
 */

import { llmToHumanFallback } from './conversion';

// Simple test runner
let passed = 0;
let failed = 0;

function test(name: string, fn: () => void) {
    try {
        fn();
        console.log(`✓ ${name}`);
        passed++;
    } catch (e: any) {
        console.log(`✗ ${name}`);
        console.log(`  Error: ${e.message}`);
        failed++;
    }
}

function expect(actual: string) {
    return {
        toContain(expected: string) {
            if (!actual.includes(expected)) {
                throw new Error(`Expected "${actual.substring(0, 100)}..." to contain "${expected}"`);
            }
        },
        toEqual(expected: string) {
            if (actual !== expected) {
                throw new Error(`Expected "${expected}" but got "${actual}"`);
            }
        }
    };
}

console.log('\n=== LLM to Human Conversion Tests ===\n');

// Root Scalars
console.log('--- Root Scalars ---');

test('single scalar', () => {
    const input = 'name=dx-project';
    const output = llmToHumanFallback(input);
    expect(output).toContain('name');
    expect(output).toContain('dx-project');
});

test('multiple scalars on same line', () => {
    const input = 'name=dx-project version=1.2.3 author=essensefromexistence';
    const output = llmToHumanFallback(input);
    expect(output).toContain('name');
    expect(output).toContain('dx-project');
    expect(output).toContain('version');
    expect(output).toContain('1.2.3');
});

test('underscores converted to spaces', () => {
    const input = 'title=Hello_World';
    const output = llmToHumanFallback(input);
    expect(output).toContain('Hello World');
});

// Inline Objects
console.log('\n--- Inline Objects ---');

test('simple inline object', () => {
    const input = 'context:3[task=Hiking_trip location=Boulder season=spring]';
    const output = llmToHumanFallback(input);
    expect(output).toContain('[context]');
    expect(output).toContain('task');
    expect(output).toContain('Hiking trip');
    expect(output).toContain('Boulder');
});

test('inline object with nested array', () => {
    const input = 'workspace:2[paths[4]=@/www @/backend @/mobile @/shared default=@/www]';
    const output = llmToHumanFallback(input);
    expect(output).toContain('[workspace]');
    expect(output).toContain('paths');
    expect(output).toContain('@/www');
});

// Key-Value Pairs (@=)
console.log('\n--- Key-Value Pairs (@=) ---');

test('key-value pairs format', () => {
    const input = 'server:6@=[host localhost port 8080 debug true maxConn 100 timeout 30000 workers 4]';
    const output = llmToHumanFallback(input);
    expect(output).toContain('[server]');
    expect(output).toContain('host');
    expect(output).toContain('localhost');
    expect(output).toContain('port');
    expect(output).toContain('8080');
});

// Simple Arrays
console.log('\n--- Simple Arrays ---');

test('simple array with =', () => {
    const input = 'friends:8=ana luis sam maria carlos emma david sophia';
    const output = llmToHumanFallback(input);
    expect(output).toContain('[friends]');
    expect(output).toContain('- ana');
    expect(output).toContain('- luis');
});

// Tabular Data
console.log('\n--- Tabular Data with Suffix Expansion ---');

test('orders with @ORD- @2025-01- @@ex.com suffixes', () => {
    const input = 'orders:8(id customer email items total status date)@ORD- @2025-01- @@ex.com[001 Alice_Chen alice@ WIDGET:2:30|GADGET:1:50 110 shipped 10;002 Bob_Smith bob@ THING:3:15 45 delivered 11]';
    const output = llmToHumanFallback(input);
    console.log('  Output preview:', output.substring(0, 300));
    
    expect(output).toContain('[orders:1]');
    expect(output).toContain('[orders:2]');
    expect(output).toContain('ORD-001');
    expect(output).toContain('2025-01-10');
    expect(output).toContain('alice@ex.com');
    expect(output).toContain('Alice Chen');
});

test('hikes with @_Trail @4. suffixes', () => {
    const input = 'hikes:2(id name dist elev companion sunny rating)@_Trail @4.[1 Blue_Lake 7.5 320 ana true 5, 2 Ridge_View 9.2 540 luis false 2]';
    const output = llmToHumanFallback(input);
    console.log('  Output preview:', output.substring(0, 300));
    
    expect(output).toContain('[hikes:1]');
    expect(output).toContain('[hikes:2]');
    expect(output).toContain('Blue Lake Trail');
    expect(output).toContain('4.5');
    expect(output).toContain('Ridge View Trail');
    expect(output).toContain('4.2');
});

// Full Examples
console.log('\n--- Full File Examples ---');

test('example4_config.sr - key-value pairs format', () => {
    const input = `server:6@=[host localhost port 8080 debug true maxConn 100 timeout 30000 workers 4]
database:5@=[url postgres://localhost:5432/db pool 10 timeout 30 ssl true maxRetries 3]
cache:4@=[enabled true ttl 3600 maxSize 1000 strategy lru]`;
    
    const output = llmToHumanFallback(input);
    expect(output).toContain('[server]');
    expect(output).toContain('localhost');
    expect(output).toContain('[database]');
    expect(output).toContain('postgres://localhost:5432/db');
    expect(output).toContain('[cache]');
    expect(output).toContain('lru');
});

test('example5_leaf.sr - mixed format', () => {
    const input = `name=dx-project version=1.2.3 author=essensefromexistence
workspace:2[paths[4]=@/www @/backend @/mobile @/shared default=@/www]
editors:2[items[5]=neovim zed vscode cursor windsurf default=neovim]`;
    
    const output = llmToHumanFallback(input);
    expect(output).toContain('name');
    expect(output).toContain('dx-project');
    expect(output).toContain('[workspace]');
    expect(output).toContain('[editors]');
});

// Summary
console.log('\n=== Results ===');
console.log(`Passed: ${passed}`);
console.log(`Failed: ${failed}`);
console.log(`Total: ${passed + failed}`);

if (failed > 0) {
    process.exit(1);
}
