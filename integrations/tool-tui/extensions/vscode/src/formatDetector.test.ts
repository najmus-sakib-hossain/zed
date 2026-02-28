/**
 * Tests for Format Detector
 * 
 * Feature: dx-serializer-v3
 * 
 * Tests Property 1 from the design document:
 * - Property 1: Format Detection Accuracy
 * 
 * **Validates: Requirements 5.1-5.6**
 */

import * as fc from 'fast-check';
import {
    detectFormat,
    detectJson,
    detectYaml,
    detectToml,
    detectCsv,
    detectLlm,
    detectHumanV3,
    isSourceFormat,
    isDxFormat,
    DetectedFormat,
} from './formatDetector';

// ============================================================================
// Unit Tests
// ============================================================================

export function runUnitTests(): void {
    console.log('Running unit tests for Format Detector...\n');

    let passed = 0;
    let failed = 0;

    const tests: Array<{ name: string; test: () => boolean }> = [
        // JSON Detection (Requirement 5.1)
        {
            name: 'detectJson: detects JSON object',
            test: () => {
                const result = detectJson('{"name": "test", "version": "1.0"}');
                return result.confidence >= 0.8;
            }
        },
        {
            name: 'detectJson: detects JSON array',
            test: () => {
                const result = detectJson('[1, 2, 3]');
                return result.confidence >= 0.8;
            }
        },
        {
            name: 'detectJson: low confidence for invalid JSON',
            test: () => {
                const result = detectJson('{invalid json}');
                return result.confidence < 0.5;
            }
        },

        // YAML Detection (Requirement 5.2)
        {
            name: 'detectYaml: detects YAML with ---',
            test: () => {
                const result = detectYaml('---\nname: test\nversion: 1.0');
                return result.confidence >= 0.5;
            }
        },
        {
            name: 'detectYaml: detects YAML key: value',
            test: () => {
                const result = detectYaml('name: test\nversion: 1.0\nauthor: me');
                return result.confidence >= 0.3;
            }
        },
        {
            name: 'detectYaml: detects YAML list items',
            test: () => {
                // YAML with key: and list items should have good confidence
                const result = detectYaml('items:\n- one\n- two\n- three');
                return result.confidence >= 0.3;
            }
        },

        // TOML Detection (Requirement 5.3)
        {
            name: 'detectToml: detects TOML sections',
            test: () => {
                const result = detectToml('[package]\nname = "test"\nversion = "1.0"');
                return result.confidence >= 0.5;
            }
        },
        {
            name: 'detectToml: detects key = value pairs',
            test: () => {
                const result = detectToml('name = "test"\nversion = "1.0"\nauthor = "me"');
                return result.confidence >= 0.3;
            }
        },

        // CSV Detection (Requirement 5.4)
        {
            name: 'detectCsv: detects CSV with header',
            test: () => {
                const result = detectCsv('name,version,author\ntest,1.0,me\nother,2.0,you');
                return result.confidence >= 0.5;
            }
        },
        {
            name: 'detectCsv: low confidence for single line',
            test: () => {
                const result = detectCsv('name,version,author');
                return result.confidence < 0.3;
            }
        },

        // LLM Detection (Requirement 5.5)
        {
            name: 'detectLlm: detects #c: context marker (legacy)',
            test: () => {
                // Legacy format still supported
                const result = detectLlm('#c:nm|test;v|1.0');
                return result.confidence >= 0.4;
            }
        },
        {
            name: 'detectLlm: detects root-level key|value pairs (new format)',
            test: () => {
                // New format: root-level key|value pairs
                const result = detectLlm('nm|test\nv|1.0\nau|author');
                return result.confidence >= 0.3;
            }
        },
        {
            name: 'detectLlm: detects #f( section marker',
            test: () => {
                const result = detectLlm('#f(nm|repo|cont)\nforge|https://example.com|none');
                return result.confidence >= 0.5;
            }
        },
        {
            name: 'detectLlm: detects combined markers (legacy)',
            test: () => {
                const result = detectLlm('#c:nm|test\n#f(nm|repo)\nforge|url');
                return result.confidence >= 0.8;
            }
        },
        {
            name: 'detectLlm: detects combined markers (new format)',
            test: () => {
                const result = detectLlm('nm|test\n#f(nm|repo)\nforge|url');
                return result.confidence >= 0.5;
            }
        },

        // Human V3 Detection (Requirement 5.6)
        {
            name: 'detectHumanV3: detects padded key = value',
            test: () => {
                const result = detectHumanV3('name                 = dx\nversion              = 0.0.1');
                return result.confidence >= 0.4;
            }
        },
        {
            name: 'detectHumanV3: detects [section] headers',
            test: () => {
                const result = detectHumanV3('name                 = dx\n\n[forge]\nrepository           = url');
                return result.confidence >= 0.5;
            }
        },
        {
            name: 'detectHumanV3: detects pipe arrays',
            test: () => {
                const result = detectHumanV3('[stack]              = Lang | Runtime | Compiler\njavascript           = js | bun | tsc');
                return result.confidence >= 0.5;
            }
        },
        {
            name: 'detectHumanV3: low confidence for old format',
            test: () => {
                const result = detectHumanV3('# ════════════════════\n[config]\nname = test');
                return result.confidence < 0.3;
            }
        },

        // Main detectFormat function
        {
            name: 'detectFormat: returns json for JSON',
            test: () => {
                const result = detectFormat('{"name": "test"}');
                return result.format === 'json';
            }
        },
        {
            name: 'detectFormat: returns llm for LLM (legacy)',
            test: () => {
                const result = detectFormat('#c:nm|test\n#f(nm|repo)\nforge|url');
                return result.format === 'llm';
            }
        },
        {
            name: 'detectFormat: returns llm for LLM (new format)',
            test: () => {
                const result = detectFormat('nm|test\n#f(nm|repo)\nforge|url');
                return result.format === 'llm';
            }
        },
        {
            name: 'detectFormat: returns human-v3 for Human V3',
            test: () => {
                const result = detectFormat('name                 = dx\nversion              = 0.0.1');
                return result.format === 'human-v3';
            }
        },
        {
            name: 'detectFormat: returns unknown for empty',
            test: () => {
                const result = detectFormat('');
                return result.format === 'unknown';
            }
        },

        // Helper functions
        {
            name: 'isSourceFormat: true for json/yaml/toml/csv',
            test: () => {
                return isSourceFormat('json') && isSourceFormat('yaml') &&
                    isSourceFormat('toml') && isSourceFormat('csv');
            }
        },
        {
            name: 'isSourceFormat: false for llm/human-v3',
            test: () => {
                return !isSourceFormat('llm') && !isSourceFormat('human-v3');
            }
        },
        {
            name: 'isDxFormat: true for llm/human-v3',
            test: () => {
                return isDxFormat('llm') && isDxFormat('human-v3');
            }
        },
        {
            name: 'isDxFormat: false for source formats',
            test: () => {
                return !isDxFormat('json') && !isDxFormat('yaml');
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
 * Property 1: Format Detection Accuracy
 * For any valid format content, detection SHALL return the correct format
 * 
 * **Validates: Requirements 5.1-5.6**
 */
export function testFormatDetectionAccuracy(): void {
    // Test JSON detection
    fc.assert(
        fc.property(
            fc.record({
                name: fc.string({ minLength: 1, maxLength: 10 }),
                version: fc.string({ minLength: 1, maxLength: 10 }),
            }),
            (obj) => {
                const json = JSON.stringify(obj);
                const result = detectFormat(json);
                return result.format === 'json' && result.confidence >= 0.5;
            }
        ),
        { numRuns: 50 }
    );
    console.log('✓ Property 1a: JSON detection accuracy');

    // Test LLM detection
    fc.assert(
        fc.property(
            fc.tuple(
                fc.constantFrom('nm', 'v', 'tt', 'ds'),
                fc.string({ minLength: 1, maxLength: 10 })
            ),
            ([key, value]) => {
                const llm = `#c:${key}|${value}`;
                const result = detectFormat(llm);
                return result.format === 'llm' && result.confidence >= 0.3;
            }
        ),
        { numRuns: 50 }
    );
    console.log('✓ Property 1b: LLM detection accuracy');

    // Test Human V3 detection
    fc.assert(
        fc.property(
            fc.tuple(
                fc.constantFrom('name', 'version', 'title', 'author'),
                fc.string({ minLength: 1, maxLength: 10 })
            ),
            ([key, value]) => {
                const humanV3 = `${key.padEnd(20)} = ${value}\n${'version'.padEnd(20)} = 1.0.0`;
                const result = detectFormat(humanV3);
                return result.format === 'human-v3' && result.confidence >= 0.3;
            }
        ),
        { numRuns: 50 }
    );
    console.log('✓ Property 1c: Human V3 detection accuracy');
}

// ============================================================================
// Run All Tests
// ============================================================================

export function runAllPropertyTests(): void {
    console.log('Running Property tests for Format Detector...\n');

    testFormatDetectionAccuracy();

    console.log('\n✓ All Format Detector property tests passed!');
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
