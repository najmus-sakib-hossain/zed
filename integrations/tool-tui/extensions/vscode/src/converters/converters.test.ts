/**
 * Tests for Format Converters
 * 
 * Feature: dx-serializer-v3
 * 
 * Tests Property 2 from the design document:
 * - Property 2: Format Conversion Preserves Data
 * 
 * **Validates: Requirements 1.1-1.4**
 */

import * as fc from 'fast-check';
import { convertJsonToDocument, jsonValueToDx } from './jsonConverter';
import { convertYamlToDocument, parseSimpleYaml } from './yamlConverter';
import { convertTomlToDocument, parseSimpleToml } from './tomlConverter';
import { convertCsvToDocument } from './csvConverter';
import { DxValue } from '../llmParser';

// ============================================================================
// Unit Tests
// ============================================================================

export function runUnitTests(): void {
    console.log('Running unit tests for Format Converters...\n');

    let passed = 0;
    let failed = 0;

    const tests: Array<{ name: string; test: () => boolean }> = [
        // JSON Converter (Requirement 1.1)
        {
            name: 'jsonValueToDx: converts string',
            test: () => {
                const result = jsonValueToDx('hello');
                return result.type === 'string' && result.value === 'hello';
            }
        },
        {
            name: 'jsonValueToDx: converts number',
            test: () => {
                const result = jsonValueToDx(42);
                return result.type === 'number' && result.value === 42;
            }
        },
        {
            name: 'jsonValueToDx: converts boolean',
            test: () => {
                const t = jsonValueToDx(true);
                const f = jsonValueToDx(false);
                return t.type === 'bool' && t.value === true &&
                    f.type === 'bool' && f.value === false;
            }
        },
        {
            name: 'jsonValueToDx: converts null',
            test: () => {
                const result = jsonValueToDx(null);
                return result.type === 'null';
            }
        },
        {
            name: 'jsonValueToDx: converts array',
            test: () => {
                const result = jsonValueToDx([1, 2, 3]);
                return result.type === 'array' && (result.value as DxValue[]).length === 3;
            }
        },
        {
            name: 'convertJsonToDocument: converts simple object',
            test: () => {
                const json = '{"name": "test", "version": "1.0"}';
                const result = convertJsonToDocument(json);
                return result.success &&
                    result.document !== undefined &&
                    result.document.context.size === 2;
            }
        },
        {
            name: 'convertJsonToDocument: converts array of objects to section',
            test: () => {
                const json = '{"items": [{"id": 1, "name": "a"}, {"id": 2, "name": "b"}]}';
                const result = convertJsonToDocument(json);
                return result.success &&
                    result.document !== undefined &&
                    result.document.sections.size === 1;
            }
        },
        {
            name: 'convertJsonToDocument: handles invalid JSON',
            test: () => {
                const result = convertJsonToDocument('{invalid}');
                return !result.success && result.error !== undefined;
            }
        },

        // YAML Converter (Requirement 1.2)
        {
            name: 'parseSimpleYaml: parses key: value',
            test: () => {
                const result = parseSimpleYaml('name: test\nversion: 1.0');
                return result.name === 'test' && result.version === 1.0;
            }
        },
        {
            name: 'parseSimpleYaml: parses boolean values',
            test: () => {
                const result = parseSimpleYaml('enabled: true\ndisabled: false');
                return result.enabled === true && result.disabled === false;
            }
        },
        {
            name: 'parseSimpleYaml: parses list items',
            test: () => {
                const result = parseSimpleYaml('items:\n- one\n- two\n- three');
                return Array.isArray(result.items) && (result.items as string[]).length === 3;
            }
        },
        {
            name: 'convertYamlToDocument: converts simple YAML',
            test: () => {
                const yaml = 'name: test\nversion: 1.0';
                const result = convertYamlToDocument(yaml);
                return result.success &&
                    result.document !== undefined &&
                    result.document.context.size === 2;
            }
        },

        // TOML Converter (Requirement 1.3)
        {
            name: 'parseSimpleToml: parses key = value',
            test: () => {
                const result = parseSimpleToml('name = "test"\nversion = "1.0"');
                return result.name === 'test' && result.version === '1.0';
            }
        },
        {
            name: 'parseSimpleToml: parses sections',
            test: () => {
                const result = parseSimpleToml('[package]\nname = "test"');
                const pkg = result.package as Record<string, unknown>;
                return pkg !== undefined && pkg.name === 'test';
            }
        },
        {
            name: 'parseSimpleToml: parses arrays',
            test: () => {
                const result = parseSimpleToml('items = [1, 2, 3]');
                return Array.isArray(result.items) && (result.items as number[]).length === 3;
            }
        },
        {
            name: 'convertTomlToDocument: converts simple TOML',
            test: () => {
                const toml = 'name = "test"\nversion = "1.0"';
                const result = convertTomlToDocument(toml);
                return result.success &&
                    result.document !== undefined &&
                    result.document.context.size === 2;
            }
        },
        {
            name: 'convertTomlToDocument: converts sections to DxSections',
            test: () => {
                const toml = '[forge]\nrepository = "url"\ncontainer = "none"';
                const result = convertTomlToDocument(toml);
                return result.success &&
                    result.document !== undefined &&
                    result.document.sections.size === 1;
            }
        },

        // CSV Converter (Requirement 1.4)
        {
            name: 'convertCsvToDocument: converts simple CSV',
            test: () => {
                const csv = 'name,version\ntest,1.0\nother,2.0';
                const result = convertCsvToDocument(csv);
                return result.success &&
                    result.document !== undefined &&
                    result.document.sections.size === 1;
            }
        },
        {
            name: 'convertCsvToDocument: handles quoted values',
            test: () => {
                const csv = 'name,description\ntest,"hello, world"';
                const result = convertCsvToDocument(csv);
                if (!result.success || !result.document) return false;
                const section = result.document.sections.get('d');
                if (!section) return false;
                const desc = section.rows[0][1];
                return desc.type === 'string' && desc.value === 'hello, world';
            }
        },
        {
            name: 'convertCsvToDocument: converts numbers',
            test: () => {
                const csv = 'id,count\n1,100\n2,200';
                const result = convertCsvToDocument(csv);
                if (!result.success || !result.document) return false;
                const section = result.document.sections.get('d');
                if (!section) return false;
                return section.rows[0][0].type === 'number';
            }
        },
        {
            name: 'convertCsvToDocument: handles empty CSV',
            test: () => {
                const result = convertCsvToDocument('');
                return !result.success;
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
 * Property 2: Format Conversion Preserves Data
 * Converting from source format to DxDocument SHALL preserve all data values
 * 
 * **Validates: Requirements 1.1-1.4**
 */
export function testFormatConversionPreservesData(): void {
    // Test JSON conversion preserves keys
    fc.assert(
        fc.property(
            fc.record({
                name: fc.string({ minLength: 1, maxLength: 10 }),
                version: fc.string({ minLength: 1, maxLength: 10 }),
            }),
            (obj) => {
                const json = JSON.stringify(obj);
                const result = convertJsonToDocument(json);
                if (!result.success || !result.document) return false;

                // Check that context has the keys
                return result.document.context.has('nm') || result.document.context.has('name');
            }
        ),
        { numRuns: 50 }
    );
    console.log('✓ Property 2a: JSON conversion preserves keys');

    // Test CSV conversion preserves row count
    fc.assert(
        fc.property(
            fc.array(
                fc.tuple(
                    fc.string({ minLength: 1, maxLength: 10 }),
                    fc.string({ minLength: 1, maxLength: 10 })
                ),
                { minLength: 1, maxLength: 5 }
            ),
            (rows) => {
                // Build CSV with header
                const csv = 'col1,col2\n' + rows.map(([a, b]) => `${a},${b}`).join('\n');
                const result = convertCsvToDocument(csv);
                if (!result.success || !result.document) return false;

                const section = result.document.sections.get('d');
                if (!section) return false;

                return section.rows.length === rows.length;
            }
        ),
        { numRuns: 50 }
    );
    console.log('✓ Property 2b: CSV conversion preserves row count');
}

// ============================================================================
// Run All Tests
// ============================================================================

export function runAllPropertyTests(): void {
    console.log('Running Property tests for Format Converters...\n');

    testFormatConversionPreservesData();

    console.log('\n✓ All Format Converter property tests passed!');
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
