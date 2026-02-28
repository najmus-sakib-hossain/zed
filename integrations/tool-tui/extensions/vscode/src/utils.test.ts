/**
 * Property-based tests for DX Serializer utility functions
 * 
 * Feature: dx-serializer-extension-fix, Property 7: File Extension Filtering
 * 
 * For any file path:
 * - Paths ending exactly with `.dx` SHALL be identified as DX files
 * - Paths with compound extensions (`.dx.json`, `.dx.yml`, `.dx.bak`, etc.) SHALL NOT be identified as DX files
 * - Paths with non-file schemes SHALL NOT be identified as DX files
 * 
 * **Validates: Requirements 4.1-4.7**
 */

import * as fc from 'fast-check';
import { isExactlyDxPath } from './utils';

// Generators for file paths

/**
 * Generate a valid filename (alphanumeric with some allowed chars)
 */
const validFilename = fc.stringOf(
    fc.constantFrom(...'abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789_-'),
    { minLength: 1, maxLength: 20 }
);

/**
 * Generate a valid directory path
 */
const validDirPath = fc.array(validFilename, { minLength: 0, maxLength: 3 })
    .map((parts: string[]) => parts.join('/'));

/**
 * Generate a pure .dx file path
 */
const pureDxPath = fc.tuple(validDirPath, validFilename)
    .map(([dir, name]: [string, string]) => dir ? `${dir}/${name}.dx` : `${name}.dx`);

/**
 * Generate a compound extension (something after .dx)
 */
const compoundExtension = fc.constantFrom(
    '.json', '.yml', '.yaml', '.bak', '.backup', '.old', '.tmp', '.txt'
);

/**
 * Generate a path with compound extension (.dx.something)
 */
const compoundDxPath = fc.tuple(validDirPath, validFilename, compoundExtension)
    .map(([dir, name, ext]: [string, string, string]) => dir ? `${dir}/${name}.dx${ext}` : `${name}.dx${ext}`);

/**
 * Generate a non-.dx file path
 */
const nonDxExtension = fc.constantFrom(
    '.json', '.yml', '.yaml', '.ts', '.js', '.txt', '.md', '.toml', '.xml'
);

const nonDxPath = fc.tuple(validDirPath, validFilename, nonDxExtension)
    .map(([dir, name, ext]: [string, string, string]) => dir ? `${dir}/${name}${ext}` : `${name}${ext}`);

// Feature: dx-serializer-extension-fix, Property 7: File Extension Filtering
// **Validates: Requirements 4.1-4.7**

/**
 * Property 7.1: Pure .dx files are identified
 * For any valid filename, appending .dx should result in a recognized DX file
 * **Validates: Requirements 4.1**
 */
export function testPureDxFilesIdentified(): void {
    fc.assert(
        fc.property(pureDxPath, (path: string) => {
            const result = isExactlyDxPath(path);
            if (result !== true) {
                throw new Error(`Expected '${path}' to be identified as DX file`);
            }
            return true;
        }),
        { numRuns: 100 }
    );
    console.log('✓ Property 7.1: Pure .dx files are identified');
}


/**
 * Property 7.2: Compound extensions are rejected
 * For any .dx file with an additional extension, it should be rejected
 * **Validates: Requirements 4.2-4.7**
 */
export function testCompoundExtensionsRejected(): void {
    fc.assert(
        fc.property(compoundDxPath, (path: string) => {
            const result = isExactlyDxPath(path);
            if (result !== false) {
                throw new Error(`Expected '${path}' to be rejected (compound extension)`);
            }
            return true;
        }),
        { numRuns: 100 }
    );
    console.log('✓ Property 7.2: Compound extensions are rejected');
}

/**
 * Property 7.3: Non-.dx files are rejected
 * For any file without .dx extension, it should be rejected
 * **Validates: Requirements 4.1**
 */
