/**
 * Standalone property-based tests for DX Serializer utility functions
 * 
 * This file tests the path-based utility functions without VS Code dependencies.
 * 
 * Feature: dx-serializer-extension, Property 6: File type filtering correctness
 * **Validates: Requirements 5.1, 5.2, 5.3**
 */

import * as fc from 'fast-check';

// ============================================================================
// Standalone implementation of isExactlyDxPath (no vscode dependency)
// ============================================================================

/**
 * Check if a path string represents exactly a .dx file or a file named exactly "dx"
 */
function isExactlyDxPath(path: string): boolean {
    if (!path) {
        return false;
    }

    // Get the filename from the path
    const filename = path.split(/[/\\]/).pop() || '';

    // Check if filename is exactly "dx" (no extension, no prefix, no suffix)
    if (filename === 'dx') {
        return true;
    }

    // Must end with exactly .dx (case-sensitive)
    if (!filename.endsWith('.dx')) {
        return false;
    }

    // Must have a name before .dx (not just ".dx")
    const nameWithoutExt = filename.slice(0, -3);
    if (!nameWithoutExt || nameWithoutExt === '.' || nameWithoutExt.endsWith('.')) {
        return false;
    }

    return true;
}

// ============================================================================
// Generators for file paths
// ============================================================================

const validFilename = fc.stringOf(
    fc.constantFrom(...'abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789_-'),
    { minLength: 1, maxLength: 20 }
);

const validDirPath = fc.array(validFilename, { minLength: 0, maxLength: 3 })
    .map((parts: string[]) => parts.join('/'));

const pureDxPath = fc.tuple(validDirPath, validFilename)
    .map(([dir, name]: [string, string]) => dir ? `${dir}/${name}.dx` : `${name}.dx`);

// Generator for files named exactly "dx"
const exactDxFilename = fc.tuple(validDirPath)
    .map(([dir]: [string]) => dir ? `${dir}/dx` : 'dx');

const compoundExtension = fc.constantFrom(
    '.json', '.yml', '.yaml', '.bak', '.backup', '.old', '.tmp', '.txt'
);

const compoundDxPath = fc.tuple(validDirPath, validFilename, compoundExtension)
    .map(([dir, name, ext]: [string, string, string]) => dir ? `${dir}/${name}.dx${ext}` : `${name}.dx${ext}`);

const nonDxExtension = fc.constantFrom(
    '.json', '.yml', '.yaml', '.ts', '.js', '.txt', '.md', '.toml', '.xml'
);

const nonDxPath = fc.tuple(validDirPath, validFilename, nonDxExtension)
    .map(([dir, name, ext]: [string, string, string]) => dir ? `${dir}/${name}${ext}` : `${name}${ext}`);


// ============================================================================
// Property Tests
// ============================================================================

function testPureDxFilesIdentified(): void {
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
    console.log('✓ Property 6.1: Pure .dx files are identified');
}

function testExactDxFilenameIdentified(): void {
    fc.assert(
        fc.property(exactDxFilename, (path: string) => {
            const result = isExactlyDxPath(path);
            if (result !== true) {
                throw new Error(`Expected '${path}' (exact "dx" filename) to be identified as DX file`);
            }
            return true;
        }),
        { numRuns: 100 }
    );
    console.log('✓ Property 6.1b: Files named exactly "dx" are identified');
}

function testCompoundExtensionsRejected(): void {
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
    console.log('✓ Property 6.2: Compound extensions are rejected');
}

function testNonDxFilesRejected(): void {
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
    console.log('✓ Property 6.3: Non-.dx files are rejected');
}

function runAllPropertyTests(): void {
    console.log('Running Property 6: File type filtering correctness tests...\n');

    testPureDxFilesIdentified();
    testExactDxFilenameIdentified();
    testCompoundExtensionsRejected();
    testNonDxFilesRejected();

    console.log('\n✓ All Property 6 tests passed!');
}

// ============================================================================
// Unit Tests
// ============================================================================

function runUnitTests(): void {
    console.log('Running unit tests for isExactlyDxPath...\n');

    const tests: Array<{ path: string; expected: boolean; description: string }> = [
        // Should accept - .dx extension
        { path: 'config.dx', expected: true, description: 'simple .dx file' },
        { path: 'data.dx', expected: true, description: 'simple .dx file' },
        { path: 'my-file.dx', expected: true, description: '.dx file with hyphen' },
        { path: 'file_name.dx', expected: true, description: '.dx file with underscore' },
        { path: '/home/user/config.dx', expected: true, description: '.dx file with Unix path' },
        { path: 'C:\\Users\\config.dx', expected: true, description: '.dx file with Windows path' },
        { path: './relative/path/file.dx', expected: true, description: '.dx file with relative path' },
        { path: 'my.config.dx', expected: true, description: '.dx file with dots in name' },
        { path: 'app.v2.dx', expected: true, description: '.dx file with version in name' },

        // Should accept - files named exactly "dx"
        { path: 'dx', expected: true, description: 'file named exactly "dx"' },
        { path: '/home/user/dx', expected: true, description: '"dx" file with Unix path' },
        { path: 'C:\\Users\\dx', expected: true, description: '"dx" file with Windows path' },
        { path: './project/dx', expected: true, description: '"dx" file with relative path' },
        { path: 'workspace/frontend/dx', expected: true, description: '"dx" file in nested directory' },

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

        // Should reject - files with "dx" as part of name but not exactly "dx"
        { path: 'mydx', expected: false, description: 'file ending with dx but not exactly "dx"' },
        { path: 'dxconfig', expected: false, description: 'file starting with dx but not exactly "dx"' },
        { path: 'my-dx-file', expected: false, description: 'file containing dx but not exactly "dx"' },
        { path: 'dx.json', expected: false, description: '"dx" with extension' },
        { path: 'dx.yml', expected: false, description: '"dx" with yml extension' },

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

// Run tests
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
