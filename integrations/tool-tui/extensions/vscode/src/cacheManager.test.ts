/**
 * Tests for Cache Manager Path Utilities
 * 
 * Feature: dx-serializer-v3
 * 
 * Tests Property 8 from the design document:
 * - Property 8: Cache Path Preservation
 * 
 * **Validates: Requirements 4.4**
 * 
 * Note: These are unit tests for path utilities only.
 * Full integration tests require VS Code extension host.
 */

import * as fc from 'fast-check';
import * as path from 'path';

// ============================================================================
// Inline Path Utilities (to avoid vscode import)
// ============================================================================

interface CacheConfig {
    cacheDir: string;
    humanExtension: string;
    machineExtension: string;
}

const DEFAULT_CACHE_CONFIG: CacheConfig = {
    cacheDir: '.dx/cache',
    humanExtension: '.human',
    machineExtension: '.machine',
};

function getRelativePath(filePath: string, workspaceRoot: string): string {
    return path.relative(workspaceRoot, filePath);
}

function getCacheDir(workspaceRoot: string, config: CacheConfig = DEFAULT_CACHE_CONFIG): string {
    return path.join(workspaceRoot, config.cacheDir);
}

function getCachePaths(
    filePath: string,
    workspaceRoot: string,
    config: CacheConfig = DEFAULT_CACHE_CONFIG
): { humanPath: string; machinePath: string } {
    const relativePath = getRelativePath(filePath, workspaceRoot);
    const baseName = path.basename(relativePath, path.extname(relativePath));
    const dirName = path.dirname(relativePath);

    const cacheDir = getCacheDir(workspaceRoot, config);
    const cacheSubDir = path.join(cacheDir, dirName);

    return {
        humanPath: path.join(cacheSubDir, baseName + config.humanExtension),
        machinePath: path.join(cacheSubDir, baseName + config.machineExtension),
    };
}

// ============================================================================
// Unit Tests
// ============================================================================

export function runUnitTests(): void {
    console.log('Running unit tests for Cache Manager...\n');

    let passed = 0;
    let failed = 0;

    const tests: Array<{ name: string; test: () => boolean }> = [
        // Path utilities
        {
            name: 'getRelativePath: returns relative path',
            test: () => {
                const result = getRelativePath('/workspace/src/file.dx', '/workspace');
                return result === 'src/file.dx' || result === 'src\\file.dx';
            }
        },
        {
            name: 'getCachePaths: generates correct human path',
            test: () => {
                const { humanPath } = getCachePaths('/workspace/src/config.dx', '/workspace');
                return humanPath.includes('.dx') &&
                    humanPath.includes('cache') &&
                    humanPath.endsWith('.human');
            }
        },
        {
            name: 'getCachePaths: generates correct machine path',
            test: () => {
                const { machinePath } = getCachePaths('/workspace/src/config.dx', '/workspace');
                return machinePath.includes('.dx') &&
                    machinePath.includes('cache') &&
                    machinePath.endsWith('.machine');
            }
        },
        {
            name: 'getCachePaths: preserves subdirectory structure',
            test: () => {
                const { humanPath } = getCachePaths('/workspace/src/nested/config.dx', '/workspace');
                return humanPath.includes('src') && humanPath.includes('nested');
            }
        },
        {
            name: 'getCachePaths: handles root level files',
            test: () => {
                const { humanPath } = getCachePaths('/workspace/config.dx', '/workspace');
                return humanPath.includes('.dx') &&
                    humanPath.includes('cache') &&
                    humanPath.includes('config.human');
            }
        },
        {
            name: 'DEFAULT_CACHE_CONFIG: has correct values',
            test: () => {
                return DEFAULT_CACHE_CONFIG.cacheDir === '.dx/cache' &&
                    DEFAULT_CACHE_CONFIG.humanExtension === '.human' &&
                    DEFAULT_CACHE_CONFIG.machineExtension === '.machine';
            }
        },
    ];

    for (const { name, test } of tests) {
        try {
            if (test()) {
                console.log(`  ✓ ${name}`);
                passed++;
            } else {
                console.log(`  ✗ ${name}`);
                failed++;
            }
        } catch (error) {
            console.log(`  ✗ ${name}: ${error}`);
            failed++;
        }
    }

    console.log(`\nUnit tests: ${passed} passed, ${failed} failed`);

    if (failed > 0) {
        throw new Error(`${failed} unit tests failed`);
    }
}

// ============================================================================
// Property Tests
// ============================================================================

/**
 * Property 8: Cache Path Preservation
 * Cache file paths SHALL preserve the subdirectory structure of source files
 * 
 * **Validates: Requirements 4.4**
 */
export function testCachePathPreservation(): void {
    fc.assert(
        fc.property(
            fc.array(
                fc.stringOf(fc.constantFrom(...'abcdefghijklmnopqrstuvwxyz'), { minLength: 1, maxLength: 10 }),
                { minLength: 1, maxLength: 3 }
            ),
            fc.stringOf(fc.constantFrom(...'abcdefghijklmnopqrstuvwxyz'), { minLength: 1, maxLength: 10 }),
            (pathParts, fileName) => {
                const workspaceRoot = '/workspace';
                const filePath = path.join(workspaceRoot, ...pathParts, fileName + '.dx');
                const { humanPath, machinePath } = getCachePaths(filePath, workspaceRoot);

                // Check that subdirectory structure is preserved
                for (const part of pathParts) {
                    if (!humanPath.includes(part) || !machinePath.includes(part)) {
                        return false;
                    }
                }

                // Check that file name is preserved (without extension)
                if (!humanPath.includes(fileName) || !machinePath.includes(fileName)) {
                    return false;
                }

                return true;
            }
        ),
        { numRuns: 50 }
    );
    console.log('✓ Property 8: Cache Path Preservation');
}

// ============================================================================
// Run All Tests
// ============================================================================

export function runAllPropertyTests(): void {
    console.log('Running Property tests for Cache Manager...\n');

    testCachePathPreservation();

    console.log('\n✓ All Cache Manager property tests passed!');
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