export function testNonDxFilesRejected(): void {
    fc.assert(
        fc.property(nonDxPath, (path: string) => {
            const result = isExactlyDxPath(path);
            if (result !== false) {
                throw new Error(`Expected '${path}' to be rejected (non-.dx file)`);
            }
            return true;
        }),
        { numRuns: 100 }
    );
    console.log('✓ Property 7.3: Non-.dx files are rejected');
}

/**
 * Run all property tests
 */
export function runAllPropertyTests(): void {
    console.log('Running Property 7: File Extension Filtering tests...\n');

    testPureDxFilesIdentified();
    testCompoundExtensionsRejected();
    testNonDxFilesRejected();

    console.log('\n✓ All Property 7 tests passed!');
}

// Unit tests for specific examples
export function runUnitTests(): void {
    console.log('Running unit tests for isExactlyDxPath...\n');

    const tests: Array<{ path: string; expected: boolean; description: string }> = [
        // Should accept
        { path: 'config.dx', expected: true, description: 'simple .dx file' },
        { path: 'data.dx', expected: true, description: 'simple .dx file' },
        { path: 'my-file.dx', expected: true, description: '.dx file with hyphen' },
        { path: 'file_name.dx', expected: true, description: '.dx file with underscore' },
        { path: '/home/user/config.dx', expected: true, description: '.dx file with Unix path' },
        { path: 'C:\\Users\\config.dx', expected: true, description: '.dx file with Windows path' },
        { path: './relative/path/file.dx', expected: true, description: '.dx file with relative path' },
        { path: 'my.config.dx', expected: true, description: '.dx file with dots in name' },
        { path: 'app.v2.dx', expected: true, description: '.dx file with version in name' },

        // Should reject - compound extensions
        { path: 'config.dx.json', expected: false, description: 'compound .dx.json' },
        { path: 'config.dx.yml', expected: false, description: 'compound .dx.yml' },
        { path: 'config.dx.yaml', expected: false, description: 'compound .dx.yaml' },
        { path: 'config.dx.bak', expected: false, description: 'compound .dx.bak' },
        { path: 'config.dx.backup', expected: false, description: 'compound .dx.backup' },
        { path: 'config.dx.old', expected: false, description: 'compound .dx.old' },
        { path: 'config.dx.tmp', expected: false, description: 'compound .dx.tmp' },

        // Should reject - non-.dx files
        { path: 'config.json', expected: false, description: 'JSON file' },
        { path: 'config.yml', expected: false, description: 'YAML file' },
        { path: 'config.toml', expected: false, description: 'TOML file' },
        { path: 'script.ts', expected: false, description: 'TypeScript file' },
        { path: 'readme.md', expected: false, description: 'Markdown file' },

        // Should reject - edge cases
        { path: '', expected: false, description: 'empty string' },
        { path: '.dx', expected: false, description: 'just .dx' },
        { path: '..dx', expected: false, description: 'double dot .dx' },
        { path: '/path/to/.dx', expected: false, description: 'hidden .dx file' },
        { path: 'my.dx.config', expected: false, description: '.dx in middle' },
        { path: 'config', expected: false, description: 'no extension' },
    ];

    let passed = 0;
    let failed = 0;

    for (const test of tests) {
        const result = isExactlyDxPath(test.path);
        if (result === test.expected) {
            console.log(`  ✓ ${test.description}: '${test.path}' -> ${result}`);
            passed++;
        } else {
            console.log(`  ✗ ${test.description}: '${test.path}' -> ${result} (expected ${test.expected})`);
            failed++;
        }
    }

    console.log(`\nUnit tests: ${passed} passed, ${failed} failed`);

    if (failed > 0) {
        throw new Error(`${failed} unit tests failed`);
    }
}

// Run tests if this file is executed directly
if (require.main === module) {
    try {
        runUnitTests();
        console.log('');
        runAllPropertyTests();
    } catch (error) {
        console.error('Tests failed:', error);
        process.exit(1);
    }
}
